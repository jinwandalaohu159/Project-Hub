//! Repository addresses and names as GitHub accepts them.

pub fn is_github_https(url: &str) -> bool {
    url.trim().to_lowercase().starts_with("https://github.com/")
}

/// Turns "owner/repo", an https URL or an SSH URL into an https clone URL GitHub accepts.
pub fn normalize_repo_url(input: &str) -> Result<String, String> {
    let value = input.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err("REPO_URL_EMPTY".to_string());
    }
    if let Some(rest) = value.strip_prefix("git@github.com:") {
        return Ok(format!("https://github.com/{}", with_git_suffix(rest)));
    }
    if value.starts_with("https://") || value.starts_with("http://") || value.starts_with("ssh://") {
        return Ok(if is_github_https(value) { with_git_suffix(value) } else { value.to_string() });
    }
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() == 2 && parts.iter().all(|part| !part.is_empty()) {
        return Ok(format!("https://github.com/{}", with_git_suffix(value)));
    }
    Err("REPO_URL_INVALID".to_string())
}

fn with_git_suffix(value: &str) -> String {
    if value.ends_with(".git") {
        value.to_string()
    } else {
        format!("{value}.git")
    }
}

/// GitHub repository names allow letters, digits, '-', '_' and '.'.
pub fn sanitize_repo_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches(['-', '.']).to_string();
    if out.is_empty() {
        "project".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_repo_addresses() {
        assert_eq!(normalize_repo_url("octo/demo").unwrap(), "https://github.com/octo/demo.git");
        assert_eq!(
            normalize_repo_url("https://github.com/octo/demo/").unwrap(),
            "https://github.com/octo/demo.git"
        );
        assert_eq!(
            normalize_repo_url("git@github.com:octo/demo.git").unwrap(),
            "https://github.com/octo/demo.git"
        );
        assert!(normalize_repo_url("not a repo").is_err());
    }

    #[test]
    fn sanitizes_repo_names() {
        assert_eq!(sanitize_repo_name("Project Hub"), "Project-Hub");
        assert_eq!(sanitize_repo_name("我的项目"), "project");
        assert_eq!(sanitize_repo_name("demo_v2.0"), "demo_v2.0");
    }
}
