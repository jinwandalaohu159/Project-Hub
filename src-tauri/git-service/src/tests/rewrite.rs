//! Editing and deleting save records, which rewrites the history after them.

use super::*;

fn record_by_title(repo: &TestRepo, title: &str) -> CommitLog {
    svc()
        .get_log(&repo.path, 50)
        .unwrap()
        .into_iter()
        .find(|commit| commit.message == title)
        .unwrap_or_else(|| panic!("record {title} not listed"))
}

#[test]
fn edit_record_message_keeps_files_and_later_records() {
    let repo = repo_with_records();
    let head_tree = git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]);
    let two = record_by_title(&repo, "two");

    svc().edit_record_message(&repo.path, &two.hash, "renamed\n\nwith details").unwrap();

    let log = svc().get_log(&repo.path, 50).unwrap();
    let titles: Vec<&str> = log.iter().map(|commit| commit.message.as_str()).collect();
    assert_eq!(titles, vec!["three", "renamed", "one", "initial"]);
    let renamed = &log[1];
    assert_eq!(renamed.body, "with details");
    assert_eq!(renamed.time, two.time);
    assert_eq!(git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]), head_tree);
    assert!(log[0].is_current);
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn delete_middle_record_keeps_later_files() {
    let repo = repo_with_records();
    let head_tree = git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]);
    let two = record_by_title(&repo, "two");

    let outcome = svc().delete_record(&repo.path, &two.hash).unwrap();

    assert!(!outcome.kept_as_unsaved);
    let titles: Vec<String> = svc().get_log(&repo.path, 50).unwrap().into_iter().map(|c| c.message).collect();
    assert_eq!(titles, vec!["three", "one", "initial"]);
    assert_eq!(git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]), head_tree);
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
    // The old history stays recoverable by hand.
    assert!(git_out(&repo.path, &["for-each-ref", "refs/project-hub/rewrite-backup/"]).contains("rewrite-backup"));
}

#[test]
fn delete_current_record_keeps_files_as_unsaved_changes() {
    let repo = repo_with_records();
    let three = record_by_title(&repo, "three");

    let outcome = svc().delete_record(&repo.path, &three.hash).unwrap();

    assert!(outcome.kept_as_unsaved);
    assert_eq!(fs::read_to_string(std::path::Path::new(&repo.path).join("a.txt")).unwrap(), "v3");
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
    let log = svc().get_log(&repo.path, 50).unwrap();
    assert_eq!(log[0].message, "two");
    assert!(log[0].is_current);
}

#[test]
fn delete_only_record_is_refused() {
    let repo = TestRepo::new();
    let initial = record_by_title(&repo, "initial");
    let error = svc().delete_record(&repo.path, &initial.hash).unwrap_err();
    assert!(error.starts_with(rewrite::RECORD_UNDELETABLE), "{error}");
    assert_eq!(svc().get_log(&repo.path, 50).unwrap().len(), 1);
}

#[test]
fn delete_first_record_reroots_later_ones() {
    let repo = repo_with_records();
    let head_tree = git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]);
    let initial = record_by_title(&repo, "initial");

    svc().delete_record(&repo.path, &initial.hash).unwrap();

    let titles: Vec<String> = svc().get_log(&repo.path, 50).unwrap().into_iter().map(|c| c.message).collect();
    assert_eq!(titles, vec!["three", "two", "one"]);
    assert_eq!(git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]), head_tree);
}

#[test]
fn rewriting_after_restore_keeps_later_records_listed() {
    let repo = repo_with_records();
    let one = record_by_title(&repo, "one");
    let three = record_by_title(&repo, "three");
    let three_tree = git_out(&repo.path, &["rev-parse", &format!("{}^{{tree}}", three.hash)]);
    svc().restore_commit(&repo.path, &one.hash).unwrap();

    // "two" is only listed through the restore checkpoint now.
    let two = record_by_title(&repo, "two");
    svc().delete_record(&repo.path, &two.hash).unwrap();

    let log = svc().get_log(&repo.path, 50).unwrap();
    let titles: Vec<&str> = log.iter().map(|commit| commit.message.as_str()).collect();
    assert_eq!(titles, vec!["three", "one", "initial"]);
    assert!(log[1].is_current);
    let new_three = &log[0];
    assert_ne!(new_three.hash, three.hash);
    assert_eq!(git_out(&repo.path, &["rev-parse", &format!("{}^{{tree}}", new_three.hash)]), three_tree);
    // Restoring the rebuilt record still gives its exact files.
    svc().restore_commit(&repo.path, &new_three.hash).unwrap();
    assert_eq!(fs::read_to_string(std::path::Path::new(&repo.path).join("a.txt")).unwrap(), "v3");
}

