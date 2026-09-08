import threading
from contextlib import contextmanager

import grpc

from worker import blind_pb2


class HeartbeatSender:
    """Sends a Heartbeat on a fixed interval, from a background thread, until stopped. Best-effort: a failed heartbeat is swallowed rather than crashing the worker, since a missed heartbeat just risks the task being reclaimed, which is already the system's designed recovery path."""

    def __init__(self, stub, task_id: str, worker_id: str, interval_seconds: float = 5.0):
        self._stub = stub
        self._task_id = task_id
        self._worker_id = worker_id
        self._interval = interval_seconds
        self._stop_event = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def start(self) -> None:
        self._thread.start()

    def stop(self) -> None:
        self._stop_event.set()
        self._thread.join()

    def _run(self) -> None:
        # wait() returns True if stop_event fires before the interval elapses
        # False on timeout so this loop naturally exits as soon as stop() is called
        while not self._stop_event.wait(self._interval):
            try:
                self._stub.Heartbeat(
                    blind_pb2.HeartbeatRequest(task_id=self._task_id, worker_id=self._worker_id)
                )
            except grpc.RpcError:
                pass


@contextmanager
def heartbeat_while_processing(stub, task_id: str, worker_id: str, interval_seconds: float = 5.0):
    sender = HeartbeatSender(stub, task_id, worker_id, interval_seconds)
    sender.start()
    try:
        yield
    finally:
        sender.stop()