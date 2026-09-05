//! Integration tests for SubmitRun: spins up the real gRPC server against
//! the real database and exercises it through the generated client

use crate::pb::scheduler_client::SchedulerClient;
use crate::pb::scheduler_server::SchedulerServer;
use crate::SchedulerService;
use sqlx::PgPool;
use std::net::TcpListener as StdTcpListener;
use tokio::net::TcpListener as TokioTcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

pub async fn test_pool() -> PgPool {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    PgPool::connect(&url).await.expect("failed to connect to test database")
}

/// Boots a real scheduler server on a short local port and returns a connected client to it
pub async fn spawn_client(pool: PgPool) -> SchedulerClient<tonic::transport::Channel> {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.set_nonblocking(true).expect("set_nonblocking");
    let addr = listener.local_addr().expect("local_addr");
    let incoming = TcpListenerStream::new(
        TokioTcpListener::from_std(listener).expect("tokio TcpListener::from_std"),
    );

    let service = SchedulerService { pool };
    tokio::spawn(async move {
        Server::builder()
            .add_service(SchedulerServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .expect("server should not fail");
    });

    SchedulerClient::connect(format!("http://{addr}"))
        .await
        .expect("client should connect")
}

/// Writes `content` to a unique temp file and returns its path
pub fn write_temp_file(content: &str, label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "blind_test_{}_{}_{}",
        std::process::id(),
        label,
        crate::ids::hash_content(content)
    ));
    std::fs::write(&path, content).expect("write temp file");
    path
}

/// Deletes everything a test run created so repeated test runs don't accumulate unnecessary data
pub async fn cleanup_run(pool: &PgPool, run_id: &str) {
    sqlx::query(
        "DELETE FROM rubric_results WHERE task_id IN (SELECT task_id FROM tasks WHERE run_id = $1)",
    )
    .bind(run_id)
    .execute(pool)
    .await
    .ok();
    sqlx::query(
        "DELETE FROM pairwise_results WHERE task_id IN (SELECT task_id FROM tasks WHERE run_id = $1)",
    )
    .bind(run_id)
    .execute(pool)
    .await
    .ok();
    sqlx::query("DELETE FROM tasks WHERE run_id = $1")
        .bind(run_id)
        .execute(pool)
        .await
        .expect("cleanup tasks");
    sqlx::query("DELETE FROM runs WHERE run_id = $1")
        .bind(run_id)
        .execute(pool)
        .await
        .expect("cleanup runs");
}