use crate::pb::task_result::Outcome;
use crate::pb::{Mode, PairwiseResult, RunRequest, TaskResult, Verdict, WorkerId};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file, TASK_QUEUE_TEST_LOCK};
use tonic::Code;

#[tokio::test]
async fn report_result_rejects_wrong_outcome_kind_for_task_mode() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"hello\"}\n", "report_mode_mismatch");
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

    // Task is rubric mode, but sending a pairwise-shaped outcome.
    let status = client
        .report_result(TaskResult {
            task_id: task.task_id.clone(),
            outcome: Some(Outcome::Pairwise(PairwiseResult {
                verdict: Verdict::ModelA as i32,
                rationale: "wrong shape".into(),
            })),
        })
        .await
        .expect_err("report_result should reject a mismatched outcome kind");
    assert_eq!(status.code(), Code::InvalidArgument);

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn report_result_rejects_unknown_task_id() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let status = client
        .report_result(TaskResult {
            task_id: "task_does_not_exist".into(),
            outcome: Some(Outcome::Pairwise(PairwiseResult {
                verdict: Verdict::ModelA as i32,
                rationale: "irrelevant".into(),
            })),
        })
        .await
        .expect_err("report_result should reject an unknown task_id");
    assert_eq!(status.code(), Code::NotFound);
}

#[tokio::test]
async fn report_result_rejects_empty_task_id() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let status = client
        .report_result(TaskResult { task_id: String::new(), outcome: None })
        .await
        .expect_err("report_result should reject an empty task_id");
    assert_eq!(status.code(), Code::InvalidArgument);
}
