# PrtSc 0.7 — validation

Environment: Linux sandbox + Windows x64 GNU test EXEs under Wine, Xvfb 1280×900. Synthetic/local images only; NOT native Windows hardware acceptance. Previous iteration details are historical in QA-0.6-history.md.

## Current checks
- **68 tests passed on Linux and as Windows EXEs under Wine** (36+10+5+4+3+5+5). New cases: QR open/copy target geometry; physical key-event canonicalization/roundtrip/reserved keys; larger badge and ICO directory payloads; eight idle sockets cannot delay a real request; wrong Host/Origin rejected before valid request. Header limit, timeout, token and single-use protections retained.
- Native GUI: injected actual Ctrl+Alt+K key events with the shortcut field focused (no SetWindowText shortcut injection). Field recorded Ctrl+Alt+K; Esc restored PrintScreen; reserved Ctrl+Shift+R did not overwrite it. Saved combo triggered capture. Reopening settings and pressing the already bound combo recorded it without opening capture; Tab did not change it. Existing conflict rollback mechanism unchanged; prior conflict test retained as historical, not rerun this iteration.
- Native QR fixture: http://127.0.0.1:19876/qr-test. A plain URL-row click dispatched exactly that URL to a local test-only Windows URL-handler stub (no external browser/site). After clearing the clipboard, a separate right-hand overlapping-pages click copied the exact URL. Editor remained open, no text dialog appeared. Earlier attempt used Wine's default handler and did not launch a browser; successful dispatch used HKCR stub. No live URL/browser claim.
- Actual EXE resources loaded at 32/16px by Windows LoadImage and drawn to DIB; screenshot inspected. Native window title area also shows tiger. Original multi-resolution ICO inspected on light/dark at actual sizes and pixel zoom. Real Windows tray theme/DPI still requires user acceptance.
- Edge/Compact × dark/light renders produced; larger badge inspected in normal selection; geometry tests cover full/tiny/right-bottom selections without panel/QR overlap. Native QR screenshot confirms separate copy icon and green copied state.

## Google handoff
- Six real Chromium runs, original PNG and fast JPEG, annotated 888×436 selection; **eight empty preconnections held open** in each run. Every external request is intercepted/fulfilled locally. Correct multipart POST field, filename, dimensions, no Referrer and exactly one local request verified, including intercepted 303 redirect. No external image upload.
- Local page-to-intercepted-POST range 36–112ms in warm headless Chromium. This is NOT native default-browser activation or Google result latency.
- Same Windows EXEs under Wine, one synthetic run per condition: local HTTP response with 0/1/8 idle connections: v0.6 3.41/271.31/2380.06ms; v0.7 1.09/1.36/1.99ms. Deliberately forced preconnect scenario, not typical end-to-end user latency. Data in google-preconnect-comparison.json.
- Direct RGBA JPEG path is decoded/validated by existing tests and browser tests. Release opt-level2 replaces size optimization; no unsupported universal performance claim.

## Unchanged limits
- Windows preview, unsigned. No native Windows-PC validation of actual browser startup/live Lens results, mixed/high DPI, multi-monitor, GPU lag/flicker, notification delivery or reused Explorer exact selection.
- Native MP4/MF output not accepted; earlier Wine setup failed. Existing NV12 tests pass; GIF optional-cursor checks were performed in 0.6, not repeated here. No audio.
- WinRT OCR needs Windows runtime/language packs. Earlier Litterbox/Catbox requests returned HTTP412; no accepted live hosting/Imgur test.
- Cooperative PrintWindow is not a guarantee of protected Telegram capture. No Google Keep, Linux/macOS, DXGI or certified extreme RAM/CPU/latency targets.
