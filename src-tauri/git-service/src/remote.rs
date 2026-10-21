//! Talking to the cloud repository: its address, signing in, fetching and uploading.

use crate::{GitService, PUSH_REJECTED};

/// The version (branch) the cloud keeps for `branch`, as this computer last fetched it.
pub(crate) fn cloud_ref(branch: &str) -> String {
    format!("refs/remotes/origin/{branch}")
}

/// Header that hands a GitHub token to git through the environment, so it never lands in the
/// command line or the repository config.
fn auth_header(token: Option<&str>) -> Option<String> {
    token.map(|token| {
        format!(
            "AUTHORIZATION: basic {}",
            base64_encode(format!("x-access-token:{}", token).as_bytes())
        )
    })
}

/// Never stop to ask for a password: without a usable sign-in git fails instead of hanging.
fn auth_envs(header: Option<&str>) -> Vec<(&str, &str)> {
    let mut envs = vec![("GCM_INTERACTIVE", "never"), ("GIT_TERMINAL_PROMPT", "0")];
    if let Some(header) = header {
        envs.push(("GIT_CONFIG_COUNT", "1"));
        envs.push(("GIT_CONFIG_KEY_0", "http.https://github.com/.extraheader"));
        envs.push(("GIT_CONFIG_VALUE_0", header));
    }
    envs
}

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

    /// Brings this computer's copy of the cloud's current version up to date. Returns false when
    /// the cloud does not have this version (yet).
    pub fn fetch(&self, repo_path: &str, token: Option<&str>) -> Result<bool, String> {
        let branch = self.require_current_branch(repo_path)?;
        if self.get_remote_url(repo_path, "origin").is_none() {
            return Err("REMOTE_NOT_LINKED".to_string());
        }
        let header = auth_header(token);
        let refspec = format!("+refs/heads/{branch}:{}", cloud_ref(&branch));
        match self.run_git_with_env(repo_path, &["fetch", "--no-tags", "origin", &refspec], &auth_envs(header.as_deref())) {
            Ok(_) => Ok(true),
            Err(error) if error.contains("couldn't find remote ref") => {
                // Removed from the cloud since: forget the stale copy too.
                let _ = self.run_git(repo_path, &["update-ref", "-d", &cloud_ref(&branch)]);
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    /// Pushes the current version to `origin`.
    pub fn push(&self, repo_path: &str, token: Option<&str>, force: bool) -> Result<(), String> {
        let branch = self.require_current_branch(repo_path)?;
        if self.get_remote_url(repo_path, "origin").is_none() {
            return Err("REMOTE_NOT_LINKED".to_string());
        }

        let header = auth_header(token);
        let envs = auth_envs(header.as_deref());

        let refspec = format!("refs/heads/{0}:refs/heads/{0}", branch);
        // Only overwrite what this computer last saw in the cloud; newer uploads from elsewhere win.
        let lease = self
            .resolve_commit(repo_path, &cloud_ref(&branch))
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
        let _ = self.run_git(repo_path, &["update-ref", &cloud_ref(&branch), "HEAD"]);
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
