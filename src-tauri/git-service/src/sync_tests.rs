//! Uploading and downloading between two computers and one cloud repository, all on local disk:
//! a bare repository plays GitHub.

use super::*;
use std::fs;
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git").args(args).current_dir(dir).output().unwrap();
    assert!(output.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// A save record of every file, as the app makes one.
fn save(dir: &Path, file: &str, content: &str, message: &str) {
    fs::write(dir.join(file), content).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

fn count(dir: &Path) -> usize {
    git(dir, &["rev-list", "--count", "HEAD"]).parse().unwrap()
}

struct World {
    _root: tempfile::TempDir,
    a: std::path::PathBuf,
    b: std::path::PathBuf,
}

/// The cloud and computers A and B, all on `main` with the same first record.
fn world() -> World {
    let root = tempfile::TempDir::new().unwrap();
    let cloud = root.path().join("cloud.git");
    fs::create_dir_all(&cloud).unwrap();
    git(&cloud, &["init", "-q", "--bare", "-b", "main"]);
    let a = root.path().join("a");
    fs::create_dir_all(&a).unwrap();
    git(&a, &["init", "-q", "-b", "main"]);
    for dir in [&a] {
        git(dir, &["config", "user.email", "t@t.com"]);
        git(dir, &["config", "user.name", "T"]);
    }
    save(&a, "app.txt", "v1", "first");
    git(&a, &["remote", "add", "origin", cloud.to_str().unwrap()]);
    GitService::new().push(a.to_str().unwrap(), None, false).unwrap();

    let b = root.path().join("b");
    git(root.path(), &["clone", "-q", cloud.to_str().unwrap(), "b"]);
    git(&b, &["config", "user.email", "t@t.com"]);
    git(&b, &["config", "user.name", "T"]);
    World { _root: root, a, b }
}

fn path(dir: &Path) -> &str {
    dir.to_str().unwrap()
}

#[test]
fn a_computer_behind_the_cloud_downloads_the_newer_records() {
    let w = world();
    let svc = GitService::new();
    // B saves two more records and uploads them: the cloud is at 3, A still at 1.
    save(&w.b, "app.txt", "v2", "second");
    save(&w.b, "app.txt", "v3", "third");
    svc.push(path(&w.b), None, false).unwrap();

    // Before asking the cloud, A cannot know.
    assert_eq!(svc.sync_state(path(&w.a)).unwrap().behind, 0);
    assert!(svc.fetch(path(&w.a), None).unwrap());
    let state = svc.sync_state(path(&w.a)).unwrap();
    assert_eq!((state.ahead, state.behind), (0, 2));

    assert_eq!(svc.pull(path(&w.a), None).unwrap().received, 2);
    assert_eq!(count(&w.a), 3);
    // Every record comes down on its own, not just the end state.
    assert_eq!(git(&w.a, &["log", "--format=%s"]), "third\nsecond\nfirst");
    assert_eq!(git(&w.a, &["show", "HEAD~1:app.txt"]), "v2");
    assert_eq!(fs::read_to_string(w.a.join("app.txt")).unwrap(), "v3");
    assert_eq!(svc.pull(path(&w.a), None).unwrap().received, 0);
}

#[test]
fn unsaved_changes_survive_a_download_that_does_not_touch_them() {
    let w = world();
    let svc = GitService::new();
    save(&w.b, "app.txt", "v2", "second");
    svc.push(path(&w.b), None, false).unwrap();
    fs::write(w.a.join("notes.txt"), "work in progress").unwrap();

    assert_eq!(svc.pull(path(&w.a), None).unwrap().received, 1);
    assert_eq!(fs::read_to_string(w.a.join("app.txt")).unwrap(), "v2");
    assert_eq!(fs::read_to_string(w.a.join("notes.txt")).unwrap(), "work in progress");
}

#[test]
fn a_download_over_unsaved_changes_to_the_same_file_changes_nothing() {
    let w = world();
    let svc = GitService::new();
    save(&w.b, "app.txt", "v2 from B", "second");
    svc.push(path(&w.b), None, false).unwrap();
    fs::write(w.a.join("app.txt"), "A's unsaved edit").unwrap();

    let error = svc.pull(path(&w.a), None).unwrap_err();
    assert_eq!(error, "SYNC_LOCAL_CHANGES:app.txt");
    assert_eq!(fs::read_to_string(w.a.join("app.txt")).unwrap(), "A's unsaved edit");
    assert_eq!(count(&w.a), 1);
}

#[test]
fn records_on_both_sides_need_a_choice_and_using_the_cloud_keeps_ours_as_a_version() {
    let w = world();
    let svc = GitService::new();
    save(&w.b, "app.txt", "v2 from B", "B's second");
    svc.push(path(&w.b), None, false).unwrap();
    save(&w.a, "app.txt", "v2 from A", "A's second");
    fs::write(w.a.join("draft.txt"), "A unsaved").unwrap();

    assert_eq!(svc.pull(path(&w.a), None).unwrap_err(), "SYNC_DIVERGED");
    let state = svc.sync_state(path(&w.a)).unwrap();
    assert_eq!((state.ahead, state.behind), (1, 1));
    assert_eq!(fs::read_to_string(w.a.join("app.txt")).unwrap(), "v2 from A", "a refused download changes nothing");

    let backup = svc.use_cloud_version(path(&w.a), None).unwrap();
    assert!(backup.starts_with("main-local-"), "{backup}");
    // Now the same as the cloud...
    assert_eq!(fs::read_to_string(w.a.join("app.txt")).unwrap(), "v2 from B");
    assert!(!w.a.join("draft.txt").exists());
    let state = svc.sync_state(path(&w.a)).unwrap();
    assert_eq!((state.ahead, state.behind), (0, 0));
    // ...and A's own record and unsaved work live on in the backup version.
    assert_eq!(git(&w.a, &["log", "-1", "--format=%s", &backup]), "A's second");
    svc.switch_branch(path(&w.a), &backup).unwrap();
    let draft = svc.get_draft(path(&w.a)).unwrap().expect("unsaved changes kept with the backup");
    assert_eq!(draft.branch, backup);
    svc.restore_draft(path(&w.a)).unwrap();
    assert_eq!(fs::read_to_string(w.a.join("draft.txt")).unwrap(), "A unsaved");
    assert_eq!(fs::read_to_string(w.a.join("app.txt")).unwrap(), "v2 from A");
}

#[test]
fn a_version_the_cloud_does_not_have_is_reported_not_downloaded() {
    let w = world();
    let svc = GitService::new();
    git(&w.a, &["switch", "-q", "-c", "experiment"]);
    assert!(!svc.fetch(path(&w.a), None).unwrap());
    assert_eq!(svc.pull(path(&w.a), None).unwrap_err(), "SYNC_NOT_IN_CLOUD");
}
