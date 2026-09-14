# Double Blind

A distributed, fault-tolerant LLM evaluation platform. CLI command: `blind`.

Judge scoring is blinded to both model identity and response position/order, to control for the position bias and self-preference bias that LLM-as-judge setups are known to suffer from.

## Status

Core system is built and working end-to-end: scheduler, worker, and CLI all verified together against a real Postgres database, using a deterministic `FakeClient` in place of real model APIs.

- [x] Scheduler (Rust, gRPC): `SubmitRun`, `GetTask`, `ReportResult`, `Heartbeat`, `GetRun` — all implemented and tested
- [x] Worker (Python): claim/process/report loop, heartbeat sender, blind/unblind pairwise judging
- [x] `blind` CLI (Rust): `blind run` and `blind show`, verified end-to-end
- [x] Postgres-backed queue with automatic dead-worker reclaim
- [ ] Chaos test (kill a worker mid-task, verify no lost/duplicated results) — in progress
- [ ] Real model/judge clients (currently `FakeClient` only)
- [ ] Pairwise mode's CLI output (win/loss/tie table)
- [ ] Local multi-worker scale-out benchmark

## Architecture

- **Scheduler** (Rust): expands a run request into individual tasks, validates that the judge isn't among the models being evaluated, computes content-addressed task IDs, and writes tasks to the Postgres-backed queue. Exposes the gRPC service that both the CLI and workers talk to. Also serves `GetRun`, computing the leaderboard as a plain SQL aggregate (mean score per model) directly against Postgres.
- **Queue** (Postgres): a `tasks` table. A worker that misses its heartbeat has its task automatically reclaimed by the same claim query used for fresh work (reassignment timeout defaults to 30s, configurable via `REASSIGNMENT_TIMEOUT_SECONDS`).
- **Workers** (Python): pull tasks over gRPC, call the contestant model and the judge model, score against the rubric, report results. Heartbeat interval defaults to 5s, configurable via `HEARTBEAT_INTERVAL_SECONDS`.
- **Storage** (Postgres): task status for checkpointing, plus completed scores, verdicts, and per-criterion rationale.
- **CLI** (`blind`, Rust): a gRPC client to the scheduler. `blind run` submits an eval and polls until it's done; `blind show <run_id>` prints a completed run's leaderboard.

## Local dev

```
cargo run --bin scheduler         # starts the gRPC server
python -m worker                  # run in one or more terminals and each is a worker process
blind run --prompts tasks.jsonl --judge gpt-5
```