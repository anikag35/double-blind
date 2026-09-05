-- The worker needs a real, readable file path for the rubric (to get
-- criteria descriptions/weights when prompting the judge).
ALTER TABLE runs ADD COLUMN rubric_path text NOT NULL;