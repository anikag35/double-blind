use crate::pb::{Mode, RunRequest};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file};

#[tokio::test]
async fn submit_run_rubric_mode_creates_one_task_per_model_times_prompt() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file(
        "{\"prompt\": \"p1\"}\n{\"prompt\": \"p2\"}\n",
        "rubric_prompts",
    );

    let response = client
        .submit_run(RunRequest {
            models: vec!["claude".into(), "gemini".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Rubric as i32,
            compare: false,
        })
        .await
        .expect("submit_run should succeed");

    let run_id = response.into_inner().run_id;
    assert!(run_id.starts_with("run_"), "run_id was {run_id}");

    let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE run_id = $1")
        .bind(&run_id)
        .fetch_one(&pool)
        .await
        .expect("count tasks");
    assert_eq!(task_count, 4); // 2 models x 2 prompts

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn submit_run_defaults_to_rubric_mode_and_default_judge_when_unspecified() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file("{\"prompt\": \"p1\"}\n", "default_mode_prompts");

    let response = client
        .submit_run(RunRequest {
            models: vec!["gemini".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: String::new(), // built-in default judge
            rubric_path: String::new(),
            mode: Mode::Unspecified as i32,
            compare: false,
        })
        .await
        .expect("submit_run should succeed");

    let run_id = response.into_inner().run_id;

    let (mode, judge, rubric_path): (String, String, String) =
        sqlx::query_as("SELECT mode, judge, rubric_path FROM runs WHERE run_id = $1")
            .bind(&run_id)
            .fetch_one(&pool)
            .await
            .expect("fetch run row");
    assert_eq!(mode, "rubric");
    assert_eq!(judge, "claude-opus");
    assert!(rubric_path.ends_with("default_rubric.yaml"));
    assert!(std::fs::metadata(&rubric_path).is_ok(), "rubric_path should exist on disk: {rubric_path}");

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}