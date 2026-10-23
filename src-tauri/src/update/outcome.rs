//! How the last update went, kept for the app to tell the user once it is open again, and the
//! version a failed update stopped at, so that one is not retried on every start.

use crate::settings::app_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;

const OUTCOME_FILE: &str = "update-outcome.json";
const SKIP_FILE: &str = "update-skip-version";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOutcome {
    pub version: String,
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
}

/// Records that updating to `version` succeeded (`error` is `None`) or failed.
pub fn record(version: &str, error: Option<&str>) {
    let dir = app_data_dir();
    let _ = fs::create_dir_all(&dir);
    let outcome = UpdateOutcome { version: version.to_string(), ok: error.is_none(), error: error.map(str::to_string) };
    if let Ok(json) = serde_json::to_string(&outcome) {
        let _ = fs::write(dir.join(OUTCOME_FILE), json);
    }
    let skip = dir.join(SKIP_FILE);
    match error {
        Some(_) => drop(fs::write(skip, version)),
        None => drop(fs::remove_file(skip)),
    }
}

/// Takes the outcome of the last update, if the app has not shown it yet.
pub fn take() -> Option<UpdateOutcome> {
    let path = app_data_dir().join(OUTCOME_FILE);
    let outcome = serde_json::from_str(&fs::read_to_string(&path).ok()?).ok();
    let _ = fs::remove_file(path);
    outcome
}

/// A version whose update failed is not retried automatically on every start; the user can still
/// install it by hand.
pub fn skipped_version() -> Option<String> {
    fs::read_to_string(app_data_dir().join(SKIP_FILE)).ok().map(|v| v.trim().to_string())
}
