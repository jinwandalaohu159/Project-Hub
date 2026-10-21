use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Written inside `.git` so a project can be recognised after its folder is renamed or moved.
const PROJECT_MARKER_FILE: &str = "project-hub-id";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_git_repo: bool,
    pub added_at: DateTime<Utc>,
    /// Computed on every listing; never trusted from disk.
    #[serde(default)]
    pub missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelocatedProject {
    pub id: String,
    pub name: String,
    pub old_path: String,
    pub new_path: String,
}

pub struct ProjectService {
    storage_path: String,
    projects: Vec<Project>,
}

impl ProjectService {
    pub fn new(storage_path: &str) -> Self {
        let projects = Self::load_projects(storage_path).unwrap_or_default();
        Self {
            storage_path: storage_path.to_string(),
            projects,
        }
    }

    pub fn add_project(&mut self, path: &str) -> Result<Project, String> {
        self.add_project_with_id(path, None)
    }

    /// Registers a folder, keeping `id` when it comes from an import so logs and markers still match.
    pub fn add_project_with_id(&mut self, path: &str, id: Option<&str>) -> Result<Project, String> {
        let dir = Path::new(path);
        if !dir.exists() {
            return Err(format!("PATH_NOT_FOUND:{path}"));
        }
        if !dir.is_dir() {
            return Err(format!("NOT_A_FOLDER:{path}"));
        }
        if self.projects.iter().any(|p| same_path(&p.path, path)) {
            return Err(format!("PROJECT_EXISTS:{path}"));
        }

        let id = match id {
            Some(id) if !self.projects.iter().any(|p| p.id == id) => id.to_string(),
            _ => Uuid::new_v4().to_string(),
        };
        let project = Project {
            id,
            name: folder_name(dir),
            path: path.to_string(),
            is_git_repo: dir.join(".git").exists(),
            added_at: Utc::now(),
            missing: false,
        };
        write_marker(dir, &project.id);

        self.projects.push(project.clone());
        self.save_projects()?;
        Ok(project)
    }

    pub fn remove_project(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .projects
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| format!("Project not found: {}", id))?;
        self.projects.remove(index);
        self.save_projects()?;
        Ok(())
    }

    pub fn list_projects(&self) -> Vec<Project> {
        self.projects
            .iter()
            .cloned()
            .map(|mut project| {
                let dir = Path::new(&project.path);
                project.missing = !dir.is_dir();
                if !project.missing {
                    project.is_git_repo = dir.join(".git").exists();
                }
                project
            })
            .collect()
    }

    pub fn get_project(&self, id: &str) -> Option<Project> {
        self.list_projects().into_iter().find(|p| p.id == id)
    }

    /// Writes the identity marker into a project whose repository was just created.
    pub fn stamp_marker(&self, id: &str) {
        if let Some(project) = self.projects.iter().find(|p| p.id == id) {
            write_marker(Path::new(&project.path), id);
        }
    }

    /// Points an existing project at a new folder (after the user renamed or moved it).
    pub fn relocate_project(&mut self, id: &str, new_path: &str) -> Result<Project, String> {
        let dir = Path::new(new_path);
        if !dir.is_dir() {
            return Err(format!("NOT_A_FOLDER:{new_path}"));
        }
        if self
            .projects
            .iter()
            .any(|p| p.id != id && same_path(&p.path, new_path))
        {
            return Err(format!("FOLDER_IN_USE:{new_path}"));
        }
        let project = self
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("Project not found: {}", id))?;
        project.path = new_path.to_string();
        project.name = folder_name(dir);
        project.is_git_repo = dir.join(".git").exists();
        write_marker(dir, id);
        let updated = project.clone();
        self.save_projects()?;
        Ok(updated)
    }

    /// Finds missing projects whose folder was renamed or moved next to its old location, using the
    /// marker inside `.git`. Also stamps markers into healthy projects that predate them.
    pub fn auto_relocate_missing(&mut self) -> Vec<RelocatedProject> {
        let mut relocated = Vec::new();
        let mut taken: Vec<String> = self.projects.iter().map(|p| p.path.clone()).collect();

        for project in self.projects.iter_mut() {
            let dir = Path::new(&project.path);
            if dir.is_dir() {
                if dir.join(".git").is_dir() && read_marker(dir).is_none() {
                    write_marker(dir, &project.id);
                }
                continue;
            }

            let Some(found) = find_marked_folder(dir, &project.id, &taken) else {
                continue;
            };
            let new_path = found.to_string_lossy().to_string();
            relocated.push(RelocatedProject {
                id: project.id.clone(),
                name: folder_name(&found),
                old_path: project.path.clone(),
                new_path: new_path.clone(),
            });
            taken.push(new_path.clone());
            project.path = new_path;
            project.name = folder_name(&found);
            project.is_git_repo = true;
        }

        if !relocated.is_empty() {
            let _ = self.save_projects();
        }
        relocated
    }

    fn load_projects(path: &str) -> Result<Vec<Project>, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    fn save_projects(&self) -> Result<(), String> {
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.projects).map_err(|e| e.to_string())?;
        fs::write(&self.storage_path, json).map_err(|e| e.to_string())
    }
}

