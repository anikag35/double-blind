use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use sha2::{Digest, Sha256};

/// Hashes a fixed, ordered list of fields into a single hex digest.
///
/// Each field is written as an 8-byte big-endian length prefix followed by
/// its raw UTF-8 bytes, before being fed to SHA-256. The fixed-width prefix
/// (rather than a delimiter) makes the encoding unambiguous regardless of
/// what characters appear inside any field.
fn hash_fields(fields: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for field in fields {
        let bytes = field.as_bytes();
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    hex::encode(hasher.finalize())
}

/// SHA-256 hex digest of arbitrary file content (used for prompt_hash and
/// rubric_hash, per design-doc.md's task ID scheme).
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn rubric_task_id(run_id: &str, model: &str, prompt_hash: &str, judge: &str, rubric_hash: &str) -> String {
    hash_fields(&["rubric", run_id, model, prompt_hash, judge, rubric_hash])
}

/// `model_a`/`model_b` are sorted before hashing so the task_id doesn't
/// depend on which model happened to be listed first.
/// Orders a model pair consistently, independent of caller-supplied order.
/// Used both for hashing (`pairwise_task_id`) and for building task payloads,
/// so `model_a`/`model_b` in a stored payload always match what was hashed.
pub fn sort_pair<'a>(model_a: &'a str, model_b: &'a str) -> (&'a str, &'a str) {
    if model_a <= model_b {
        (model_a, model_b)
    } else {
        (model_b, model_a)
    }
}

pub fn pairwise_task_id(
    run_id: &str,
    model_a: &str,
    model_b: &str,
    prompt_hash: &str,
    judge: &str,
    condition: &str,
) -> String {
    let (a, b) = sort_pair(model_a, model_b);
    hash_fields(&["pairwise", run_id, a, b, prompt_hash, judge, condition])
}

/// Derives whether model A's response is shown first, as a pure function of
/// the task_id. A retried task reproduces the same position, since it's
/// seeded from an ID that never changes across retries.
pub fn model_a_shown_first(task_id: &str) -> bool {
    let seed_bytes = hex::decode(&task_id[..16]).expect("task_id is a valid hex digest");
    let seed = u64::from_be_bytes(
        seed_bytes
            .try_into()
            .expect("first 16 hex chars decode to exactly 8 bytes"),
    );
    let mut rng = StdRng::seed_from_u64(seed);
    rng.random_bool(0.5)
}

/// Generates a run_id in the `run_XXXXXX` shape shown throughout design-doc.md.
/// Not content-addressed: two runs with identical inputs still get distinct IDs.
pub fn generate_run_id() -> String {
    let n: u32 = rand::rng().random_range(0..0x1000000);
    format!("run_{n:06x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubric_task_id_is_deterministic() {
        let a = rubric_task_id("run_1", "claude", "ph1", "gpt-5", "rh1");
        let b = rubric_task_id("run_1", "claude", "ph1", "gpt-5", "rh1");
        assert_eq!(a, b);
    }

    #[test]
    fn rubric_task_id_changes_with_any_field() {
        let base = rubric_task_id("run_1", "claude", "ph1", "gpt-5", "rh1");
        assert_ne!(base, rubric_task_id("run_2", "claude", "ph1", "gpt-5", "rh1"));
        assert_ne!(base, rubric_task_id("run_1", "gemini", "ph1", "gpt-5", "rh1"));
        assert_ne!(base, rubric_task_id("run_1", "claude", "ph2", "gpt-5", "rh1"));
        assert_ne!(base, rubric_task_id("run_1", "claude", "ph1", "gpt-4o", "rh1"));
        assert_ne!(base, rubric_task_id("run_1", "claude", "ph1", "gpt-5", "rh2"));
    }

    #[test]
    fn pairwise_task_id_is_order_independent() {
        let a = pairwise_task_id("run_1", "claude", "gemini", "ph1", "gpt-5", "blind");
        let b = pairwise_task_id("run_1", "gemini", "claude", "ph1", "gpt-5", "blind");
        assert_eq!(a, b);
    }

    #[test]
    fn pairwise_task_id_changes_with_condition() {
        let blind = pairwise_task_id("run_1", "claude", "gemini", "ph1", "gpt-5", "blind");
        let unblind = pairwise_task_id("run_1", "claude", "gemini", "ph1", "gpt-5", "unblind");
        assert_ne!(blind, unblind);
    }

    #[test]
    fn length_prefixing_avoids_field_boundary_ambiguity() {
        // Without a fixed-width length prefix, ("ab", "c") and ("a", "bc")
        // could hash identically under naive concatenation.
        let a = hash_fields(&["ab", "c"]);
        let b = hash_fields(&["a", "bc"]);
        assert_ne!(a, b);
    }

    #[test]
    fn model_a_shown_first_is_deterministic() {
        let task_id = rubric_task_id("run_1", "claude", "ph1", "gpt-5", "rh1");
        assert_eq!(model_a_shown_first(&task_id), model_a_shown_first(&task_id));
    }

    #[test]
    fn generate_run_id_has_expected_shape() {
        let id = generate_run_id();
        assert!(id.starts_with("run_"));
        assert_eq!(id.len(), "run_".len() + 6);
        assert!(id[4..].chars().all(|c| c.is_ascii_hexdigit()));
    }
}
