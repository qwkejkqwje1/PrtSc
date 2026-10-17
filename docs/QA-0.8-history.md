# PrtSc 0.8 — validation

Environment: Linux + Windows x64 GNU EXEs under Wine, Xvfb 1280×900. Only synthetic local images, injected keyboard/mouse events and test URL handler. No native Windows hardware or live browser/Google acceptance.

## Automated checks
**70 tests pass on Linux and as Windows test EXEs under Wine**: 36 core +10 screenshot/handoff +5 v03 +4 v04 +3 v05 +5 v06 +5 v07 +2 v08. Serial execution due to shared language/layout globals. Updated QR geometry test expects no row for a region too small to contain it. New cases assert the row stays fully inside normal/fullscreen/right-bottom selections, does not overlap panels and stays vertically near the QR action, plus Snapshot VK formatting with all modifier combinations. Previous security/encoding/undo/resize/clipboard geometry tests retained.

## PrintScreen recording — current native GUI test
- Start with a previously bound PrintScreen, no overlay. Focus the recording field and inject Ctrl+Alt+K: field records that combo, no capture.
- Inject **only VK_SNAPSHOT KEYUP**: field becomes PrintScreen, no capture. Separately send direct WM_KEYUP to test the no-hook fallback: same result.
- Save Ctrl+Alt+K. Reopen Settings and physically inject PrintScreen keydown+keyup, now not the saved binding: field becomes PrintScreen, overlay remains absent. Save.
- Press PrintScreen away from the field: overlay appears. Cancel capture and refocus Settings; pressing already-bound PrintScreen records it without capturing. Ctrl+Shift+PrintScreen also records without capturing.
- Leave recording via Tab/cancel Settings; PrintScreen captures again. This exercises focus-scoped interception and release, not only string assignment. See hotkey-gui.log and printscreen-recorded.png.
- Hook handles only Snapshot while the field is focused; it is removed on blur/destruction. Ordinary keys use the EDIT subclass; conflict-safe RegisterHotKey save remains unchanged. Windows 11 Snipping Tool policy and real physical keyboards still require native Windows acceptance.

## QR — current native GUI test
- Synthetic QR contains http://127.0.0.1:19876/qr-test. Selected region x100 y90 w680 h495. UI row x436 y557 w340 h24: inside the selection near the QR action.
- Clear clipboard, click copy pages at762,569: exact URL copied, overlay remains present, no text dialog.
- Click URL at450,569: test-only Windows HTTP association receives the exact URL, editor overlay closes, no text dialog. No website/network request occurs: only system dispatch and editor close are validated, not a real browser loading a site.
- Row is UI chrome, not part of export/clipboard-image flattening. Too-small regions omit it rather than placing it outside the selected image. Original QR button remains available.
- Screenshots in qa/: qr-inside.png, qr-copied.png, qr-opened-closed.png; action transcript qr-gui.log.

## Artwork and unchanged functionality
Original tiger redrawn with broad striped face, small rounded ears, narrowed eyes and longer fangs. Optical 16/20px uses the same rounded-ear geometry with thicker outer edge and simplified two-fang mouth, not the earlier tall-ear silhouette. Generated ICO covers16/20/24/32/48/64/128/256. Final preview and native window icon inspected. No claim of user aesthetic approval or full Windows tray/DPI acceptance.

Google path unchanged from0.7: preconnect-safe one-response inline upload, direct RGBA JPEG and standard URL dispatch. Existing Rust handoff/security tests pass again; intercepted Chromium multipart runs and timing comparison are historical0.7 evidence, NOT new live Google or default-browser latency measurements. See QA-0.7-history.md.

## Remaining acceptance limits
Unsigned Windows preview. Real Windows browser launch/live Lens results, high/mixed DPI and multiple monitors, system notifications/reused Explorer selection and GPU behavior remain unverified. Native Media Foundation MP4 was not accepted under Wine; older GIF/cursor checks are historical0.6. WinRT OCR requires Windows runtime/languages. Previous Litterbox/Catbox returnedHTTP412; no new live-hosting acceptance. PrintWindow does not guarantee protected Telegram capture. No audio, Google Keep, Linux/macOS, DXGI or certified extreme latency/RAM/CPU targets.
