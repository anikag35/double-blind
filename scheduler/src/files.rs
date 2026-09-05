use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Criterion {
    pub name: String,
    pub description: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rubric {
    pub scale: String,
    pub criteria: Vec<Criterion>,
}

pub fn parse_rubric(content: &str) -> Result<Rubric, String> {
    let parsed: Rubric =
        serde_norway::from_str(content).map_err(|e| format!("invalid rubric file: {e}"))?;
    if parsed.criteria.is_empty() {
        return Err("rubric file lists no criteria".to_string());
    }
    Ok(parsed)
}

#[derive(Debug, Deserialize)]
struct PromptLine {
    prompt: String,
}

/// Parses a JSONL file: one `{"prompt": "..."}` object per non-blank line.
pub fn parse_prompts(content: &str) -> Result<Vec<String>, String> {
    let prompts: Result<Vec<String>, String> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
        .map(|(i, line)| {
            let parsed: PromptLine = serde_json::from_str(line)
                .map_err(|e| format!("invalid prompt on line {}: {e}", i + 1))?;
            Ok(parsed.prompt)
        })
        .collect();
    let prompts = prompts?;
    if prompts.is_empty() {
        return Err("prompts file has no prompts".to_string());
    }
    Ok(prompts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rubric_matching_design_doc_example() {
        let content = r#"
scale: 1-5
criteria:
  - name: correctness
    description: Is the response factually and technically accurate?
    weight: 1
  - name: completeness
    description: Does the response fully address the prompt?
    weight: 1
"#;
        let rubric = parse_rubric(content).unwrap();
        assert_eq!(rubric.scale, "1-5");
        assert_eq!(rubric.criteria.len(), 2);
        assert_eq!(rubric.criteria[0].name, "correctness");
        assert_eq!(rubric.criteria[0].weight, 1.0);
    }

    #[test]
    fn rejects_rubric_with_no_criteria() {
        let content = "scale: 1-5\ncriteria: []\n";
        assert!(parse_rubric(content).is_err());
    }

    #[test]
    fn parses_prompts_jsonl_skipping_blank_lines() {
        let content = "{\"prompt\": \"first\"}\n\n{\"prompt\": \"second\"}\n";
        assert_eq!(parse_prompts(content).unwrap(), vec!["first", "second"]);
    }

    #[test]
    fn rejects_prompts_file_with_no_prompts() {
        assert!(parse_prompts("\n\n").is_err());
    }

    #[test]
    fn rejects_malformed_prompt_line_with_line_number() {
        let content = "{\"prompt\": \"ok\"}\n{not json}\n";
        let err = parse_prompts(content).unwrap_err();
        assert!(err.contains("line 2"), "error should cite line 2: {err}");
    }
}
