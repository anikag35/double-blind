#!/usr/bin/env python3
"""
MANUAL TEST

1. Starts a real scheduler and two real worker processes
2. Submits a run, waits for a worker to claim the task and send at least one heartbeat
3. SIGKILLs that exact worker process 
4. Waits past the reassignment window 
5. Verifies the surviving worker completes the task with exactly one result row 

Run: python scripts/chaos_test_tier2.py

Uses a short REASSIGNMENT_TIMEOUT_SECONDS by default for fast iteration
"""
import os
import signal
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))

REASSIGNMENT_TIMEOUT_SECONDS = int(os.environ.get("REASSIGNMENT_TIMEOUT_SECONDS", 5))
HEARTBEAT_INTERVAL_SECONDS = int(os.environ.get("HEARTBEAT_INTERVAL_SECONDS", 2))


def log(msg: str) -> None:
    print(f"[chaos-tier2] {msg}", flush=True)


def load_env_file() -> None:
    env_path = REPO_ROOT / ".env"
    for line in env_path.read_text().splitlines():
        if "=" in line and not line.startswith("#"):
            key, value = line.split("=", 1)
            os.environ.setdefault(key, value)


def psql(sql: str) -> str:
    result = subprocess.run(
        ["psql", os.environ["DATABASE_URL"], "-t", "-A", "-c", sql],
        capture_output=True, text=True, check=True,
    )
    return result.stdout.strip()


def wait_for_scheduler(proc: subprocess.Popen, timeout: float = 60) -> None:
    # Parent shouldn't touch the grpc module before forking the worker subprocesses below because gRPC's C-core
    # doesn't survive fork() cleanly once it's been initialized in the parent
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


def main() -> None:
    load_env_file()

    log_dir = Path(tempfile.mkdtemp(prefix="blind_chaos_tier2_"))
    prompts_path = log_dir / "prompts.jsonl"
    prompts_path.write_text('{"prompt": "what is the capital of France?"}\n')
    log(f"process logs will be written under {log_dir}")

    procs: dict[str, subprocess.Popen] = {}
    try:
        log(f"starting scheduler (reassignment timeout: {REASSIGNMENT_TIMEOUT_SECONDS}s)...")
        scheduler_env = {**os.environ, "REASSIGNMENT_TIMEOUT_SECONDS": str(REASSIGNMENT_TIMEOUT_SECONDS)}
        procs["scheduler"] = subprocess.Popen(
            ["cargo", "run", "--bin", "scheduler"],
            cwd=REPO_ROOT / "scheduler",
            env=scheduler_env,
            stdout=open(log_dir / "scheduler.log", "w"),
            stderr=subprocess.STDOUT,
        )
        wait_for_scheduler(procs["scheduler"])
        log("scheduler is up")

        worker_env = {**os.environ, "HEARTBEAT_INTERVAL_SECONDS": str(HEARTBEAT_INTERVAL_SECONDS)}
        log("starting worker A and worker B...")
        for name in ("worker_a", "worker_b"):
            procs[name] = subprocess.Popen(
                [sys.executable, "-m", "worker"],
                cwd=REPO_ROOT,
                env=worker_env,
                stdout=open(log_dir / f"{name}.log", "w"),
                stderr=subprocess.STDOUT,
            )
        time.sleep(2)  # let both connect before we submit work

        import grpc
        from worker import blind_pb2, blind_pb2_grpc

        stub = blind_pb2_grpc.SchedulerStub(grpc.insecure_channel("localhost:50051"))
        response = stub.SubmitRun(blind_pb2.RunRequest(
            models=["claude"],
            prompts_path=str(prompts_path),
            judge="gpt-5",
            mode=blind_pb2.MODE_RUBRIC,
        ))
        run_id = response.run_id
        log(f"submitted run {run_id}")

        log("waiting for a worker to claim the task and send a heartbeat...")
        task_id = original_claimant = None
        deadline = time.time() + 20
        while time.time() < deadline:
            out = psql(
                f"SELECT task_id, claimed_by FROM tasks "
                f"WHERE run_id = '{run_id}' AND status = 'claimed' AND last_heartbeat IS NOT NULL"
            )
            if out:
                task_id, original_claimant = out.split("|")
                break
            time.sleep(0.5)
        if not task_id:
            raise RuntimeError("no worker claimed the task in time")
        log(f"task {task_id} claimed by {original_claimant}")

        # worker_id format is worker-<hostname>-<pid>
        # the pid is always the last hyphen-delimited segment, however many hyphens the hostname has
        claimant_pid = int(original_claimant.rsplit("-", 1)[1])
        log(f"KILLING worker pid {claimant_pid} with SIGKILL (no graceful shutdown)")
        os.kill(claimant_pid, signal.SIGKILL)

        log(f"waiting past the {REASSIGNMENT_TIMEOUT_SECONDS}s reassignment window...")
        time.sleep(REASSIGNMENT_TIMEOUT_SECONDS + 3)

        log("waiting for the surviving worker to complete the task...")
        status = final_claimant = None
        deadline = time.time() + 20
        while time.time() < deadline:
            out = psql(f"SELECT status, claimed_by FROM tasks WHERE task_id = '{task_id}'")
            status, final_claimant = out.split("|")
            if status == "done":
                break
            time.sleep(0.5)

        result_count = psql(f"SELECT COUNT(*) FROM rubric_results WHERE task_id = '{task_id}'")
        log(f"final status: {status}, claimed_by: {final_claimant}")
        log(f"rubric_results rows for this task: {result_count}")

        assert status == "done", f"expected task done, got status={status}"
        assert final_claimant != original_claimant, "task should have been reclaimed by a different worker"
        assert result_count == "1", f"expected exactly 1 result row, got {result_count}"

        log("PASS: unclean worker death did not lose or duplicate the result")

        psql(f"DELETE FROM rubric_results WHERE task_id = '{task_id}'")
        psql(f"DELETE FROM tasks WHERE run_id = '{run_id}'")
        psql(f"DELETE FROM runs WHERE run_id = '{run_id}'")

    finally:
        log("cleaning up processes...")
        for name, proc in procs.items():
            if proc.poll() is None:
                proc.terminate()
        for name, proc in procs.items():
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()


if __name__ == "__main__":
    main()