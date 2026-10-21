use git_service::Identity;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub const DEFAULT_MCP_PORT: u16 = 47821;
const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubAccount {
    pub login: String,
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

impl GithubAccount {
    /// GitHub links commits to the account through this address without exposing a real email.
    pub fn noreply_email(&self) -> String {
        format!("{}+{}@users.noreply.github.com", self.id, self.login)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub author_name: String,
    pub author_email: String,
    pub mcp_enabled: bool,
    pub mcp_port: u16,
    pub github: Option<GithubAccount>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            author_name: String::new(),
            author_email: String::new(),
            mcp_enabled: true,
            mcp_port: DEFAULT_MCP_PORT,
            github: None,
        }
    }
}

impl AppSettings {
    /// Custom author first, then the signed-in GitHub account; `None` lets git config decide.
    pub fn effective_identity(&self) -> Option<Identity> {
        let name = self.author_name.trim();
        let email = self.author_email.trim();
        let github_name = self
            .github
            .as_ref()
            .map(|gh| gh.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| gh.login.clone()));
        let github_email = self.github.as_ref().map(GithubAccount::noreply_email);

        let name = if name.is_empty() { github_name } else { Some(name.to_string()) };
        let email = if email.is_empty() { github_email } else { Some(email.to_string()) };
        if name.is_none() && email.is_none() {
            return None;
        }
        Some(Identity {
            name: name.unwrap_or_default(),
            email: email.unwrap_or_default(),
        })
    }
}

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("project-hub")
}

pub fn projects_storage_path() -> PathBuf {
    app_data_dir().join("projects.json")
}

fn settings_path() -> PathBuf {
    app_data_dir().join(SETTINGS_FILE_NAME)
}

pub fn load_settings() -> AppSettings {
    fs::read_to_string(settings_path())
        .ok()
        .and_then(|content| serde_json::from_str::<AppSettings>(&content).ok())
        .map(|mut settings| {
            if settings.mcp_port == 0 {
                settings.mcp_port = DEFAULT_MCP_PORT;
            }
            settings
        })
        .unwrap_or_default()
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> GithubAccount {
        GithubAccount {
            login: "octo".into(),
            id: 42,
            name: Some("Octo Cat".into()),
            avatar_url: None,
            email: None,
        }
    }

    #[test]
    fn identity_prefers_custom_author() {
        let settings = AppSettings {
            author_name: "Me".into(),
            author_email: "me@example.com".into(),
            github: Some(account()),
            ..Default::default()
        };
        let id = settings.effective_identity().unwrap();
        assert_eq!(id.name, "Me");
        assert_eq!(id.email, "me@example.com");
    }

    #[test]
    fn identity_falls_back_to_github_noreply() {
        let settings = AppSettings {
            github: Some(account()),
            ..Default::default()
        };
        let id = settings.effective_identity().unwrap();
        assert_eq!(id.name, "Octo Cat");
        assert_eq!(id.email, "42+octo@users.noreply.github.com");
    }

    #[test]
    fn identity_absent_without_any_source() {
        assert!(AppSettings::default().effective_identity().is_none());
    }
}
