//! Editing and deleting save records.
//!
//! Every record is a full snapshot of the files, so the records after a changed one are rebuilt
//! with exactly the same files on top of the new history: nothing is merged and nothing can
//! conflict. Only the current version's own history (its branch, the records kept listed after a
//! restore and its unsaved version) is rewritten; other versions keep their copies.

use crate::history::{LOG_FIELD_SEP, LOG_RECORD_SEP};
use crate::refs::{draft_ref, ref_segment, unique_suffix};
use crate::{DeleteRecordOutcome, GitService};
use std::collections::{HashMap, HashSet};
use std::fs;

/// Error code: the record is the only (or the current first) record and cannot be removed.
pub const RECORD_UNDELETABLE: &str = "RECORD_UNDELETABLE";
/// Error code: the record is not part of the current version's history.
pub const RECORD_NOT_IN_VERSION: &str = "RECORD_NOT_IN_VERSION";

const REWRITE_BACKUP_PREFIX: &str = "refs/project-hub/rewrite-backup/";
const REWRITE_TEMP_PREFIX: &str = "refs/project-hub/rewrite-tmp/";
const REWRITE_BACKUP_KEEP: usize = 30;
/// From this many rebuilt records on, one `git fast-import` beats a `commit-tree` per record.
const BULK_WRITE_FROM: usize = 3;
const META_FORMAT: &str = "--format=%H%x1f%T%x1f%P%x1f%an%x1f%ae%x1f%ad%x1f%cn%x1f%ce%x1f%cd%x1f%B%x1e";

#[derive(Debug, Clone)]
struct RecordMeta {
    hash: String,
    tree: String,
    parents: Vec<String>,
    author_name: String,
    author_email: String,
    /// Git's raw form (`<unix seconds> <offset>`), so the original time zone survives.
    author_date: String,
    committer_name: String,
    committer_email: String,
    committer_date: String,
    message: String,
}

/// What happens to the record being rewritten.
enum Change {
    Message(String),
    Delete,
}

/// A record in the new history: an existing commit, or the n-th record this rewrite writes.
#[derive(Debug, Clone, PartialEq)]
enum NewId {
    Existing(String),
    Written(usize),
}

/// A record to write: `meta`'s files, author and times on new parents, with `message`.
struct Rebuild<'a> {
    meta: &'a RecordMeta,
    parents: Vec<NewId>,
    message: String,
}

impl GitService {
    /// Replaces the title and details of a save record; its files and time stay the same.
    pub fn edit_record_message(&self, repo_path: &str, record: &str, message: &str) -> Result<(), String> {
        let message = message.trim();
        if message.is_empty() {
            return Err("The title cannot be empty.".to_string());
        }
        lock_repo!(repo_path);
        self.rewrite_record(repo_path, record, Change::Message(message.to_string()))
            .map(|_| ())
    }

    /// Removes a save record from the current version. Later records keep exactly their files.
    /// Deleting the current record leaves the files untouched; its changes become unsaved changes.
    pub fn delete_record(&self, repo_path: &str, record: &str) -> Result<DeleteRecordOutcome, String> {
        lock_repo!(repo_path);
        let kept_as_unsaved = self.rewrite_record(repo_path, record, Change::Delete)?;
        Ok(DeleteRecordOutcome { kept_as_unsaved })
    }