#[test]
fn rewriting_base_keeps_unsaved_version_usable() {
    let repo = repo_with_records();
    repo.modify_file("a.txt", "draft work");
    svc().stash_to_draft(&repo.path).unwrap();
    let three = record_by_title(&repo, "three");

    svc().edit_record_message(&repo.path, &three.hash, "three edited").unwrap();

    let draft = svc().get_draft(&repo.path).unwrap().expect("draft kept");
    assert_eq!(draft.base, git_out(&repo.path, &["rev-parse", "HEAD"]));
    svc().restore_draft(&repo.path).unwrap();
    assert_eq!(fs::read_to_string(std::path::Path::new(&repo.path).join("a.txt")).unwrap(), "draft work");
}

#[test]
fn record_from_other_version_is_rejected() {
    let repo = repo_with_records();
    repo.create_branch("other");
    svc().switch_branch(&repo.path, "other").unwrap();
    repo.modify_file("b.txt", "other");
    svc().save_version(&repo.path, "other only").unwrap();
    let other_only = record_by_title(&repo, "other only");
    svc().switch_branch(&repo.path, "master").or_else(|_| svc().switch_branch(&repo.path, "main")).unwrap();

    let error = svc().delete_record(&repo.path, &other_only.hash).unwrap_err();
    assert!(error.starts_with(rewrite::RECORD_NOT_IN_VERSION), "{error}");
}

fn trees_by_title(repo: &TestRepo) -> Vec<(String, String)> {
    svc()
        .get_log(&repo.path, 100)
        .unwrap()
        .into_iter()
        .map(|commit| {
            let tree = git_out(&repo.path, &["rev-parse", &format!("{}^{{tree}}", commit.hash)]);
            (commit.message, tree)
        })
        .collect()
}

#[test]
fn deleting_early_record_in_long_history_keeps_every_later_snapshot() {
    let repo = TestRepo::new();
    for index in 1..=8 {
        repo.modify_file("a.txt", &format!("v{index}"));
        repo.modify_file(&format!("f{index}.txt"), "x");
        svc().save_version(&repo.path, &format!("r{index}")).unwrap();
    }
    let before = trees_by_title(&repo);
    let r1 = record_by_title(&repo, "r1");

    svc().delete_record(&repo.path, &r1.hash).unwrap();

    let after = trees_by_title(&repo);
    let expected: Vec<(String, String)> = before.into_iter().filter(|(title, _)| title != "r1").collect();
    assert_eq!(after, expected);
    let log = svc().get_log(&repo.path, 100).unwrap();
    assert!(log[0].is_current);
    assert_eq!(log[0].author, "Test");
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
    assert!(git_out(&repo.path, &["for-each-ref", "refs/project-hub/rewrite-tmp/"]).is_empty());
}

#[test]
fn editing_record_before_a_merge_keeps_the_merged_work() {
    let repo = repo_with_records();
    let main = svc().get_current_branch(&repo.path).unwrap();
    svc().create_and_switch_branch(&repo.path, "side").unwrap();
    repo.modify_file("side.txt", "side work");
    svc().save_version(&repo.path, "side work").unwrap();
    svc().switch_branch(&repo.path, &main).unwrap();
    svc().merge_branch(&repo.path, "side").unwrap();
    let head_tree = git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]);
    let one = record_by_title(&repo, "one");

    svc().edit_record_message(&repo.path, &one.hash, "one edited").unwrap();

    assert_eq!(git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]), head_tree);
    let parents = git_out(&repo.path, &["log", "-1", "--format=%P"]);
    assert_eq!(parents.split_whitespace().count(), 2, "merge keeps both sides");
    let side_tip = git_out(&repo.path, &["rev-parse", "side"]);
    assert!(parents.contains(&side_tip));
    assert!(svc().get_log(&repo.path, 50).unwrap().iter().any(|c| c.message == "one edited"));
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn editing_with_empty_title_is_refused() {
    let repo = repo_with_records();
    let two = record_by_title(&repo, "two");
    assert!(svc().edit_record_message(&repo.path, &two.hash, "   ").is_err());
    assert_eq!(record_by_title(&repo, "two").hash, two.hash);
}
