//! Programs the user added themselves: any program, with its own arguments.

use super::{Launch, OpenerPrefs};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Arguments of a program added without any: just the folder.
const DEFAULT_ARGS: &str = "{path}";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomOpener {
    pub id: String,
    pub name: String,
    pub program: String,
    /// Arguments, `{path}` standing for the folder; quote ones with spaces.
    pub args: String,
}

impl CustomOpener {
    pub(super) fn launch(&self) -> Launch {
        let args = split_args(&self.args);
        // A macOS app bundle is a folder: `open -a App folder` hands it the folder, anything else
        // the user wrote goes after `--args`.
        if cfg!(target_os = "macos") && self.program.trim_end_matches('/').ends_with(".app") {
            let extra: Vec<String> = args.into_iter().filter(|arg| arg != DEFAULT_ARGS).collect();
            let mut open_args = vec!["-a".to_string(), self.program.clone(), DEFAULT_ARGS.to_string()];
            if !extra.is_empty() {
                open_args.push("--args".to_string());
                open_args.extend(extra);
            }
            return Launch { program: PathBuf::from("/usr/bin/open"), args: open_args, console: false };
        }
        Launch { program: PathBuf::from(&self.program), args, console: false }
    }
}

/// Adds a program of the user's; returns its id.
pub fn add(prefs: &mut OpenerPrefs, name: &str, program: &str, args: &str) -> Result<String, String> {
    let name = name.trim();
    let program = program.trim();
    if name.is_empty() {
        return Err("NAME_EMPTY".to_string());
    }
    if program.is_empty() || !Path::new(program).exists() {
        return Err(format!("OPENER_PROGRAM_MISSING:{program}"));
    }
    let id = (1..)
        .map(|n| format!("custom-{n}"))
        .find(|id| !prefs.custom.iter().any(|custom| &custom.id == id))
        .expect("an unused id");
    let args = args.trim();
    prefs.custom.push(CustomOpener {
        id: id.clone(),
        name: name.to_string(),
        program: program.to_string(),
        args: if args.is_empty() { DEFAULT_ARGS.to_string() } else { args.to_string() },
    });
    Ok(id)
}

/// Removes a program of the user's, and every choice that named it.
pub fn remove(prefs: &mut OpenerPrefs, id: &str) {
    prefs.custom.retain(|custom| custom.id != id);
    prefs.hidden.retain(|hidden| hidden != id);
    if prefs.default.as_deref() == Some(id) {
        prefs.default = None;
    }
}

/// Splits an argument line on spaces, keeping "quoted parts" together.
fn split_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut has_arg = false;
    for ch in line.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                has_arg = true;
            }
            ch if ch.is_whitespace() && !quoted => {
                if has_arg {
                    args.push(std::mem::take(&mut current));
                    has_arg = false;
                }
            }
            ch => {
                current.push(ch);
                has_arg = true;
            }
        }
    }
    if has_arg {
        args.push(current);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_arguments_keeping_quoted_parts() {
        assert_eq!(split_args(r#"--new-window "{path}"  -x"#), vec!["--new-window", "{path}", "-x"]);
        assert_eq!(split_args(r#""C:\My Folder\a b""#), vec![r"C:\My Folder\a b"]);
        assert_eq!(split_args(r#"-e """#), vec!["-e", ""]);
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn refuses_programs_that_do_not_exist() {
        let mut prefs = OpenerPrefs::default();
        assert!(add(&mut prefs, "x", "/no/such/program", "").unwrap_err().starts_with("OPENER_PROGRAM_MISSING"));
        assert_eq!(add(&mut prefs, " ", "/bin/sh", ""), Err("NAME_EMPTY".to_string()));
    }
}
