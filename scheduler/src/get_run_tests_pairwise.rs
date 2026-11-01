use crate::pb::leaderboard::Result as LeaderboardOutcome;
use crate::pb::task_result::Outcome;
use crate::pb::{Mode, PairwiseResult, RunId, RunRequest, TaskResult, Verdict, WorkerId};
use crate::submit_run_tests::{cleanup_run, spawn_client, test_pool, write_temp_file, TASK_QUEUE_TEST_LOCK};

async fn report(client: &mut crate::pb::scheduler_client::SchedulerClient<tonic::transport::Channel>, task_id: String, verdict: Verdict) {
    client
        .report_result(TaskResult {
            task_id,
            outcome: Some(Outcome::Pairwise(PairwiseResult { verdict: verdict as i32, rationale: "x".into() })),
        })
        .await
        .expect("report_result should succeed");
}

#[tokio::test]
async fn get_run_returns_pairwise_counts_for_blind_only_run() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file(
        "{\"prompt\": \"p1\"}\n{\"prompt\": \"p2\"}\n{\"prompt\": \"p3\"}\n",
        "get_run_pairwise_blind",
    );
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

    // 3 blind tasks: model_a wins, model_b wins, tie.
    let verdicts = [Verdict::ModelA, Verdict::ModelB, Verdict::Tie];
    for verdict in verdicts {
        let task = client
            .get_task(WorkerId { worker_id: "worker-1".into() })
            .await
            .expect("get_task should succeed")
            .into_inner();
        report(&mut client, task.task_id, verdict).await;
    }

    let leaderboard = client
        .get_run(RunId { run_id: run_id.clone() })
        .await
        .expect("get_run should succeed")
        .into_inner();

    assert!(leaderboard.all_tasks_done);
    let counts = match leaderboard.result {
        Some(LeaderboardOutcome::Pairwise(p)) => p,
        other => panic!("expected pairwise counts, got {other:?}"),
    };
    assert_eq!(counts.model_a, "claude");
    assert_eq!(counts.model_b, "gemini");
    let blind = counts.blind.expect("blind counts should be present");
    assert_eq!(blind.model_a_wins, 1);
    assert_eq!(blind.model_b_wins, 1);
    assert_eq!(blind.ties, 1);
    assert!(!counts.has_unblind);
    assert!(counts.unblind.is_none());

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}

#[tokio::test]
async fn get_run_returns_separate_blind_and_unblind_counts_when_compared() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let mut client = spawn_client(pool.clone()).await;

    let prompts_path = write_temp_file(
        "{\"prompt\": \"p1\"}\n{\"prompt\": \"p2\"}\n",
        "get_run_pairwise_compare",
    );
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

    // 2 prompts x 2 conditions = 4 tasks. Make blind and unblind diverge:
    // blind -> both ties; unblind -> both model_a wins (a self-preference-style shift).
    for _ in 0..4 {
        let task = client
            .get_task(WorkerId { worker_id: "worker-1".into() })
            .await
            .expect("get_task should succeed")
            .into_inner();
        let verdict = if task.condition == crate::pb::Condition::Blind as i32 {
            Verdict::Tie
        } else {
            Verdict::ModelA
        };
        report(&mut client, task.task_id, verdict).await;
    }

    let leaderboard = client
        .get_run(RunId { run_id: run_id.clone() })
        .await
        .expect("get_run should succeed")
        .into_inner();

    let counts = match leaderboard.result {
        Some(LeaderboardOutcome::Pairwise(p)) => p,
        other => panic!("expected pairwise counts, got {other:?}"),
    };
    assert!(counts.has_unblind);
    let blind = counts.blind.expect("blind counts present");
    assert_eq!((blind.model_a_wins, blind.model_b_wins, blind.ties), (0, 0, 2));
    let unblind = counts.unblind.expect("unblind counts present since compare=true");
    assert_eq!((unblind.model_a_wins, unblind.model_b_wins, unblind.ties), (2, 0, 0));

    cleanup_run(&pool, &run_id).await;
    std::fs::remove_file(prompts_path).ok();
}