pub fn judge_exclusion(judge: &str, models: &[String]) -> Result<(), String> {
    if models.iter().any(|m| m == judge) {
        return Err(format!(
            "judge ({judge}) must not also be one of the evaluated models"
        ));
    }
    Ok(())
}

/// Pairwise mode compares exactly two models head-to-head (MVP scope:
/// "two models at a time") — a run with any other count is rejected.
pub fn pairwise_model_count(models: &[String]) -> Result<(), String> {
    if models.len() != 2 {
        return Err(format!(
            "pairwise mode requires exactly 2 models, got {}",
            models.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_judge_not_in_models() {
        let models = vec!["claude".to_string(), "gemini".to_string()];
        assert!(judge_exclusion("gpt-5", &models).is_ok());
    }

    #[test]
    fn rejects_judge_in_models() {
        let models = vec!["claude".to_string(), "gemini".to_string()];
        let err = judge_exclusion("claude", &models).unwrap_err();
        assert!(err.contains("claude"));
    }

    #[test]
    fn allows_exactly_two_models_for_pairwise() {
        let models = vec!["claude".to_string(), "gemini".to_string()];
        assert!(pairwise_model_count(&models).is_ok());
    }

    #[test]
    fn rejects_one_model_for_pairwise() {
        let models = vec!["claude".to_string()];
        assert!(pairwise_model_count(&models).is_err());
    }

    #[test]
    fn rejects_more_than_two_models_for_pairwise() {
        let models = vec!["claude".to_string(), "gemini".to_string(), "chatgpt".to_string()];
        assert!(pairwise_model_count(&models).is_err());
    }
}
