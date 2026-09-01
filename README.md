# Double Blind

A distributed, fault-tolerant LLM evaluation platform. CLI command: `blind`.

Judge scoring is blinded to both model identity and response position/order, to control for the position bias and self-preference bias that LLM-as-judge setups are known to suffer from.

## Planned architecture (temporary)

- Scheduler / worker pool (Rust) — task distribution, per-provider rate limiting, retry-with-backoff
- Checkpointing / persistence (Postgres) — resumable runs, idempotent task execution
- Eval / stats layer (Python) — confidence intervals, significance testing, judge calibration, blinded scoring
- Deployment — multi-worker scale-out, throughput benchmarks
