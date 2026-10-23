//! Release tags against a bare repository that plays GitHub.

use super::*;
use std::path::Path;

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git").args(args).current_dir(dir).output().unwrap();
    assert!(output.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn save(dir: &Path, file: &str, message: &str) {
    fs::write(dir.join(file), message).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

/// A project with two records and a cloud.
fn project() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::TempDir::new().unwrap();
    let cloud = root.path().join("cloud.git");
    fs::create_dir_all(&cloud).unwrap();
    git(&cloud, &["init", "-q", "--bare", "-b", "main"]);
    let work = root.path().join("work");
    fs::create_dir_all(&work).unwrap();
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["config", "user.email", "t@t.com"]);
    git(&work, &["config", "user.name", "T"]);
    save(&work, "a.txt", "first");
    git(&work, &["remote", "add", "origin", cloud.to_str().unwrap()]);
    (root, work, cloud)
}

#[test]
fn records_since_a_tag_are_the_release_notes() {
    let (_root, work, _cloud) = project();
    let path = work.to_str().unwrap();
    let svc = GitService::new();
    git(&work, &["tag", "v1.0.0"]);
    save(&work, "b.txt", "Add export");
    fs::write(work.join("c.txt"), "x").unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "Fix login", "-m", "The button did nothing."]);

    let since = svc.records_since(path, Some("v1.0.0"), 50).unwrap();
    assert_eq!(
        since,
        vec![
            RecordSummary { title: "Fix login".into(), body: "The button did nothing.".into() },
            RecordSummary { title: "Add export".into(), body: String::new() },
        ]
    );
    // Without a tag: every record.
    assert_eq!(svc.records_since(path, Some("v9.9.9"), 50).unwrap().len(), 3);
    assert_eq!(svc.records_since(path, None, 1).unwrap().len(), 1);
}

#[test]
fn pushing_a_tag_uploads_it_once_and_refuses_moving_it() {
    let (_root, work, cloud) = project();
    let path = work.to_str().unwrap();
    let svc = GitService::new();
    svc.push(path, None, false).unwrap();

    svc.push_tag(path, "v1.0.0", None).unwrap();
    assert_eq!(git(&cloud, &["rev-parse", "v1.0.0^{commit}"]), git(&work, &["rev-parse", "HEAD"]));
    // Again on the same record: fine.
    svc.push_tag(path, "v1.0.0", None).unwrap();

    save(&work, "b.txt", "later");
    assert!(svc.push_tag(path, "v1.0.0", None).unwrap_err().starts_with("TAG_EXISTS"));
}

#[test]
fn tags_made_elsewhere_come_down() {
    let (root, work, cloud) = project();
    let svc = GitService::new();
    svc.push(work.to_str().unwrap(), None, false).unwrap();
    git(&cloud, &["tag", "v2.0.0", "main"]);

    let other = root.path().join("other");
    git(root.path(), &["clone", "-q", "--no-tags", cloud.to_str().unwrap(), "other"]);
    let other_path = other.to_str().unwrap();
    assert!(svc.tag_commit(other_path, "v2.0.0").is_none());
    svc.fetch_tags(other_path, None).unwrap();
    assert!(svc.tag_commit(other_path, "v2.0.0").is_some());
}
