#!/usr/bin/env python3
"""
Local multi-worker scale-out benchmark (manual - not part of pytest/CI).

Submits the same NUM_TASKS-task rubric run twice: once processed by 1
worker, once by WORKER_COUNT workers, and reports the wall-clock time
for each. Uses DelayedFakeClient (deterministic, free, fixed per-call
delay) rather than real APIs, so the benchmark measures what it's
actually meant to: whether the scheduler/queue's horizontal scaling
works, not real-provider latency variance.

Run: python scripts/scale_benchmark.py
"""
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))

import socket

NUM_TASKS = int(os.environ.get("BENCHMARK_NUM_TASKS", 24))
WORKER_COUNT = int(os.environ.get("BENCHMARK_WORKER_COUNT", 6))
DELAY_SECONDS = float(os.environ.get("BENCHMARK_DELAY_SECONDS", 1.0))


def log(msg: str) -> None:
    print(f"[scale-benchmark] {msg}", flush=True)


def load_env_file() -> None:
    env_path = REPO_ROOT / ".env"
    for line in env_path.read_text().splitlines():
        if "=" in line and not line.startswith("#"):
            key, value = line.split("=", 1)
            os.environ.setdefault(key.strip(), value.strip())


def psql(sql: str) -> str:
    result = subprocess.run(
        ["psql", os.environ["DATABASE_URL"], "-t", "-A", "-c", sql],
        capture_output=True, text=True, check=True,
    )
    return result.stdout.strip()


def wait_for_scheduler(proc: subprocess.Popen, timeout: float = 60) -> None:
    # Plain TCP check, not gRPC - see chaos_test_tier2.py for why the parent
    # must not touch the grpc module before forking worker subprocesses.
    deadline = time.time() + timeout
    while time.time() < deadline:
        if proc.poll() is not None:
            raise RuntimeError("scheduler process exited before coming up")
        try:
            with socket.create_connection(("localhost", 50051), timeout=1):
                return
        except OSError:
            time.sleep(0.2)
    raise TimeoutError("scheduler never became ready")


def run_pass(worker_count: int, log_dir: Path, prompts_path: Path) -> float:
    """Submits one run, starts worker_count workers, waits for completion, returns elapsed seconds."""
    import grpc
    from worker import blind_pb2, blind_pb2_grpc

    stub = blind_pb2_grpc.SchedulerStub(grpc.insecure_channel("localhost:50051"))
    response = stub.SubmitRun(blind_pb2.RunRequest(
        models=["claude-haiku-4-5"],
        prompts_path=str(prompts_path),
        judge="claude-sonnet-5",
        mode=blind_pb2.MODE_RUBRIC,
    ))
    run_id = response.run_id
    log(f"  run {run_id}: {NUM_TASKS} tasks, {worker_count} worker(s)")

    worker_env = {
        **os.environ,
        "WORKER_CLIENT": "delayed-fake",
        "FAKE_CLIENT_DELAY_SECONDS": str(DELAY_SECONDS),
    }
    workers = []
    for i in range(worker_count):
        workers.append(subprocess.Popen(
            [sys.executable, "-m", "worker"],
            cwd=REPO_ROOT,
            env=worker_env,
            stdout=open(log_dir / f"worker_{worker_count}w_{i}.log", "w"),
            stderr=subprocess.STDOUT,
        ))

    start = time.monotonic()
    while True:
        out = psql(f"SELECT COUNT(*) FROM tasks WHERE run_id = '{run_id}' AND status != 'done'")
        if out == "0":
            break
        time.sleep(0.2)
    elapsed = time.monotonic() - start

    for w in workers:
        w.terminate()
    for w in workers:
        try:
            w.wait(timeout=5)
        except subprocess.TimeoutExpired:
            w.kill()

    psql(f"DELETE FROM rubric_results WHERE task_id IN (SELECT task_id FROM tasks WHERE run_id = '{run_id}')")
    psql(f"DELETE FROM tasks WHERE run_id = '{run_id}'")
    psql(f"DELETE FROM runs WHERE run_id = '{run_id}'")

    return elapsed


def main() -> None:
    load_env_file()
    log_dir = Path(tempfile.mkdtemp(prefix="blind_scale_benchmark_"))
    prompts_path = log_dir / "prompts.jsonl"
    prompts_path.write_text("".join(f'{{"prompt": "p{i}"}}\n' for i in range(NUM_TASKS)))
    log(f"process logs under {log_dir}")
    log(f"{NUM_TASKS} tasks, {DELAY_SECONDS}s simulated latency per model/judge call")

    scheduler = subprocess.Popen(
        ["cargo", "run", "--bin", "scheduler"],
        cwd=REPO_ROOT / "scheduler",
        env=os.environ,
        stdout=open(log_dir / "scheduler.log", "w"),
        stderr=subprocess.STDOUT,
    )
    try:
        wait_for_scheduler(scheduler)
        log("scheduler is up")

        results = {}
        for worker_count in (1, WORKER_COUNT):
            results[worker_count] = run_pass(worker_count, log_dir, prompts_path)

        log("")
        log("=== Results ===")
        baseline = results[1]
        for worker_count, elapsed in results.items():
            speedup = baseline / elapsed
            log(f"  {worker_count} worker(s): {elapsed:.1f}s  ({speedup:.1f}x vs. 1 worker)")
    finally:
        scheduler.terminate()
        try:
            scheduler.wait(timeout=5)
        except subprocess.TimeoutExpired:
            scheduler.kill()


if __name__ == "__main__":
    main()