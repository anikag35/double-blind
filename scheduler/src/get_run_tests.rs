use crate::pb::task_result::Outcome;
use crate::pb::{CriterionScore, Mode, RubricResult, RunId, RunRequest, TaskResult, WorkerId};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file, TASK_QUEUE_TEST_LOCK};
use tonic::Code;

#[tokio::test]
async fn get_run_reports_not_done_until_every_task_is_done() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file(
        "{\"prompt\": \"p1\"}\n{\"prompt\": \"p2\"}\n",
        "get_run_not_done",
    );
    let run_id = client
        .submit_run(RunRequest {
            models: vec!["claude".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Rubric as i32,
            compare: false,
        })
        .await
        .expect("submit_run should succeed")
        .into_inner()
        .run_id;

    // Nothing claimed/reported yet
    let leaderboard = client
        .get_run(RunId { run_id: run_id.clone() })
        .await
        .expect("get_run should succeed")
        .into_inner();
    assert!(!leaderboard.all_tasks_done);
    assert!(leaderboard.entries.is_empty());

    // Claim and finish only one of the two tasks
    let task = client
        .get_task(WorkerId { worker_id: "worker-1".into() })
        .await
        .expect("get_task should succeed")
        .into_inner();
    client
        .report_result(TaskResult {
            task_id: task.task_id,
            outcome: Some(Outcome::Rubric(RubricResult {
                composite_score: 4.0,
                criteria: vec![CriterionScore { name: "correctness".into(), score: 4, rationale: "ok".into() }],
            })),
        })
        .await
        .expect("report_result should succeed");

    let leaderboard = client
        .get_run(RunId { run_id: run_id.clone() })
        .await
        .expect("get_run should succeed")
        .into_inner();
    assert!(!leaderboard.all_tasks_done, "one task is still pending");

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn get_run_returns_ranked_leaderboard_once_all_tasks_done() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"p1\"}\n", "get_run_ranked");
    let run_id = client
        .submit_run(RunRequest {
            models: vec!["claude".into(), "gemini".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Rubric as i32,
            compare: false,
        })
        .await
        .expect("submit_run should succeed")
        .into_inner()
        .run_id;

    // Claim both tasks and report distinct scores so ranking is unambiguous
    for _ in 0..2 {
        let task = client
            .get_task(WorkerId { worker_id: "worker-1".into() })
            .await
            .expect("get_task should succeed")
            .into_inner();
        let score = if task.model == "claude" { 4.5 } else { 2.5 };
        client
            .report_result(TaskResult {
                task_id: task.task_id,
                outcome: Some(Outcome::Rubric(RubricResult {
                    composite_score: score,
                    criteria: vec![CriterionScore { name: "correctness".into(), score: 4, rationale: "ok".into() }],
                })),
            })
            .await
            .expect("report_result should succeed");
    }

    let leaderboard = client
        .get_run(RunId { run_id: run_id.clone() })
        .await
        .expect("get_run should succeed")
        .into_inner();

    assert!(leaderboard.all_tasks_done);
    assert_eq!(leaderboard.entries.len(), 2);
    assert_eq!(leaderboard.entries[0].rank, 1);
    assert_eq!(leaderboard.entries[0].model, "claude");
    assert_eq!(leaderboard.entries[0].mean_score, 4.5);
    assert_eq!(leaderboard.entries[1].rank, 2);
    assert_eq!(leaderboard.entries[1].model, "gemini");
    assert_eq!(leaderboard.entries[1].mean_score, 2.5);

    let (status,): (String,) = sqlx::query_as("SELECT status FROM runs WHERE run_id = $1")
        .bind(&run_id)
        .fetch_one(&pool)
        .await
        .expect("fetch run row");
    assert_eq!(status, "done", "GetRun should flip runs.status once all tasks are done");

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn get_run_rejects_unknown_run_id() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let status = client
        .get_run(RunId { run_id: "run_does_not_exist".into() })
        .await
        .expect_err("get_run should reject an unknown run_id");
    assert_eq!(status.code(), Code::NotFound);
}

#[tokio::test]
async fn get_run_rejects_empty_run_id() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let status = client
        .get_run(RunId { run_id: String::new() })
        .await
        .expect_err("get_run should reject an empty run_id");
    assert_eq!(status.code(), Code::InvalidArgument);
}