    /// Returns true when the branch tip was the deleted record and its changes are now unsaved.
    fn rewrite_record(&self, repo_path: &str, record: &str, change: Change) -> Result<bool, String> {
        let branch = self.require_current_branch(repo_path)?;
        let record = self
            .resolve_commit(repo_path, record)
            .ok_or_else(|| format!("{}: unknown record", RECORD_NOT_IN_VERSION))?;

        let branch_ref = format!("refs/heads/{}", branch);
        let draft = draft_ref(&branch);
        let tips = self.version_tips(repo_path, &branch_ref, &self.checkpoint_ref_prefix(&branch), &draft)?;
        let branch_tip = tips
            .iter()
            .find(|(name, _)| *name == branch_ref)
            .map(|(_, tip)| tip.clone())
            .ok_or_else(|| "The current version has no records.".to_string())?;
        let target = self.record_meta(repo_path, &record)?;
        let descendants = self.descendants(repo_path, &record, &tips)?;
        let descendant_set: HashSet<&str> = descendants.iter().map(|meta| meta.hash.as_str()).collect();

        // Only the branch and the records kept listed after a restore define this version's history.
        let in_version = tips
            .iter()
            .filter(|(name, _)| *name != draft)
            .any(|(_, tip)| *tip == record || descendant_set.contains(tip.as_str()));
        if !in_version {
            return Err(format!("{}: the record is not part of this version", RECORD_NOT_IN_VERSION));
        }

        // old record -> its place in the new history; `None` when it is dropped without replacement.
        let mut replaced: HashMap<String, Option<NewId>> = HashMap::new();
        let mut rebuilds: Vec<Rebuild> = Vec::new();
        let mut kept_as_unsaved = false;
        match change {
            Change::Message(message) => {
                rebuilds.push(Rebuild {
                    meta: &target,
                    parents: target.parents.iter().cloned().map(NewId::Existing).collect(),
                    message,
                });
                replaced.insert(record.clone(), Some(NewId::Written(0)));
            }
            Change::Delete => {
                let parent = target.parents.first().cloned();
                if branch_tip == record {
                    let Some(parent) = parent.as_deref() else {
                        return Err(format!("{}: the first record in use cannot be deleted", RECORD_UNDELETABLE));
                    };
                    kept_as_unsaved = self.tree_of(repo_path, parent)? != target.tree;
                }
                replaced.insert(record.clone(), parent.map(NewId::Existing));
            }
        }

        for meta in &descendants {
            let mut parents: Vec<NewId> = Vec::new();
            for parent in &meta.parents {
                let mapped = match replaced.get(parent) {
                    Some(new_parent) => new_parent.clone(),
                    None => Some(NewId::Existing(parent.clone())),
                };
                if let Some(mapped) = mapped {
                    if !parents.contains(&mapped) {
                        parents.push(mapped);
                    }
                }
            }
            let unchanged = parents.len() == meta.parents.len()
                && parents
                    .iter()
                    .zip(&meta.parents)
                    .all(|(new, old)| *new == NewId::Existing(old.clone()));
            let id = if unchanged {
                NewId::Existing(meta.hash.clone())
            } else {
                rebuilds.push(Rebuild {
                    meta,
                    parents,
                    message: meta.message.clone(),
                });
                NewId::Written(rebuilds.len() - 1)
            };
            replaced.insert(meta.hash.clone(), Some(id));
        }

        let written = self.write_records(repo_path, &rebuilds)?;
        let resolve = |id: &NewId| match id {
            NewId::Existing(hash) => hash.clone(),
            NewId::Written(index) => written[*index].clone(),
        };
        let replaced: HashMap<String, Option<String>> = replaced
            .iter()
            .map(|(old, new)| (old.clone(), new.as_ref().map(resolve)))
            .collect();

        self.move_version_refs(repo_path, &branch, &branch_ref, &draft, &tips, &replaced)?;
        self.prune_rewrite_backups(repo_path, &branch);
        Ok(kept_as_unsaved)
    }

    /// One transaction: every ref of the version moves to its new history, or none does. The old
    /// tips are kept under backup refs so the previous history can still be recovered by hand.
    fn move_version_refs(
        &self,
        repo_path: &str,
        branch: &str,
        branch_ref: &str,
        draft: &str,
        tips: &[(String, String)],
        replaced: &HashMap<String, Option<String>>,
    ) -> Result<(), String> {
        let backup_prefix = format!("{}{}/{}-", REWRITE_BACKUP_PREFIX, ref_segment(branch), unique_suffix());
        let mut input = String::new();
        let mut created: HashSet<String> = HashSet::new();
        for (index, (name, tip)) in tips.iter().enumerate() {
            let Some(new_tip) = replaced.get(tip) else {
                continue;
            };
            if new_tip.as_deref() == Some(tip.as_str()) {
                continue;
            }
            input.push_str(&format!("create {}{} {}\n", backup_prefix, index, tip));
            if name == branch_ref || name == draft {
                let new_tip = new_tip.as_ref().ok_or_else(|| {
                    format!("{}: the version would have no records left", RECORD_UNDELETABLE)
                })?;
                input.push_str(&format!("update {} {} {}\n", name, new_tip, tip));
            } else {
                // Records kept listed after a restore are named after the record they point at.
                input.push_str(&format!("delete {} {}\n", name, tip));
                if let Some(new_tip) = new_tip {
                    let new_name = self.checkpoint_ref(branch, new_tip);
                    let exists = tips.iter().any(|(other, _)| *other == new_name);
                    if !exists && created.insert(new_name.clone()) {
                        input.push_str(&format!("update {} {}\n", new_name, new_tip));
                    }
                }
            }
        }
        if !input.is_empty() {
            self.run_git_with_input(repo_path, &["update-ref", "--stdin"], &[], &input)?;
        }
        Ok(())
    }

