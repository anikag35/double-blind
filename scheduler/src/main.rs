mod pb {
    tonic::include_proto!("blind");
}

use pb::scheduler_server::{Scheduler, SchedulerServer};
use pb::{
    Empty, HeartbeatRequest, Leaderboard, RunHandle, RunId, RunRequest, Task, TaskResult, WorkerId,
};
use tonic::{transport::Server, Request, Response, Status};

#[derive(Default)]
struct SchedulerService;

#[tonic::async_trait]
impl Scheduler for SchedulerService {
    async fn submit_run(
        &self,
        _request: Request<RunRequest>,
    ) -> Result<Response<RunHandle>, Status> {
        Err(Status::unimplemented("SubmitRun not yet implemented"))
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
    let addr = "[::1]:50051".parse()?;
    let service = SchedulerService::default();

    println!("scheduler: listening on {addr}");

    Server::builder()
        .add_service(SchedulerServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
