import os
import socket

from worker.identity import generate_worker_id


def test_worker_id_includes_hostname_and_pid():
    worker_id = generate_worker_id()
    assert socket.gethostname() in worker_id
    assert str(os.getpid()) in worker_id


def test_worker_id_starts_with_worker_prefix():
    assert generate_worker_id().startswith("worker-")
