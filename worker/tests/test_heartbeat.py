import time

import grpc

from worker.heartbeat import HeartbeatSender, heartbeat_while_processing


class RecordingStub:
    def __init__(self):
        self.calls = []

    def Heartbeat(self, request):
        self.calls.append(request)


class FailingStub:
    def Heartbeat(self, request):
        raise grpc.RpcError("simulated failure")


def test_sends_heartbeats_on_interval():
    stub = RecordingStub()
    sender = HeartbeatSender(stub, task_id="task_1", worker_id="worker_1", interval_seconds=0.05)

    sender.start()
    time.sleep(0.23)  # should fire ~4 times at a 0.05s interval
    sender.stop()

    assert len(stub.calls) >= 3
    assert all(c.task_id == "task_1" and c.worker_id == "worker_1" for c in stub.calls)


def test_stop_prevents_further_heartbeats():
    stub = RecordingStub()
    sender = HeartbeatSender(stub, task_id="task_1", worker_id="worker_1", interval_seconds=0.05)

    sender.start()
    time.sleep(0.12)
    sender.stop()
    count_after_stop = len(stub.calls)
    time.sleep(0.2)

    assert len(stub.calls) == count_after_stop


def test_survives_a_failing_heartbeat_call():
    sender = HeartbeatSender(FailingStub(), task_id="task_1", worker_id="worker_1", interval_seconds=0.05)
    sender.start()
    time.sleep(0.12)
    sender.stop()  # must not raise, and must not hang


def test_context_manager_starts_and_stops():
    stub = RecordingStub()
    with heartbeat_while_processing(stub, "task_1", "worker_1", interval_seconds=0.05):
        time.sleep(0.12)
    count_after_exit = len(stub.calls)
    time.sleep(0.15)

    assert count_after_exit >= 1
    assert len(stub.calls) == count_after_exit
