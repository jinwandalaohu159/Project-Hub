//! The signed-in account's token: one file in the app data folder, encrypted for the current user
//! on Windows and readable only by its owner elsewhere.

use crate::settings::app_data_dir;
use std::{fs, path::PathBuf};

const TOKEN_FILE_NAME: &str = "github-token.bin";

fn token_path() -> PathBuf {
    app_data_dir().join(TOKEN_FILE_NAME)
}

pub fn store_token(token: &str) -> Result<(), String> {
    let path = token_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, protect(token.as_bytes())?).map_err(|e| e.to_string())?;
    restrict_to_owner(&path)
}

/// Outside Windows the token is not encrypted, so at least no other account may read it.
#[cfg(unix)]
fn restrict_to_owner(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn restrict_to_owner(_path: &std::path::Path) -> Result<(), String> {
    Ok(())
}

pub fn load_token() -> Option<String> {
    let path = token_path();
    let bytes = fs::read(&path).ok()?;
    let token = String::from_utf8(unprotect(&bytes).ok()?).ok()?;
    let token = token.trim().to_string();
    (!token.is_empty()).then_some(token)
}

pub fn delete_token() {
    let _ = fs::remove_file(token_path());
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("TOKEN_ENCRYPT_FAILED".to_string());
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as HLOCAL);
    }
    Ok(bytes)
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return Err("TOKEN_DECRYPT_FAILED".to_string());
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as HLOCAL);
    }
    Ok(bytes)
}

#[cfg(not(windows))]
fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}

#[cfg(not(windows))]
fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_round_trip() {
        let secret = b"gho_example";
        let sealed = protect(secret).unwrap();
        assert_eq!(unprotect(&sealed).unwrap(), secret);
    }
}
