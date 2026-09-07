use crate::pb::task_result::Outcome;
use crate::pb::{CriterionScore, Mode, RubricResult, RunRequest, TaskResult, WorkerId};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file, TASK_QUEUE_TEST_LOCK};

#[tokio::test]
async fn report_result_stores_rubric_result_and_marks_task_done() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"hello\"}\n", "report_rubric");
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

    let task = client
        .get_task(WorkerId { worker_id: "worker-1".into() })
        .await
        .expect("get_task should succeed")
        .into_inner();
    assert!(task.available);

    client
        .report_result(TaskResult {
            task_id: task.task_id.clone(),
            outcome: Some(Outcome::Rubric(RubricResult {
                composite_score: 4.25,
                criteria: vec![CriterionScore {
                    name: "correctness".into(),
                    score: 4,
                    rationale: "mostly right".into(),
                }],
            })),
        })
        .await
        .expect("report_result should succeed");

    let (status,): (String,) = sqlx::query_as("SELECT status FROM tasks WHERE task_id = $1")
        .bind(&task.task_id)
        .fetch_one(&pool)
        .await
        .expect("fetch task row");
    assert_eq!(status, "done");

    let (composite_score, criteria): (f64, serde_json::Value) =
        sqlx::query_as("SELECT composite_score, criteria FROM rubric_results WHERE task_id = $1")
            .bind(&task.task_id)
            .fetch_one(&pool)
            .await
            .expect("fetch rubric_results row");
    assert_eq!(composite_score, 4.25);
    assert_eq!(criteria[0]["name"], "correctness");
    assert_eq!(criteria[0]["score"], 4);

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn report_result_is_idempotent_on_duplicate_report() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"hello\"}\n", "report_rubric_dup");
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

    let task = client
        .get_task(WorkerId { worker_id: "worker-1".into() })
        .await
        .expect("get_task should succeed")
        .into_inner();

    let make_result = || TaskResult {
        task_id: task.task_id.clone(),
        outcome: Some(Outcome::Rubric(RubricResult {
            composite_score: 3.0,
            criteria: vec![CriterionScore { name: "correctness".into(), score: 3, rationale: "ok".into() }],
        })),
    };

    client.report_result(make_result()).await.expect("first report should succeed");
    // Simulates at-least-once redelivery: the same result reported twice.
    client.report_result(make_result()).await.expect("duplicate report should also succeed, not error");

    let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rubric_results WHERE task_id = $1")
        .bind(&task.task_id)
        .fetch_one(&pool)
        .await
        .expect("count rubric_results rows");
    assert_eq!(row_count, 1, "duplicate report must not create a second row");

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}
