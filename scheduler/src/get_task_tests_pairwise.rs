use crate::pb::{Condition, Mode, RunRequest, WorkerId};
use crate::submit_run_tests::{
    cleanup_run, spawn_client, test_pool, write_temp_file, TASK_QUEUE_TEST_LOCK,
};

#[tokio::test]
async fn get_task_claims_both_pairwise_conditions_with_correct_fields() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"hello\"}\n", "get_task_pairwise");
    let run_id = client
        .submit_run(RunRequest {
            models: vec!["claude".into(), "gemini".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Pairwise as i32,
            compare: true,
        })
        .await
        .expect("submit_run should succeed")
        .into_inner()
        .run_id;

    let first = client
        .get_task(WorkerId { worker_id: "worker-1".into() })
        .await
        .expect("get_task should succeed")
        .into_inner();
    let second = client
        .get_task(WorkerId { worker_id: "worker-2".into() })
        .await
        .expect("get_task should succeed")
        .into_inner();

    for task in [&first, &second] {
        assert!(task.available);
        assert_eq!(task.run_id, run_id);
        assert_eq!(task.mode, Mode::Pairwise as i32);
        // Models are sorted before hashing/storage, so "claude" (< "gemini")
        // is always model_a regardless of submission order.
        assert_eq!(task.model_a, "claude");
        assert_eq!(task.model_b, "gemini");
        assert_eq!(task.model, ""); // rubric-only field stays empty
        assert!(task.condition == Condition::Blind as i32 || task.condition == Condition::Unblind as i32);
    }
    assert_ne!(first.task_id, second.task_id);
    assert_ne!(first.condition, second.condition, "one task should be blind, the other unblind");

    let none_left = client
        .get_task(WorkerId { worker_id: "worker-3".into() })
        .await
        .expect("get_task should succeed")
        .into_inner();
    assert!(!none_left.available);

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}
