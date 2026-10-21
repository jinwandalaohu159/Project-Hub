//! Launch-at-login for macOS (LaunchAgent) and Linux (XDG autostart entry).

#![cfg_attr(target_os = "windows", allow(dead_code))]

#[cfg(not(target_os = "windows"))]
use std::{fs, path::PathBuf};

const APP_ID: &str = "com.projecthub.app";
const HIDDEN_ARG: &str = "--hidden";

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// LaunchAgent that starts the app hidden in the menu bar when the user logs in.
pub fn launch_agent_plist(exe: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{APP_ID}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>{HIDDEN_ARG}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
"#,
        xml_escape(exe)
    )
}

/// Quotes one argument of a desktop-entry `Exec` line (freedesktop spec).
fn desktop_exec_arg(value: &str) -> String {
    let needs_quotes = value
        .chars()
        .any(|ch| ch.is_whitespace() || "\"'\\><~|&;$*?#()`".contains(ch));
    if !needs_quotes {
        return value.to_string();
    }
    let mut quoted = String::from("\"");
    for ch in value.chars() {
        if matches!(ch, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    quoted
}

pub fn desktop_entry(exe: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Project Hub\nExec={} {HIDDEN_ARG}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        desktop_exec_arg(exe)
    )
}

#[cfg(not(target_os = "windows"))]
fn launcher_path() -> Result<String, String> {
    // An AppImage runs from a temporary mount; the stable file is the AppImage itself.
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        if !appimage.trim().is_empty() {
            return Ok(appimage);
        }
    }
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "windows"))]
fn entry_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "APP_DIR_UNKNOWN".to_string())?;
    if cfg!(target_os = "macos") {
        Ok(home.join("Library").join("LaunchAgents").join(format!("{APP_ID}.plist")))
    } else {
        let config = dirs::config_dir().unwrap_or_else(|| home.join(".config"));
        Ok(config.join("autostart").join("project-hub.desktop"))
    }
}

#[cfg(not(target_os = "windows"))]
fn expected_entry() -> Result<String, String> {
    let exe = launcher_path()?;
    Ok(if cfg!(target_os = "macos") {
        launch_agent_plist(&exe)
    } else {
        desktop_entry(&exe)
    })
}

/// Enabled only when the entry exists and still points at this installation.
#[cfg(not(target_os = "windows"))]
pub fn is_enabled() -> Result<bool, String> {
    let path = entry_path()?;
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(false);
    };
    Ok(content == expected_entry()?)
}

#[cfg(not(target_os = "windows"))]
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let path = entry_path()?;
    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, expected_entry()?).map_err(|e| e.to_string())?;
    } else if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    if is_enabled()? != enabled {
        return Err("AUTOSTART_UNCONFIRMED".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_escapes_path_and_starts_hidden() {
        let plist = launch_agent_plist("/Applications/Project & Hub.app/Contents/MacOS/project-hub");
        assert!(plist.contains("<string>/Applications/Project &amp; Hub.app/Contents/MacOS/project-hub</string>"));
        assert!(plist.contains("<string>--hidden</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>\n  <true/>"));
    }

    #[test]
    fn desktop_entry_quotes_paths_with_spaces() {
        assert!(desktop_entry("/usr/bin/project-hub").contains("Exec=/usr/bin/project-hub --hidden\n"));
        assert!(desktop_entry("/home/me/My Apps/Project Hub.AppImage")
            .contains("Exec=\"/home/me/My Apps/Project Hub.AppImage\" --hidden\n"));
        assert!(desktop_entry("/opt/a$b").contains("Exec=\"/opt/a\\$b\" --hidden"));
    }
}
