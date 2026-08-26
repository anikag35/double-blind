# Double Blind

A distributed, fault-tolerant LLM evaluation platform. CLI command: `blind`.

Judge scoring is blinded to both model identity and response position/order, to control for the position bias and self-preference bias that LLM-as-judge setups are known to suffer from.

## Status

Not yet started — see the build plan in `~/Desktop/portfolio-refresh/eval-platform-timeline.md`.

## Getting started

1. Follow `~/Desktop/portfolio-refresh/git-setup-guide.md` to initialize this as a git repo and push it to GitHub as `double-blind`.
2. Work through Week 1 of the timeline: design doc, architecture sketch, tech stack decision, CI skeleton.

## Planned architecture (fill in as design solidifies)

- Scheduler / worker pool (Rust) — task distribution, per-provider rate limiting, retry-with-backoff
- Checkpointing / persistence (Postgres) — resumable runs, idempotent task execution
- Eval / stats layer (Python) — confidence intervals, significance testing, judge calibration, blinded scoring
- Deployment — multi-worker scale-out, throughput benchmarks
