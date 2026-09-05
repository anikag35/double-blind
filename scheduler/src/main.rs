mod db;
mod expand;
mod files;
mod ids;
#[cfg(test)]
mod submit_run_tests;
#[cfg(test)]
mod submit_run_tests_rubric;
mod validate;
mod pb {
    tonic::include_proto!("blind");
}

use std::fs;

use pb::scheduler_server::{Scheduler, SchedulerServer};
use pb::{
    Empty, HeartbeatRequest, Leaderboard, Mode, RunHandle, RunId, RunRequest, Task, TaskResult,
    WorkerId,
};
use sqlx::PgPool;
use tonic::{transport::Server, Request, Response, Status};

/// Used when --judge is omitted
const DEFAULT_JUDGE: &str = "claude-opus";

/// Used when --rubric is omitted and no ./rubric.yaml exists
const DEFAULT_RUBRIC: &str = include_str!("../default_rubric.yaml");

struct SchedulerService {
    pool: PgPool,
}

#[tonic::async_trait]
impl Scheduler for SchedulerService {
    async fn submit_run(
        &self,
        request: Request<RunRequest>,
    ) -> Result<Response<RunHandle>, Status> {
        let req = request.into_inner();

        if req.models.is_empty() {
            return Err(Status::invalid_argument("models list is empty"));
        }

        let judge = if req.judge.is_empty() {
            DEFAULT_JUDGE.to_string()
        } else {
            req.judge.clone()
        };
        validate::judge_exclusion(&judge, &req.models).map_err(Status::invalid_argument)?;

        let mode = Mode::try_from(req.mode).unwrap_or(Mode::Unspecified);
        let mode_str = match mode {
            Mode::Unspecified | Mode::Rubric => "rubric",
            Mode::Pairwise => "pairwise",
        };
        if mode_str == "pairwise" {
            validate::pairwise_model_count(&req.models).map_err(Status::invalid_argument)?;
        }

        let prompts_content = fs::read_to_string(&req.prompts_path).map_err(|e| {
            Status::invalid_argument(format!(
                "failed to read prompts_path {}: {e}",
                req.prompts_path
            ))
        })?;
        let prompts = files::parse_prompts(&prompts_content).map_err(Status::invalid_argument)?;

        let rubric_content = if req.rubric_path.is_empty() {
            DEFAULT_RUBRIC.to_string()
        } else {
            fs::read_to_string(&req.rubric_path).map_err(|e| {
                Status::invalid_argument(format!(
                    "failed to read rubric_path {}: {e}",
                    req.rubric_path
                ))
            })?
        };
        // Validated for well-formedness now so a bad rubric fails fast at
        // submit time, rather than surfacing much later on a worker
        files::parse_rubric(&rubric_content).map_err(Status::invalid_argument)?;
        let rubric_hash = ids::hash_content(&rubric_content);

        let new_run = db::NewRun {
            models: &req.models,
            prompts_path: &req.prompts_path,
            judge: &judge,
            rubric_hash: &rubric_hash,
            mode: mode_str,
            compare: req.compare,
        };

        let run_id = if mode_str == "pairwise" {
            db::insert_run(&self.pool, &new_run, |run_id| {
                expand::expand_pairwise(
                    run_id,
                    &req.models[0],
                    &req.models[1],
                    &prompts,
                    &judge,
                    req.compare,
                )
            })
            .await
        } else {
            db::insert_run(&self.pool, &new_run, |run_id| {
                expand::expand_rubric(run_id, &req.models, &prompts, &judge, &rubric_hash)
            })
            .await
        }
        .map_err(|e| Status::internal(format!("failed to create run: {e}")))?;

        Ok(Response::new(RunHandle { run_id }))
    }

    async fn get_task(&self, _request: Request<WorkerId>) -> Result<Response<Task>, Status> {
        Err(Status::unimplemented("GetTask not yet implemented"))
    }

    async fn report_result(
        &self,
        _request: Request<TaskResult>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("ReportResult not yet implemented"))
    }

    async fn heartbeat(
        &self,
        _request: Request<HeartbeatRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("Heartbeat not yet implemented"))
    }

    async fn get_run(&self, _request: Request<RunId>) -> Result<Response<Leaderboard>, Status> {
        Err(Status::unimplemented("GetRun not yet implemented"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = db::connect(&database_url).await?;

    let addr = "[::1]:50051".parse()?;
    let service = SchedulerService { pool };

    println!("scheduler: listening on {addr}");

    Server::builder()
        .add_service(SchedulerServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}