    /// The branch tip, the records kept listed after restores and the unsaved version of `branch`.
    fn version_tips(
        &self,
        repo_path: &str,
        branch_ref: &str,
        checkpoint_prefix: &str,
        draft: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let output = self.run_git(
            repo_path,
            &[
                "for-each-ref",
                "--format=%(refname)%09%(objectname)",
                branch_ref,
                checkpoint_prefix,
                draft,
            ],
        )?;
        Ok(output
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .map(|(name, tip)| (name.trim().to_string(), tip.trim().to_string()))
            .filter(|(name, _)| name == branch_ref || name == draft || name.starts_with(checkpoint_prefix))
            .collect())
    }

    fn record_meta(&self, repo_path: &str, record: &str) -> Result<RecordMeta, String> {
        let output = self.run_git(
            repo_path,
            &["log", "-1", "--no-show-signature", "--date=raw", META_FORMAT, record],
        )?;
        parse_records(&output)
            .into_iter()
            .next()
            .ok_or_else(|| format!("{}: unknown record", RECORD_NOT_IN_VERSION))
    }

    /// Records built on `record` that lead to any of `tips`, parents before children.
    fn descendants(
        &self,
        repo_path: &str,
        record: &str,
        tips: &[(String, String)],
    ) -> Result<Vec<RecordMeta>, String> {
        let mut revisions = format!("^{}\n", record);
        for (_, tip) in tips {
            revisions.push_str(tip);
            revisions.push('\n');
        }
        let output = self.run_git_with_input(
            repo_path,
            &[
                "log",
                "--stdin",
                "--no-show-signature",
                "--ancestry-path",
                "--topo-order",
                "--reverse",
                "--date=raw",
                META_FORMAT,
            ],
            &[],
            &revisions,
        )?;
        Ok(parse_records(&output))
    }

    /// Writes the records in order and returns their hashes. Long rebuilds go through a single
    /// `git fast-import`; short ones, and any the importer refuses, use `commit-tree`.
    fn write_records(&self, repo_path: &str, rebuilds: &[Rebuild]) -> Result<Vec<String>, String> {
        if rebuilds.len() >= BULK_WRITE_FROM && rebuilds.iter().all(|rebuild| importable(rebuild.meta)) {
            if let Ok(hashes) = self.write_records_bulk(repo_path, rebuilds) {
                return Ok(hashes);
            }
        }
        let mut hashes: Vec<String> = Vec::with_capacity(rebuilds.len());
        for rebuild in rebuilds {
            let parents: Vec<String> = rebuild
                .parents
                .iter()
                .map(|parent| match parent {
                    NewId::Existing(hash) => hash.clone(),
                    NewId::Written(index) => hashes[*index].clone(),
                })
                .collect();
            hashes.push(self.commit_record(repo_path, rebuild.meta, &parents, &rebuild.message)?);
        }
        Ok(hashes)
    }

    /// A copy of `meta` with other parents and message; author, times and files are kept.
    fn commit_record(
        &self,
        repo_path: &str,
        meta: &RecordMeta,
        parents: &[String],
        message: &str,
    ) -> Result<String, String> {
        let mut args = vec!["commit-tree", meta.tree.as_str()];
        for parent in parents {
            args.push("-p");
            args.push(parent.as_str());
        }
        let envs = [
            ("GIT_AUTHOR_NAME", meta.author_name.as_str()),
            ("GIT_AUTHOR_EMAIL", meta.author_email.as_str()),
            ("GIT_AUTHOR_DATE", meta.author_date.as_str()),
            ("GIT_COMMITTER_NAME", meta.committer_name.as_str()),
            ("GIT_COMMITTER_EMAIL", meta.committer_email.as_str()),
            ("GIT_COMMITTER_DATE", meta.committer_date.as_str()),
        ];
        let hash = self.run_git_with_input(repo_path, &args, &envs, &with_final_newline(message))?;
        Ok(hash.trim().to_string())
    }

