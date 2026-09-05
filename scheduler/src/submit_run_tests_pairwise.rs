use crate::pb::{Mode, RunRequest};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file};

#[tokio::test]
async fn submit_run_pairwise_without_compare_is_blind_only() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file(
        "{\"prompt\": \"p1\"}\n{\"prompt\": \"p2\"}\n{\"prompt\": \"p3\"}\n",
        "pairwise_no_compare",
    );

    let response = client
        .submit_run(RunRequest {
            models: vec!["claude".into(), "gemini".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Pairwise as i32,
            compare: false,
        })
        .await
        .expect("submit_run should succeed");

    let run_id = response.into_inner().run_id;

    let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE run_id = $1")
        .bind(&run_id)
        .fetch_one(&pool)
        .await
        .expect("count tasks");
    assert_eq!(task_count, 3); // one per prompt, blind only

    let conditions: Vec<String> =
        sqlx::query_scalar("SELECT payload->>'condition' FROM tasks WHERE run_id = $1")
            .bind(&run_id)
            .fetch_all(&pool)
            .await
            .expect("fetch conditions");
    assert!(conditions.iter().all(|c| c == "blind"));

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn submit_run_pairwise_with_compare_runs_both_conditions() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file(
        "{\"prompt\": \"p1\"}\n{\"prompt\": \"p2\"}\n{\"prompt\": \"p3\"}\n",
        "pairwise_compare",
    );

    let response = client
        .submit_run(RunRequest {
            models: vec!["claude".into(), "gemini".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Pairwise as i32,
            compare: true,
        })
        .await
        .expect("submit_run should succeed");

    let run_id = response.into_inner().run_id;

    let blind_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE run_id = $1 AND payload->>'condition' = 'blind'",
    )
    .bind(&run_id)
    .fetch_one(&pool)
    .await
    .expect("count blind tasks");
    let unblind_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE run_id = $1 AND payload->>'condition' = 'unblind'",
    )
    .bind(&run_id)
    .fetch_one(&pool)
    .await
    .expect("count unblind tasks");
    assert_eq!(blind_count, 3);
    assert_eq!(unblind_count, 3);

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}
