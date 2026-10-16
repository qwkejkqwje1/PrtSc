# PrtSc 0.6 — validation and remaining acceptance

Environment: Linux sandbox, cross-compiled Windows x64 GNU, Wine + Xvfb1280×900. No native Windows PC, Telegram account or Google account used. Every image is synthetic/local test content.

## Automated checks
- **63 tests passed** both on Linux and as Windows test EXEs under Wine:36 core +10 screenshot/handoff +5 v0.3 +4 v0.4 +3 v0.5 +5 v0.6. Serial test execution because language/layout globals are process-wide. Logs in qa/.
- New tests: Pen default/order; no Select button; Move beside Picker; first action Lens; dedicated OCR/QR; per-tool visible icon colors; all eight selection-border hit-tests and resize/clamp cases; small badge; QR result row non-overlap at normal/fullscreen/tiny/right-bottom placements; hotkey parser/reserved-key validation and persistence.
- Existing privacy/redaction/undo/UTF16/QR/NV12/GIF/hosting tests retained. Browser token/path rejection and single-use behavior now expect **one inline page**, not a separate image download.

## Browser — no external upload
- Real Chromium builds and submits File/DataTransfer multipart, follows intercepted Google303. Every external response is fulfilled locally; no screenshot is sent to Google in this test.
- Three PNG +three JPEG runs,888×436 annotated selection. Exactly **one** local request in every run; correct field, filename, MIME, decoded dimensions, POST body and no Referrer checked.
- Timings in google-browser06.json are warm/local, noisy, **not native browser launch or live Google result latency**. No universal speedup is claimed. The code returns to the earlier one-response transport and ShellExecuteW instead of the0.5 fetch/second-response and ShellExecuteEx/COM path.

## Native GUI exercised under Wine
- Native resize: drag left edge from188 to130 with Pen active. Size changed888×436→946×436; exported PNG is946×436. No annotation scaling. All eight geometric cases separately unit-tested.
- QR fixture LUMA-QR-TEST-2026 automatically shown under the frame. Direct QR button and a **real result-row click after emptying the clipboard** copied that exact value. Editor remained present, no text dialog opened. Final screenshot has ASCII `[OK]`, avoiding a missing checkmark glyph. OCR no longer owns a QR submenu or indicator.
- Hotkey changed from PrintScreen to Ctrl+Alt+S through Settings; actual synthetic key events reopened capture. Another process temporarily reserved Ctrl+Alt+K: settings refused the collision, stayed open and retained the prior binding. After cancelling settings, Ctrl+Alt+S still captured. Test settings persisted that binding.
- Native ChooseColor: Pen=[203,37,92], Arrow=[40,190,100]; both icons show their respective colors concurrently, switching back restores Pen swatch. JSON confirmed both per-tool values. Tested Edge light/dark, Compact dark; four headless style/theme renders individually inspected. RU/EN settings show renamed Edge style, capture-key field and cursor checkbox. No screen-side menu commands remain in product code.

## GIF cursor verification
- Visible-cursor capture uses GetCursorInfo, hotspot-aware DrawIconEx into the capture DIB before scaling/encoding, GdiFlush and proper icon-bitmap cleanup.
- Diagnostic probe found **Wine's foreign cursor handle cannot be passed to GetIconInfo (error1402)**, even though GetCursorInfo reports it visible. Product now falls back to a process-local standard arrow when that handle is unavailable. This preserves position/motion; exact arbitrary foreign cursor shapes still need native Windows acceptance.
- Final live-region GIF pair:680×495,13 frames each. Cursor ON:five distinct arrow positions in an otherwise static ROI; Cursor OFF:no changed cursor pixels in that ROI. First/last visible-cursor frames inspected: upright scene/text, actual movement, no frozen image. See cursor-validation.json and cursor_on-visible/last.png.
- Test mouse moves were160→240 screen x at y180. The OFF recording also contains an unrelated previous-save notification at its top edge under Wine; it does not intersect the validated cursor ROI. No claim is made that all system notifications are excluded from video.

## Unchanged acceptance limits
- **Not a certified native Windows release.** Native browser activation/Google results, true Windows notifications, existing Explorer-window refresh/selection, high/mixed DPI, multiple monitors, GPU flicker/lag still need real hardware.
- Native Media Foundation MP4 output/orientation not accepted here; earlier Wine container setup failed0xD0000005. Top-down NV12 tests pass; live GIF with optional cursor is verified. No audio.
- WinRT OCR needs Windows runtime/language packs; no Wine end-to-end OCR acceptance.
- Previous live Litterbox/Catbox uploads returnedHTTP412. No successful external hosting/Imgur acceptance added. Hosting links/request validation retained.
- PrintWindow is cooperative capture, not a certified Telegram protection bypass. Protected contents may remain black.
- No guaranteed40ms/100ms capture/clipboard,5MB RAM/0% CPU, Google Keep upload, Linux/macOS or DXGI backend.
- EXE unsigned, size and OS-only imports recorded in RELEASE.json / qa/pe-imports.txt.

Build/test: cargo test --locked --all-targets -- --test-threads=1; cargo build --locked --release --target x86_64-pc-windows-gnu. JS/C fixtures are development tools, not runtime requirements. Original0.5 checks in older changelog are historical and not new0.6 acceptance claims.
