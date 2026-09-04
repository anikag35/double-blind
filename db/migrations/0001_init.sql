--- Core queue tables: runs, tasks, results.

CREATE TABLE runs (
    run_id       text PRIMARY KEY,
    models       jsonb NOT NULL,
    prompts_path text NOT NULL,
    judge        text NOT NULL,
    rubric_hash  text NOT NULL,
    mode         text NOT NULL CHECK (mode IN ('rubric', 'pairwise')),
    compare      boolean NOT NULL DEFAULT false,
    status       text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'done')),
    created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE tasks (
    task_id        text PRIMARY KEY,
    run_id         text NOT NULL REFERENCES runs (run_id),
    status         text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'claimed', 'done')),
    claimed_by     text,
    last_heartbeat timestamptz,
    payload        jsonb NOT NULL,
    created_at     timestamptz NOT NULL DEFAULT now()
);

-- The claim query filters on exactly these two columns every time a worker polls.
CREATE INDEX tasks_claim_idx ON tasks (status, last_heartbeat);

-- The scheduler looks up a run's tasks when computing a leaderboard.
CREATE INDEX tasks_run_id_idx ON tasks (run_id);

-- Rubric mode: one row per (model, prompt) task, per implementation-plan.md.
CREATE TABLE rubric_results (
    task_id         text PRIMARY KEY REFERENCES tasks (task_id),
    composite_score double precision NOT NULL,
    criteria        jsonb NOT NULL,
    completed_at    timestamptz NOT NULL DEFAULT now()
);

-- Pairwise mode: one row per (model_a, model_b, prompt, condition) task.
CREATE TABLE pairwise_results (
    task_id      text PRIMARY KEY REFERENCES tasks (task_id),
    verdict      text NOT NULL CHECK (verdict IN ('model_a', 'model_b', 'tie')),
    rationale    text NOT NULL,
    completed_at timestamptz NOT NULL DEFAULT now()
);