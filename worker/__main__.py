import grpc

from worker import blind_pb2_grpc
from worker.fake_client import FakeClient
from worker.identity import generate_worker_id
from worker.run import run_worker, scheduler_address


def main() -> None:
    address = scheduler_address()
    worker_id = generate_worker_id()
    print(f"worker: {worker_id} connecting to scheduler at {address}")

    channel = grpc.insecure_channel(address)
    stub = blind_pb2_grpc.SchedulerStub(channel)
    # FakeClient stands in for real model/judge clients until those are built.
    client = FakeClient()

    run_worker(stub, client, worker_id=worker_id)


if __name__ == "__main__":
    main()
