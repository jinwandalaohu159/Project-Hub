//! Running git: every call goes through here so flags and environment stay consistent.

use crate::GitService;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(windows)]
pub(crate) const CREATE_NO_WINDOW: u32 = 0x08000000;

impl GitService {
    pub(crate) fn git_dir(&self, repo_path: &str) -> Result<PathBuf, String> {
        let output = self.run_git(repo_path, &["rev-parse", "--absolute-git-dir"])?;
        Ok(PathBuf::from(output.trim()))
    }

    pub(crate) fn resolve_commit(&self, repo_path: &str, reference: &str) -> Option<String> {
        let spec = format!("{}^{{commit}}", reference);
        self.run_git(repo_path, &["rev-parse", "--verify", "-q", &spec])
            .ok()
            .map(|hash| hash.trim().to_string())
            .filter(|hash| !hash.is_empty())
    }

    pub(crate) fn tree_of(&self, repo_path: &str, commit: &str) -> Result<String, String> {
        let spec = format!("{}^{{tree}}", commit);
        Ok(self.run_git(repo_path, &["rev-parse", &spec])?.trim().to_string())
    }

    pub(crate) fn git_succeeds(&self, repo_path: &str, args: &[&str]) -> bool {
        self.base_command(repo_path, args, &[])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub(crate) fn base_command(&self, repo_path: &str, args: &[&str], envs: &[(&str, &str)]) -> Command {
        let mut command = hidden_command("git");
        // Repositories copied from another machine or user would otherwise fail with
        // "detected dubious ownership"; non-ASCII paths should stay readable.
        command.args(["-c", "safe.directory=*", "-c", "core.quotepath=false"]);
        // Git's automatic clean-up can repack a large history for many seconds after a save; it runs
        // in the background instead (see `housekeeping.rs`).
        command.args(["-c", "gc.auto=0", "-c", "maintenance.auto=false"]);
        if let Some(identity) = &self.identity {
            if !identity.name.trim().is_empty() {
                command.arg("-c").arg(format!("user.name={}", identity.name.trim()));
            }
            if !identity.email.trim().is_empty() {
                command.arg("-c").arg(format!("user.email={}", identity.email.trim()));
            }
        }
        command
            .args(args)
            .current_dir(repo_path)
            .env("GIT_TERMINAL_PROMPT", "0")
            // Status polling must not grab index.lock while a restore or save is running.
            .env("GIT_OPTIONAL_LOCKS", "0");
        for (key, value) in envs {
            command.env(key, value);
        }
        command
    }

    /// Like [`Self::run_git_with_env`], feeding `input` to git's standard input.
    pub(crate) fn run_git_with_input(
        &self,
        repo_path: &str,
        args: &[&str],
        envs: &[(&str, &str)],
        input: &str,
    ) -> Result<String, String> {
        if !Path::new(repo_path).is_dir() {
            return Err(format!("Project folder not found: {}", repo_path));
        }
        let mut child = self
            .base_command(repo_path, args, envs)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to execute git: {}", e))?;
        // Written from another thread: git may fill its output pipe before it has read all input.
        let writer = child.stdin.take().map(|mut stdin| {
            let input = input.as_bytes().to_vec();
            std::thread::spawn(move || stdin.write_all(&input))
        });
        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;
        let written = writer
            .map(|handle| handle.join().unwrap_or(Ok(())))
            .unwrap_or(Ok(()));
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git {} failed: {}", args.join(" "), stderr.trim()));
        }
        written.map_err(|e| format!("Failed to write to git: {}", e))?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub(crate) fn run_git(&self, repo_path: &str, args: &[&str]) -> Result<String, String> {
        self.run_git_with_env(repo_path, args, &[])
    }

    pub(crate) fn run_git_with_env(
        &self,
        repo_path: &str,
        args: &[&str],
        envs: &[(&str, &str)],
    ) -> Result<String, String> {
        if !Path::new(repo_path).is_dir() {
            return Err(format!("Project folder not found: {}", repo_path));
        }
        let output = self
            .base_command(repo_path, args, envs)
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let message = format!("{}\n{}", stderr.trim(), stdout.trim())
                .trim()
                .to_string();
            return Err(format!("git {} failed: {}", args.join(" "), message));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

pub(crate) fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}
