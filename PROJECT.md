# Project: Muster Mac Security & Compatibility Audit & Remediation

## Architecture
- **Desktop Runtime**: Tauri v2 (`@tauri-apps/api`, `@tauri-apps/cli`) on macOS Apple Silicon (`aarch64-apple-darwin`).
- **Backend (Rust)**: Native WebKit cookie store integration (`WKHTTPCookieStore` via Objective-C runtime `objc2`), OS Keychain credential vault via `keyring`, `reqwest` HTTP client with custom `RequestGate` token-bucket concurrency pacing, and Tauri IPC command handlers in `src-tauri/src/lib.rs`.
- **Frontend (React + Vite + TypeScript)**: React 18 SPA, Tailwind CSS, Lucide icons, Zustand state store (`useAppStore.ts`) with persistence, DOMPurify HTML sanitization for Moodle CMS content, and strict Content Security Policy (CSP).

## Feature Inventory
| # | Feature / Remediation Item | Description | Milestone | Status |
|---|---------------------------|-------------|-----------|--------|
| 1 | Mutex Lock Release Before Network I/O | Release `ai_guard` lock before awaiting LLM HTTP request in `generate_summary` (`lib.rs`) | M1 | DONE |
| 2 | RequestGate Concurrency Pacing Fix | Fix `acquire()` race condition to schedule `next_allowed` reservations under concurrency (`throttle.rs`) | M1 | DONE |
| 3 | WebKit Cookie Injection Keys Fix | Correct Objective-C `NSDictionary` keys (`c"Name"`, `c"Value"`, etc.) in `webview_cookies.rs` | M1 | DONE |
| 4 | WebKit Navigation Cookie Synchronization | Ensure in-app browser waits for async cookie injection before navigating (`lib.rs` / `webview_cookies.rs`) | M1 | DONE |
| 5 | IPC File Confinement & Path Sanitization | Restrict `clear_downloads` in `lib.rs` to validate paths within download dir and reject `..` traversal | M1 | DONE |
| 6 | PII HTML Dump & Credential Hardening | Disable unencrypted debug HTML dumps and enforce secure keyring storage for auth tokens | M1 | DONE |
| 7 | Fix Invalid Clippy Regexes in Scraper | Replace unsupported lookaround and backreference regexes in `extract_author` and `nearest_heading_before` | M1 | DONE |
| 8 | Rust Test Suite Fixture Resilience | Fix `sample()` in `scraper.rs` with mock/synthetic fixtures and add `.no_proxy()` to integration tests | M1 | DONE |
| 9 | LocalStorage Secret Stripping | Exclude `aiApiKey` from browser `localStorage` in `useAppStore.ts` partialize | M2 | DONE |
| 10 | Strict Content Security Policy Hardening | Add `object-src 'none'; base-uri 'self'; frame-ancestors 'none';` to `tauri.conf.json` | M2 | DONE |
| 11 | Markdown Link Protocol & Host Sanitization | Restrict protocols to `http:`, `https:`, `mailto:` and use strict hostname matching in `MarkdownRenderer.tsx` | M2 | DONE |
| 12 | Package Script & UI Cleanliness | Clean missing scripts in `package.json` and update macOS Keychain copy in `translations.ts` | M2 | DONE |
| 13 | Comprehensive Verification & Test Suite | 54/54 tests passing, `npm run build` clean, 0 warnings, Forensic Audit CLEAN | M3 | DONE |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | M1: Backend Security, Concurrency & Native macOS Hardening | Features 1, 2, 3, 4, 5, 6, 7, 8 in `src-tauri/` | None | DONE |
| 2 | M2: Frontend Security, CSP & Storage Hardening | Features 9, 10, 11, 12 in `src/`, `tauri.conf.json`, `package.json` | None | DONE |
| 3 | M3: E2E Verification & Forensic Integrity Gate | Feature 13: Full compiler, test, security grep, and forensic audit validation | M1, M2 | DONE |

## Interface Contracts
### Rust Backend ↔ Frontend IPC
- `generate_summary(content: String, api_key: Option<String>, api_url: Option<String>, model: Option<String>) -> Result<String, String>`: Drops lock immediately, returns summary.
- `open_in_app_webview(url: String, title: String) -> Result<(), String>`: Injects cookies asynchronously and navigates without SSO redirect loops.
- `clear_downloads(save_path: String) -> Result<(), String>`: Validates path strictly against download root without path traversal vulnerability.
- `useAppStore` (Frontend): Stores app state in `localStorage` excluding sensitive API keys.

## Code Layout
- `src-tauri/src/lib.rs`: Tauri IPC commands, state initialization, window management, path containment.
- `src-tauri/src/moodle/auth.rs`: Session authentication, OS Keyring storage, token lifecycle.
- `src-tauri/src/moodle/scraper.rs`: HTML parser, course/assignment extraction, fallback fixtures.
- `src-tauri/src/moodle/throttle.rs`: `RequestGate` token-bucket and concurrency pacer.
- `src-tauri/src/moodle/webview_cookies.rs`: Native macOS WebKit `WKHTTPCookieStore` FFI.
- `src-tauri/tauri.conf.json`: Tauri security, CSP, window configs.
- `src/stores/useAppStore.ts`: Zustand store with partialize filter excluding secrets.
- `src/components/ui/MarkdownRenderer.tsx`: Safe markdown renderer and protocol sanitizer.
- `src/pages/CourseDetail.tsx`: Moodle HTML rendering with DOMPurify sanitization.
