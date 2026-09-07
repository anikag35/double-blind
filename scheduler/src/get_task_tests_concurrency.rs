use crate::pb::{Mode, RunRequest, WorkerId};
use crate::submit_run_tests::{
    cleanup_run, spawn_client, test_pool, write_temp_file, TASK_QUEUE_TEST_LOCK,
};
use std::collections::HashSet;

/// Exercises SKIP LOCKED: concurrent workers must never be handed the same task.
#[tokio::test]
async fn concurrent_get_task_calls_never_double_claim_a_task() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let client = spawn_client(pool.clone()).await;

    let prompt_lines = (0..10)
        .map(|i| format!("{{\"prompt\": \"p{i}\"}}"))
        .collect::<Vec<_>>()
        .join("\n");
    let prompts_path = write_temp_file(&prompt_lines, "get_task_concurrency");

    let run_id = client
        .clone()
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

    // More concurrent callers than tasks, so some must legitimately see
    // available: false rather than a duplicate task.
    let mut handles = Vec::new();
    for i in 0..20 {
        let mut client = client.clone();
        handles.push(tokio::spawn(async move {
            client
                .get_task(WorkerId {
                    worker_id: format!("worker-{i}"),
                })
                .await
                .expect("get_task should succeed")
                .into_inner()
        }));
    }

    let mut claimed_ids = HashSet::new();
    let mut available_count = 0;
    for handle in handles {
        let task = handle.await.expect("task should not panic");
        if task.available {
            available_count += 1;
            let newly_seen = claimed_ids.insert(task.task_id);
            assert!(newly_seen, "the same task_id was claimed by two workers");
        }
    }

    assert_eq!(available_count, 10, "exactly the 10 tasks should be claimed, no more, no fewer");

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}
