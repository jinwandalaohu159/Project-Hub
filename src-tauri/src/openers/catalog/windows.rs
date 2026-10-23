//! Windows: programs in their usual folders, in the installed-programs list (whatever drive they
//! went on) or above their command-line script on the PATH.

use super::*;

fn env_dir(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap_or_default()
}

fn local_programs(relative: &str) -> PathBuf {
    env_dir("LOCALAPPDATA").join("Programs").join(relative)
}

fn program_files(relative: &str) -> PathBuf {
    env_dir("ProgramFiles").join(relative)
}

fn system32(relative: &str) -> PathBuf {
    env_dir("SystemRoot").join("System32").join(relative)
}

/// A shell in a console window of its own, started in the folder.
fn console(program: PathBuf, args: &[&str]) -> Launch {
    Launch { console: true, ..launch(program, args) }
}

/// One entry of Windows' installed-programs list.
struct Installed {
    name: String,
    location: PathBuf,
    /// Usually the program itself.
    icon: PathBuf,
}

/// Windows' installed-programs list (what "Apps & features" shows), which knows where each
/// program went, whatever drive it was installed on. Read at most every few seconds.
fn installed_programs() -> std::sync::Arc<Vec<Installed>> {
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use winreg::{enums::*, RegKey};
    static CACHE: OnceLock<Mutex<Option<(Instant, Arc<Vec<Installed>>)>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if let Some((read, list)) = cache.as_ref() {
        if read.elapsed() < Duration::from_secs(10) {
            return list.clone();
        }
    }
    const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL_32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
    let mut list = Vec::new();
    for (root, path) in [(HKEY_CURRENT_USER, UNINSTALL), (HKEY_LOCAL_MACHINE, UNINSTALL), (HKEY_LOCAL_MACHINE, UNINSTALL_32)] {
        let Ok(key) = RegKey::predef(root).open_subkey(path) else { continue };
        for entry in key.enum_keys().flatten() {
            let Ok(app) = key.open_subkey(&entry) else { continue };
            let Ok(name) = app.get_value::<String, _>("DisplayName") else { continue };
            let clean = |value: String| PathBuf::from(value.split(',').next().unwrap_or("").trim().trim_matches('"'));
            list.push(Installed {
                name,
                location: clean(app.get_value("InstallLocation").unwrap_or_default()),
                icon: clean(app.get_value("DisplayIcon").unwrap_or_default()),
            });
        }
    }
    let list = Arc::new(list);
    *cache = Some((Instant::now(), list.clone()));
    list
}

/// `relative` inside a listed program whose name starts with `listed_as`.
fn listed(listed_as: &str, relative: &str) -> Option<PathBuf> {
    let file_name = Path::new(relative).file_name()?;
    installed_programs().iter().filter(|app| app.name.starts_with(listed_as)).find_map(|app| {
        let from_icon = app.icon.file_name() == Some(file_name);
        first_existing(
            [from_icon.then(|| app.icon.clone()), Some(app.location.join(relative)).filter(|_| !app.location.as_os_str().is_empty())]
                .into_iter()
                .flatten(),
        )
    })
}

/// The program `exe` above a command-line script found on the PATH (e.g. `bin\code.cmd` sits
/// below Code.exe).
fn above_script(scripts: &[&str], exe: &str) -> Option<PathBuf> {
    let script = on_path(scripts)?;
    script.ancestors().skip(1).take(4).map(|dir| dir.join(exe)).find(|path| path.is_file())
}

/// A program in its usual folders, in the installed-programs list, or above its script on the PATH.
fn program(exe: &str, folders: &[&str], listed_as: &str, scripts: &[&str]) -> Option<PathBuf> {
    first_existing(folders.iter().flat_map(|folder| [local_programs(folder).join(exe), program_files(folder).join(exe)]))
        .or_else(|| listed(listed_as, exe))
        .or_else(|| above_script(scripts, exe))
}

fn editor(exe: &str, folders: &[&str], listed_as: &str, scripts: &[&str]) -> Option<Launch> {
    program(exe, folders, listed_as, scripts).map(|program| launch(program, &["{path}"]))
}

