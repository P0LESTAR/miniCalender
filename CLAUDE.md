# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

miniCalender — a desktop calendar widget that embeds into the Windows desktop layer. Licensed under MIT (Copyright 2026 Polestar).

## Tech Stack

- **Frontend**: Svelte 5 (runes syntax) + TypeScript + Vite
- **Backend**: Tauri v2 + Rust
- **Desktop Integration**: Windows Progman/WorkerW embedding (desktop-level widget)
- **Google Calendar**: Manual OAuth2 + REST API (no oauth2 crate — removed for compatibility)
- **Token Storage**: `keyring` crate (Windows Credential Manager)
- **OAuth Credentials**: Build-time env via `dotenvy` crate + `build.rs` + `env!()` macro. Credentials in `src-tauri/.env` (gitignored).

## Build

Requires MSVC Build Tools. The MSVC linker must be on PATH before Git's `link.exe`:

```bash
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64:$PATH"
export LIB="C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Tools\\MSVC\\14.44.35207\\lib\\x64;C:\\Program Files (x86)\\Windows Kits\\10\\Lib\\10.0.26100.0\\um\\x64;C:\\Program Files (x86)\\Windows Kits\\10\\Lib\\10.0.26100.0\\ucrt\\x64"
export INCLUDE="C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Tools\\MSVC\\14.44.35207\\include;C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.26100.0\\ucrt;C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.26100.0\\um;C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.26100.0\\shared"
```

| Command | Purpose |
|---|---|
| `npm run dev` | Vite dev server (frontend only, port 1420) |
| `npm run build` | Vite production build → `dist/` |
| `npx tauri build` | Full release build (frontend + Rust → exe + installers) |
| `npx tauri dev` | Dev mode with hot reload |
| `cargo check` | Check Rust compilation (run from `src-tauri/`) |

Output: `src-tauri/target/release/mini-calender.exe`

**Build gotcha**: If the exe is running, `npx tauri build` fails with "access denied". Run `taskkill //F //IM mini-calender.exe` first.

## Architecture

```
src/                          # Svelte 5 frontend
├── App.svelte                # Shell: TitleBar + Calendar + Settings
├── app.css                   # Global CSS variables (dark glassmorphism theme)
├── lib/
│   ├── types.ts              # CalendarEvent, CalendarDay, DayEvent, EventPosition
│   ├── stores/
│   │   └── calendar.svelte.ts  # Reactive store ($state/$derived runes)
│   └── components/
│       ├── TitleBar.svelte   # Custom titlebar with manual drag (SetWindowPos) + sync button
│       ├── Calendar.svelte   # Month grid with unified event bars + drag & drop + EventModal
│       ├── EventModal.svelte # Add/delete event dialog with date range picker + color palette
│       └── Settings.svelte   # Settings panel (Google auth, opacity, size, theme)

src-tauri/src/                # Rust backend
├── main.rs                   # Entry point
├── lib.rs                    # Tauri setup, tray, embed_in_desktop(), move_window/get_window_position commands
├── build.rs                  # Loads .env at build time (dotenvy), exposes as compile-time env vars
├── google_calendar.rs        # OAuth2 flow, token management, Google Calendar CRUD (6 Tauri commands)
└── state.rs                  # AppState with Mutex, 5-min event cache
```

## Key Design Decisions

