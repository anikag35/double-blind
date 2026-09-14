use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ModelsFile {
    models: Vec<String>,
}

/// Parses a models.yaml file's content into a plain list of model names
pub fn parse_models(content: &str) -> Result<Vec<String>, String> {
    let parsed: ModelsFile =
        serde_norway::from_str(content).map_err(|e| format!("invalid models file: {e}"))?;
    if parsed.models.is_empty() {
        return Err("models file lists no models".to_string());
    }
    Ok(parsed.models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_models_list() {
        let content = "models:\n  - claude\n  - gemini\n  - chatgpt\n";
        assert_eq!(parse_models(content).unwrap(), vec!["claude", "gemini", "chatgpt"]);
    }

    #[test]
    fn rejects_empty_models_list() {
        assert!(parse_models("models: []\n").is_err());
    }

    #[test]
    fn rejects_malformed_yaml() {
        assert!(parse_models("not: valid: yaml: at: all: [").is_err());
    }
}