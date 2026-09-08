import os
import time

from worker import blind_pb2
from worker.client import Client
from worker.heartbeat import heartbeat_while_processing
from worker.identity import generate_worker_id
from worker.process_task import process_task

DEFAULT_SCHEDULER_ADDRESS = "localhost:50051"
POLL_INTERVAL_SECONDS = 1.0
HEARTBEAT_INTERVAL_SECONDS = 5.0


def scheduler_address() -> str:
    return os.environ.get("SCHEDULER_ADDRESS", DEFAULT_SCHEDULER_ADDRESS)


def run_worker(
    stub,
    client: Client,
    worker_id: str | None = None,
    max_tasks: int | None = None,
    poll_interval_seconds: float = POLL_INTERVAL_SECONDS,
    heartbeat_interval_seconds: float = HEARTBEAT_INTERVAL_SECONDS,
) -> None:
    # Polls GetTask in a loop: process and report each task, sending heartbeats while it's in progress
    # Runs forever if max_tasks is None and stops after processing that many tasks otherwise
    worker_id = worker_id or generate_worker_id()
    processed = 0

    while max_tasks is None or processed < max_tasks:
        task = stub.GetTask(blind_pb2.WorkerId(worker_id=worker_id))
        if not task.available:
            time.sleep(poll_interval_seconds)
            continue

        with heartbeat_while_processing(stub, task.task_id, worker_id, heartbeat_interval_seconds):
            result = process_task(task, client)

        stub.ReportResult(result)
        processed += 1