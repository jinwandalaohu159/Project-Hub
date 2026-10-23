//! Self-update from the publisher's GitHub releases. The release source is fixed in the program and
//! has nothing to do with the user's own GitHub login.
//!
//! - `check`: is there a newer release with an installer for this computer
//! - `download`: fetch that installer
//! - `on_launch`: with automatic updates on, start updating as the app opens
//! - `handoff`: the app cannot replace itself, so it starts a helper process with a job and quits
//! - `helper` does that job (install, reopen) and `helper_window` shows its progress
//! - `install`: what installing and reopening mean on each platform
//! - `outcome`: how the last update went, for the app to say once it is open again

mod check;
mod download;
pub mod handoff;
mod helper;
pub mod helper_window;
mod install;
pub mod on_launch;
pub mod outcome;

pub use check::{check, UpdateInfo};
pub use download::download;
pub use install::app_location;

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
