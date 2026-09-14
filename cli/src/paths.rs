use std::path::Path;

/// Resolves a user-supplied path to an absolute one, relative to the CLI's own cwd
//  The scheduler processes' may differ b/c it runs as a separate long-lived process
pub fn resolve_path(path: &str) -> std::io::Result<String> {
    let absolute = std::fs::canonicalize(path)?;
    Ok(absolute.to_string_lossy().into_owned())
}

/// Resolves --rubric's bare-flag behavior: if rubric_arg is empty (the CLI's
/// sentinel for "flag omitted, or present but bare"), look for ./rubric.yaml
/// relative to the CLI's cwd
pub fn resolve_rubric_arg(rubric_arg: &str) -> std::io::Result<String> {
    if !rubric_arg.is_empty() {
        return resolve_path(rubric_arg);
    }
    let default = Path::new("rubric.yaml");
    if default.exists() {
        resolve_path("rubric.yaml")
    } else {
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    // set_current_dir changes process-wide state
    // cargo test runs tests in parallel by default, so any test that changes cwd must hold this lock
    // for the duration, or two tests can race and corrupt each other
    static CWD_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("blind_cli_test_{}_{label}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_path_returns_an_absolute_path() {
        let dir = temp_dir("resolve_path");
        let file = dir.join("tasks.jsonl");
        fs::write(&file, "{}").unwrap();

        let resolved = resolve_path(file.to_str().unwrap()).unwrap();
        assert!(Path::new(&resolved).is_absolute());
        assert_eq!(resolved, file.canonicalize().unwrap().to_string_lossy());
    }

    #[test]
    fn resolve_path_errors_on_missing_file() {
        assert!(resolve_path("/definitely/does/not/exist.jsonl").is_err());
    }

    #[test]
    fn resolve_rubric_arg_resolves_an_explicit_path() {
        let dir = temp_dir("explicit_rubric");
        let file = dir.join("my_rubric.yaml");
        fs::write(&file, "scale: 1-5").unwrap();

        let resolved = resolve_rubric_arg(file.to_str().unwrap()).unwrap();
        assert!(Path::new(&resolved).is_absolute());
    }

    #[test]
    fn resolve_rubric_arg_finds_rubric_yaml_in_cwd() {
        let _guard = CWD_TEST_LOCK.lock().unwrap();
        let dir = temp_dir("bare_rubric_found");
        fs::write(dir.join("rubric.yaml"), "scale: 1-5").unwrap();

        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let result = resolve_rubric_arg("");
        std::env::set_current_dir(original_cwd).unwrap();

        let resolved = result.unwrap();
        assert!(!resolved.is_empty());
        assert!(resolved.ends_with("rubric.yaml"));
    }

    #[test]
    fn resolve_rubric_arg_returns_empty_when_bare_and_no_file_present() {
        let _guard = CWD_TEST_LOCK.lock().unwrap();
        let dir = temp_dir("bare_rubric_missing");

        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let result = resolve_rubric_arg("");
        std::env::set_current_dir(original_cwd).unwrap();

        assert_eq!(result.unwrap(), "");
    }
}