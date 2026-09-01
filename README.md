# Double Blind

A distributed, fault-tolerant LLM evaluation platform. CLI command: `blind`.

Judge scoring is blinded to both model identity and response position/order, to control for the position bias and self-preference bias that LLM-as-judge setups are known to suffer from.

## Architecture

- **Scheduler** (Rust): expands a run request into individual tasks, validates that the judge isn't among the models being evaluated, computes content-addressed task IDs, and writes tasks to the Postgres-backed queue. Exposes the gRPC service that both the CLI and workers talk to.
- **Queue** (Postgres): a `tasks` table. A worker that misses its heartbeat for 30s has its task automatically reclaimed by the same query.
- **Workers** (Python): pull tasks over gRPC, call the contestant model and the judge model, score against the rubric, report results.
- **Storage** (Postgres): task status for checkpointing, plus completed scores, verdicts, and per-criterion rationale.
- **Stats layer** (Python): bootstrap confidence intervals for rubric mode. Uses McNemar's test for pairwise blinded-vs-unblinded comparisons.
- **CLI** (`blind`, Rust): a gRPC client to the scheduler. Use `blind run` to kick off an eval and `blind show` to view results.
