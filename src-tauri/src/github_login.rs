//! Browser sign-in through GitHub's device flow: the app gets a short code, the user enters it on
//! github.com and approves Project Hub, and the app receives a token. Nothing needs installing.
//! https://docs.github.com/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow

use serde::{Deserialize, Serialize};
use std::{
    sync::{Mutex, MutexGuard, PoisonError},
    thread,
    time::{Duration, Instant},
};

/// Project Hub's OAuth app. Its id is public by design; the device flow uses no client secret.
const CLIENT_ID: &str = "Ov23liWjntb7wXM3jTxC";
/// Creating repositories and pushing to them, the same as a token made for the app by hand.
const SCOPES: &str = "repo";
const USER_AGENT: &str = "Project-Hub";

/// What the user needs to finish signing in on github.com.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[derive(Deserialize)]
struct CodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    interval: Option<u64>,
}

/// The sign-in waiting for the user. Starting another one or cancelling replaces it, which ends
/// the wait for the old one.
struct Pending {
    device_code: String,
    interval: Duration,
    expires_at: Instant,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn pending() -> MutexGuard<'static, Option<Pending>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

fn is_pending(device_code: &str) -> bool {
    pending().as_ref().is_some_and(|p| p.device_code == device_code)
}

/// Ends the given sign-in, unless another one has replaced it meanwhile.
fn finish(device_code: &str) {
    let mut pending = pending();
    if pending.as_ref().is_some_and(|p| p.device_code == device_code) {
        *pending = None;
    }
}

fn post<T: serde::de::DeserializeOwned>(url: &str, form: &[(&str, &str)]) -> Result<T, String> {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .build()
        .post(url)
        .set("Accept", "application/json")
        .set("User-Agent", USER_AGENT)
        .send_form(form)
        .map_err(|e| format!("Cannot reach GitHub: {e}"))?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Asks GitHub for a code to show the user; `wait_for_token` then waits until they approve it.
pub fn start() -> Result<DeviceCode, String> {
    let response: CodeResponse = post(
        "https://github.com/login/device/code",
        &[("client_id", CLIENT_ID), ("scope", SCOPES)],
    )?;
    *pending() = Some(Pending {
        device_code: response.device_code,
        interval: Duration::from_secs(response.interval.max(1)),
        expires_at: Instant::now() + Duration::from_secs(response.expires_in),
    });
    Ok(DeviceCode {
        user_code: response.user_code,
        verification_uri: response.verification_uri,
        expires_in: response.expires_in,
    })
}

/// Waits for the user to approve or decline the code from `start` and returns the token.
pub fn wait_for_token() -> Result<String, String> {
    let Some((device_code, mut interval, expires_at)) = pending()
        .as_ref()
        .map(|p| (p.device_code.clone(), p.interval, p.expires_at))
    else {
        return Err("GITHUB_LOGIN_CANCELLED".to_string());
    };
    let result = poll(&device_code, &mut interval, expires_at);
    finish(&device_code);
    result
}

fn poll(device_code: &str, interval: &mut Duration, expires_at: Instant) -> Result<String, String> {
    loop {
        // GitHub asks for a pause between polls; sleep in short steps to notice a cancel quickly.
        let wake_at = Instant::now() + *interval;
        while Instant::now() < wake_at {
            thread::sleep(Duration::from_millis(200));
            if !is_pending(device_code) {
                return Err("GITHUB_LOGIN_CANCELLED".to_string());
            }
        }
        if Instant::now() >= expires_at {
            return Err("GITHUB_LOGIN_EXPIRED".to_string());
        }
        let response: TokenResponse = post(
            "https://github.com/login/oauth/access_token",
            &[
                ("client_id", CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ],
        )?;
        if let Some(token) = response.access_token {
            return Ok(token);
        }
        match response.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => {
                *interval = response.interval.map(Duration::from_secs).unwrap_or(*interval + Duration::from_secs(5));
            }
            Some("expired_token") => return Err("GITHUB_LOGIN_EXPIRED".to_string()),
            Some("access_denied") => return Err("GITHUB_LOGIN_DENIED".to_string()),
            other => return Err(format!("GITHUB_LOGIN_FAILED:{}", other.unwrap_or("unknown"))),
        }
    }
}

/// Stops waiting for the sign-in in progress, if any.
pub fn cancel() {
    *pending() = None;
}
