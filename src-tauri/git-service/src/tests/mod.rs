//! Tests for `GitService`, grouped by area. The helpers here build throwaway repositories.

use super::*;
use std::fs;
use std::process::Command;

mod branches;
mod draft;
mod history;
mod ignore;
mod remote;
mod rewrite;
mod status;
mod sync;

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

fn git_out(path: &str, args: &[&str]) -> String {
    let output = Command::new("git").args(args).current_dir(path).output().unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn read(repo: &TestRepo, name: &str) -> Option<String> {
    fs::read_to_string(std::path::Path::new(&repo.path).join(name)).ok()
}

fn unborn_repo_with_file() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();
    fs::write(dir.path().join("a.txt"), "x").unwrap();
    dir
}

/// initial -> "one" -> "two" -> "three", each changing a.txt.
fn repo_with_records() -> TestRepo {
    let repo = TestRepo::new();
    for (index, title) in ["one", "two", "three"].iter().enumerate() {
        repo.modify_file("a.txt", &format!("v{}", index + 1));
        svc().save_version(&repo.path, title).unwrap();
    }
    repo
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
fn settled_reads_wait_for_running_operations() {
    let repo = repo_with_records();
    let svc = svc();
    let value = svc.settled(&repo.path, |s| s.has_unsaved_changes(&repo.path).unwrap());
    assert!(!value);
}

#[test]
fn input_larger_than_pipe_buffer_is_passed_to_git() {
    let repo = TestRepo::new();
    let message = format!("big\n\n{}", "x".repeat(300_000));
    let tree = git_out(&repo.path, &["rev-parse", "HEAD^{tree}"]);
    let hash = svc()
        .run_git_with_input(&repo.path, &["commit-tree", &tree], &[], &message)
        .unwrap();
    let body = git_out(&repo.path, &["log", "-1", "--format=%b", hash.trim()]);
    assert_eq!(body.len(), 300_000);
}
