//! The unsaved version kept per version when the working tree is replaced or discarded.

use super::*;

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
fn worktree_diff_includes_new_files() {
    let repo = TestRepo::new();
    repo.modify_file("fresh.txt", "fresh content");
    let diff = svc().worktree_diff(&repo.path, 10_000).unwrap();
    assert!(diff.contains("fresh.txt"));
    assert!(diff.contains("fresh content"));
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}
