//! Hiding files from versions: the project's `.gitignore` rules, which are saved and uploaded with
//! the project, next to Project Hub's default rules, which only live on this computer.
//!
//! Adding a rule for files that earlier records already contain also stops saving them (they stay
//! on disk), otherwise the rule would have no effect; that shows up as an unsaved change.

use crate::status::DEFAULT_IGNORE_RULES;
use crate::GitService;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const GITIGNORE: &str = ".gitignore";

/// Rules that would hide the whole project.
const EVERYTHING: &[&str] = &["*", "**", "/*", "/**", "/", ".", "./", "*/", "**/*", "/**/*"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreRules {
    /// The rules in the project's `.gitignore`, in file order (comments and blank lines left out).
    pub rules: Vec<String>,
    /// Project Hub's own rules, applied on this computer only.
    pub defaults: Vec<String>,
    /// What is hidden right now; a wholly hidden folder appears once, as "folder/".
    pub hidden: Vec<String>,
    pub hidden_total: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IgnoreUpdate {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Requested rules that were already there (to add) or not there (to remove).
    pub unchanged: Vec<String>,
    /// Files earlier records contain that the new rules hide; they stay on disk and stop being saved.
    pub stopped_saving: Vec<String>,
}

/// A rule as written in `.gitignore`: one non-empty line that does not hide everything.
pub fn validate_ignore_rule(rule: &str) -> Result<String, String> {
    let rule = rule.trim();
    if rule.is_empty() {
        return Err("IGNORE_RULE_EMPTY".to_string());
    }
    if rule.contains('\n') || rule.contains('\r') {
        return Err(format!("IGNORE_RULE_MULTILINE:{rule}"));
    }
    if rule.starts_with('#') {
        return Err(format!("IGNORE_RULE_COMMENT:{rule}"));
    }
    if EVERYTHING.contains(&rule) || rule == GITIGNORE || rule == format!("/{GITIGNORE}") {
        return Err(format!("IGNORE_RULE_TOO_BROAD:{rule}"));
    }
    Ok(rule.to_string())
}

fn is_rule_line(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && !line.starts_with('#')
}

impl GitService {
    pub fn ignore_rules(&self, repo_path: &str, limit: usize) -> Result<IgnoreRules, String> {
        self.ensure_default_ignores(repo_path)?;
        let content = fs::read_to_string(Path::new(repo_path).join(GITIGNORE)).unwrap_or_default();
        let hidden = self.ignored_paths(repo_path)?;
        Ok(IgnoreRules {
            rules: content.lines().filter(|line| is_rule_line(line)).map(|line| line.trim().to_string()).collect(),
            defaults: DEFAULT_IGNORE_RULES.iter().map(|rule| rule.to_string()).collect(),
            hidden_total: hidden.len(),
            hidden: hidden.into_iter().take(limit).collect(),
        })
    }

    /// Adds and removes `.gitignore` rules in one go. Every rule is checked first, so a bad one
    /// changes nothing.
    pub fn update_ignore_rules(&self, repo_path: &str, add: &[String], remove: &[String]) -> Result<IgnoreUpdate, String> {
        let add = add.iter().map(|rule| validate_ignore_rule(rule)).collect::<Result<Vec<_>, _>>()?;
        let remove: Vec<String> = remove.iter().map(|rule| rule.trim().to_string()).filter(|rule| !rule.is_empty()).collect();
        lock_repo!(repo_path);
        self.ensure_default_ignores(repo_path)?;

        let path = Path::new(repo_path).join(GITIGNORE);
        let content = fs::read_to_string(&path).unwrap_or_default();
        let newline = if content.contains("\r\n") { "\r\n" } else { "\n" };
        let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
        let mut update = IgnoreUpdate::default();

        for rule in remove {
            let before = lines.len();
            lines.retain(|line| line.trim() != rule);
            if lines.len() < before {
                update.removed.push(rule);
            } else {
                update.unchanged.push(rule);
            }
        }
        for rule in add {
            if lines.iter().any(|line| line.trim() == rule) || update.added.contains(&rule) {
                update.unchanged.push(rule);
            } else {
                update.added.push(rule.clone());
                lines.push(rule);
            }
        }
        if update.added.is_empty() && update.removed.is_empty() {
            return Ok(update);
        }

        // Only files the new rules hide stop being saved: files kept on purpose despite an
        // existing rule (e.g. a tracked .env.local) are left alone.
        let tracked_hidden_before = self.tracked_but_ignored(repo_path)?;
        let mut text = lines.join(newline);
        text.push_str(newline);
        fs::write(&path, text).map_err(|e| format!("FILE_ERROR:{e}"))?;
        let newly_hidden: Vec<String> = self
            .tracked_but_ignored(repo_path)?
            .difference(&tracked_hidden_before)
            .cloned()
            .collect();
        for chunk in newly_hidden.chunks(100) {
            // Literal: a file named like a pattern ("[draft].md") must match only itself.
            let mut args = vec!["--literal-pathspecs", "rm", "--cached", "--quiet", "--ignore-unmatch", "--"];
            args.extend(chunk.iter().map(String::as_str));
            self.run_git(repo_path, &args)?;
        }
        update.stopped_saving = newly_hidden;
        Ok(update)
    }

    /// Files the records contain although a rule hides them.
    fn tracked_but_ignored(&self, repo_path: &str) -> Result<BTreeSet<String>, String> {
        let output = self.run_git(repo_path, &["ls-files", "--cached", "--ignored", "--exclude-standard", "-z"])?;
        Ok(output.split('\0').filter(|path| !path.is_empty()).map(str::to_string).collect())
    }
}
