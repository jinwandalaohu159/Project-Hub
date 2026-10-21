//! Versions: listing, creating, switching, merging and deleting them.

use super::*;

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
fn switch_branch_cleans_untracked_files() {
    let repo = TestRepo::new();
    repo.create_branch("other");
    let temp_file = std::path::Path::new(&repo.path).join("temp.txt");
    fs::write(&temp_file, "not saved").unwrap();

    svc().switch_branch(&repo.path, "other").unwrap();

    assert!(!temp_file.exists());
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
