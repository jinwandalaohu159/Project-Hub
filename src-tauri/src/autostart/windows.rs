//! Windows launch-at-login through a scheduled task (with registry clean-up and elevation fallback).


#[cfg(target_os = "windows")]
pub(crate) const AUTOSTART_REG_VALUE: &str = "Project Hub";

#[cfg(target_os = "windows")]
pub(crate) const AUTOSTART_REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
pub(crate) const AUTOSTART_TASK_NAME: &str = "Project Hub";

#[cfg(target_os = "windows")]
pub(crate) const AUTOSTART_TASK_DELAY_XML: &str = "PT3S";

#[cfg(target_os = "windows")]
pub(crate) fn autostart_exe() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(exe.display().to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn autostart_task_action() -> Result<String, String> {
    Ok(format!("\"{}\" --hidden", autostart_exe()?))
}

#[cfg(target_os = "windows")]
pub(crate) fn autostart_registry_command() -> Result<String, String> {
    autostart_task_action()
}

#[cfg(target_os = "windows")]
pub(crate) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(target_os = "windows")]
pub(crate) fn autostart_task_xml() -> Result<String, String> {
    let command = xml_escape(&format!("\"{}\"", autostart_exe()?));

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <Delay>{AUTOSTART_TASK_DELAY_XML}</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>3</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{command}</Command>
      <Arguments>--hidden</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    ))
}

