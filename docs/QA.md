# PrtSc 1.1.0 — validation and limits

## Current automated run

- **91 tests passed on Linux** and **91 passed as Windows x64 test EXEs under Wine**, serial mode. Includes the 85 regression tests from 1.0 and six new translation tests.
- Translation tests cover UTF-8/emoji/query escaping, fixed HTTPS origin, allowlisted targets, blank/control input, exact 1800-byte URL limit, no silent truncation, text-free manual-paste URLs, old-settings defaults and button order/non-overlap in both layouts.
- Windows release and Linux QA binaries compiled with `--locked`; `Cargo.lock` records version 1.1.0. No dependency was added for translation.

## Translation dialog and dispatch

The real Windows preview was exercised under Wine/Xvfb using a UTF-8 **fixture**, not a pretend OCR result. Actual ShellExecute navigation was routed to a local test executable registered as the HTTPS handler in an isolated Wine prefix; **no test text was sent to Google**.

Verified:
1. Opening the preview does not launch the browser.
2. Cancel and Escape return to the original editor.
3. Edited Unicode text and the chosen language arrive intact in the explicit HTTPS dispatch. Successful dispatch closes preview and editor.
4. Long input disables direct URL translation. An attempted direct command still does not navigate or truncate.
5. Copy and open preserves the full clipboard text and dispatches a landing URL without text; the user must paste manually.
6. Blank text disables both actions. Settings persist only the target language, not recognized text or a translation URL.
7. On this Wine runtime, the real translation action reaches OCR, reports unavailable WinRT OCR, preserves the editor and does not open a browser.

This proves preview, encoding and dispatch behavior, **not Windows OCR accuracy, translation quality or successful loading of Google's live website**. Local OCR needs Windows OCR language packages; browser translation needs network access. A shell dispatch success is not proof of a loaded page. Clipboard/browser history can retain text.

Evidence: `qa/translation-native.json`, `qa/test-summary.json`, `qa/tests-linux.log`, `qa/tests-wine.log`. The C probes are test fixtures, not app dependencies. `--translation-preview <UTF8_FILE>` deliberately bypasses OCR for reproducible GUI checks.

## Google image-search regression

Six runs in real headless Chromium, all external requests intercepted locally: three original-PNG and three fast-JPEG uploads, with eight idle preconnections kept open. All generated a multipart POST, correct image field/filename and 888×436 pixels, no Referrer, and completed a simulated 303 redirect. Exactly one loopback page response per run. Local page-to-POST time was about **42–99 ms** for this synthetic warm-browser case. **Not live Google latency, cold-browser time, or a performance promise.** See `qa/google-browser07.json`.

## Screenshots

`assets/screenshots/` contains current screenshots of the actual app under Wine. Editor examples are synthetic demo content; the translation preview uses a documented sample-text fixture. No completed translation or live OCR result is fabricated. English/Russian UI remains supported; Chinese is README copy, not new app localization. The user-provided black-and-white icon is unchanged.

## Inherited validation

The earlier 1.0 GUI/GIF/render-damage validation is preserved in [QA-1.0-history.md](QA-1.0-history.md) and the historical files in `qa/`. Those results are historical, not all rerun as GUI scenarios in 1.1. The complete Rust regression suite was rerun. Previous release-specific screenshots are historical; current gallery is under `assets/screenshots/`.

## Still requires real Windows acceptance

MP4 Media Foundation output (Wine setup fails with 0xD0000005), WinRT OCR language behavior, mixed DPI/multi-monitor, system notifications, real browser reuse/navigation, Explorer integration and live Google results. GIF is the recording format verified in this environment in 1.0. Windows N may need Media Feature Pack. Builds are unsigned.

No new live upload-host testing was done; historical Catbox/Litterbox HTTP 412 is unresolved. PrintWindow is cooperative capture, not guaranteed bypass of protected content. No audio, Linux/macOS GUI or latency/RAM guarantees are claimed.