    fn write_records_bulk(&self, repo_path: &str, rebuilds: &[Rebuild]) -> Result<Vec<String>, String> {
        let suffix = unique_suffix();
        let marks_path = self.git_dir(repo_path)?.join(format!("project-hub-{}.marks", suffix));
        let temp_ref = format!("{}{}", REWRITE_TEMP_PREFIX, suffix);

        let mut stream = String::new();
        for (index, rebuild) in rebuilds.iter().enumerate() {
            let meta = rebuild.meta;
            let message = with_final_newline(&rebuild.message);
            // `reset` first: a record without parents must not continue from the previous one.
            stream.push_str(&format!("reset {temp_ref}\ncommit {temp_ref}\nmark :{}\n", index + 1));
            stream.push_str(&ident_line("author", &meta.author_name, &meta.author_email, &meta.author_date));
            stream.push_str(&ident_line("committer", &meta.committer_name, &meta.committer_email, &meta.committer_date));
            stream.push_str(&format!("data {}\n{}\n", message.len(), message));
            for (position, parent) in rebuild.parents.iter().enumerate() {
                let parent = match parent {
                    NewId::Existing(hash) => hash.clone(),
                    NewId::Written(written) => format!(":{}", written + 1),
                };
                stream.push_str(&format!("{} {}\n", if position == 0 { "from" } else { "merge" }, parent));
            }
            // The whole file tree of the record, set at once.
            stream.push_str(&format!("M 040000 {} \"\"\n\n", meta.tree));
        }

        let marks_arg = format!("--export-marks={}", marks_path.to_string_lossy());
        let imported = self.run_git_with_input(
            repo_path,
            &["fast-import", "--quiet", "--date-format=raw", &marks_arg],
            &[],
            &stream,
        );
        let marks = fs::read_to_string(&marks_path).unwrap_or_default();
        let _ = fs::remove_file(&marks_path);
        let _ = self.run_git(repo_path, &["update-ref", "-d", &temp_ref]);
        imported?;

        let by_mark: HashMap<usize, String> = marks
            .lines()
            .filter_map(|line| {
                let (mark, hash) = line.trim().split_once(' ')?;
                Some((mark.strip_prefix(':')?.parse().ok()?, hash.trim().to_string()))
            })
            .collect();
        (1..=rebuilds.len())
            .map(|mark| {
                by_mark
                    .get(&mark)
                    .cloned()
                    .ok_or_else(|| "fast-import did not report every record".to_string())
            })
            .collect()
    }

    fn prune_rewrite_backups(&self, repo_path: &str, branch: &str) {
        let prefix = format!("{}{}/", REWRITE_BACKUP_PREFIX, ref_segment(branch));
        let Ok(output) = self.run_git(
            repo_path,
            &["for-each-ref", "--sort=-refname", "--format=%(refname)", &prefix],
        ) else {
            return;
        };
        let stale: String = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .skip(REWRITE_BACKUP_KEEP)
            .map(|name| format!("delete {}\n", name))
            .collect();
        if !stale.is_empty() {
            let _ = self.run_git_with_input(repo_path, &["update-ref", "--stdin"], &[], &stale);
        }
    }
}

fn with_final_newline(message: &str) -> String {
    let mut text = message.to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// fast-import writes identities as `name <email>`; anything that would break that line is left
/// to `commit-tree`, which takes them from the environment.
fn importable(meta: &RecordMeta) -> bool {
    let clean = |value: &str| !value.contains(['<', '>', '\n', '\r']);
    clean(&meta.author_name)
        && clean(&meta.author_email)
        && clean(&meta.committer_name)
        && clean(&meta.committer_email)
        && !meta.author_date.is_empty()
        && !meta.committer_date.is_empty()
}

fn ident_line(kind: &str, name: &str, email: &str, date: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        format!("{kind} <{}> {date}\n", email.trim())
    } else {
        format!("{kind} {name} <{}> {date}\n", email.trim())
    }
}

fn parse_records(output: &str) -> Vec<RecordMeta> {
    output
        .split(LOG_RECORD_SEP)
        .filter_map(|record| {
            let record = record.trim_start_matches(['\n', '\r']);
            let fields: Vec<&str> = record.splitn(10, LOG_FIELD_SEP).collect();
            if fields.len() < 10 || fields[0].trim().is_empty() {
                return None;
            }
            Some(RecordMeta {
                hash: fields[0].trim().to_string(),
                tree: fields[1].trim().to_string(),
                parents: fields[2].split_whitespace().map(ToString::to_string).collect(),
                author_name: fields[3].to_string(),
                author_email: fields[4].to_string(),
                author_date: fields[5].trim().to_string(),
                committer_name: fields[6].to_string(),
                committer_email: fields[7].to_string(),
                committer_date: fields[8].trim().to_string(),
                message: fields[9].trim_end().to_string(),
            })
        })
        .collect()
}
