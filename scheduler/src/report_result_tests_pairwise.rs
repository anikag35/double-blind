use crate::pb::task_result::Outcome;
use crate::pb::{Mode, PairwiseResult, RunRequest, TaskResult, Verdict, WorkerId};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file, TASK_QUEUE_TEST_LOCK};

#[tokio::test]
async fn report_result_stores_pairwise_result_and_marks_task_done() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"hello\"}\n", "report_pairwise");
    let run_id = client
        .submit_run(RunRequest {
            models: vec!["claude".into(), "gemini".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Pairwise as i32,
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
            outcome: Some(Outcome::Pairwise(PairwiseResult {
                verdict: Verdict::ModelA as i32,
                rationale: "more thorough".into(),
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

    let (verdict, rationale): (String, String) =
        sqlx::query_as("SELECT verdict, rationale FROM pairwise_results WHERE task_id = $1")
            .bind(&task.task_id)
            .fetch_one(&pool)
            .await
            .expect("fetch pairwise_results row");
    assert_eq!(verdict, "model_a");
    assert_eq!(rationale, "more thorough");

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}
