//! Self-update from the publisher's GitHub releases. The release source is fixed in the program and
//! has nothing to do with the user's own GitHub login.
//!
//! - `check`: is there a newer release with an installer for this computer
//! - `download`: fetch that installer
//! - `install`: run it, quit, and reopen the updated app

mod check;
mod download;
mod install;

pub use check::{check, UpdateInfo};
pub use download::download;
pub use install::launch_installer;

use std::time::Duration;

/// Where new versions are published. Change only together with `scripts/release.mjs`.
pub const UPDATE_REPO: &str = "jinwandalaohu159/Project-Hub";
const USER_AGENT: &str = "Project-Hub-Updater";

pub fn releases_page() -> String {
    format!("https://github.com/{UPDATE_REPO}/releases")
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(timeout)
        .build()
}