fn folder_name(dir: &Path) -> String {
    dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

fn normalize_path(path: &str) -> String {
    let normalized = path.replace('\\', "/").trim_end_matches('/').to_string();
    if cfg!(windows) {
        normalized.to_lowercase()
    } else {
        normalized
    }
}

pub fn same_path(a: &str, b: &str) -> bool {
    normalize_path(a) == normalize_path(b)
}

fn marker_path(dir: &Path) -> Option<PathBuf> {
    let git_dir = dir.join(".git");
    git_dir.is_dir().then(|| git_dir.join(PROJECT_MARKER_FILE))
}

pub fn read_marker(dir: &Path) -> Option<String> {
    let content = fs::read_to_string(marker_path(dir)?).ok()?;
    let id = content.trim().to_string();
    (!id.is_empty()).then_some(id)
}

fn write_marker(dir: &Path, id: &str) {
    if let Some(path) = marker_path(dir) {
        let _ = fs::write(path, id);
    }
}

/// Looks for a folder carrying `id` among the siblings of the missing path, then one level deeper
/// (covers "renamed" and "moved into a sibling folder").
fn find_marked_folder(missing: &Path, id: &str, taken: &[String]) -> Option<PathBuf> {
    let mut search_root = missing.parent()?;
    while !search_root.is_dir() {
        search_root = search_root.parent()?;
    }

    let is_match = |candidate: &Path| {
        let candidate_str = candidate.to_string_lossy();
        read_marker(candidate).as_deref() == Some(id)
            && !taken.iter().any(|path| same_path(path, &candidate_str))
    };

    let siblings: Vec<PathBuf> = fs::read_dir(search_root)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    if let Some(found) = siblings.iter().find(|path| is_match(path)) {
        return Some(found.clone());
    }
    for sibling in siblings.iter().take(200) {
        let Ok(children) = fs::read_dir(sibling) else {
            continue;
        };
        for child in children.flatten().take(200) {
            let path = child.path();
            if path.is_dir() && is_match(&path) {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_storage() -> (String, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("projects.json");
        (path.to_str().unwrap().to_string(), dir)
    }

    #[test]
    fn add_project_list_it() {
        let (storage, _dir) = temp_storage();
        let tmp = tempfile::TempDir::new().unwrap();
        let project_path = tmp.path().to_str().unwrap();

        let mut svc = ProjectService::new(&storage);
        let _added = svc.add_project(project_path).unwrap();
        let list = svc.list_projects();

        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, tmp.path().file_name().unwrap().to_str().unwrap());
        assert_eq!(list[0].path, project_path);
    }

    #[test]
    fn add_project_rejects_nonexistent_path() {
        let (storage, _dir) = temp_storage();
        let mut svc = ProjectService::new(&storage);
        let result = svc.add_project("/nonexistent/path");
        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("PATH_NOT_FOUND:"));
    }

    #[test]
    fn add_project_rejects_duplicate() {
        let (storage, _dir) = temp_storage();
        let tmp = tempfile::TempDir::new().unwrap();
        let project_path = tmp.path().to_str().unwrap();

        let mut svc = ProjectService::new(&storage);
        svc.add_project(project_path).unwrap();
        let result = svc.add_project(project_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().starts_with("PROJECT_EXISTS:"));
    }

    #[test]
    fn remove_project() {
        let (storage, _dir) = temp_storage();
        let tmp = tempfile::TempDir::new().unwrap();
        let project_path = tmp.path().to_str().unwrap();

        let mut svc = ProjectService::new(&storage);
        let added = svc.add_project(project_path).unwrap();
        svc.remove_project(&added.id).unwrap();
        assert!(svc.list_projects().is_empty());
    }

    #[test]
    fn remove_project_not_found() {
        let (storage, _dir) = temp_storage();
        let mut svc = ProjectService::new(&storage);
        let result = svc.remove_project("fake-id");
        assert!(result.is_err());
    }

    #[test]
    fn persist_and_restore() {
        let (storage, _dir) = temp_storage();
        let tmp = tempfile::TempDir::new().unwrap();
        let project_path = tmp.path().to_str().unwrap();

        {
            let mut svc = ProjectService::new(&storage);
            svc.add_project(project_path).unwrap();
        }

        let svc2 = ProjectService::new(&storage);
        assert_eq!(svc2.list_projects().len(), 1);
        assert_eq!(svc2.list_projects()[0].path, project_path);
    }

    #[test]
    fn detect_git_repo() {
        let (storage, _dir) = temp_storage();
        let tmp = tempfile::TempDir::new().unwrap();

        let mut svc = ProjectService::new(&storage);
        let added = svc.add_project(tmp.path().to_str().unwrap()).unwrap();
        assert!(!added.is_git_repo);

        fs::create_dir(tmp.path().join(".git")).unwrap();
        let _added2 = svc.add_project(tmp.path().to_str().unwrap());
        // duplicate, but we can test with another dir
        let tmp2 = tempfile::TempDir::new().unwrap();
        fs::create_dir(tmp2.path().join(".git")).unwrap();
        let added2 = svc.add_project(tmp2.path().to_str().unwrap()).unwrap();
        assert!(added2.is_git_repo);
    }

    #[test]
    fn missing_project_is_flagged_and_can_be_relocated() {
        let (storage, _dir) = temp_storage();
        let root = tempfile::TempDir::new().unwrap();
        let old = root.path().join("old-name");
        fs::create_dir_all(old.join(".git")).unwrap();

        let mut svc = ProjectService::new(&storage);
        let added = svc.add_project(old.to_str().unwrap()).unwrap();
        assert_eq!(read_marker(&old).as_deref(), Some(added.id.as_str()));

        let other = root.path().join("picked-by-hand");
        fs::rename(&old, &other).unwrap();
        assert!(svc.list_projects()[0].missing);

        let moved = svc.relocate_project(&added.id, other.to_str().unwrap()).unwrap();
        assert_eq!(moved.name, "picked-by-hand");
        assert!(!svc.list_projects()[0].missing);
    }

    #[test]
    fn auto_relocate_finds_renamed_and_moved_folders() {
        let (storage, _dir) = temp_storage();
        let root = tempfile::TempDir::new().unwrap();
        let a = root.path().join("alpha");
        let b = root.path().join("beta");
        fs::create_dir_all(a.join(".git")).unwrap();
        fs::create_dir_all(b.join(".git")).unwrap();

        let mut svc = ProjectService::new(&storage);
        let pa = svc.add_project(a.to_str().unwrap()).unwrap();
        let pb = svc.add_project(b.to_str().unwrap()).unwrap();

        fs::rename(&a, root.path().join("alpha-renamed")).unwrap();
        fs::create_dir_all(root.path().join("archive")).unwrap();
        fs::rename(&b, root.path().join("archive").join("beta")).unwrap();

        let relocated = svc.auto_relocate_missing();
        assert_eq!(relocated.len(), 2);
        let list = svc.list_projects();
        let fa = list.iter().find(|p| p.id == pa.id).unwrap();
        let fb = list.iter().find(|p| p.id == pb.id).unwrap();
        assert!(!fa.missing && fa.name == "alpha-renamed");
        assert!(!fb.missing && fb.path.ends_with("beta"));

        let reloaded = ProjectService::new(&storage);
        assert!(reloaded.list_projects().iter().all(|p| !p.missing));
    }

    #[test]
    fn add_project_with_id_keeps_imported_id() {
        let (storage, _dir) = temp_storage();
        let tmp = tempfile::TempDir::new().unwrap();
        let mut svc = ProjectService::new(&storage);
        let added = svc
            .add_project_with_id(tmp.path().to_str().unwrap(), Some("imported-id"))
            .unwrap();
        assert_eq!(added.id, "imported-id");
    }
}
