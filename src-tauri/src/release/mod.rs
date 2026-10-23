//! Publishing a project's versions on GitHub Releases with the account signed in to Project Hub.
//! Whoever builds (an agent, for now) hands over the files; this does everything that touches
//! GitHub.
//!
//! - `facts`: what the project's files say (version numbers, a cloud build)
//! - here: publishing files from this computer (a draft, filled, then made public in one go, so a
//!   half-uploaded release is never public), starting the cloud build by uploading the version
//!   tag, and reading how a release is doing

pub mod facts;

use crate::github::{
    self,
    releases::{self, Release, RepoRef, WorkflowJob, WorkflowRun},
};
use git_service::{GitService, RecordSummary, PUSH_REJECTED};
use std::path::{Path, PathBuf};

/// How many save records release notes are drawn from at most.
pub const NOTES_RECORDS: usize = 50;

/// A project ready to be released: on GitHub, with the account's token.
pub struct ReleaseTarget<'a> {
    pub git: &'a GitService,
    pub path: &'a str,
    pub repo: RepoRef,
    pub token: String,
}

impl<'a> ReleaseTarget<'a> {
    pub fn open(git: &'a GitService, path: &'a str) -> Result<ReleaseTarget<'a>, String> {
        let url = git.get_remote_url(path, "origin").ok_or_else(|| "REMOTE_NOT_LINKED".to_string())?;
        let repo = RepoRef::from_url(&url)
            .ok_or_else(|| format!("Releases are published on GitHub, but this project's repository is {url}."))?;
        let token = github::load_token().ok_or_else(|| "GITHUB_NOT_SIGNED_IN".to_string())?;
        Ok(ReleaseTarget { git, path, repo, token })
    }

    /// The newest published (non-draft) release.
    pub fn latest_published(&self) -> Result<Option<Release>, String> {
        Ok(releases::list_releases(&self.token, &self.repo)?.into_iter().find(|release| !release.draft))
    }

    /// Save records since the release tagged `tag` (all of them without one), for release notes.
    pub fn records_since(&self, tag: Option<&str>) -> Result<Vec<RecordSummary>, String> {
        // The tag may have been made elsewhere (by a cloud build, say).
        let _ = self.git.fetch_tags(self.path, Some(&self.token));
        self.git.records_since(self.path, tag, NOTES_RECORDS)
    }

    /// Release notes from the titles of the save records since the latest release.
    pub fn default_notes(&self) -> Result<String, String> {
        let latest = self.latest_published()?;
        let records = self.records_since(latest.as_ref().map(|release| release.tag_name.as_str()))?;
        Ok(records.iter().map(|record| format!("- {}", record.title)).collect::<Vec<_>>().join("\n"))
    }

    /// A release is made from what the cloud has: uploads the saved records first.
    fn upload_records(&self) -> Result<(), String> {
        self.git.push(self.path, Some(&self.token), false).map_err(|error| {
            if error.starts_with(PUSH_REJECTED) {
                "The cloud has records this computer does not; download them first (or overwrite the cloud after asking the user).".to_string()
            } else {
                error
            }
        })
    }

    /// Publishes `tag` with `files` built on this computer. An existing release for the tag (a draft
    /// left by an interrupted upload, or one a cloud build made) gets the files instead; a release
    /// made here stays a draft until every file is on it.
    pub fn publish_files(&self, tag: &str, title: &str, notes: &str, prerelease: bool, files: &[PathBuf]) -> Result<Release, String> {
        self.upload_records()?;
        let mut release = match releases::release_for_tag(&self.token, &self.repo, tag)? {
            Some(release) => release,
            None => {
                let head = self.git.get_repo_state(self.path)?.head;
                if head.trim().is_empty() {
                    return Err("The project has no saved record yet.".to_string());
                }
                releases::create_draft(&self.token, &self.repo, tag, &head, title, notes, prerelease)?
            }
        };
        for file in files {
            let asset = releases::upload_asset(&self.token, &self.repo, &release, file)?;
            release.assets.retain(|existing| existing.name != asset.name);
            release.assets.push(asset);
        }
        if release.draft {
            release = releases::publish(&self.token, &self.repo, &release)?;
        }
        Ok(release)
    }

    /// Uploads the version tag on the current save record, which starts the project's release
    /// workflows (they create and fill the release). Returns those workflows.
    pub fn start_cloud_build(&self, tag: &str) -> Result<Vec<String>, String> {
        let workflows = facts::tag_workflows(Path::new(self.path));
        if workflows.is_empty() {
            return Err("The project has no cloud build: no workflow in .github/workflows is started by pushing a tag.".to_string());
        }
        self.upload_records()?;
        self.git.push_tag(self.path, tag, Some(&self.token))?;
        Ok(workflows)
    }

    /// How the release for `tag` (default: the newest) is doing.
    pub fn status(&self, tag: Option<&str>) -> Result<ReleaseStatus, String> {
        let all = releases::list_releases(&self.token, &self.repo)?;
        let tag = match tag {
            Some(tag) => tag.to_string(),
            None => all.first().map(|release| release.tag_name.clone()).ok_or("The repository has no releases yet.")?,
        };
        let release = all.into_iter().find(|release| release.tag_name == tag);
        let run = releases::runs_for_tag(&self.token, &self.repo, &tag)?.into_iter().next();
        let jobs = match &run {
            Some(run) => releases::run_jobs(&self.token, &self.repo, run.id)?,
            None => Vec::new(),
        };
        Ok(ReleaseStatus { tag, release, run, jobs })
    }
}

/// A release and the cloud build its tag started.
pub struct ReleaseStatus {
    pub tag: String,
    pub release: Option<Release>,
    /// The newest workflow run the tag started.
    pub run: Option<WorkflowRun>,
    pub jobs: Vec<WorkflowJob>,
}

impl ReleaseStatus {
    pub fn state(&self) -> &'static str {
        match &self.release {
            Some(release) if release.draft => "draft",
            Some(_) => "published",
            None => "not created yet",
        }
    }

    pub fn summary(&self) -> &'static str {
        let building = self.run.as_ref().is_some_and(|run| run.status.as_deref() != Some("completed"));
        let failed = self.jobs.iter().any(|job| matches!(job.conclusion.as_deref(), Some("failure" | "cancelled" | "timed_out")));
        match (self.state(), building, failed) {
            (_, _, true) => "A cloud build job failed; see its url for the log.",
            (_, true, _) => "The cloud build is still running; check again in a few minutes.",
            ("published", _, _) => "Published.",
            ("draft", _, _) => "The release is a draft (not public yet).",
            _ => "No release for this version yet.",
        }
    }
}
