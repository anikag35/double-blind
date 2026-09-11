use crate::expand::TaskRow;
use crate::ids::generate_run_id;
use crate::report::{validate_and_extract, StoredResult};
use crate::pb::task_result::Outcome;
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

pub enum ReportError {
    TaskNotFound,
    InvalidRequest(String),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for ReportError {
    fn from(e: sqlx::Error) -> Self {
        ReportError::Db(e)
    }
}

/// Stores a task's result and marks it done in one transaction. Locking the task row here means a concurrent claim query's SKIP LOCKED will skip this task rather than reassigning it mid-report.
pub async fn report_result(
    pool: &PgPool,
    task_id: &str,
    outcome: Option<Outcome>,
) -> Result<(), ReportError> {
    let mut tx = pool.begin().await?;

    let mode: Option<String> = sqlx::query_scalar(
        "SELECT runs.mode FROM tasks JOIN runs ON runs.run_id = tasks.run_id \
         WHERE tasks.task_id = $1 FOR UPDATE OF tasks",
    )
        .bind(task_id)
        .fetch_optional(&mut *tx)
        .await?;
    let mode = mode.ok_or(ReportError::TaskNotFound)?;

    let stored = validate_and_extract(&mode, outcome).map_err(ReportError::InvalidRequest)?;

    match stored {
        StoredResult::Rubric { composite_score, criteria } => {
            sqlx::query(
                "INSERT INTO rubric_results (task_id, composite_score, criteria) VALUES ($1, $2, $3) \
                 ON CONFLICT (task_id) DO NOTHING",
            )
            .bind(task_id)
            .bind(composite_score)
            .bind(criteria)
            .execute(&mut *tx)
            .await?;
        }
        StoredResult::Pairwise { verdict, rationale } => {
            sqlx::query(
                "INSERT INTO pairwise_results (task_id, verdict, rationale) VALUES ($1, $2, $3) \
                 ON CONFLICT (task_id) DO NOTHING",
            )
            .bind(task_id)
            .bind(verdict)
            .bind(rationale)
            .execute(&mut *tx)
            .await?;
        }
    }

    sqlx::query("UPDATE tasks SET status = 'done' WHERE task_id = $1")
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Bumps last_heartbeat, only if the task is still claimed by this worker. Returns false if the task doesn't exist, was reclaimed by someone else, or is already done.
pub async fn update_heartbeat(pool: &PgPool, task_id: &str, worker_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE tasks SET last_heartbeat = now() \
         WHERE task_id = $1 AND claimed_by = $2 AND status = 'claimed'",
    )
    .bind(task_id)
    .bind(worker_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub struct LeaderboardResult {
    pub all_tasks_done: bool,
    /// (model, mean composite_score), ordered highest first. Empty until all_tasks_done is true.
    pub entries: Vec<(String, f64)>,
}

/// Returns None if run_id doesn't exist. Lazily flips runs.status to 'done' the first time every task for the run is found done. Entries only computed once done - reading rubric_results, so a pairwise-mode run's entries stay empty (pairwise leaderboard support isn't built yet).
pub async fn get_run_leaderboard(
    pool: &PgPool,
    run_id: &str,
) -> Result<Option<LeaderboardResult>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let run_exists: Option<String> = sqlx::query_scalar("SELECT run_id FROM runs WHERE run_id = $1")
        .bind(run_id)
        .fetch_optional(&mut *tx)
        .await?;
    if run_exists.is_none() {
        return Ok(None);
    }

    let not_done_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE run_id = $1 AND status != 'done'")
            .bind(run_id)
            .fetch_one(&mut *tx)
            .await?;
    let all_tasks_done = not_done_count == 0;

    if all_tasks_done {
        sqlx::query("UPDATE runs SET status = 'done' WHERE run_id = $1 AND status != 'done'")
            .bind(run_id)
            .execute(&mut *tx)
            .await?;
    }

    let entries: Vec<(String, f64)> = if all_tasks_done {
        sqlx::query_as(
            "SELECT tasks.payload->>'model' AS model, AVG(rubric_results.composite_score) AS mean_score \
             FROM tasks JOIN rubric_results ON rubric_results.task_id = tasks.task_id \
             WHERE tasks.run_id = $1 \
             GROUP BY tasks.payload->>'model' \
             ORDER BY mean_score DESC",
        )
        .bind(run_id)
        .fetch_all(&mut *tx)
        .await?
    } else {
        vec![]
    };

    tx.commit().await?;

    Ok(Some(LeaderboardResult { all_tasks_done, entries }))
}