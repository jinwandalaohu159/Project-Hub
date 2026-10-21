//! Reading and writing one log: its size and age limits, appending entries (one JSON object per
//! line) and pruning what falls outside the limits.

use super::location::{project_log_dir, project_log_path, project_log_settings_path};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const DEFAULT_LOG_MAX_MB: u64 = 50;

pub(crate) const DEFAULT_LOG_RETENTION_DAYS: u64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LogSettings {
    max_mb: u64,
    retention_days: u64,
    log_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LogEntry {
    timestamp_ms: u64,
    level: String,
    project_id: String,
    project_name: String,
    action: String,
    message: String,
}

pub(crate) fn default_log_settings(project_id: &str) -> Result<LogSettings, String> {
    Ok(LogSettings {
        max_mb: DEFAULT_LOG_MAX_MB,
        retention_days: DEFAULT_LOG_RETENTION_DAYS,
        log_dir: project_log_dir(project_id)?.display().to_string(),
    })
}

pub(crate) fn load_log_settings(project_id: &str) -> Result<LogSettings, String> {
    let default = default_log_settings(project_id)?;
    let path = project_log_settings_path(project_id)?;
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Ok(default),
    };

    let mut settings: LogSettings = serde_json::from_str(&content).unwrap_or(default);
    settings.max_mb = settings.max_mb.clamp(1, 1024);
    settings.retention_days = settings.retention_days.clamp(1, 3650);
    settings.log_dir = project_log_dir(project_id)?.display().to_string();
    Ok(settings)
}

pub(crate) fn save_log_settings(project_id: &str, settings: &LogSettings) -> Result<(), String> {
    let log_dir = project_log_dir(project_id)?;
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(project_log_settings_path(project_id)?, content).map_err(|e| e.to_string())
}

/// Stores new limits and applies them to the existing log right away.
pub(crate) fn update_log_settings(project_id: &str, max_mb: u64, retention_days: u64) -> Result<LogSettings, String> {
    let mut settings = default_log_settings(project_id)?;
    settings.max_mb = max_mb.clamp(1, 1024);
    settings.retention_days = retention_days.clamp(1, 3650);
    save_log_settings(project_id, &settings)?;
    prune_log(project_id)?;
    load_log_settings(project_id)
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub(crate) fn prune_log(project_id: &str) -> Result<(), String> {
    let settings = load_log_settings(project_id)?;
    let path = project_log_path(project_id)?;
    if !path.exists() {
        return Ok(());
    }

    let mut content = String::new();
    OpenOptions::new()
        .read(true)
        .open(&path)
        .map_err(|e| e.to_string())?
        .read_to_string(&mut content)
        .map_err(|e| e.to_string())?;

    let cutoff_ms = now_ms().saturating_sub(settings.retention_days * 24 * 60 * 60 * 1000);
    let mut retained = Vec::new();
    for line in content.lines() {
        let keep = serde_json::from_str::<LogEntry>(line)
            .map(|entry| entry.timestamp_ms >= cutoff_ms)
            .unwrap_or(true);
        if keep {
            retained.push(line.to_string());
        }
    }

    let max_bytes = (settings.max_mb * 1024 * 1024) as usize;
    let mut total_bytes = retained.iter().map(|line| line.len() + 1).sum::<usize>();
    while total_bytes > max_bytes && !retained.is_empty() {
        let removed = retained.remove(0);
        total_bytes = total_bytes.saturating_sub(removed.len() + 1);
    }

    let mut output = retained.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    fs::write(path, output).map_err(|e| e.to_string())
}

/// The newest `limit` entries (at most 1000), newest first.
pub(crate) fn read_log_entries(project_id: &str, limit: usize) -> Result<Vec<LogEntry>, String> {
    prune_log(project_id)?;
    let path = project_log_path(project_id)?;
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut entries = content
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
        .take(limit.clamp(1, 1000))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp_ms));
    Ok(entries)
}

/// Appending and pruning rewrite the same file; one writer at a time keeps lines from getting lost.
pub(super) static LOG_WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn write_log_entry(
    project_id: &str,
    project_name: &str,
    level: &str,
    action: &str,
    message: &str,
) -> Result<(), String> {
    let _guard = LOG_WRITE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    write_log_entry_unlocked(project_id, project_name, level, action, message)
}

/// [`write_log_entry`] for a caller that already holds [`LOG_WRITE`].
pub(super) fn write_log_entry_unlocked(
    project_id: &str,
    project_name: &str,
    level: &str,
    action: &str,
    message: &str,
) -> Result<(), String> {
    let log_dir = project_log_dir(project_id)?;
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let entry = LogEntry {
        timestamp_ms: now_ms(),
        level: normalize_level(level).to_string(),
        project_id: project_id.to_string(),
        project_name: project_name.to_string(),
        action: action.to_string(),
        message: message.to_string(),
    };
    let line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(project_log_path(project_id)?)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())?;
    prune_log(project_id)
}

/// Best-effort entry in a project's log, for background paths (MCP) where errors must not surface.
pub(crate) fn append_project_log(project_id: &str, project_name: &str, action: &str, message: &str) {
    let _ = write_log_entry(project_id, project_name, "info", action, message);
}

fn normalize_level(level: &str) -> &'static str {
    match level {
        "error" => "error",
        "warn" | "warning" => "warn",
        _ => "info",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_fall_back_to_info() {
        assert_eq!(normalize_level("error"), "error");
        assert_eq!(normalize_level("warning"), "warn");
        assert_eq!(normalize_level(""), "info");
        assert_eq!(normalize_level("anything"), "info");
    }
}
