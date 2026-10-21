//! Save records: saving, commit identity, the record list, restoring a record and setting up
//! a folder for versioning.

use super::*;

#[test]
fn stash_and_pop() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "changed");
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
    svc().stash(&repo.path).unwrap();
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
    svc().stash_pop(&repo.path).unwrap();
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn discard_changes() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "changed");
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
    svc().discard_changes(&repo.path).unwrap();
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn add_and_commit() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "updated");
    svc().add_all(&repo.path).unwrap();
    svc().commit(&repo.path, "update readme").unwrap();
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn save_version_sets_missing_local_commit_identity() {
    let repo = TestRepo::new();
    Command::new("git")
        .args(["config", "--unset", "user.email"])
        .current_dir(&repo.path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "--unset", "user.name"])
        .current_dir(&repo.path)
        .output()
        .unwrap();
    repo.modify_file("README.md", "updated");

    svc()
        .save_version(&repo.path, "save without identity")
        .unwrap();

    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn unstage_files() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "updated");
    svc().add_files(&repo.path, &["README.md"]).unwrap();
    svc().unstage_files(&repo.path, &["README.md"]).unwrap();
    // file should still show as modified but unstaged
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn get_log() {
    let repo = TestRepo::new();
    let logs = svc().get_log(&repo.path, 10).unwrap();
    assert!(!logs.is_empty());
    assert_eq!(logs[0].message, "initial");
    assert!(!logs[0].hash.is_empty());
    assert!(logs[0].time.is_some());
}

#[test]
fn restore_commit_moves_cleanly_without_losing_later_records() {
    let repo = TestRepo::new();
    let initial_hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&repo.path)
        .output()
        .unwrap();
    let initial_hash = String::from_utf8_lossy(&initial_hash.stdout)
        .trim()
        .to_string();

    repo.modify_file("README.md", "latest");
    svc().add_all(&repo.path).unwrap();
    svc().commit(&repo.path, "latest").unwrap();
    let latest_hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&repo.path)
        .output()
        .unwrap();
    let latest_hash = String::from_utf8_lossy(&latest_hash.stdout)
        .trim()
        .to_string();

    svc().restore_commit(&repo.path, &initial_hash).unwrap();
    assert_eq!(
        fs::read_to_string(std::path::Path::new(&repo.path).join("README.md")).unwrap(),
        "# test"
    );
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
    let logs = svc().get_log(&repo.path, 10).unwrap();
    assert_eq!(logs.len(), 2);
    assert!(!logs
        .iter()
        .any(|log| log.message.starts_with("Restore save record")));
    assert!(logs.iter().any(|log| log.message == "latest"));
    assert!(logs.iter().any(|log| log.message == "initial"));

    svc().restore_commit(&repo.path, &latest_hash).unwrap();
    assert_eq!(
        fs::read_to_string(std::path::Path::new(&repo.path).join("README.md")).unwrap(),
        "latest"
    );
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
    let logs = svc().get_log(&repo.path, 10).unwrap();
    assert_eq!(logs.len(), 2);
    assert!(!logs
        .iter()
        .any(|log| log.message.starts_with("Restore save record")));
    assert!(logs.iter().any(|log| log.message == "latest"));
    assert!(logs.iter().any(|log| log.message == "initial"));
}

#[test]
fn init_repo() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    assert!(!svc().is_git_repo(path));

    fs::write(dir.path().join("hello.txt"), "world").unwrap();
    svc().init_repo(path).unwrap();

    assert!(svc().is_git_repo(path));
    let branches = svc().get_branches(path).unwrap();
    assert!(!branches.is_empty());
    let logs = svc().get_log(path, 10).unwrap();
    assert!(!logs.is_empty());
    assert_eq!(logs[0].message, "Initial version");
    assert!(svc().has_unsaved_changes(path).unwrap());
}

#[test]
fn identity_overrides_repo_config() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "by me");
    let service = GitService::with_identity(Some(Identity {
        name: "Ada".to_string(),
        email: "ada@example.com".to_string(),
    }));
    service.save_version(&repo.path, "authored").unwrap();
    assert_eq!(git_out(&repo.path, &["log", "-1", "--format=%an <%ae>"]), "Ada <ada@example.com>");
}

#[test]
fn log_includes_message_body() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "body");
    svc().save_version(&repo.path, "Title line\n\nFirst detail\nSecond detail").unwrap();
    let logs = svc().get_log(&repo.path, 1).unwrap();
    assert_eq!(logs[0].message, "Title line");
    assert_eq!(logs[0].body, "First detail\nSecond detail");
}

#[test]
fn restore_keeps_ignored_local_file_that_old_record_tracked() {
    let repo = TestRepo::new();
    repo.modify_file("app.log", "tracked once");
    Command::new("git").args(["add", "app.log"]).current_dir(&repo.path).output().unwrap();
    Command::new("git").args(["commit", "-m", "track log"]).current_dir(&repo.path).output().unwrap();
    let old = git_out(&repo.path, &["rev-parse", "HEAD"]);
    Command::new("git").args(["rm", "--cached", "app.log"]).current_dir(&repo.path).output().unwrap();
    Command::new("git").args(["commit", "-m", "untrack log"]).current_dir(&repo.path).output().unwrap();
    // *.log is a default ignore rule; this local copy exists only on disk now.
    repo.modify_file("app.log", "precious local content");
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());

    let outcome = svc().restore_commit(&repo.path, &old).unwrap();
    assert!(outcome.draft_saved);
    assert_eq!(read(&repo, "app.log").unwrap(), "tracked once");

    svc().restore_draft(&repo.path).unwrap();
    assert_eq!(read(&repo, "app.log").unwrap(), "precious local content");
}

#[test]
fn repository_without_records_gets_an_initial_version() {
    let dir = unborn_repo_with_file();
    let path = dir.path().to_str().unwrap();
    assert!(svc().get_branches(path).unwrap().is_empty());

    assert!(svc().ensure_initial_version(path).unwrap());
    let branches = svc().get_branches(path).unwrap();
    assert_eq!(branches.len(), 1);
    assert!(branches[0].is_current);
    assert_eq!(svc().get_log(path, 5).unwrap()[0].message, "Initial version");
    // Existing files stay as unsaved changes, and versions can be created again.
    assert_eq!(fs::read_to_string(dir.path().join("a.txt")).unwrap(), "x");
    assert!(svc().has_unsaved_changes(path).unwrap());
    svc().create_branch(path, "v2").unwrap();

    // Only ever once.
    assert!(!svc().ensure_initial_version(path).unwrap());
    assert_eq!(svc().get_log(path, 5).unwrap().len(), 1);
}

#[test]
fn ensure_initial_version_ignores_folders_without_git() {
    let dir = tempfile::TempDir::new().unwrap();
    assert!(!svc().ensure_initial_version(dir.path().to_str().unwrap()).unwrap());
    assert!(!dir.path().join(".git").exists());
}

#[test]
fn ensure_versioned_sets_up_any_folder_once() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    fs::write(dir.path().join("notes.txt"), "keep me").unwrap();

    let outcome = svc().ensure_versioned(path).unwrap();
    assert!(outcome.initialized && outcome.initial_version);
    assert_eq!(svc().get_branches(path).unwrap().len(), 1);
    assert_eq!(fs::read_to_string(dir.path().join("notes.txt")).unwrap(), "keep me");

    assert_eq!(svc().ensure_versioned(path).unwrap(), VersioningOutcome::default());
    assert!(svc().ensure_versioned(dir.path().join("missing").to_str().unwrap()).is_err());
}
