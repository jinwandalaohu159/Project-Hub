use super::*;
use crate::status::{parse_porcelain_z, KEEP_FILE_NAME};
use std::fs;
use std::process::Command;

struct TestRepo {
    path: String,
    _dir: tempfile::TempDir,
}

impl TestRepo {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap().to_string();
        Command::new("git")
            .args(["init"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&path)
            .output()
            .unwrap();
        // Create initial commit so HEAD exists
        fs::write(dir.path().join("README.md"), "# test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(&path)
            .output()
            .unwrap();
        TestRepo { path, _dir: dir }
    }

    fn create_branch(&self, name: &str) {
        Command::new("git")
            .args(["branch", name])
            .current_dir(&self.path)
            .output()
            .unwrap();
    }

    fn _checkout_branch(&self, name: &str) {
        Command::new("git")
            .args(["checkout", name])
            .current_dir(&self.path)
            .output()
            .unwrap();
    }

    fn modify_file(&self, name: &str, content: &str) {
        fs::write(std::path::Path::new(&self.path).join(name), content).unwrap();
    }
}

fn svc() -> GitService {
    GitService::new()
}

#[test]
fn is_git_repo_true() {
    let repo = TestRepo::new();
    assert!(svc().is_git_repo(&repo.path));
}

#[test]
fn is_git_repo_false() {
    let dir = tempfile::TempDir::new().unwrap();
    assert!(!svc().is_git_repo(dir.path().to_str().unwrap()));
}

#[test]
fn get_branches_returns_main() {
    let repo = TestRepo::new();
    let branches = svc().get_branches(&repo.path).unwrap();
    assert!(!branches.is_empty());
    let main = branches.iter().find(|b| b.is_current).unwrap();
    assert!(main.last_commit_message.is_some());
}

#[test]
fn get_branches_multiple() {
    let repo = TestRepo::new();
    repo.create_branch("feature-a");
    repo.create_branch("feature-b");
    let branches = svc().get_branches(&repo.path).unwrap();
    assert_eq!(branches.len(), 3);
}

#[test]
fn get_current_branch() {
    let repo = TestRepo::new();
    let current = svc().get_current_branch(&repo.path).unwrap();
    assert!(!current.is_empty());
}

#[test]
fn switch_branch() {
    let repo = TestRepo::new();
    repo.create_branch("feature-x");
    svc().switch_branch(&repo.path, "feature-x").unwrap();
    let current = svc().get_current_branch(&repo.path).unwrap();
    assert_eq!(current, "feature-x");
}

#[test]
fn get_status_clean() {
    let repo = TestRepo::new();
    let status = svc().get_status(&repo.path).unwrap();
    assert!(status.is_empty());
}

#[test]
fn get_status_modified() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "changed");
    let status = svc().get_status(&repo.path).unwrap();
    assert!(!status.is_empty());
    assert_eq!(status[0].status, FileStatus::Modified);
}

#[test]
fn get_status_summary_limits_displayed_files() {
    let repo = TestRepo::new();
    repo.modify_file("a.txt", "a");
    repo.modify_file("b.txt", "b");
    repo.modify_file("c.txt", "c");

    let summary = svc().get_status_summary(&repo.path, 2).unwrap();

    assert_eq!(summary.total, 3);
    assert_eq!(summary.files.len(), 2);
    assert!(summary.truncated);
}

#[test]
fn default_ignores_hide_dependency_and_build_dirs() {
    let repo = TestRepo::new();
    let node_module_file = std::path::Path::new(&repo.path)
        .join("node_modules")
        .join("pkg")
        .join("index.js");
    let dist_file = std::path::Path::new(&repo.path)
        .join("dist")
        .join("bundle.js");
    fs::create_dir_all(node_module_file.parent().unwrap()).unwrap();
    fs::create_dir_all(dist_file.parent().unwrap()).unwrap();
    fs::write(&node_module_file, "module").unwrap();
    fs::write(&dist_file, "bundle").unwrap();

    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
    assert!(svc().get_status(&repo.path).unwrap().is_empty());
}

#[test]
fn add_all_does_not_save_default_ignored_dirs() {
    let repo = TestRepo::new();
    let node_module_file = std::path::Path::new(&repo.path)
        .join("node_modules")
        .join("pkg")
        .join("index.js");
    fs::create_dir_all(node_module_file.parent().unwrap()).unwrap();
    fs::write(&node_module_file, "module").unwrap();

    svc().add_all(&repo.path).unwrap();
    let staged = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(&repo.path)
        .output()
        .unwrap();

    assert!(String::from_utf8_lossy(&staged.stdout).trim().is_empty());
}

