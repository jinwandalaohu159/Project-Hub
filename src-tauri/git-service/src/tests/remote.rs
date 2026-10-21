//! Cloud and transfer: pushing to a remote, bundles for export/import.

use super::*;

#[test]
fn push_to_remote_and_detect_rejection() {
    let repo = TestRepo::new();
    let remote = tempfile::TempDir::new().unwrap();
    let remote_path = remote.path().to_str().unwrap();
    Command::new("git").args(["init", "--bare"]).current_dir(remote_path).output().unwrap();
    svc().set_remote_url(&repo.path, "origin", remote_path).unwrap();

    let initial = git_out(&repo.path, &["rev-parse", "HEAD"]);
    repo.modify_file("README.md", "pushed");
    svc().save_version(&repo.path, "to cloud").unwrap();
    svc().push(&repo.path, None, false).unwrap();
    let state = svc().sync_state(&repo.path).unwrap();
    assert!(state.has_upstream);
    assert_eq!(state.ahead, 0);

    svc().restore_commit(&repo.path, &initial).unwrap();
    repo.modify_file("README.md", "diverged");
    svc().save_version(&repo.path, "diverged").unwrap();
    let error = svc().push(&repo.path, None, false).unwrap_err();
    assert!(error.starts_with(PUSH_REJECTED), "{}", error);
    svc().push(&repo.path, None, true).unwrap();
}

#[test]
fn bundle_roundtrip_restores_files_history_and_unsaved_changes() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "saved");
    svc().save_version(&repo.path, "second").unwrap();
    repo.modify_file("wip.txt", "work in progress");
    let branch = svc().get_current_branch(&repo.path).unwrap();

    let out = tempfile::TempDir::new().unwrap();
    let bundle = out.path().join("project.bundle");
    let meta = svc().create_bundle(&repo.path, &bundle).unwrap();
    assert!(meta.has_draft);
    assert_eq!(meta.head_branch, branch);
    // Exporting must not disturb the source project.
    assert!(svc().get_draft(&repo.path).unwrap().is_none());
    assert_eq!(read(&repo, "wip.txt").unwrap(), "work in progress");

    let target_root = tempfile::TempDir::new().unwrap();
    let target = target_root.path().join("restored");
    svc()
        .restore_bundle(&target, &bundle, &meta.head_branch, Some("https://github.com/o/r.git"))
        .unwrap();
    let target_str = target.to_str().unwrap();
    assert_eq!(fs::read_to_string(target.join("README.md")).unwrap(), "saved");
    assert_eq!(fs::read_to_string(target.join("wip.txt")).unwrap(), "work in progress");
    assert_eq!(svc().get_log(target_str, 10).unwrap().len(), 2);
    assert_eq!(svc().get_remote_url(target_str, "origin").as_deref(), Some("https://github.com/o/r.git"));

    let existing = target_root.path().join("existing");
    fs::create_dir_all(&existing).unwrap();
    fs::write(existing.join("README.md"), "local copy").unwrap();
    svc().restore_bundle(&existing, &bundle, &meta.head_branch, None).unwrap();
    let existing_str = existing.to_str().unwrap();
    assert_eq!(fs::read_to_string(existing.join("README.md")).unwrap(), "local copy");
    assert!(svc().has_unsaved_changes(existing_str).unwrap());
    assert!(svc().get_draft(existing_str).unwrap().is_some());
}

#[test]
fn base64_matches_reference() {
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"x-access-token:abc"), "eC1hY2Nlc3MtdG9rZW46YWJj");
}

#[test]
fn overwrite_does_not_delete_uploads_from_another_computer() {
    let repo = TestRepo::new();
    let remote = tempfile::TempDir::new().unwrap();
    let remote_path = remote.path().to_str().unwrap();
    Command::new("git").args(["init", "--bare"]).current_dir(remote_path).output().unwrap();
    svc().set_remote_url(&repo.path, "origin", remote_path).unwrap();
    svc().push(&repo.path, None, false).unwrap();
    let branch = svc().get_current_branch(&repo.path).unwrap();

    // Another computer uploads a record this one never fetched.
    let other = tempfile::TempDir::new().unwrap();
    let other_path = other.path().join("clone");
    Command::new("git").args(["clone", remote_path, other_path.to_str().unwrap()]).output().unwrap();
    let other_str = other_path.to_str().unwrap();
    Command::new("git").args(["config", "user.email", "o@o.com"]).current_dir(other_str).output().unwrap();
    Command::new("git").args(["config", "user.name", "O"]).current_dir(other_str).output().unwrap();
    fs::write(other_path.join("remote.txt"), "from elsewhere").unwrap();
    Command::new("git").args(["add", "."]).current_dir(other_str).output().unwrap();
    Command::new("git").args(["commit", "-m", "elsewhere"]).current_dir(other_str).output().unwrap();
    Command::new("git").args(["push", "origin", &branch]).current_dir(other_str).output().unwrap();

    repo.modify_file("README.md", "local");
    svc().save_version(&repo.path, "local").unwrap();
    let error = svc().push(&repo.path, None, true).unwrap_err();
    assert!(error.starts_with(PUSH_REJECTED), "{}", error);
    let remote_head = git_out(remote_path, &["log", "-1", "--format=%s", &branch]);
    assert_eq!(remote_head, "elsewhere");
}

#[test]
fn bundle_restores_into_a_repository_without_records() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "saved");
    svc().save_version(&repo.path, "second").unwrap();
    let out = tempfile::TempDir::new().unwrap();
    let bundle = out.path().join("p.bundle");
    let meta = svc().create_bundle(&repo.path, &bundle).unwrap();

    let target = unborn_repo_with_file();
    let target_str = target.path().to_str().unwrap();
    svc().restore_bundle(target.path(), &bundle, &meta.head_branch, None).unwrap();
    assert_eq!(svc().get_log(target_str, 10).unwrap().len(), 2);
    assert_eq!(fs::read_to_string(target.path().join("a.txt")).unwrap(), "x");

    // A repository that does have records is still never overwritten.
    assert!(svc().restore_bundle(target.path(), &bundle, &meta.head_branch, None).is_err());
}
