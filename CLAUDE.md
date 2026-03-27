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

## Architecture

```
src/                          # Svelte 5 frontend
├── App.svelte                # Shell: TitleBar + Calendar + Settings
├── app.css                   # Global CSS variables (dark glassmorphism theme)
├── lib/
│   ├── types.ts              # CalendarEvent, CalendarDay interfaces
│   ├── stores/
│   │   └── calendar.svelte.ts  # Reactive store ($state/$derived runes)
│   └── components/
│       ├── TitleBar.svelte   # Custom titlebar with manual drag (SetWindowPos)
│       ├── Calendar.svelte   # Month grid with inline event bars + EventModal
│       ├── EventModal.svelte # Add/delete event dialog
│       └── Settings.svelte   # Settings panel (Google auth, opacity, size, theme)

src-tauri/src/                # Rust backend
├── main.rs                   # Entry point
├── lib.rs                    # Tauri setup, tray, embed_in_desktop(), move_window/get_window_position commands
├── google_calendar.rs        # OAuth2 flow, token management, Google Calendar CRUD (5 Tauri commands)
└── state.rs                  # AppState with Mutex, 5-min event cache
```

## Key Design Decisions

- **Window drag**: `startDragging()` doesn't work for WorkerW child windows. Uses Rust-side `SetWindowPos`/`GetWindowRect` via `move_window` and `get_window_position` Tauri commands.
- **Desktop embedding**: Finds Progman → sends 0x052C → finds WorkerW behind SHELLDLL_DefView → `SetParent`. `WS_EX_TOOLWINDOW` prevents Show Desktop from hiding it.
- **OAuth2**: Hand-rolled HTTP flow with PKCE (oauth2 crate removed due to v5 type complexity). Tokens stored in Windows Credential Manager via keyring. Refresh is automatic.
- **Google Calendar API credentials**: Placeholder `CLIENT_ID`/`CLIENT_SECRET` in `google_calendar.rs` — must be replaced with real credentials from Google Cloud Console.

## Svelte Conventions

- Svelte 5 runes only: `$state`, `$derived`, `$derived.by`, `$props()`, `$effect`
- Store is a class with runes in `.svelte.ts` file
- Korean UI text throughout (일정, 설정, 추가, 삭제, etc.)

## Future Plans

- Mobile expansion via Tauri v2 Mobile or Capacitor (shared Svelte frontend)
- Google Calendar is the sync layer — no separate backend server needed