#[test]
fn has_unsaved_changes_clean() {
    let repo = TestRepo::new();
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn has_unsaved_changes_dirty() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "changed");
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn has_unsaved_changes_detects_empty_untracked_file() {
    let repo = TestRepo::new();
    let file_path = std::path::Path::new(&repo.path).join("new-file.txt");
    fs::write(file_path, "").unwrap();
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn has_unsaved_changes_detects_nonempty_untracked_file() {
    let repo = TestRepo::new();
    let file_path = std::path::Path::new(&repo.path).join("new-file.txt");
    fs::write(file_path, "hello").unwrap();
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn has_unsaved_changes_detects_empty_untracked_dir() {
    let repo = TestRepo::new();
    fs::create_dir(std::path::Path::new(&repo.path).join("empty-dir")).unwrap();
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());

    let status = svc().get_status(&repo.path).unwrap();
    assert!(status.iter().any(|diff| diff.path == "empty-dir"));
}

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
fn dirty_submodule_content_is_not_parent_unsaved_change() {
    let repo = TestRepo::new();
    let child = tempfile::TempDir::new().unwrap();
    let child_path = child.path().to_str().unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(child_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(child_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(child_path)
        .output()
        .unwrap();
    fs::write(child.path().join("child.txt"), "clean").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(child_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "child initial"])
        .current_dir(child_path)
        .output()
        .unwrap();

    Command::new("git")
        .args([
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            child_path,
            "cc-skills",
        ])
        .current_dir(&repo.path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo.path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add submodule"])
        .current_dir(&repo.path)
        .output()
        .unwrap();

    fs::write(
        std::path::Path::new(&repo.path)
            .join("cc-skills")
            .join("child.txt"),
        "dirty",
    )
    .unwrap();

    let summary = svc().get_status_summary(&repo.path, 200).unwrap();
    assert_eq!(summary.total, 0);
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
    svc()
        .save_version(&repo.path, "save dirty submodule only")
        .unwrap();
}
#[test]
fn add_and_commit_tracks_empty_dir_marker() {
    let repo = TestRepo::new();
    let empty_dir = std::path::Path::new(&repo.path).join("empty-dir");
    fs::create_dir(&empty_dir).unwrap();

    svc().save_version(&repo.path, "add empty folder").unwrap();

    assert!(empty_dir.join(KEEP_FILE_NAME).exists());
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn switch_branch_cleans_untracked_files() {
    let repo = TestRepo::new();
    repo.create_branch("other");
    let temp_file = std::path::Path::new(&repo.path).join("temp.txt");
    fs::write(&temp_file, "not saved").unwrap();

    svc().switch_branch(&repo.path, "other").unwrap();

    assert!(!temp_file.exists());
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
fn create_and_delete_branch() {
    let repo = TestRepo::new();
    svc().create_branch(&repo.path, "feature-new").unwrap();
    let branches = svc().get_branches(&repo.path).unwrap();
    assert!(branches.iter().any(|b| b.name == "feature-new"));

    svc()
        .delete_branch(&repo.path, "feature-new", true)
        .unwrap();
    let branches = svc().get_branches(&repo.path).unwrap();
    assert!(!branches.iter().any(|b| b.name == "feature-new"));
}

#[test]
fn create_branch_starts_from_current_files_with_initial_commit_only() {
    let repo = TestRepo::new();
    svc().create_branch(&repo.path, "blank-version").unwrap();
    svc().switch_branch(&repo.path, "blank-version").unwrap();

    let logs = svc().get_log(&repo.path, 10).unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].message, "Initial commit");
    assert_eq!(
        fs::read_to_string(std::path::Path::new(&repo.path).join("README.md")).unwrap(),
        "# test"
    );
}

#[test]
fn create_and_switch_branch() {
    let repo = TestRepo::new();
    svc()
        .create_and_switch_branch(&repo.path, "feature-switch")
        .unwrap();
    let current = svc().get_current_branch(&repo.path).unwrap();
    assert_eq!(current, "feature-switch");

    let logs = svc().get_log(&repo.path, 10).unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].message, "Initial commit");
    assert_eq!(
        fs::read_to_string(std::path::Path::new(&repo.path).join("README.md")).unwrap(),
        "# test"
    );
}

#[test]
fn merge_branch() {
    let repo = TestRepo::new();
    svc().create_branch(&repo.path, "feature-merge").unwrap();
    svc().switch_branch(&repo.path, "feature-merge").unwrap();
    repo.modify_file("new-file.txt", "content");
    svc().add_all(&repo.path).unwrap();
    svc().commit(&repo.path, "add new file").unwrap();

    svc()
        .switch_branch(&repo.path, &svc().get_current_branch(&repo.path).unwrap())
        .unwrap();
    let all_branches = svc().get_branches(&repo.path).unwrap();
    let main = all_branches
        .iter()
        .find(|b| !b.name.starts_with("feature"))
        .unwrap();
    svc().switch_branch(&repo.path, &main.name).unwrap();
    svc().merge_branch(&repo.path, "feature-merge").unwrap();
    assert!(!svc().has_merge_conflicts(&repo.path));
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

fn git_out(path: &str, args: &[&str]) -> String {
    let output = Command::new("git").args(args).current_dir(path).output().unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn read(repo: &TestRepo, name: &str) -> Option<String> {
    fs::read_to_string(std::path::Path::new(&repo.path).join(name)).ok()
}

#[test]
fn parse_porcelain_z_handles_renames_and_spaces() {
    let output = "R  new name.txt\0old name.txt\0?? 新文件.md\0 D gone.txt\0A  added.txt\0 M changed.txt\0";
    let diffs = parse_porcelain_z(output);
    assert_eq!(diffs.len(), 5);
    assert_eq!(diffs[0].status, FileStatus::Renamed);
    assert_eq!(diffs[0].path, "new name.txt");
    assert_eq!(diffs[0].old_path.as_deref(), Some("old name.txt"));
    assert_eq!(diffs[1].status, FileStatus::Untracked);
    assert_eq!(diffs[1].path, "新文件.md");
    assert_eq!(diffs[2].status, FileStatus::Deleted);
    assert_eq!(diffs[3].status, FileStatus::Added);
    assert_eq!(diffs[4].status, FileStatus::Modified);
}

#[test]
fn status_keeps_non_ascii_paths_readable() {
    let repo = TestRepo::new();
    repo.modify_file("说明.md", "hello");
    let status = svc().get_status(&repo.path).unwrap();
    assert!(status.iter().any(|diff| diff.path == "说明.md"));
}

#[test]
fn restore_commit_keeps_unsaved_changes_as_draft() {
    let repo = TestRepo::new();
    let initial = git_out(&repo.path, &["rev-parse", "HEAD"]);
    repo.modify_file("README.md", "saved");
    svc().save_version(&repo.path, "second").unwrap();

    repo.modify_file("README.md", "unsaved edit");
    repo.modify_file("new.txt", "brand new");
    let outcome = svc().restore_commit(&repo.path, &initial).unwrap();
    assert!(outcome.draft_saved);
    assert_eq!(read(&repo, "README.md").unwrap(), "# test");
    assert!(read(&repo, "new.txt").is_none());

    let draft = svc().get_draft(&repo.path).unwrap().expect("draft kept");
    assert_eq!(draft.files, 2);

    let outcome = svc().restore_draft(&repo.path).unwrap();
    assert!(!outcome.draft_saved);
    assert_eq!(read(&repo, "README.md").unwrap(), "unsaved edit");
    assert_eq!(read(&repo, "new.txt").unwrap(), "brand new");
    assert!(svc().get_draft(&repo.path).unwrap().is_none());
    // The base of the unsaved version is current again and the later record is still listed.
    let logs = svc().get_log(&repo.path, 10).unwrap();
    assert!(logs.iter().any(|log| log.message == "second" && log.is_current));
}

#[test]
fn restore_draft_removes_files_deleted_in_draft() {
    let repo = TestRepo::new();
    repo.modify_file("keep.txt", "keep");
    svc().save_version(&repo.path, "add keep").unwrap();
    let with_keep = git_out(&repo.path, &["rev-parse", "HEAD"]);

    fs::remove_file(std::path::Path::new(&repo.path).join("keep.txt")).unwrap();
    svc().stash_to_draft(&repo.path).unwrap();
    assert_eq!(read(&repo, "keep.txt").unwrap(), "keep");

    svc().restore_draft(&repo.path).unwrap();
    assert!(read(&repo, "keep.txt").is_none());
    assert_eq!(git_out(&repo.path, &["rev-parse", "HEAD"]), with_keep);
}

#[test]
fn restore_draft_keeps_current_changes_as_new_draft() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "first draft");
    svc().stash_to_draft(&repo.path).unwrap();
    repo.modify_file("README.md", "second draft");

    let outcome = svc().restore_draft(&repo.path).unwrap();
    assert!(outcome.draft_saved);
    assert_eq!(read(&repo, "README.md").unwrap(), "first draft");

    svc().restore_draft(&repo.path).unwrap();
    assert_eq!(read(&repo, "README.md").unwrap(), "second draft");
}

#[test]
fn restore_commit_when_clean_does_not_create_draft() {
    let repo = TestRepo::new();
    let initial = git_out(&repo.path, &["rev-parse", "HEAD"]);
    repo.modify_file("README.md", "saved");
    svc().save_version(&repo.path, "second").unwrap();
    let outcome = svc().restore_commit(&repo.path, &initial).unwrap();
    assert!(!outcome.draft_saved);
    assert!(svc().get_draft(&repo.path).unwrap().is_none());
}

#[test]
fn discard_changes_keeps_hidden_safety_copy() {
    let repo = TestRepo::new();
    repo.modify_file("README.md", "oops");
    svc().discard_changes(&repo.path).unwrap();
    let archived = git_out(
        &repo.path,
        &["for-each-ref", "--format=%(refname)", "refs/project-hub/draft-archive/"],
    );
    assert!(!archived.is_empty());
    assert!(svc().get_draft(&repo.path).unwrap().is_none());
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
fn worktree_diff_includes_new_files() {
    let repo = TestRepo::new();
    repo.modify_file("fresh.txt", "fresh content");
    let diff = svc().worktree_diff(&repo.path, 10_000).unwrap();
    assert!(diff.contains("fresh.txt"));
    assert!(diff.contains("fresh content"));
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}

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
fn branches_with_slashes_keep_separate_history_and_drafts() {
    let repo = TestRepo::new();
    let main = svc().get_current_branch(&repo.path).unwrap();
    svc().create_branch(&repo.path, "feature/x").unwrap();

    svc().switch_branch(&repo.path, "feature/x").unwrap();
    let base = git_out(&repo.path, &["rev-parse", "HEAD"]);
    repo.modify_file("x.txt", "x");
    svc().save_version(&repo.path, "x work").unwrap();
    svc().restore_commit(&repo.path, &base).unwrap();
    repo.modify_file("x.txt", "x draft");
    svc().stash_to_draft(&repo.path).unwrap();

    svc().switch_branch(&repo.path, &main).unwrap();
    let logs = svc().get_log(&repo.path, 20).unwrap();
    assert!(!logs.iter().any(|log| log.message == "x work"));
    assert!(svc().get_draft(&repo.path).unwrap().is_none());

    svc().switch_branch(&repo.path, "feature/x").unwrap();
    assert!(svc().get_log(&repo.path, 20).unwrap().iter().any(|log| log.message == "x work"));
    assert!(svc().get_draft(&repo.path).unwrap().is_some());
    svc().switch_branch(&repo.path, &main).unwrap();
}

#[test]
fn deleting_a_version_archives_its_draft_and_forgets_its_records() {
    let repo = TestRepo::new();
    let main = svc().get_current_branch(&repo.path).unwrap();
    svc().create_and_switch_branch(&repo.path, "temp").unwrap();
    let base = git_out(&repo.path, &["rev-parse", "HEAD"]);
    repo.modify_file("t.txt", "saved");
    svc().save_version(&repo.path, "temp record").unwrap();
    svc().restore_commit(&repo.path, &base).unwrap();
    repo.modify_file("t.txt", "kept");
    svc().stash_to_draft(&repo.path).unwrap();
    svc().switch_branch(&repo.path, &main).unwrap();

    svc().delete_branch(&repo.path, "temp", true).unwrap();
    let archived = git_out(&repo.path, &["for-each-ref", "--format=%(refname)", "refs/project-hub/draft-archive/temp/"]);
    assert!(!archived.is_empty());

    svc().create_and_switch_branch(&repo.path, "temp").unwrap();
    let logs = svc().get_log(&repo.path, 20).unwrap();
    assert!(!logs.iter().any(|log| log.message == "temp record"));
    assert!(svc().get_draft(&repo.path).unwrap().is_none());
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

fn unborn_repo_with_file() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();
    fs::write(dir.path().join("a.txt"), "x").unwrap();
    dir
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
