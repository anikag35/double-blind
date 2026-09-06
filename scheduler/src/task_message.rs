use crate::db::ClaimedTask;
use crate::pb::{Condition, Mode, Task};

/// Builds the wire-format `Task` a worker receives from a claimed DB row.
/// Which optional fields get populated depends on the run's mode.
pub fn build_task(claimed: ClaimedTask) -> Result<Task, String> {
    let get_str = |key: &str| -> Result<String, String> {
        claimed
            .payload
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("task payload missing '{key}'"))
    };

    let mut task = Task {
        available: true,
        task_id: claimed.task_id.clone(),
        run_id: claimed.run_id.clone(),
        mode: 0,
        judge: claimed.judge.clone(),
        rubric_path: claimed.rubric_path.clone(),
        prompt: get_str("prompt")?,
        prompt_hash: get_str("prompt_hash")?,
        model: String::new(),
        model_a: String::new(),
        model_b: String::new(),
        condition: Condition::Unspecified as i32,
        model_a_shown_first: false,
    };

    match claimed.mode.as_str() {
        "rubric" => {
            task.mode = Mode::Rubric as i32;
            task.model = get_str("model")?;
        }
        "pairwise" => {
            task.mode = Mode::Pairwise as i32;
            task.model_a = get_str("model_a")?;
            task.model_b = get_str("model_b")?;
            task.condition = match get_str("condition")?.as_str() {
                "blind" => Condition::Blind as i32,
                "unblind" => Condition::Unblind as i32,
                other => return Err(format!("unknown condition in payload: {other}")),
            };
            task.model_a_shown_first = claimed
                .payload
                .get("model_a_shown_first")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "task payload missing 'model_a_shown_first'".to_string())?;
        }
        other => return Err(format!("unknown mode in database: {other}")),
    }

    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claimed(mode: &str, judge: &str, rubric_path: &str, payload: serde_json::Value) -> ClaimedTask {
        ClaimedTask {
            task_id: "task_abc".to_string(),
            run_id: "run_123".to_string(),
            payload,
            mode: mode.to_string(),
            judge: judge.to_string(),
            rubric_path: rubric_path.to_string(),
        }
    }

    #[test]
    fn builds_rubric_task() {
        let payload = serde_json::json!({
            "model": "claude",
            "prompt": "hello",
            "prompt_hash": "ph1",
        });
        let task = build_task(claimed("rubric", "gpt-5", "/rubric.yaml", payload)).unwrap();

        assert!(task.available);
        assert_eq!(task.mode, Mode::Rubric as i32);
        assert_eq!(task.model, "claude");
        assert_eq!(task.prompt, "hello");
        assert_eq!(task.prompt_hash, "ph1");
        assert_eq!(task.judge, "gpt-5");
        assert_eq!(task.rubric_path, "/rubric.yaml");
        // Pairwise-only fields stay at their zero values for a rubric task.
        assert_eq!(task.model_a, "");
        assert_eq!(task.model_b, "");
        assert_eq!(task.condition, Condition::Unspecified as i32);
        assert!(!task.model_a_shown_first);
    }

    #[test]
    fn builds_pairwise_task() {
        let payload = serde_json::json!({
            "model_a": "claude",
            "model_b": "gemini",
            "prompt": "hello",
            "prompt_hash": "ph1",
            "condition": "unblind",
            "model_a_shown_first": true,
        });
        let task = build_task(claimed("pairwise", "gpt-5", "/rubric.yaml", payload)).unwrap();

        assert_eq!(task.mode, Mode::Pairwise as i32);
        assert_eq!(task.model_a, "claude");
        assert_eq!(task.model_b, "gemini");
        assert_eq!(task.condition, Condition::Unblind as i32);
        assert!(task.model_a_shown_first);
        // Rubric-only field stays empty for a pairwise task.
        assert_eq!(task.model, "");
    }

    #[test]
    fn rejects_unknown_mode() {
        let payload = serde_json::json!({"prompt": "p", "prompt_hash": "h"});
        assert!(build_task(claimed("nonsense", "gpt-5", "/r.yaml", payload)).is_err());
    }

    #[test]
    fn rejects_missing_required_field() {
        let payload = serde_json::json!({"prompt": "p"}); // missing prompt_hash
        assert!(build_task(claimed("rubric", "gpt-5", "/r.yaml", payload)).is_err());
    }

    #[test]
    fn rejects_unknown_condition() {
        let payload = serde_json::json!({
            "model_a": "a", "model_b": "b", "prompt": "p", "prompt_hash": "h",
            "condition": "sideways", "model_a_shown_first": true,
        });
        assert!(build_task(claimed("pairwise", "gpt-5", "/r.yaml", payload)).is_err());
    }
}
