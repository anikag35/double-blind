use crate::ids::{hash_content, model_a_shown_first, pairwise_task_id, rubric_task_id, sort_pair};
use serde_json::Value;

pub struct TaskRow {
    pub task_id: String,
    pub payload: Value,
}

/// One task per (model, prompt) pair.
pub fn expand_rubric(
    run_id: &str,
    models: &[String],
    prompts: &[String],
    judge: &str,
    rubric_hash: &str,
) -> Vec<TaskRow> {
    let mut tasks = Vec::with_capacity(models.len() * prompts.len());
    for model in models {
        for prompt in prompts {
            let prompt_hash = hash_content(prompt);
            let task_id = rubric_task_id(run_id, model, &prompt_hash, judge, rubric_hash);
            let payload = serde_json::json!({
                "model": model,
                "prompt": prompt,
                "prompt_hash": prompt_hash,
            });
            tasks.push(TaskRow { task_id, payload });
        }
    }
    tasks
}

/// One task per (prompt, condition) pair. `compare = false` runs only the
/// blind condition; `compare = true` adds the unblind condition on top.
pub fn expand_pairwise(
    run_id: &str,
    model_a: &str,
    model_b: &str,
    prompts: &[String],
    judge: &str,
    compare: bool,
) -> Vec<TaskRow> {
    let (model_a, model_b) = sort_pair(model_a, model_b);
    let conditions: &[&str] = if compare { &["blind", "unblind"] } else { &["blind"] };

    let mut tasks = Vec::with_capacity(prompts.len() * conditions.len());
    for prompt in prompts {
        let prompt_hash = hash_content(prompt);
        for &condition in conditions {
            let task_id = pairwise_task_id(run_id, model_a, model_b, &prompt_hash, judge, condition);
            let shown_first = model_a_shown_first(&task_id);
            let payload = serde_json::json!({
                "model_a": model_a,
                "model_b": model_b,
                "prompt": prompt,
                "prompt_hash": prompt_hash,
                "condition": condition,
                "model_a_shown_first": shown_first,
            });
            tasks.push(TaskRow { task_id, payload });
        }
    }
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompts() -> Vec<String> {
        vec!["p1".to_string(), "p2".to_string(), "p3".to_string()]
    }

    #[test]
    fn rubric_expands_to_models_times_prompts() {
        let models = vec!["claude".to_string(), "gemini".to_string()];
        let tasks = expand_rubric("run_1", &models, &prompts(), "gpt-5", "rh1");
        assert_eq!(tasks.len(), 2 * 3);
    }

    #[test]
    fn rubric_task_ids_are_unique() {
        let models = vec!["claude".to_string(), "gemini".to_string()];
        let tasks = expand_rubric("run_1", &models, &prompts(), "gpt-5", "rh1");
        let mut ids: Vec<&str> = tasks.iter().map(|t| t.task_id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), tasks.len());
    }

    #[test]
    fn pairwise_blind_only_by_default() {
        let tasks = expand_pairwise("run_1", "claude", "gemini", &prompts(), "gpt-5", false);
        assert_eq!(tasks.len(), 3); // one per prompt, blind only
        for t in &tasks {
            assert_eq!(t.payload["condition"], "blind");
        }
    }

    #[test]
    fn pairwise_compare_runs_both_conditions() {
        let tasks = expand_pairwise("run_1", "claude", "gemini", &prompts(), "gpt-5", true);
        assert_eq!(tasks.len(), 6); // 2 conditions per prompt
        let blind_count = tasks.iter().filter(|t| t.payload["condition"] == "blind").count();
        let unblind_count = tasks.iter().filter(|t| t.payload["condition"] == "unblind").count();
        assert_eq!(blind_count, 3);
        assert_eq!(unblind_count, 3);
    }

    #[test]
    fn pairwise_task_id_and_payload_order_agree_regardless_of_input_order() {
        // Submitting (gemini, claude) or (claude, gemini) must produce the
        // same task_ids AND the same model_a/model_b in the payload
        let forward = expand_pairwise("run_1", "claude", "gemini", &prompts(), "gpt-5", false);
        let reversed = expand_pairwise("run_1", "gemini", "claude", &prompts(), "gpt-5", false);

        for (f, r) in forward.iter().zip(reversed.iter()) {
            assert_eq!(f.task_id, r.task_id);
            assert_eq!(f.payload["model_a"], r.payload["model_a"]);
            assert_eq!(f.payload["model_b"], r.payload["model_b"]);
        }
    }

    #[test]
    fn model_a_shown_first_matches_ids_module_for_same_task_id() {
        let tasks = expand_pairwise("run_1", "claude", "gemini", &prompts(), "gpt-5", false);
        for t in &tasks {
            let expected = model_a_shown_first(&t.task_id);
            assert_eq!(t.payload["model_a_shown_first"], expected);
        }
    }
}