#[cfg(target_os = "windows")]
pub(crate) fn write_autostart_task_xml() -> Result<String, String> {
    let task_dir = dirs::data_dir()
        .ok_or_else(|| "Cannot determine app data directory".to_string())?
        .join("project-hub")
        .join("tasks");
    std::fs::create_dir_all(&task_dir).map_err(|e| e.to_string())?;

    let task_path = task_dir.join("Project Hub.xml");
    let xml = autostart_task_xml()?;
    let mut bytes = Vec::with_capacity(2 + xml.len() * 2);
    bytes.extend_from_slice(&[0xff, 0xfe]);
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&task_path, bytes).map_err(|e| e.to_string())?;
    Ok(task_path.display().to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn decode_windows_output(bytes: &[u8]) -> String {
    let (text, _, _) = encoding_rs::GBK.decode(bytes);
    text.trim().to_string()
}

#[cfg(target_os = "windows")]
pub(crate) fn schtasks(args: &[&str]) -> Result<std::process::Output, String> {
    use std::os::windows::process::CommandExt;
    use std::time::{Duration, Instant};
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut child = std::process::Command::new("schtasks.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let started_at = Instant::now();
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return child.wait_with_output().map_err(|e| e.to_string());
        }

        if started_at.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Windows startup task command timed out.".to_string());
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn schtasks_ok(args: &[&str]) -> Result<(), String> {
    let output = schtasks(args)?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = decode_windows_output(&output.stderr);
    let stdout = decode_windows_output(&output.stdout);
    Err(format!("{stderr}{stdout}").trim().to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_command_arg(value: &str) -> String {
    if !value.is_empty()
        && !value
            .chars()
            .any(|ch| ch.is_whitespace() || ch == '"' || ch == '\\')
    {
        return value.to_string();
    }

    let mut escaped = String::from("\"");
    let mut backslashes = 0;

    for ch in value.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                escaped.push_str(&"\\".repeat(backslashes * 2 + 1));
                escaped.push('"');
                backslashes = 0;
            }
            _ => {
                escaped.push_str(&"\\".repeat(backslashes));
                backslashes = 0;
                escaped.push(ch);
            }
        }
    }

    escaped.push_str(&"\\".repeat(backslashes * 2));
    escaped.push('"');
    escaped
}

#[cfg(target_os = "windows")]
pub(crate) fn wide_null(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
pub(crate) fn is_access_denied_message(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("access is denied")
        || lower.contains("access denied")
        || message.contains("拒绝访问")
        || message.contains("存取被拒")
        || message.contains("权限")
        || lower.contains("requires elevation")
        || lower.contains("administrator")
        || lower.contains("permission denied")
}

#[cfg(target_os = "windows")]
pub(crate) fn schtasks_ok_elevated(args: &[&str]) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED},
        System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE},
        UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
    };

    const ERROR_CANCELLED_EXIT_CODE: u32 = 1223;
    const SW_HIDE: i32 = 0;

    let parameters = args
        .iter()
        .map(|arg| windows_command_arg(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let verb = wide_null("runas");
    let file = wide_null("schtasks.exe");
    let params = wide_null(&parameters);

    let mut execute_info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: params.as_ptr(),
        nShow: SW_HIDE,
        ..Default::default()
    };

    let started = unsafe { ShellExecuteExW(&mut execute_info) };
    if started == 0 {
        let error = unsafe { GetLastError() };
        if error == ERROR_CANCELLED {
            return Err("已取消 Windows 管理员授权，开机自启动未修改。".to_string());
        }

        return Err(format!(
            "无法打开 Windows 管理员授权窗口（错误代码 {error}），开机自启动未修改。"
        ));
    }

    if execute_info.hProcess.is_null() {
        return Err("未能启动 Windows 开机自启动提权命令。".to_string());
    }

    let mut exit_code = 1;
    unsafe {
        WaitForSingleObject(execute_info.hProcess, INFINITE);
        if GetExitCodeProcess(execute_info.hProcess, &mut exit_code) == 0 {
            let error = GetLastError();
            CloseHandle(execute_info.hProcess);
            return Err(format!(
                "无法读取 Windows 开机自启动任务结果（错误代码 {error}）。"
            ));
        }
        CloseHandle(execute_info.hProcess);
    }

    match exit_code {
        0 => Ok(()),
        ERROR_CANCELLED_EXIT_CODE => {
            Err("已取消 Windows 管理员授权，开机自启动未修改。".to_string())
        }
        code => Err(format!(
            "Windows 开机自启动任务操作失败（schtasks 退出码 {code}）。请确认已允许管理员授权。"
        )),
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn schtasks_ok_allow_elevation(args: &[&str], allow_elevation: bool) -> Result<(), String> {
    match schtasks_ok(args) {
        Ok(()) => Ok(()),
        Err(error) if allow_elevation && is_access_denied_message(&error) => {
            schtasks_ok_elevated(args)
        }
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn get_registry_autostart_enabled() -> Result<bool, String> {
    use std::io::ErrorKind;
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = match hkcu.open_subkey(AUTOSTART_REG_PATH) {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    };

    match run_key.get_value::<String, _>(AUTOSTART_REG_VALUE) {
        Ok(value) => Ok(value == autostart_registry_command()?),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn set_registry_autostart_enabled(enabled: bool) -> Result<(), String> {
    use std::io::ErrorKind;
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = hkcu
        .create_subkey(AUTOSTART_REG_PATH)
        .map_err(|e| e.to_string())?;

    if enabled {
        run_key
            .set_value(AUTOSTART_REG_VALUE, &autostart_registry_command()?)
            .map_err(|e| e.to_string())?;
    } else if let Err(error) = run_key.delete_value(AUTOSTART_REG_VALUE) {
        if error.kind() != ErrorKind::NotFound {
            return Err(error.to_string());
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn get_task_autostart_enabled() -> Result<bool, String> {
    let output = schtasks(&["/Query", "/TN", AUTOSTART_TASK_NAME, "/XML"])?;
    if !output.status.success() {
        return Ok(false);
    }

    let xml = decode_windows_output(&output.stdout);
    Ok(xml.contains(&autostart_exe()?) && xml.contains("--hidden"))
}

#[cfg(target_os = "windows")]
pub(crate) fn migrate_autostart_to_task(allow_elevation: bool) {
    let task_enabled = get_task_autostart_enabled().unwrap_or(false);
    let registry_enabled = get_registry_autostart_enabled().unwrap_or(false);
    if !registry_enabled && !task_enabled {
        return;
    }

    if !task_enabled {
        if let Ok(task_path) = write_autostart_task_xml() {
            let _ = schtasks_ok_allow_elevation(
                &[
                    "/Create",
                    "/TN",
                    AUTOSTART_TASK_NAME,
                    "/XML",
                    &task_path,
                    "/F",
                ],
                allow_elevation,
            );
        }
        return;
    }

    let _ = set_registry_autostart_enabled(false);
}

#[cfg(target_os = "windows")]
pub(crate) fn get_autostart_enabled_platform() -> Result<bool, String> {
    Ok(get_task_autostart_enabled()? || get_registry_autostart_enabled()?)
}

#[cfg(target_os = "windows")]
pub(crate) fn set_autostart_enabled_platform(enabled: bool) -> Result<(), String> {
    if get_autostart_enabled_platform().unwrap_or(false) == enabled {
        return Ok(());
    }

    if enabled {
        let task_path = write_autostart_task_xml()?;
        schtasks_ok_allow_elevation(
            &[
                "/Create",
                "/TN",
                AUTOSTART_TASK_NAME,
                "/XML",
                &task_path,
                "/F",
            ],
            true,
        )?;
        set_registry_autostart_enabled(false)?;
    } else {
        let _ = schtasks_ok_allow_elevation(&["/Delete", "/TN", AUTOSTART_TASK_NAME, "/F"], true);
        set_registry_autostart_enabled(false)?;
    }

    let actual_enabled = get_autostart_enabled_platform()?;
    if actual_enabled != enabled {
        return Err("无法确认开机自启动状态，请检查 Windows 启动项权限。".to_string());
    }

    Ok(())
}
