use crate::pb::{Mode, RunRequest};
use crate::submit_run_tests::{spawn_client, test_pool, write_temp_file};
use tonic::Code;

#[tokio::test]
async fn submit_run_rejects_judge_that_is_also_a_model() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let prompts_path = write_temp_file("{\"prompt\": \"p1\"}\n", "judge_exclusion");

    let status = client
        .submit_run(RunRequest {
            models: vec!["claude".into(), "gemini".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "claude".into(), // also in models -> must be rejected
            rubric_path: String::new(),
            mode: Mode::Rubric as i32,
            compare: false,
        })
        .await
        .expect_err("submit_run should reject judge that is also a model");

    assert_eq!(status.code(), Code::InvalidArgument);
    assert!(status.message().contains("claude"));

    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn submit_run_rejects_pairwise_with_one_model() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let prompts_path = write_temp_file("{\"prompt\": \"p1\"}\n", "pairwise_one_model");

    let status = client
        .submit_run(RunRequest {
            models: vec!["claude".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Pairwise as i32,
            compare: false,
        })
        .await
        .expect_err("submit_run should reject pairwise mode with 1 model");

    assert_eq!(status.code(), Code::InvalidArgument);

    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn submit_run_rejects_pairwise_with_three_models() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let prompts_path = write_temp_file("{\"prompt\": \"p1\"}\n", "pairwise_three_models");

    let status = client
        .submit_run(RunRequest {
            models: vec!["claude".into(), "gemini".into(), "chatgpt".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Pairwise as i32,
            compare: false,
        })
        .await
        .expect_err("submit_run should reject pairwise mode with 3 models");

    assert_eq!(status.code(), Code::InvalidArgument);

    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn submit_run_rejects_empty_models_list() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let prompts_path = write_temp_file("{\"prompt\": \"p1\"}\n", "empty_models");

    let status = client
        .submit_run(RunRequest {
            models: vec![],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Rubric as i32,
            compare: false,
        })
        .await
        .expect_err("submit_run should reject an empty models list");

    assert_eq!(status.code(), Code::InvalidArgument);

    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn submit_run_rejects_missing_prompts_file() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let status = client
        .submit_run(RunRequest {
            models: vec!["claude".into()],
            prompts_path: "/nonexistent/path/does_not_exist.jsonl".into(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Rubric as i32,
            compare: false,
        })
        .await
        .expect_err("submit_run should reject a missing prompts file");

    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn submit_run_rejects_malformed_prompts_file() {
    let pool = test_pool().await;
    let mut client = spawn_client(pool).await;

    let prompts_path = write_temp_file("not valid json\n", "malformed_prompts");

    let status = client
        .submit_run(RunRequest {
            models: vec!["claude".into()],
            prompts_path: prompts_path.to_string_lossy().to_string(),
            judge: "gpt-5".into(),
            rubric_path: String::new(),
            mode: Mode::Rubric as i32,
            compare: false,
        })
        .await
        .expect_err("submit_run should reject a malformed prompts file");

    assert_eq!(status.code(), Code::InvalidArgument);

    std::fs::remove_file(prompts_path).ok();
}