/// JetBrains IDEs keep their version in the folder name; Toolbox leaves a script instead.
fn jetbrains(folder: &str, exe: &str, script: &str) -> Option<Launch> {
    let relative = format!("bin\\{exe}");
    in_versioned_dirs(&[program_files("JetBrains"), env_dir("LOCALAPPDATA").join("Programs")], folder, &relative)
        .or_else(|| listed(folder, &relative))
        .or_else(|| on_path(&[exe]))
        .or_else(|| first_existing([env_dir("LOCALAPPDATA").join(format!("JetBrains\\Toolbox\\scripts\\{script}.cmd"))]))
        .map(|program| launch(program, &["{path}"]))
}

pub(in crate::openers) fn known() -> Vec<Known> {
    vec![
        known!("windows-terminal", "Windows Terminal", Terminal, || {
            on_path(&["wt.exe"])
                .or_else(|| first_existing([env_dir("LOCALAPPDATA").join("Microsoft\\WindowsApps\\wt.exe")]))
                .map(|program| launch(program, &["-d", "{path}"]))
        }),
        known!("pwsh", "PowerShell 7", Terminal, || {
            on_path(&["pwsh.exe"])
                .or_else(|| first_existing([program_files("PowerShell\\7\\pwsh.exe")]))
                .map(|program| console(program, &["-NoExit"]))
        }),
        known!("powershell", "Windows PowerShell", Terminal, || {
            first_existing([system32("WindowsPowerShell\\v1.0\\powershell.exe")])
                .map(|program| console(program, &["-NoExit"]))
        }),
        known!("cmd", "命令提示符 / Command Prompt", Terminal, || {
            first_existing([system32("cmd.exe")]).map(|program| console(program, &[]))
        }),
        known!("git-bash", "Git Bash", Terminal, || {
            program("git-bash.exe", &["Git"], "Git", &["git.exe"]).map(|program| launch(program, &["--cd={path}"]))
        }),
        known!("vscode", "VS Code", Editor, || {
            editor("Code.exe", &["Microsoft VS Code"], "Microsoft Visual Studio Code", &["code.cmd"])
        }),
        known!("cursor", "Cursor", Editor, || editor("Cursor.exe", &["cursor"], "Cursor", &["cursor.cmd"])),
        known!("windsurf", "Windsurf", Editor, || editor("Windsurf.exe", &["Windsurf"], "Windsurf", &["windsurf.cmd"])),
        known!("vscodium", "VSCodium", Editor, || editor("VSCodium.exe", &["VSCodium"], "VSCodium", &["codium.cmd"])),
        known!("zed", "Zed", Editor, || editor("Zed.exe", &["Zed"], "Zed", &["zed.exe"])),
        known!("sublime", "Sublime Text", Editor, || {
            editor("sublime_text.exe", &["Sublime Text", "Sublime Text 3"], "Sublime Text", &["subl.exe"])
        }),
        known!("pycharm", "PyCharm", Editor, || jetbrains("PyCharm", "pycharm64.exe", "pycharm")),
        known!("idea", "IntelliJ IDEA", Editor, || jetbrains("IntelliJ IDEA", "idea64.exe", "idea")),
        known!("webstorm", "WebStorm", Editor, || jetbrains("WebStorm", "webstorm64.exe", "webstorm")),
        known!("goland", "GoLand", Editor, || jetbrains("GoLand", "goland64.exe", "goland")),
        known!("clion", "CLion", Editor, || jetbrains("CLion", "clion64.exe", "clion")),
        known!("rustrover", "RustRover", Editor, || jetbrains("RustRover", "rustrover64.exe", "rustrover")),
        known!("phpstorm", "PhpStorm", Editor, || jetbrains("PhpStorm", "phpstorm64.exe", "phpstorm")),
        known!("rider", "Rider", Editor, || jetbrains("JetBrains Rider", "rider64.exe", "rider")),
        known!("android-studio", "Android Studio", Editor, || {
            editor("bin\\studio64.exe", &["Android\\Android Studio"], "Android Studio", &[])
        }),
        known!("explorer", "文件资源管理器 / File Explorer", Files, || {
            first_existing([env_dir("SystemRoot").join("explorer.exe")]).map(|program| launch(program, &["{path}"]))
        }),
    ]
}
