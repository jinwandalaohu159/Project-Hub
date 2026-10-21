//! Hiding files: .gitignore rules, files that stop being saved, and refused rules.

use super::*;

fn write(repo: &TestRepo, name: &str, content: &str) {
    let path = std::path::Path::new(&repo.path).join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn adds_rules_and_hides_new_files() {
    let repo = TestRepo::new();
    write(&repo, "cache/data.bin", "x");

    let update = svc().update_ignore_rules(&repo.path, &strings(&["cache/", "*.tmp"]), &[]).unwrap();
    assert_eq!(update.added, ["cache/", "*.tmp"]);
    assert!(update.stopped_saving.is_empty());

    let rules = svc().ignore_rules(&repo.path, 50).unwrap();
    assert_eq!(rules.rules, ["cache/", "*.tmp"]);
    assert!(rules.hidden.contains(&"cache/".to_string()));
    assert!(rules.defaults.contains(&"node_modules/".to_string()));

    // Only .gitignore itself is left to save.
    let status = svc().get_status(&repo.path).unwrap();
    let paths: Vec<&str> = status.iter().map(|diff| diff.path.as_str()).collect();
    assert_eq!(paths, [".gitignore"]);
}

#[test]
fn hiding_saved_files_stops_saving_them_but_keeps_them_on_disk() {
    let repo = TestRepo::new();
    write(&repo, "build-output/app.js", "built");
    write(&repo, "[draft].md", "draft");
    svc().save_version(&repo.path, "with output").unwrap();

    let update = svc()
        .update_ignore_rules(&repo.path, &strings(&["/build-output/", "/[[]draft].md"]), &[])
        .unwrap();
    assert_eq!(update.stopped_saving, ["[draft].md", "build-output/app.js"]);
    assert!(std::path::Path::new(&repo.path).join("build-output/app.js").exists());

    svc().save_version(&repo.path, "hide output").unwrap();
    let tracked = git_out(&repo.path, &["ls-files"]);
    assert!(!tracked.contains("build-output") && !tracked.contains("draft"), "{tracked}");
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn files_kept_on_purpose_are_left_alone() {
    let repo = TestRepo::new();
    write(&repo, ".gitignore", "secret.txt\n");
    write(&repo, "secret.txt", "kept anyway");
    git_out(&repo.path, &["add", "-f", ".gitignore", "secret.txt"]);
    git_out(&repo.path, &["commit", "-m", "keep secret"]);

    let update = svc().update_ignore_rules(&repo.path, &strings(&["*.log"]), &[]).unwrap();
    assert!(update.stopped_saving.is_empty());
    assert!(git_out(&repo.path, &["ls-files"]).contains("secret.txt"));
}

#[test]
fn removes_rules_and_keeps_comments_and_line_endings() {
    let repo = TestRepo::new();
    write(&repo, ".gitignore", "# build\r\ndist/\r\n*.log\r\n");

    let update = svc()
        .update_ignore_rules(&repo.path, &[], &strings(&["*.log", "missing/"]))
        .unwrap();
    assert_eq!(update.removed, ["*.log"]);
    assert_eq!(update.unchanged, ["missing/"]);
    assert_eq!(read(&repo, ".gitignore").unwrap(), "# build\r\ndist/\r\n");
}

#[test]
fn repeated_rules_change_nothing() {
    let repo = TestRepo::new();
    svc().update_ignore_rules(&repo.path, &strings(&["*.log"]), &[]).unwrap();
    let update = svc().update_ignore_rules(&repo.path, &strings(&["*.log", " *.log "]), &[]).unwrap();
    assert!(update.added.is_empty());
    assert_eq!(update.unchanged, ["*.log", "*.log"]);
    assert_eq!(read(&repo, ".gitignore").unwrap(), "*.log\n");
}

#[test]
fn refuses_rules_that_hide_everything_and_changes_nothing() {
    let repo = TestRepo::new();
    for bad in ["*", "/", "**/*", ".gitignore", "", "# note", "a\nb"] {
        let result = svc().update_ignore_rules(&repo.path, &strings(&["ok.tmp", bad]), &[]);
        assert!(result.is_err(), "{bad:?} should be refused");
    }
    assert!(read(&repo, ".gitignore").is_none());
}
