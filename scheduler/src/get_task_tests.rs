use crate::pb::{Mode, RunRequest};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file};

#[tokio::test]
async fn get_task_claims_a_rubric_task_then_reports_none_available() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"hello\"}\n", "get_task_rubric");
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
        .get_task(crate::pb::WorkerId {
            worker_id: "worker-1".into(),
        })
        .await
        .expect("get_task should succeed")
        .into_inner();

    assert!(task.available);
    assert_eq!(task.run_id, run_id);
    assert_eq!(task.mode, Mode::Rubric as i32);
    assert_eq!(task.model, "claude");
    assert_eq!(task.prompt, "hello");
    assert_eq!(task.judge, "gpt-5");
    assert!(!task.task_id.is_empty());

    // Only one task existed, and it's now claimed
    let none_left = client
        .get_task(crate::pb::WorkerId {
            worker_id: "worker-2".into(),
        })
        .await
        .expect("get_task should succeed")
        .into_inner();
    assert!(!none_left.available);

    let (status, claimed_by): (String, Option<String>) =
        sqlx::query_as("SELECT status, claimed_by FROM tasks WHERE task_id = $1")
            .bind(&task.task_id)
            .fetch_one(&pool)
            .await
            .expect("fetch task row");
    assert_eq!(status, "claimed");
    assert_eq!(claimed_by.as_deref(), Some("worker-1"));

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn get_task_rejects_empty_worker_id() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let status = client
        .get_task(crate::pb::WorkerId {
            worker_id: String::new(),
        })
        .await
        .expect_err("get_task should reject an empty worker_id");

    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}