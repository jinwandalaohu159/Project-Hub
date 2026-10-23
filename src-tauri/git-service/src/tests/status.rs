//! Unsaved changes: git status, default ignores, empty folders and their markers.

use super::*;
use crate::status::{parse_ignored_dirs, parse_porcelain_z, KEEP_FILE_NAME};

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
fn generic_build_names_are_hidden_only_at_the_root() {
    let repo = TestRepo::new();
    let root = std::path::Path::new(&repo.path);
    for file in ["build/out.bin", "src/build/main.rs", "app/dist/index.ts", "pkg/__pycache__/m.pyc"] {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "x").unwrap();
    }
    fs::create_dir_all(root.join("src").join("target").join("empty")).unwrap();
    fs::create_dir_all(root.join("lib").join("node_modules").join("empty")).unwrap();

    svc().save_version(&repo.path, "save").unwrap();

    let tracked = git_out(&repo.path, &["ls-files"]);
    assert!(tracked.contains("src/build/main.rs") && tracked.contains("app/dist/index.ts"), "{tracked}");
    assert!(tracked.contains(&format!("src/target/empty/{KEEP_FILE_NAME}")), "{tracked}");
    assert!(!tracked.contains("out.bin") && !tracked.contains("__pycache__"), "{tracked}");
    assert!(!root.join("lib/node_modules/empty").join(KEEP_FILE_NAME).exists());
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn old_default_rules_are_replaced_and_user_rules_kept() {
    let repo = TestRepo::new();
    let exclude = std::path::Path::new(&repo.path).join(".git").join("info").join("exclude");
    fs::create_dir_all(exclude.parent().unwrap()).unwrap();
    fs::write(&exclude, "secret.txt

# Project Hub default ignores
node_modules/
build/
").unwrap();
    let nested = std::path::Path::new(&repo.path).join("src").join("build").join("a.rs");
    fs::create_dir_all(nested.parent().unwrap()).unwrap();
    fs::write(&nested, "a").unwrap();

    let paths: Vec<String> = svc().get_status(&repo.path).unwrap().into_iter().map(|diff| diff.path).collect();
    assert_eq!(paths, ["src/"]);
    let content = fs::read_to_string(&exclude).unwrap();
    assert!(content.starts_with("secret.txt
"), "{content}");
    assert!(content.contains("
/build/
") && !content.contains("
build/
"), "{content}");
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
fn save_with_other_changes_also_keeps_empty_dir() {
    let repo = TestRepo::new();
    let empty_dir = std::path::Path::new(&repo.path).join(".work");
    fs::create_dir(&empty_dir).unwrap();
    repo.modify_file("README.md", "changed");

    svc().save_version(&repo.path, "save both").unwrap();

    let tracked = git_out(&repo.path, &["ls-files"]);
    assert!(tracked.contains(&format!(".work/{KEEP_FILE_NAME}")), "{tracked}");
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn empty_dir_is_listed_wherever_unsaved_changes_are_checked() {
    let repo = TestRepo::new();
    fs::create_dir(std::path::Path::new(&repo.path).join(".work")).unwrap();

    let summary = svc().get_status_summary(&repo.path, 200).unwrap();
    assert_eq!(summary.total, 1);
    assert_eq!(summary.files[0].path, ".work");
    assert_eq!(summary.files[0].status, FileStatus::Untracked);
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn ignored_empty_dirs_are_not_unsaved_and_get_no_marker() {
    let repo = TestRepo::new();
    let root = std::path::Path::new(&repo.path);
    fs::write(root.join(".gitignore"), ".venv/\ntmp/\ncache/\n").unwrap();
    git_out(&repo.path, &["add", ".gitignore"]);
    git_out(&repo.path, &["commit", "-m", "ignore"]);
    for dir in [".venv/Include", "tmp", "pkg/cache/empty"] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    fs::write(root.join("pkg").join("a.txt"), "a").unwrap();
    git_out(&repo.path, &["add", "pkg/a.txt"]);
    git_out(&repo.path, &["commit", "-m", "pkg"]);

    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
    assert_eq!(svc().get_status_summary(&repo.path, 200).unwrap().total, 0);

    // A save for other changes must not write markers into ignored folders.
    repo.modify_file("README.md", "changed");
    svc().save_version(&repo.path, "save").unwrap();
    for dir in [".venv/Include", "tmp", "pkg/cache/empty"] {
        assert!(!root.join(dir).join(KEEP_FILE_NAME).exists(), "{dir}");
    }
}

#[test]
fn empty_dir_inside_new_folder_is_not_listed_twice() {
    let repo = TestRepo::new();
    let root = std::path::Path::new(&repo.path);
    fs::create_dir_all(root.join("newdir").join("sub")).unwrap();
    fs::write(root.join("newdir").join("f.txt"), "f").unwrap();

    let status = svc().get_status(&repo.path).unwrap();
    let paths: Vec<&str> = status.iter().map(|diff| diff.path.as_str()).collect();
    assert_eq!(paths, ["newdir/"]);

    svc().save_version(&repo.path, "save").unwrap();
    assert!(root.join("newdir").join("sub").join(KEEP_FILE_NAME).exists());
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn hidden_untracked_files_setting_does_not_hide_changes() {
    let repo = TestRepo::new();
    git_out(&repo.path, &["config", "status.showUntrackedFiles", "no"]);
    repo.modify_file("new.txt", "new");
    assert!(svc().has_unsaved_changes(&repo.path).unwrap());
}

#[test]
fn marker_is_removed_once_folder_has_content() {
    let repo = TestRepo::new();
    let dir = std::path::Path::new(&repo.path).join("assets");
    fs::create_dir(&dir).unwrap();
    svc().save_version(&repo.path, "empty").unwrap();
    assert!(dir.join(KEEP_FILE_NAME).exists());

    fs::write(dir.join("logo.png"), "png").unwrap();
    svc().save_version(&repo.path, "filled").unwrap();
    assert!(!dir.join(KEEP_FILE_NAME).exists());
    assert!(!svc().has_unsaved_changes(&repo.path).unwrap());
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
fn ignored_entries_are_not_changes() {
    let output = " M a.txt\0!! .venv/\0!! app.log\0R  b.txt\0old.txt\0!! 数据/缓存/\0";
    let diffs = parse_porcelain_z(output);
    assert_eq!(diffs.len(), 2);
    let ignored = parse_ignored_dirs(output);
    assert_eq!(ignored.len(), 2);
    assert!(ignored.contains(".venv") && ignored.contains("数据/缓存"));
}

#[test]
fn status_keeps_non_ascii_paths_readable() {
    let repo = TestRepo::new();
    repo.modify_file("说明.md", "hello");
    let status = svc().get_status(&repo.path).unwrap();
    assert!(status.iter().any(|diff| diff.path == "说明.md"));
}
