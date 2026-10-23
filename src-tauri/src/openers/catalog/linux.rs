//! Linux: programs on the PATH, with each terminal's own way of starting in a folder.

use super::*;

fn found(names: &[&str], args: &[&str]) -> Option<Launch> {
    on_path(names).map(|program| launch(program, args))
}

pub(in crate::openers) fn known() -> Vec<Known> {
    vec![
        known!("gnome-terminal", "GNOME Terminal", Terminal, || found(&["gnome-terminal"], &["--working-directory={path}"])),
        known!("konsole", "Konsole", Terminal, || found(&["konsole"], &["--workdir", "{path}"])),
        known!("xfce4-terminal", "Xfce Terminal", Terminal, || found(&["xfce4-terminal"], &["--working-directory={path}"])),
        known!("kitty", "kitty", Terminal, || found(&["kitty"], &["--directory", "{path}"])),
        known!("alacritty", "Alacritty", Terminal, || found(&["alacritty"], &["--working-directory", "{path}"])),
        known!("wezterm", "WezTerm", Terminal, || found(&["wezterm"], &["start", "--cwd", "{path}"])),
        known!("tilix", "Tilix", Terminal, || found(&["tilix"], &["--working-directory={path}"])),
        known!("ghostty", "Ghostty", Terminal, || found(&["ghostty"], &[])),
        known!("x-terminal-emulator", "终端 / Terminal", Terminal, || found(&["x-terminal-emulator"], &[])),
        known!("vscode", "VS Code", Editor, || found(&["code"], &["{path}"])),
        known!("cursor", "Cursor", Editor, || found(&["cursor"], &["{path}"])),
        known!("windsurf", "Windsurf", Editor, || found(&["windsurf"], &["{path}"])),
        known!("vscodium", "VSCodium", Editor, || found(&["codium"], &["{path}"])),
        known!("zed", "Zed", Editor, || found(&["zed", "zeditor"], &["{path}"])),
        known!("sublime", "Sublime Text", Editor, || found(&["subl"], &["{path}"])),
        known!("pycharm", "PyCharm", Editor, || found(&["pycharm", "pycharm-professional", "pycharm-community", "charm"], &["{path}"])),
        known!("idea", "IntelliJ IDEA", Editor, || found(&["idea", "intellij-idea-ultimate", "intellij-idea-community", "idea.sh"], &["{path}"])),
        known!("webstorm", "WebStorm", Editor, || found(&["webstorm"], &["{path}"])),
        known!("goland", "GoLand", Editor, || found(&["goland"], &["{path}"])),
        known!("clion", "CLion", Editor, || found(&["clion"], &["{path}"])),
        known!("rustrover", "RustRover", Editor, || found(&["rustrover"], &["{path}"])),
        known!("phpstorm", "PhpStorm", Editor, || found(&["phpstorm"], &["{path}"])),
        known!("android-studio", "Android Studio", Editor, || found(&["android-studio", "studio.sh"], &["{path}"])),
        known!("files", "文件管理器 / Files", Files, || found(&["xdg-open"], &["{path}"])),
    ]
}
