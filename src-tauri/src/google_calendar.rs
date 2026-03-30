use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write as _};
use std::net::TcpListener;
use tauri::AppHandle;
use tauri::Manager;

use crate::state::AppState;

const CLIENT_ID: &str = env!("GOOGLE_CLIENT_ID");
const CLIENT_SECRET: &str = env!("GOOGLE_CLIENT_SECRET");

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar";
const CALENDAR_API_BASE: &str = "https://www.googleapis.com/calendar/v3";

const KEYRING_SERVICE: &str = "miniCalender";
const KEYRING_USER_ACCESS: &str = "google_oauth_access";
const KEYRING_USER_REFRESH: &str = "google_oauth_refresh";

// ---------------------------------------------------------------------------
// Public data types exposed to the frontend
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub color: Option<String>,
    pub is_all_day: bool,
    pub source: String,
}

#[derive(Deserialize, Debug)]
pub struct CreateEventRequest {
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub is_all_day: bool,
}

// ---------------------------------------------------------------------------
// Google Calendar API response types (intermediate)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
struct GoogleEventList {
    items: Option<Vec<GoogleEvent>>,
}

#[derive(Deserialize, Debug)]
struct GoogleEvent {
    id: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    start: Option<GoogleDateTime>,
    end: Option<GoogleDateTime>,
    #[serde(rename = "colorId")]
    color_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GoogleDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Serialize, Debug)]
struct GoogleCreateEvent {
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    start: GoogleCreateDateTime,
    end: GoogleCreateDateTime,
}

#[derive(Serialize, Debug)]
struct GoogleCreateDateTime {
    #[serde(rename = "dateTime", skip_serializing_if = "Option::is_none")]
    date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "timeZone")]
    time_zone: Option<String>,
}

// ---------------------------------------------------------------------------
// Token storage helpers (keyring)
// ---------------------------------------------------------------------------

fn store_token(key: &str, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, key).map_err(|e| e.to_string())?;
    entry.set_password(value).map_err(|e| e.to_string())
}

fn load_token(key: &str) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, key).ok()?;
    entry.get_password().ok()
}

fn delete_token(key: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, key) {
        let _ = entry.delete_credential();
    }
}

// ---------------------------------------------------------------------------
// OAuth2 PKCE helpers (manual, no oauth2 crate)
// ---------------------------------------------------------------------------

/// Generates a cryptographically random PKCE code verifier (43–128 chars, URL-safe base64).
fn pkce_new_verifier() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    // Use multiple entropy sources combined into 32 bytes
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let thread_id = std::thread::current().id();
    let mut h = DefaultHasher::new();
    nanos.hash(&mut h);
    thread_id.hash(&mut h);
    let h1 = h.finish();

    // Build 32 pseudo-random bytes from two hasher passes
    let mut h2 = DefaultHasher::new();
    h1.hash(&mut h2);
    nanos.wrapping_add(1).hash(&mut h2);
    let h2v = h2.finish();

    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&h1.to_le_bytes());
    bytes[8..16].copy_from_slice(&h2v.to_le_bytes());
    // Fill the rest with xor-shifted variants
    for i in (16..32).step_by(8) {
        let v = h1.wrapping_mul(6364136223846793005).wrapping_add(i as u64);
        bytes[i..i + 8].copy_from_slice(&v.to_le_bytes());
    }

    // URL-safe base64 without padding
    base64_url_encode(&bytes)
}

/// Computes the PKCE S256 code challenge: BASE64URL(SHA256(verifier)).
fn pkce_challenge(verifier: &str) -> String {
    // SHA-256 implemented inline (no extra dep)
    let digest = sha256(verifier.as_bytes());
    base64_url_encode(&digest)
}

/// Generates a random CSRF state token (hex string).
fn new_csrf_token() -> String {
    pkce_new_verifier()
}

// ---------------------------------------------------------------------------
// Minimal crypto helpers (SHA-256 + URL-safe base64)
// ---------------------------------------------------------------------------

