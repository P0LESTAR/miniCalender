use chrono::{DateTime, Utc};
use std::sync::Mutex;

use crate::google_calendar::CalendarEvent;

/// How often to refresh cached events (in seconds).
const CACHE_TTL_SECS: i64 = 300; // 5 minutes

/// Thread-safe application state shared across Tauri commands.
pub struct AppState {
    pub inner: Mutex<AppStateInner>,
}

pub struct AppStateInner {
    /// Cached calendar events – kept small to minimize RAM.
    pub cached_events: Vec<CalendarEvent>,
    /// Whether the user has authenticated with Google.
    pub is_authenticated: bool,
    /// Last time events were fetched from the API.
    pub last_sync: Option<DateTime<Utc>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AppStateInner {
                cached_events: Vec::new(),
                is_authenticated: false,
                last_sync: None,
            }),
        }
    }

    /// Replace the cached events and mark sync time.
    pub fn update_cache(&self, events: Vec<CalendarEvent>) {
        if let Ok(mut state) = self.inner.lock() {
            state.cached_events = events;
            state.last_sync = Some(Utc::now());
        }
    }

    /// Returns `true` when the cache is stale or empty.
    pub fn needs_refresh(&self) -> bool {
        if let Ok(state) = self.inner.lock() {
            match state.last_sync {
                None => true,
                Some(last) => {
                    let elapsed = Utc::now().signed_duration_since(last).num_seconds();
                    elapsed >= CACHE_TTL_SECS
                }
            }
        } else {
            true
        }
    }

    pub fn set_authenticated(&self, value: bool) {
        if let Ok(mut state) = self.inner.lock() {
            state.is_authenticated = value;
        }
    }

    pub fn is_authenticated(&self) -> bool {
        if let Ok(state) = self.inner.lock() {
            state.is_authenticated
        } else {
            false
        }
    }

    /// Return a clone of the cached events (cheap – the vec is small).
    pub fn get_cached_events(&self) -> Vec<CalendarEvent> {
        if let Ok(state) = self.inner.lock() {
            state.cached_events.clone()
        } else {
            Vec::new()
        }
    }
}
