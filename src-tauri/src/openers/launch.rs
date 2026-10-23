//! Starting a program on a folder, detached from Project Hub.

use std::path::{Path, PathBuf};

/// How to start a program on a folder. `{path}` in `args` is the folder.
#[derive(Debug, Clone)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// Windows: a shell that needs a console window of its own.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub console: bool,
}

impl Launch {
    /// Starts the program on `folder`; it keeps running when Project Hub quits.
    pub fn spawn(&self, folder: &Path) -> Result<(), String> {
        let folder_text = folder.to_string_lossy();
        let mut command = std::process::Command::new(&self.program);
        command
            .args(self.args.iter().map(|arg| arg.replace("{path}", &folder_text)))
            .current_dir(folder)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NEW_CONSOLE: u32 = 0x00000010;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(if self.console { CREATE_NEW_CONSOLE } else { CREATE_NO_WINDOW });
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        command.spawn().map(|_| ()).map_err(|e| format!("OPENER_LAUNCH_FAILED:{e}"))
    }
}