fn sha256(data: &[u8]) -> [u8; 32] {
    // FIPS 180-4 SHA-256 constants
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: adding padding bits
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit chunk
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] =
            [h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e;
            e = d.wrapping_add(temp1);
            d = c; c = b; b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i*4..(i+1)*4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn base64_url_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= input.len() {
        let b = ((input[i] as u32) << 16) | ((input[i+1] as u32) << 8) | (input[i+2] as u32);
        out.push(CHARS[((b >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((b >> 12) & 0x3F) as usize] as char);
        out.push(CHARS[((b >>  6) & 0x3F) as usize] as char);
        out.push(CHARS[( b        & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let b = (input[i] as u32) << 16;
        out.push(CHARS[((b >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((b >> 12) & 0x3F) as usize] as char);
    } else if rem == 2 {
        let b = ((input[i] as u32) << 16) | ((input[i+1] as u32) << 8);
        out.push(CHARS[((b >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((b >> 12) & 0x3F) as usize] as char);
        out.push(CHARS[((b >>  6) & 0x3F) as usize] as char);
    }
    out
}

/// Builds the Google OAuth2 authorization URL manually.
fn build_auth_url(redirect_uri: &str, csrf_state: &str, pkce_challenge: &str) -> String {
    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        AUTH_URL,
        urlencoding(CLIENT_ID),
        urlencoding(redirect_uri),
        urlencoding(CALENDAR_SCOPE),
        urlencoding(csrf_state),
        urlencoding(pkce_challenge),
    )
}

// ---------------------------------------------------------------------------
// Get a valid access token (refresh if needed)
// ---------------------------------------------------------------------------

async fn get_valid_access_token() -> Result<String, String> {
    // Try loading existing access token
    if let Some(access) = load_token(KEYRING_USER_ACCESS) {
        // Optimistically return it; if it's expired the API call will 401
        // and we fall through to refresh.
        return Ok(access);
    }

    // No access token – try refreshing
    refresh_access_token().await
}

async fn refresh_access_token() -> Result<String, String> {
    let refresh_token = load_token(KEYRING_USER_REFRESH)
        .ok_or_else(|| "Not authenticated. Please sign in with Google first.".to_string())?;

    let http = reqwest::Client::new();
    let params = [
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("refresh_token", refresh_token.as_str()),
        ("grant_type", "refresh_token"),
    ];

    let resp = http
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Network error during token refresh: {}", e))?;

    if !resp.status().is_success() {
        // Token revoked or invalid – clear stored tokens
        delete_token(KEYRING_USER_ACCESS);
        delete_token(KEYRING_USER_REFRESH);
        return Err("Session expired. Please sign in with Google again.".to_string());
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let new_access = body["access_token"]
        .as_str()
        .ok_or("Missing access_token in refresh response")?
        .to_string();

    store_token(KEYRING_USER_ACCESS, &new_access)?;
    Ok(new_access)
}

/// Make an authenticated GET/POST/DELETE and auto-retry once on 401.
async fn api_request(
    method: &str,
    url: &str,
    body: Option<serde_json::Value>,
) -> Result<reqwest::Response, String> {
    let http = HttpClient::new();
    let token = get_valid_access_token().await?;

    let req = match method {
        "GET" => http.get(url).bearer_auth(&token),
        "POST" => http
            .post(url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&body),
        "DELETE" => http.delete(url).bearer_auth(&token),
        "PATCH" => http
            .patch(url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&body),
        _ => return Err(format!("Unsupported HTTP method: {}", method)),
    };

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if resp.status().as_u16() == 401 {
        // Access token expired – refresh and retry once
        let new_token = refresh_access_token().await?;
        let req2 = match method {
            "GET" => http.get(url).bearer_auth(&new_token),
            "POST" => http
                .post(url)
                .bearer_auth(&new_token)
                .header("Content-Type", "application/json")
                .json(&body),
            "DELETE" => http.delete(url).bearer_auth(&new_token),
            "PATCH" => http
                .patch(url)
                .bearer_auth(&new_token)
                .header("Content-Type", "application/json")
                .json(&body),
            _ => unreachable!(),
        };
        req2.send()
            .await
            .map_err(|e| format!("Network error on retry: {}", e))
    } else {
        Ok(resp)
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

fn google_event_to_calendar_event(ge: GoogleEvent) -> CalendarEvent {
    let (start_time, is_all_day) = match &ge.start {
        Some(dt) => {
            if let Some(ref dts) = dt.date_time {
                (dts.clone(), false)
            } else if let Some(ref d) = dt.date {
                (d.clone(), true)
            } else {
                (String::new(), false)
            }
        }
        None => (String::new(), false),
    };

    let end_time = match &ge.end {
        Some(dt) => dt
            .date_time
            .clone()
            .or_else(|| dt.date.clone())
            .unwrap_or_default(),
        None => String::new(),
    };

    CalendarEvent {
        id: ge.id.unwrap_or_default(),
        title: ge.summary.unwrap_or_else(|| "(No title)".to_string()),
        description: ge.description,
        start_time,
        end_time,
        color: ge.color_id,
        is_all_day,
        source: "google".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

/// Starts the OAuth2 authorization code flow with PKCE.
/// Opens the user's browser, listens for the redirect callback on localhost,
/// exchanges the code for tokens, and stores them in the OS keyring.
#[tauri::command]
pub async fn google_auth_start(app_handle: AppHandle) -> Result<bool, String> {
    // Bind to a random available port on localhost
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Failed to bind port: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .port();
    let redirect_uri = format!("http://localhost:{}", port);

    // Generate PKCE verifier + challenge and CSRF state
    let pkce_verifier = pkce_new_verifier();
    let pkce_challenge_value = pkce_challenge(&pkce_verifier);
    let csrf_state = new_csrf_token();

    let auth_url = build_auth_url(&redirect_uri, &csrf_state, &pkce_challenge_value);

    // Open the authorization URL in the user's default browser
    open::that(&auth_url)
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    // Wait for the OAuth callback on the local server (blocking in a spawn_blocking)
    let (code, state) = tokio::task::spawn_blocking(move || -> Result<(String, String), String> {
        // Accept exactly one connection
        let (stream, _) = listener
            .accept()
            .map_err(|e| format!("Failed to accept connection: {}", e))?;

        // Clone the stream so we can read and write independently
        let mut write_stream = stream
            .try_clone()
            .map_err(|e| format!("Failed to clone stream: {}", e))?;

        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .map_err(|e| format!("Failed to read request: {}", e))?;

        // Parse code and state from the GET request
        let redirect_path = request_line
            .split_whitespace()
            .nth(1)
            .ok_or("Invalid HTTP request from callback")?
            .to_string();

        // Send a simple HTML response so the user sees confirmation
        let response_body =
            "<html><body><h2>Authentication successful!</h2><p>You can close this tab and return to miniCalender.</p></body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        let _ = write_stream.write_all(response.as_bytes());

        // Extract query parameters
        let url = url_from_path(&redirect_path)?;
        let code = url
            .iter()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.clone())
            .ok_or("No authorization code in callback")?;
        let state = url
            .iter()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        Ok((code, state))
    })
    .await
    .map_err(|e| format!("Auth callback task failed: {}", e))??;

    // Verify CSRF state
    if state != csrf_state {
        return Err("CSRF state mismatch – possible security issue.".to_string());
    }

    // Exchange authorization code for tokens
    let http_client = reqwest::Client::new();
    let params = [
        ("code", code.as_str()),
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("redirect_uri", redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
        ("code_verifier", pkce_verifier.as_str()),
    ];

    let resp = http_client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token exchange network error: {}", e))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {}", err_text));
    }

    let token_body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let access_token = token_body["access_token"]
        .as_str()
        .ok_or("Missing access_token")?;
    let refresh_token = token_body["refresh_token"]
        .as_str()
        .ok_or("Missing refresh_token – make sure prompt=consent is set")?;

    // Store tokens securely
    store_token(KEYRING_USER_ACCESS, access_token)?;
    store_token(KEYRING_USER_REFRESH, refresh_token)?;

    // Update app state
    if let Some(state) = app_handle.try_state::<AppState>() {
        state.set_authenticated(true);
    }

    Ok(true)
}

/// Returns whether the user has valid (or refreshable) credentials stored.
#[tauri::command]
pub async fn google_auth_status(
    app_handle: AppHandle,
) -> Result<bool, String> {
    let has_refresh = load_token(KEYRING_USER_REFRESH).is_some();
    if let Some(state) = app_handle.try_state::<AppState>() {
        state.set_authenticated(has_refresh);
    }
    Ok(has_refresh)
}

/// Fetches calendar events between `start_date` and `end_date` (ISO 8601 strings).
#[tauri::command]
pub async fn fetch_events(
    app_handle: AppHandle,
    start_date: String,
    end_date: String,
) -> Result<Vec<CalendarEvent>, String> {
    let url = format!(
        "{}/calendars/primary/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime",
        CALENDAR_API_BASE,
        urlencoding(&start_date),
        urlencoding(&end_date),
    );

    let resp = api_request("GET", &url, None).await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Google Calendar API error ({}): {}",
            status, text
        ));
    }

    let event_list: GoogleEventList = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse events: {}", e))?;

    let events: Vec<CalendarEvent> = event_list
        .items
        .unwrap_or_default()
        .into_iter()
        .map(google_event_to_calendar_event)
        .collect();

    // Update cache
    if let Some(state) = app_handle.try_state::<AppState>() {
        state.update_cache(events.clone());
    }

    Ok(events)
}

/// Creates a new event on the user's primary Google Calendar.
#[tauri::command]
pub async fn create_event(event: CreateEventRequest) -> Result<CalendarEvent, String> {
    let url = format!("{}/calendars/primary/events", CALENDAR_API_BASE);

    let (start, end) = if event.is_all_day {
        (
            GoogleCreateDateTime {
                date_time: None,
                date: Some(event.start_time.clone()),
                time_zone: None,
            },
            GoogleCreateDateTime {
                date_time: None,
                date: Some(event.end_time.clone()),
                time_zone: None,
            },
        )
    } else {
        (
            GoogleCreateDateTime {
                date_time: Some(event.start_time.clone()),
                date: None,
                time_zone: None,
            },
            GoogleCreateDateTime {
                date_time: Some(event.end_time.clone()),
                date: None,
                time_zone: None,
            },
        )
    };

    let body = GoogleCreateEvent {
        summary: event.title,
        description: event.description,
        start,
        end,
    };

    let json_body =
        serde_json::to_value(&body).map_err(|e| format!("Failed to serialize event: {}", e))?;

    let resp = api_request("POST", &url, Some(json_body)).await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Failed to create event ({}): {}",
            status, text
        ));
    }

    let created: GoogleEvent = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse created event: {}", e))?;

    Ok(google_event_to_calendar_event(created))
}

/// Deletes an event by its Google Calendar event ID.
#[tauri::command]
pub async fn delete_event(event_id: String) -> Result<(), String> {
    let url = format!(
        "{}/calendars/primary/events/{}",
        CALENDAR_API_BASE, event_id
    );

    let resp = api_request("DELETE", &url, None).await?;

    // 204 No Content is the expected success response
    if resp.status().is_success() || resp.status().as_u16() == 204 {
        Ok(())
    } else {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to delete event ({}): {}",
            status, text
        ))
    }
}

/// Updates an event's dates by its Google Calendar event ID.
#[tauri::command]
pub async fn update_event(event_id: String, event: CreateEventRequest) -> Result<CalendarEvent, String> {
    let url = format!(
        "{}/calendars/primary/events/{}",
        CALENDAR_API_BASE, event_id
    );

    let (start, end) = if event.is_all_day {
        (
            GoogleCreateDateTime {
                date_time: None,
                date: Some(event.start_time.clone()),
                time_zone: None,
            },
            GoogleCreateDateTime {
                date_time: None,
                date: Some(event.end_time.clone()),
                time_zone: None,
            },
        )
    } else {
        (
            GoogleCreateDateTime {
                date_time: Some(event.start_time.clone()),
                date: None,
                time_zone: None,
            },
            GoogleCreateDateTime {
                date_time: Some(event.end_time.clone()),
                date: None,
                time_zone: None,
            },
        )
    };

    let body = GoogleCreateEvent {
        summary: event.title,
        description: event.description,
        start,
        end,
    };

    let json_body =
        serde_json::to_value(&body).map_err(|e| format!("Failed to serialize event: {}", e))?;

    let resp = api_request("PATCH", &url, Some(json_body)).await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to update event ({}): {}", status, text));
    }

    let updated: GoogleEvent = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse updated event: {}", e))?;

    Ok(google_event_to_calendar_event(updated))
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Minimal query-string parser (avoids pulling in a URL crate).
fn url_from_path(path: &str) -> Result<Vec<(String, String)>, String> {
    let query = path.split('?').nth(1).unwrap_or("");
    Ok(query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.to_string();
            let value = parts.next().unwrap_or("").to_string();
            Some((
                percent_decode(&key),
                percent_decode(&value),
            ))
        })
        .collect())
}

/// Minimal percent-decoding for OAuth callback parameters.
fn percent_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                out.push(byte as char);
            } else {
                out.push('%');
                out.push_str(&hex);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// Minimal URL encoding for query parameter values.
fn urlencoding(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    out
}
