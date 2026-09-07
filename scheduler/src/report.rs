use crate::pb::task_result::Outcome;
use crate::pb::Verdict;

pub enum StoredResult {
    Rubric {
        composite_score: f64,
        criteria: serde_json::Value,
    },
    Pairwise {
        verdict: String,
        rationale: String,
    },
}

/// Checks the reported outcome is well-formed and matches the task's actual mode, then extracts what needs to be stored.
pub fn validate_and_extract(mode: &str, outcome: Option<Outcome>) -> Result<StoredResult, String> {
    let outcome = outcome.ok_or_else(|| "outcome is required".to_string())?;

    match (mode, outcome) {
        ("rubric", Outcome::Rubric(r)) => {
            if r.criteria.is_empty() {
                return Err("rubric result has no criteria".to_string());
            }
            let criteria = serde_json::Value::Array(
                r.criteria
                    .iter()
                    .map(|c| serde_json::json!({"name": c.name, "score": c.score, "rationale": c.rationale}))
                    .collect(),
            );
            Ok(StoredResult::Rubric {
                composite_score: r.composite_score,
                criteria,
            })
        }
        ("pairwise", Outcome::Pairwise(p)) => {
            let verdict = match Verdict::try_from(p.verdict).unwrap_or(Verdict::Unspecified) {
                Verdict::ModelA => "model_a",
                Verdict::ModelB => "model_b",
                Verdict::Tie => "tie",
                Verdict::Unspecified => return Err("verdict is required".to_string()),
            };
            Ok(StoredResult::Pairwise {
                verdict: verdict.to_string(),
                rationale: p.rationale,
            })
        }
        (mode, Outcome::Rubric(_)) => Err(format!("task mode is {mode} but got a rubric result")),
        (mode, Outcome::Pairwise(_)) => Err(format!("task mode is {mode} but got a pairwise result")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pb::{CriterionScore, PairwiseResult, RubricResult};

    #[test]
    fn extracts_rubric_result() {
        let outcome = Outcome::Rubric(RubricResult {
            composite_score: 4.5,
            criteria: vec![CriterionScore {
                name: "correctness".into(),
                score: 5,
                rationale: "accurate".into(),
            }],
        });
        match validate_and_extract("rubric", Some(outcome)).unwrap() {
            StoredResult::Rubric { composite_score, criteria } => {
                assert_eq!(composite_score, 4.5);
                assert_eq!(criteria[0]["name"], "correctness");
                assert_eq!(criteria[0]["score"], 5);
            }
            _ => panic!("expected a rubric result"),
        }
    }

    #[test]
    fn rejects_rubric_result_with_no_criteria() {
        let outcome = Outcome::Rubric(RubricResult { composite_score: 4.5, criteria: vec![] });
        assert!(validate_and_extract("rubric", Some(outcome)).is_err());
    }

    #[test]
    fn extracts_pairwise_result() {
        let outcome = Outcome::Pairwise(PairwiseResult {
            verdict: Verdict::ModelB as i32,
            rationale: "clearer".into(),
        });
        match validate_and_extract("pairwise", Some(outcome)).unwrap() {
            StoredResult::Pairwise { verdict, rationale } => {
                assert_eq!(verdict, "model_b");
                assert_eq!(rationale, "clearer");
            }
            _ => panic!("expected a pairwise result"),
        }
    }

    #[test]
    fn rejects_unspecified_verdict() {
        let outcome = Outcome::Pairwise(PairwiseResult {
            verdict: Verdict::Unspecified as i32,
            rationale: "".into(),
        });
        assert!(validate_and_extract("pairwise", Some(outcome)).is_err());
    }

    #[test]
    fn rejects_missing_outcome() {
        assert!(validate_and_extract("rubric", None).is_err());
    }

    #[test]
    fn rejects_rubric_result_for_pairwise_task() {
        let outcome = Outcome::Rubric(RubricResult {
            composite_score: 4.5,
            criteria: vec![CriterionScore { name: "x".into(), score: 3, rationale: "y".into() }],
        });
        assert!(validate_and_extract("pairwise", Some(outcome)).is_err());
    }

    #[test]
    fn rejects_pairwise_result_for_rubric_task() {
        let outcome = Outcome::Pairwise(PairwiseResult {
            verdict: Verdict::ModelA as i32,
            rationale: "x".into(),
        });
        assert!(validate_and_extract("rubric", Some(outcome)).is_err());
    }
}
