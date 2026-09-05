use crate::expand::TaskRow;
use crate::ids::generate_run_id;
use sqlx::PgPool;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPool::connect(database_url).await
}

pub struct NewRun<'a> {
    pub models: &'a [String],
    pub prompts_path: &'a str,
    pub judge: &'a str,
    pub rubric_path: &'a str,
    pub rubric_hash: &'a str,
    pub mode: &'a str, // "rubric" | "pairwise"
    pub compare: bool,
}

/// Inserts a run and its tasks in one transaction
/// `build_tasks` receives the generated run_id since task_id depends on it 
// and is re-invoked with a fresh run_id if it collides with an existing run.
pub async fn insert_run(
    pool: &PgPool,
    run: &NewRun<'_>,
    build_tasks: impl Fn(&str) -> Vec<TaskRow>,
) -> Result<String, sqlx::Error> {
    loop {
        let run_id = generate_run_id();
        let mut tx = pool.begin().await?;

        let insert_result = sqlx::query(
            "INSERT INTO runs (run_id, models, prompts_path, judge, rubric_path, rubric_hash, mode, compare, status) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending')",
        )
        .bind(&run_id)
        .bind(serde_json::json!(run.models))
        .bind(run.prompts_path)
        .bind(run.judge)
        .bind(run.rubric_path)
        .bind(run.rubric_hash)
        .bind(run.mode)
        .bind(run.compare)
        .execute(&mut *tx)
        .await;

        if let Err(sqlx::Error::Database(db_err)) = &insert_result {
            if db_err.is_unique_violation() {
                continue; // run_id collision, retry w a fresh id
            }
        }
        insert_result?;

        for task in build_tasks(&run_id) {
            sqlx::query(
                "INSERT INTO tasks (task_id, run_id, payload) VALUES ($1, $2, $3) \
                 ON CONFLICT (task_id) DO NOTHING",
            )
            .bind(&task.task_id)
            .bind(&run_id)
            .bind(&task.payload)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        return Ok(run_id);
    }
}

#[derive(sqlx::FromRow)]
pub struct ClaimedTask {
    pub task_id: String,
    pub run_id: String,
    pub payload: serde_json::Value,
    pub mode: String,
    pub judge: String,
    pub rubric_path: String,
}

/// Atomically claims one task: a fresh `pending` task, or a `claimed` one
/// whose worker went silent (no heartbeat in the last 30 seconds)
pub async fn claim_task(
    pool: &PgPool,
    worker_id: &str,
) -> Result<Option<ClaimedTask>, sqlx::Error> {
    sqlx::query_as::<_, ClaimedTask>(
        "WITH claimed AS ( \
            UPDATE tasks \
            SET status = 'claimed', claimed_by = $1, last_heartbeat = now() \
            WHERE task_id = ( \
                SELECT task_id FROM tasks \
                WHERE status = 'pending' \
                   OR (status = 'claimed' AND last_heartbeat < now() - interval '30 seconds') \
                ORDER BY created_at \
                FOR UPDATE SKIP LOCKED \
                LIMIT 1 \
            ) \
            RETURNING task_id, run_id, payload \
        ) \
        SELECT claimed.task_id, claimed.run_id, claimed.payload, \
               runs.mode, runs.judge, runs.rubric_path \
        FROM claimed \
        JOIN runs ON runs.run_id = claimed.run_id",
    )
    .bind(worker_id)
    .fetch_optional(pool)
    .await
}