- **Window drag**: `startDragging()` doesn't work for WorkerW child windows. Uses Rust-side `SetWindowPos`/`GetWindowRect` via `move_window` and `get_window_position` Tauri commands.
- **Desktop embedding**: Finds Progman → sends 0x052C → finds WorkerW behind SHELLDLL_DefView → sets `WS_CHILD` style (removes `WS_POPUP`/`WS_CAPTION`/`WS_THICKFRAME`) → `SetParent` → `SetWindowPos` with `SWP_FRAMECHANGED`. **Order matters**: styles must be set BEFORE `SetParent`, and `SWP_FRAMECHANGED` must be called AFTER to force Windows to apply the style changes. This makes the window a true desktop child, immune to Win+D (Show Desktop).
- **OAuth2**: Hand-rolled HTTP flow with PKCE (oauth2 crate removed due to v5 type complexity). Tokens stored in Windows Credential Manager via keyring. Refresh is automatic. Credentials loaded at compile time via `env!("GOOGLE_CLIENT_ID")` from `src-tauri/.env`.
- **Google Calendar two-way sync**: `addEvent()` calls `create_event`, `removeEvent()` calls `delete_event`, `moveEvent()` calls `update_event` (PATCH API). Manual sync button in TitleBar (no automatic polling — keeps app lightweight).
- **Event color persistence**: User-selected colors stored in `localStorage` (`event-colors` key) as `{eventId: color}` map. Restored on app load and after Google sync, since Google API doesn't return custom hex colors.
- **Multi-day events**: Google all-day events use exclusive end date (end = day after last day). Converted to inclusive for rendering. EventModal auto-sets multi-day events as all-day.
- **Event drag & drop**: mousedown on event bar → 5px movement threshold → drag mode → mouseup on target day → shifts startTime/endTime by day offset. Google events call `update_event` PATCH API on drop. During drag, span-bars get `pointer-events: none` so mouseenter fires on background cells.

## Calendar Rendering

- **Unified event grid**: All events (multi-day spanning + single-day) share the same lane-based grid per week. No separate rows for different event types. Single-day events are 1-column spans in the same grid.
- **Week-based layout**: Each week = date number row + unified event grid. Grid always renders with `1fr` trailing row (even with no events) so the entire day area is clickable for adding events.
- **Lane allocation**: Greedy algorithm assigns lanes. Multi-day events get priority, then single-day. Max 3 visible lanes, overflow shown as "+N more".
- **Spanning bars**: Multi-day events use CSS Grid `grid-column` spanning. `roundLeft`/`roundRight` flags control border-radius at event start/end vs. week continuation.
- **Drag visual feedback**: Source bar becomes semi-transparent (opacity 0.4), target day gets highlight border, cursor changes to `grabbing`.

## EventModal Features

- Inline mini calendar for date range selection (click start → click end, with hover preview)
- Multi-day selection auto-enables all-day mode, hides time inputs
- 9-color palette (blue default, green, orange, red, purple, yellow, teal, pink, gray)
- Delete mode shows event title with confirmation

## Svelte Conventions

- Svelte 5 runes only: `$state`, `$derived`, `$derived.by`, `$props()`, `$effect`
- Store is a class with runes in `.svelte.ts` file
- Korean UI text throughout (일정, 설정, 추가, 삭제, etc.)

## Data Flow

- **Local events**: Saved to disk via `save_local_events` / `load_local_events` Tauri commands (JSON file in app data dir). Color included in JSON.
- **Google events**: Fetched via `fetch_events` command (±1 month range). Colors preserved via localStorage color map since Google API doesn't store custom hex colors.
- **Event move**: `moveEvent()` shifts dates locally, then calls `update_event` (PATCH) for Google events. Color map updated after move.
- **On startup**: `load()` → load local events → restore colors from localStorage → check Google auth → fetch Google events if connected.

## Google Calendar API Commands

| Command | Method | Endpoint | Purpose |
|---|---|---|---|
| `google_auth_start` | — | — | Initiates OAuth2 PKCE flow |
| `google_auth_status` | — | — | Checks if authenticated |
| `fetch_events` | GET | `/calendars/primary/events` | Fetch events in date range |
| `create_event` | POST | `/calendars/primary/events` | Create new event |
| `update_event` | PATCH | `/calendars/primary/events/{id}` | Update event dates/title |
| `delete_event` | DELETE | `/calendars/primary/events/{id}` | Delete event |

## Known Limitations

- Google Calendar disconnect command not yet implemented in Rust backend
- Scenario B dialog (offer to upload local events when Google connects) not yet implemented
- Debug `console.log` statements in `calendar.svelte.ts` should be removed before release
- Google app verification needed for public distribution (1-4 week review)

## Future Plans

- Mobile expansion via Tauri v2 Mobile or Capacitor (shared Svelte frontend)
- Google Calendar is the sync layer — no separate backend server needed
- 3-phase evolution: Tauri app → frontend separation → Shell Extension DLL
