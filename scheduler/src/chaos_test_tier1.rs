// Tier 1 chaos test:
// - Simulated, and runs in CI on every push 
// - Writes a stale-heartbeat task row directly, then verifies claim_task's SQL itself correctly identifies it as abandoned and reassigns it

use crate::db::claim_task;
use crate::submit_run_tests::{test_pool, TASK_QUEUE_TEST_LOCK};
use sqlx::PgPool;

async fn insert_run(pool: &PgPool, run_id: &str) {
    sqlx::query(
        "INSERT INTO runs (run_id, models, prompts_path, judge, rubric_path, rubric_hash, mode, compare, status) \
         VALUES ($1, '[\"claude\"]'::jsonb, '/tmp/prompts.jsonl', 'gpt-5', '/tmp/rubric.yaml', 'rh1', 'rubric', false, 'pending')",
    )
    .bind(run_id)
    .execute(pool)
    .await
    .expect("insert run");
}

async fn insert_task(pool: &PgPool, task_id: &str, run_id: &str, claimed_by: &str, heartbeat_age_seconds: i64) {
    sqlx::query(
        "INSERT INTO tasks (task_id, run_id, status, claimed_by, last_heartbeat, payload) \
         VALUES ($1, $2, 'claimed', $3, now() - (interval '1 second' * $4), \
                 '{\"model\": \"claude\", \"prompt\": \"p\", \"prompt_hash\": \"h\"}'::jsonb)",
    )
    .bind(task_id)
    .bind(run_id)
    .bind(claimed_by)
    .bind(heartbeat_age_seconds)
    .execute(pool)
    .await
    .expect("insert task");
}

async fn cleanup(pool: &PgPool, run_id: &str) {
    sqlx::query("DELETE FROM tasks WHERE run_id = $1").bind(run_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM runs WHERE run_id = $1").bind(run_id).execute(pool).await.ok();
}

#[tokio::test]
async fn claim_task_reclaims_a_task_with_a_stale_heartbeat() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let run_id = "run_chaos_tier1_stale";
    let task_id = "task_chaos_tier1_stale";

    insert_run(&pool, run_id).await;
    // past the 30s default reassignment timeout
    insert_task(&pool, task_id, run_id, "dead-worker", 31).await;

    let claimed = claim_task(&pool, "new-worker", 30.0)
        .await
        .expect("claim_task should succeed")
        .expect("an abandoned task should be claimable");
    assert_eq!(claimed.task_id, task_id);
    assert_eq!(claimed.run_id, run_id);

    let (claimed_by, last_heartbeat): (Option<String>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT claimed_by, last_heartbeat FROM tasks WHERE task_id = $1")
            .bind(task_id)
            .fetch_one(&pool)
            .await
            .expect("fetch task row");
    assert_eq!(claimed_by.as_deref(), Some("new-worker"));
    assert!(
        chrono::Utc::now() - last_heartbeat < chrono::Duration::seconds(5),
        "last_heartbeat should have been reset to now, not left at the stale value"
    );

    cleanup(&pool, run_id).await;
}

#[tokio::test]
async fn claim_task_does_not_reclaim_a_task_with_a_fresh_heartbeat() {
    let _guard = TASK_QUEUE_TEST_LOCK.lock().await;
    let pool = test_pool().await;
    let run_id = "run_chaos_tier1_fresh";
    let task_id = "task_chaos_tier1_fresh";

    insert_run(&pool, run_id).await;
    // within the 30s reassignment timeout
    insert_task(&pool, task_id, run_id, "alive-worker", 5).await;

    let claimed = claim_task(&pool, "other-worker", 30.0)
        .await
        .expect("claim_task should succeed");
    assert!(
        claimed.is_none(),
        "a task with a fresh heartbeat must not be reassigned out from under its worker"
    );

    let (claimed_by,): (Option<String>,) =
        sqlx::query_as("SELECT claimed_by FROM tasks WHERE task_id = $1")
            .bind(task_id)
            .fetch_one(&pool)
            .await
            .expect("fetch task row");
    assert_eq!(claimed_by.as_deref(), Some("alive-worker"));

    cleanup(&pool, run_id).await;
}