//! Cloud repository: remote address, sync state and pushing.

use crate::{GitService, SyncState, PUSH_REJECTED};

impl GitService {
    pub fn get_remote_url(&self, repo_path: &str, name: &str) -> Option<String> {
        self.run_git(repo_path, &["remote", "get-url", name])
            .ok()
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
    }

    pub fn set_remote_url(&self, repo_path: &str, name: &str, url: &str) -> Result<(), String> {
        if self.get_remote_url(repo_path, name).is_some() {
            self.run_git(repo_path, &["remote", "set-url", name, url])?;
        } else {
            self.run_git(repo_path, &["remote", "add", name, url])?;
        }
        Ok(())
    }

    pub fn sync_state(&self, repo_path: &str) -> Result<SyncState, String> {
        let branch = self.get_current_branch(repo_path)?;
        let remote_url = self.get_remote_url(repo_path, "origin");
        let upstream = format!("refs/remotes/origin/{}", branch);
        let has_upstream = !branch.is_empty() && self.resolve_commit(repo_path, &upstream).is_some();
        let (ahead, behind) = if has_upstream {
            let range = format!("{}...HEAD", upstream);
            let output = self.run_git(repo_path, &["rev-list", "--left-right", "--count", &range])?;
            let mut counts = output.split_whitespace().map(|n| n.parse::<usize>().unwrap_or(0));
            let behind = counts.next().unwrap_or(0);
            let ahead = counts.next().unwrap_or(0);
            (ahead, behind)
        } else {
            let ahead = self
                .run_git(repo_path, &["rev-list", "--count", "HEAD"])
                .ok()
                .and_then(|n| n.trim().parse::<usize>().ok())
                .unwrap_or(0);
            (ahead, 0)
        };
        Ok(SyncState {
            remote_url,
            branch,
            has_upstream,
            ahead,
            behind,
        })
    }

    /// Pushes the current version to `origin`. A GitHub token, when given, is passed through the
    /// environment so it never lands in the command line or the repository config.
    pub fn push(&self, repo_path: &str, token: Option<&str>, force: bool) -> Result<(), String> {
        let branch = self.require_current_branch(repo_path)?;
        if self.get_remote_url(repo_path, "origin").is_none() {
            return Err("This project is not linked to a cloud repository yet.".to_string());
        }

        let header = token.map(|token| {
            format!(
                "AUTHORIZATION: basic {}",
                base64_encode(format!("x-access-token:{}", token).as_bytes())
            )
        });
        let mut envs: Vec<(&str, &str)> = vec![("GCM_INTERACTIVE", "never")];
        if let Some(header) = header.as_deref() {
            envs.push(("GIT_CONFIG_COUNT", "1"));
            envs.push(("GIT_CONFIG_KEY_0", "http.https://github.com/.extraheader"));
            envs.push(("GIT_CONFIG_VALUE_0", header));
        }

        let refspec = format!("refs/heads/{0}:refs/heads/{0}", branch);
        // Only overwrite what this computer last saw in the cloud; newer uploads from elsewhere win.
        let lease = self
            .resolve_commit(repo_path, &format!("refs/remotes/origin/{}", branch))
            .map(|known| format!("--force-with-lease=refs/heads/{}:{}", branch, known));
        let mut args = vec!["push", "-u"];
        if force {
            args.push(lease.as_deref().unwrap_or("--force"));
        }
        args.push("origin");
        args.push(&refspec);

        self.run_git_with_env(repo_path, &args, &envs)
            .map(|_| ())
            .map_err(|error| {
                let lower = error.to_lowercase();
                if lower.contains("[rejected]")
                    || lower.contains("non-fast-forward")
                    || lower.contains("fetch first")
                    || lower.contains("stale info")
                {
                    format!("{}: {}", PUSH_REJECTED, error)
                } else {
                    error
                }
            })?;
        // Keep the tracking ref accurate even when git did not update it (e.g. forced pushes).
        let _ = self.run_git(
            repo_path,
            &["update-ref", &format!("refs/remotes/origin/{}", branch), "HEAD"],
        );
        Ok(())
    }
}

pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
    }
    out
}
