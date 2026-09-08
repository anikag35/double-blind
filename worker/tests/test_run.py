import tempfile
import time

from worker import blind_pb2
from worker.fake_client import FakeClient
from worker.run import run_worker

RUBRIC_YAML = "scale: 1-5\ncriteria:\n  - name: correctness\n    description: x\n    weight: 1\n"


def write_rubric_file() -> str:
    f = tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False)
    f.write(RUBRIC_YAML)
    f.close()
    return f.name


def make_rubric_task() -> blind_pb2.Task:
    return blind_pb2.Task(
        available=True,
        task_id="task_1",
        run_id="run_1",
        mode=blind_pb2.MODE_RUBRIC,
        judge="gpt-5",
        rubric_path=write_rubric_file(),
        prompt="p",
        prompt_hash="ph",
        model="claude",
    )


class FakeStub:
    # Records calls and returns unavailable until available_after polls have happened, then returns `task` for the rest

    def __init__(self, task: blind_pb2.Task, available_after: int = 0):
        self._task = task
        self._available_after = available_after
        self.get_task_requests = []
        self.report_calls = []
        self.heartbeat_calls = []

    @property
    def get_task_calls(self) -> int:
        return len(self.get_task_requests)

    def GetTask(self, request):
        seen = len(self.get_task_requests)
        self.get_task_requests.append(request)
        if seen >= self._available_after:
            return self._task
        return blind_pb2.Task(available=False)

    def ReportResult(self, request):
        self.report_calls.append(request)
        return blind_pb2.Empty()

    def Heartbeat(self, request):
        self.heartbeat_calls.append(request)
        return blind_pb2.Empty()


class SlowFakeClient(FakeClient):
    # Same as FakeClient but takes real wall-clock time, so a test can catch a heartbeat firing during processing

    def generate(self, model, prompt):
        time.sleep(0.15)
        return super().generate(model, prompt)


def test_processes_one_task_and_stops_at_max_tasks():
    stub = FakeStub(make_rubric_task())

    run_worker(stub, FakeClient(), worker_id="worker-1", max_tasks=1, poll_interval_seconds=0.01)

    assert len(stub.report_calls) == 1
    assert stub.report_calls[0].task_id == "task_1"


def test_polls_until_a_task_becomes_available():
    stub = FakeStub(make_rubric_task(), available_after=3)

    run_worker(stub, FakeClient(), worker_id="worker-1", max_tasks=1, poll_interval_seconds=0.01)

    assert stub.get_task_calls >= 4  # 3 unavailable polls + the one that succeeded
    assert len(stub.report_calls) == 1


def test_sends_heartbeats_while_processing_a_slow_task():
    stub = FakeStub(make_rubric_task())

    run_worker(
        stub, SlowFakeClient(), worker_id="worker-1", max_tasks=1,
        poll_interval_seconds=0.01, heartbeat_interval_seconds=0.05,
    )

    assert len(stub.heartbeat_calls) >= 1
    assert all(c.task_id == "task_1" and c.worker_id == "worker-1" for c in stub.heartbeat_calls)


def test_uses_given_worker_id_consistently():
    stub = FakeStub(make_rubric_task(), available_after=2)

    run_worker(stub, FakeClient(), worker_id="worker-xyz", max_tasks=1, poll_interval_seconds=0.01)

    assert all(r.worker_id == "worker-xyz" for r in stub.get_task_requests)
    assert stub.report_calls[0].task_id == "task_1"