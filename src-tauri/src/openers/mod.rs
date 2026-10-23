//! Opening a project folder in another program: a terminal, an editor or the file manager.
//!
//! - `catalog`: the programs Project Hub knows, found on this computer (one file per platform)
//! - `custom`: programs the user added
//! - `launch`: starting one on a folder
//!
//! Here: the list the user sees, with their hidden choices applied.

mod catalog;
mod custom;
mod launch;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub use custom::{add as add_custom, remove as remove_custom, CustomOpener};
pub use launch::Launch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenerKind {
    Terminal,
    Editor,
    Files,
    /// Programs the user added.
    Other,
}

/// What the user chose; kept in the app settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenerPrefs {
    pub hidden: Vec<String>,
    pub custom: Vec<CustomOpener>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenerView {
    pub id: String,
    pub name: String,
    pub kind: OpenerKind,
    /// The program that runs, shown when managing the list.
    pub program: String,
    pub custom: bool,
    pub hidden: bool,
}

struct Opener {
    view: OpenerView,
    launch: Launch,
}

impl Opener {
    fn new(id: &str, name: &str, kind: OpenerKind, launch: Launch, custom: bool) -> Opener {
        Opener {
            view: OpenerView {
                id: id.to_string(),
                name: name.to_string(),
                kind,
                program: launch.program.to_string_lossy().into_owned(),
                custom,
                hidden: false,
            },
            launch,
        }
    }
}

/// Every program found on this computer plus the user's own, in a stable order, with the user's
/// choices applied.
fn available(prefs: &OpenerPrefs) -> Vec<Opener> {
    let found = catalog::known()
        .into_iter()
        .filter_map(|known| Some(Opener::new(known.id, known.name, known.kind, (known.find)()?, false)));
    let added = prefs.custom.iter().map(|custom| {
        let mut opener = Opener::new(&custom.id, &custom.name, OpenerKind::Other, custom.launch(), true);
        // What the user chose, not what starts it (`open` for macOS app bundles).
        opener.view.program = custom.program.clone();
        opener
    });
    let mut openers: Vec<Opener> = found.chain(added).collect();

    for opener in &mut openers {
        opener.view.hidden = prefs.hidden.contains(&opener.view.id);
    }
    openers
}

pub fn list(prefs: &OpenerPrefs) -> Vec<OpenerView> {
    available(prefs).into_iter().map(|opener| opener.view).collect()
}

/// Opens `folder` with the program `id`.
pub fn open(prefs: &OpenerPrefs, id: &str, folder: &Path) -> Result<(), String> {
    if !folder.is_dir() {
        return Err(format!("PATH_NOT_FOUND:{}", folder.display()));
    }
    let opener = available(prefs)
        .into_iter()
        .find(|opener| opener.view.id == id)
        .ok_or_else(|| "OPENER_NOT_FOUND".to_string())?;
    opener.launch.spawn(folder)
}

pub fn set_hidden(prefs: &mut OpenerPrefs, id: &str, hidden: bool) {
    prefs.hidden.retain(|existing| existing != id);
    if hidden {
        prefs.hidden.push(id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_programs_are_added_hidden_and_removed() {
        let mut prefs = OpenerPrefs::default();
        let exe = std::env::current_exe().unwrap();
        let id = add_custom(&mut prefs, " My tool ", exe.to_str().unwrap(), "").unwrap();
        assert_eq!(prefs.custom[0].name, "My tool");
        assert_eq!(prefs.custom[0].args, "{path}");
        assert!(list(&prefs).iter().any(|view| view.id == id && view.custom && view.kind == OpenerKind::Other));

        set_hidden(&mut prefs, &id, true);
        assert!(list(&prefs).into_iter().find(|view| view.id == id).unwrap().hidden);

        remove_custom(&mut prefs, &id);
        assert!(prefs.custom.is_empty() && prefs.hidden.is_empty());
    }
}
