import os
import socket


def generate_worker_id() -> str:
    """Builds a human-readable, likely-unique worker_id: hostname + process id, so it's identifiable in logs and distinct from other workers on the same or different machines."""
    return f"worker-{socket.gethostname()}-{os.getpid()}"
