mod db;
mod expand;
mod files;
#[cfg(test)]
mod get_task_tests;
#[cfg(test)]
mod get_task_tests_concurrency;
#[cfg(test)]
mod get_task_tests_pairwise;
mod ids;
mod report;
#[cfg(test)]
mod submit_run_tests;
#[cfg(test)]
mod submit_run_tests_pairwise;
#[cfg(test)]
mod submit_run_tests_rubric;
#[cfg(test)]
mod submit_run_tests_validation;
#[cfg(test)]
mod report_result_tests_rubric;
#[cfg(test)]
mod report_result_tests_pairwise;
#[cfg(test)]
mod report_result_tests_validation;
mod task_message;
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

/// Used when --rubric is omitted. 
// This is a real file so the worker has an actual path to read criteria from
const DEFAULT_RUBRIC_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/default_rubric.yaml");

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

        let rubric_path = if req.rubric_path.is_empty() {
            DEFAULT_RUBRIC_PATH
        } else {
            &req.rubric_path
        };
        let rubric_content = fs::read_to_string(rubric_path).map_err(|e| {
            Status::invalid_argument(format!("failed to read rubric_path {rubric_path}: {e}"))
        })?;
        // Validated for well-formedness now so a bad rubric fails fast at
        // submit time, rather than surfacing much later on a worker
        files::parse_rubric(&rubric_content).map_err(Status::invalid_argument)?;
        let rubric_hash = ids::hash_content(&rubric_content);

        let new_run = db::NewRun {
            models: &req.models,
            prompts_path: &req.prompts_path,
            judge: &judge,
            rubric_path,
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

    async fn get_task(&self, request: Request<WorkerId>) -> Result<Response<Task>, Status> {
        let worker_id = request.into_inner().worker_id;
        if worker_id.is_empty() {
            return Err(Status::invalid_argument("worker_id is empty"));
        }

        let claimed = db::claim_task(&self.pool, &worker_id)
            .await
            .map_err(|e| Status::internal(format!("failed to claim task: {e}")))?;

        let task = match claimed {
            Some(claimed) => task_message::build_task(claimed).map_err(Status::internal)?,
            None => Task {
                available: false,
                ..Default::default()
            },
        };

        Ok(Response::new(task))
    }

    async fn report_result(
        &self,
        request: Request<TaskResult>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        if req.task_id.is_empty() {
            return Err(Status::invalid_argument("task_id is empty"));
        }

        db::report_result(&self.pool, &req.task_id, req.outcome)
            .await
            .map_err(|e| match e {
                db::ReportError::TaskNotFound => {
                    Status::not_found(format!("no such task: {}", req.task_id))
                }
                db::ReportError::InvalidRequest(msg) => Status::invalid_argument(msg),
                db::ReportError::Db(e) => Status::internal(format!("failed to report result: {e}")),
            })?;

        Ok(Response::new(Empty {}))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        if req.task_id.is_empty() || req.worker_id.is_empty() {
            return Err(Status::invalid_argument("task_id and worker_id are required"));
        }

        let updated = db::update_heartbeat(&self.pool, &req.task_id, &req.worker_id)
            .await
            .map_err(|e| Status::internal(format!("failed to update heartbeat: {e}")))?;

        if !updated {
            return Err(Status::not_found(format!(
                "task {} is not currently claimed by worker {}",
                req.task_id, req.worker_id
            )));
        }

        Ok(Response::new(Empty {}))
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