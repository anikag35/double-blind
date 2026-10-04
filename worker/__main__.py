import os

import grpc

from worker import blind_pb2_grpc
from worker.fake_client import FakeClient
from worker.identity import generate_worker_id
from worker.run import heartbeat_interval_seconds, run_worker, scheduler_address


def build_client():
    # Defaults to FakeClient
    # Real providers must be explicitly opted-in
    if os.environ.get("WORKER_CLIENT") == "real":
        from worker.routing_client import default_routing_client
        return default_routing_client()
    return FakeClient()


def main() -> None:
    address = scheduler_address()
    worker_id = generate_worker_id()
    heartbeat_seconds = heartbeat_interval_seconds()
    client = build_client()
    print(
        f"worker: {worker_id} connecting to scheduler at {address} "
        f"(heartbeat every {heartbeat_seconds}s, client: {type(client).__name__})"
    )

    channel = grpc.insecure_channel(address)
    stub = blind_pb2_grpc.SchedulerStub(channel)

    run_worker(stub, client, worker_id=worker_id, heartbeat_interval_seconds=heartbeat_seconds)


if __name__ == "__main__":
    main()