# Double Blind

A distributed, fault-tolerant LLM evaluation platform. CLI command: `blind`.

Judge scoring is blinded to both model identity and response position/order, to control for the position bias and self-preference bias that LLM-as-judge setups are known to suffer from.

## Status

Design complete — see [`design-doc.md`](./design-doc.md). Currently building Weeks 2–3 of the timeline (core scheduler) — see [`implementation-plan.md`](./implementation-plan.md) for the detailed build steps, architecture decisions, and open questions.

## Getting started

```
docker compose up -d postgres     # local Postgres — the only containerized service for now
cargo run --bin scheduler         # starts the scheduler's gRPC server
python -m worker                  # run in one or more terminals — each is a worker process
blind run --models --prompts tasks.jsonl   # the CLI (a gRPC client) submits a run and prints the leaderboard
```

Full setup details: `implementation-plan.md`.

## Architecture

- **Scheduler** (Rust) — expands a run request into individual tasks, validates that the judge isn't among the models being evaluated, computes content-addressed task IDs, and writes tasks to the Postgres-backed queue. Exposes the gRPC service (`SubmitRun`, `GetTask`, `ReportResult`, `Heartbeat`) that both the CLI and workers talk to.
- **Queue** (Postgres) — a `tasks` table claimed via `SELECT ... FOR UPDATE SKIP LOCKED`; a worker that misses its heartbeat for 30s has its task automatically reclaimed by the same query.
- **Workers** (Python) — pull tasks over gRPC, call the contestant model and the judge model (OpenAI, Anthropic, Gemini, Grok), score against the rubric, report results.
- **Storage** (Postgres) — task status for checkpointing, plus completed scores, verdicts, and per-criterion rationale.
- **Stats layer** (Python) — bootstrap confidence intervals for rubric mode; McNemar's test for pairwise blinded-vs-unblinded comparisons.
- **CLI** (`blind`, Rust) — a gRPC client to the scheduler; `blind run` to kick off an eval, `blind show` to view results.

Full design rationale, the worked CLI walkthrough, and every resolved design decision: [`design-doc.md`](./design-doc.md).
