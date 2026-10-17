# PrtSc 1.0.0 — validation

Environment: Linux x64; GNU Windows x64 release and test EXEs under Wine, Xvfb 1280×900. Synthetic fixtures only. No native Windows hardware. This is final scope delivery, not a certification of all Windows/runtime combinations.

## Automated tests: 85 + 85
85 Rust tests passed on Linux and the same 85 as Windows EXEs under Wine: 36 core, 10 screenshot/handoff, 5 revision03, 4 revision04, 3 revision05, 5 revision06, 5 revision07, 2 revision08, **15 final-release tests**. Tests run serially because language and compatibility headless style wrappers remain global. Windows release build completed without warnings.

New tests cover rendered-ink containment for text at stroke widths1/2/3/8, multiple lines/Cyrillic/emoji/bearings, empty caret geometry, conservative damage for all visible drawing tools including arrowheads and large step numbers, incremental-patch equality with full flatten, Multiply preserving black, bounded/reused glyphs, Unicode elision at tiny widths, union/expansion, reusable BGRA/RGBA NV12 equivalence/capacity/invalid input, continuous sample clock with slow/non-monotonic captures and final flush, reusable resize, exact provided icon/manifest, typed style aliases and extended B/X tool access.

## Native GUI regressions (Wine)
- Enter while opaque-redaction drag still owns capture: exported clipboard has the unfinished rectangle, solid[24,24,27], correct888×436 selection.
- Save while rectangle drag is held: PNG matches completed-drag export pixel-for-pixel. The test waits for save completion, not merely file creation.
- Move/Resize while held -> Ctrl+Z: original completed-figure image restored pixel-for-pixel. Move while held -> Copy exports the moved position.
- 14px/three-line text including Cyrillic: all lines survive, first Esc keeps editor and commits, Ctrl+C during typing commits/exports, Esc outside typing closes. Empty text shows UI frame/caret. UTF-8 fixture is read from a file to avoid Wine ANSI argv conversion.
- PrintScreen recording captures actual Ctrl+Alt+K and release-only Snapshot without triggering capture while field focused. Prior focused-hook behavior/security remains covered by revision08 tests.
- A normal BitBlt capture at native dimensions,80 pen strokes/640 forced mouse moves: **1 initial scene build,80 append updates**. Live-move paints use partial damage. Wine GetGuiResources returns0, so no native GDI-handle-count claim is made.
- During live arrow draw and move, captured screen after clipped repaint equals screen after explicit full invalidation pixel-for-pixel. This checks for stale edges/trails; selection drags intentionally retain full repaint.
- Settings visually inspected: labels/check boxes share white background and v6 controls, no grey label strips.
- Provided31,680-byte ICO is byte-identical to input (SHA256 a8b0b1bcc03f3b40bad45e4d532353a2856c7cdc766db55772a51e83c4096460). All8 sizes embedded. Win32 LoadImage resource1 loads16/32px, DrawIconEx preview inspected. This is not acceptance of every Windows tray/DPI scale.

## QR open / copy
Synthetic HTTP QR, result inside x100,y90,w680,h495 near QR button. Pages target copies exact full URL and keeps editor; text target dispatches exact URL and closes editor. A test-only HTTP association in the isolated Wine prefix records the URL, so this is **system dispatch, not browser navigation**; no website request occurs. The fixture waits for a newly generated revision result rather than stale diagnostics. Tiny selections, separate open/copy targets and inside-selection geometry are covered by Rust tests. UI row/caret/frames do not enter exported image.

## Google: real Chromium, no external network
Six intercepted runs (3 PNG original +3 JPEG fast),8 idle speculative connections held open. Each valid handoff uses exactly1 loopback request, then correct multipart POST/filename/image dimensions and local303 redirect with no referrer. All external requests intercepted/fulfilled or blocked. Synthetic888×436 image. Original payload70,520bytes; fast39,344bytes. Local page-to-post measurements roughly41–128ms in these warm headless runs. **Not default-browser startup, live Google response latency or a universal speed promise.** Security/origin/token tests pass again. See google-browser07.json.

## Live GIF and MP4
Two live680×495 GIF recordings at10FPS under Wine,22 frames each. Cursor ON: pointer appears/moves,73–79 dark pointer pixels in the blank test crop; OFF:0. GIF durations2.18s/2.19s vs capture reports2.213s/2.212s (small finish/rounding difference). These exercise persistent ScreenCapturer and swapped frame buffers, not only synthetic encoder output. Cursor bitmap is added before scaling; no clicks/audio visualization.

MP4 setup was actually attempted: Wine Media Foundation container creation returns **0xD0000005**. No valid MP4 output is claimed. NV12 math, buffer equivalence and contiguous timing are tested; actual MF codec output still requires real Windows (and Media Feature Pack on Windows N). GIF is the recording option verified here.

## Limitations / non-goals
No real Windows acceptance for mixed/high DPI, multiple monitors, notifications/Explorer reuse, GPU paths, browser launch/live Lens results or installed WinRT OCR languages. No new live-hosting acceptance: historical Litterbox/Catbox HTTP412 remains; a correctly versioned User-Agent is not claimed to fix it. PrintWindow is cooperative, retains one in-flight guard and cannot guarantee protected Telegram capture. No audio, Google Keep, Linux/macOS app, DXGI, signed binary or certified low-memory/latency/CPU targets.

Historical0.6–0.8 reports are explicitly historical, not evidence that runtime limitations are solved in1.0. Current logs/screenshots/JSON are in qa/. C probes are build-only test utilities, not shipped dependencies of the application.
