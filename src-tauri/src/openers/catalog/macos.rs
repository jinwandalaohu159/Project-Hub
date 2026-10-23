//! macOS: app bundles, opened with `open -a`.

use super::*;

/// An app bundle in /Applications, ~/Applications (where JetBrains Toolbox puts its apps) or
/// the system's own folders, opened with `open -a`.
fn app(names: &[&str]) -> Option<Launch> {
    let dirs = [
        PathBuf::from("/Applications"),
        home().join("Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
        PathBuf::from("/Applications/Utilities"),
    ];
    let bundle = names
        .iter()
        .find_map(|name| first_existing(dirs.iter().map(|dir| dir.join(format!("{name}.app")))))?;
    Some(Launch {
        program: PathBuf::from("/usr/bin/open"),
        args: vec!["-a".into(), bundle.to_string_lossy().into_owned(), "{path}".into()],
        console: false,
    })
}

pub(in crate::openers) fn known() -> Vec<Known> {
    vec![
        known!("terminal", "Terminal", Terminal, || app(&["Terminal"])),
        known!("iterm", "iTerm", Terminal, || app(&["iTerm"])),
        known!("warp", "Warp", Terminal, || app(&["Warp"])),
        known!("ghostty", "Ghostty", Terminal, || app(&["Ghostty"])),
        known!("vscode", "VS Code", Editor, || app(&["Visual Studio Code"])),
        known!("cursor", "Cursor", Editor, || app(&["Cursor"])),
        known!("windsurf", "Windsurf", Editor, || app(&["Windsurf"])),
        known!("vscodium", "VSCodium", Editor, || app(&["VSCodium"])),
        known!("zed", "Zed", Editor, || app(&["Zed"])),
        known!("sublime", "Sublime Text", Editor, || app(&["Sublime Text"])),
        known!("pycharm", "PyCharm", Editor, || app(&["PyCharm", "PyCharm CE", "PyCharm Community Edition", "PyCharm Professional Edition"])),
        known!("idea", "IntelliJ IDEA", Editor, || app(&["IntelliJ IDEA", "IntelliJ IDEA CE", "IntelliJ IDEA Ultimate", "IntelliJ IDEA Community Edition"])),
        known!("webstorm", "WebStorm", Editor, || app(&["WebStorm"])),
        known!("goland", "GoLand", Editor, || app(&["GoLand"])),
        known!("clion", "CLion", Editor, || app(&["CLion"])),
        known!("rustrover", "RustRover", Editor, || app(&["RustRover"])),
        known!("phpstorm", "PhpStorm", Editor, || app(&["PhpStorm"])),
        known!("rider", "Rider", Editor, || app(&["Rider"])),
        known!("android-studio", "Android Studio", Editor, || app(&["Android Studio"])),
        known!("xcode", "Xcode", Editor, || app(&["Xcode"])),
        known!("finder", "Finder", Files, || {
            Some(launch(PathBuf::from("/usr/bin/open"), &["{path}"]))
        }),
    ]
}
