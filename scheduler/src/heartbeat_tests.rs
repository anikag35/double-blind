use crate::pb::{HeartbeatRequest, Mode, RunRequest, WorkerId};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file, TASK_QUEUE_TEST_LOCK};
use tonic::Code;

#[tokio::test]
async fn heartbeat_updates_last_heartbeat_for_the_claiming_worker() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"hello\"}\n", "heartbeat_ok");
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

    let (before,): (chrono::DateTime<chrono::Utc>,) =
        sqlx::query_as("SELECT last_heartbeat FROM tasks WHERE task_id = $1")
            .bind(&task.task_id)
            .fetch_one(&pool)
            .await
            .expect("fetch task row");

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    client
        .heartbeat(HeartbeatRequest { task_id: task.task_id.clone(), worker_id: "worker-1".into() })
        .await
        .expect("heartbeat should succeed");

    let (after,): (chrono::DateTime<chrono::Utc>,) =
        sqlx::query_as("SELECT last_heartbeat FROM tasks WHERE task_id = $1")
            .bind(&task.task_id)
            .fetch_one(&pool)
            .await
            .expect("fetch task row");

    assert!(after > before, "heartbeat should advance last_heartbeat");

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn heartbeat_rejects_wrong_worker() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"hello\"}\n", "heartbeat_wrong_worker");
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

    let status = client
        .heartbeat(HeartbeatRequest { task_id: task.task_id.clone(), worker_id: "worker-2".into() })
        .await
        .expect_err("heartbeat from a worker that doesn't own the task should be rejected");
    assert_eq!(status.code(), Code::NotFound);

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn heartbeat_rejects_unknown_task_id() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let status = client
        .heartbeat(HeartbeatRequest { task_id: "task_does_not_exist".into(), worker_id: "worker-1".into() })
        .await
        .expect_err("heartbeat for an unknown task should be rejected");
    assert_eq!(status.code(), Code::NotFound);
}

#[tokio::test]
async fn heartbeat_rejects_empty_fields() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let status = client
        .heartbeat(HeartbeatRequest { task_id: String::new(), worker_id: "worker-1".into() })
        .await
        .expect_err("heartbeat with an empty task_id should be rejected");
    assert_eq!(status.code(), Code::InvalidArgument);
}