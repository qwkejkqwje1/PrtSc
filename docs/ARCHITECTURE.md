# PrtSc 1.2 architecture

Rust native Win32, windows-sys; targeted Windows projection for WinRT OCR and Media Foundation. Existing Luma class/crate names remain internal compatibility details.

## Boundaries
- `editor.rs`: annotations, transactions, undo/redo, region edge geometry. `tool_for_key` includes extended Blur/Pixelate without adding toolbar slots.
- `text.rs`: shared lazily loaded system font, font-metric layout/bounds and caret; `render.rs`: rasterization, bounded glyph cache, conservative ink bounds, Multiply highlighter and image scaling.
- `damage.rs`: conservative union/expansion geometry.
- `ui.rs`: typed Edge/Compact layout, vector icons, per-tool colors, QR result hit targets. Legacy global-style wrappers remain for old/headless callers; native paint receives style explicitly. Dock module is a stateless compatibility shim; no screen-side position state.
- `platform/windows.rs`: application lifetime, tray/editor/recording window routing and orchestration; `win_dialogs.rs`: settings, hotkey recording, text/history dialogs. This is a modest separation, not a wholesale risky rewrite of the application state machine.
- `win_native.rs`: RAII DIBs, clipboard, screen/cursor capture, Shell URL/file operations, WinRT OCR and explicit hosting.
- `win_paint.rs`: scene/chrome cache and clipped painting. `win_video.rs`: native recorder/Media Foundation. `video.rs`: GIF, reusable NV12 conversion and tested one-frame sample clock.

## Commit/export correctness
`finish_gesture` takes the active drag transaction before releasing capture. Draw commits its draft; Move/Resize record Replace in undo; current selection/region resize is retained. Both normal mouse release and export use this path. Clearing drag first prevents the synchronous WM_CAPTURECHANGED from rolling back a just-committed action. Cancel/lost capture still cancels and restores unfinished transformations.

`final_image` finishes gestures and text before flatten/crop/cache reuse. All export routes share it. Esc during text commits the text only; later Esc closes. Text draft and committed bounds use the same font/line-height as renderer; frame/caret are UI-only.

## Painting and memory
Base image + flattened scene + plain DIB + empty persistent back DIB. No full-size dim DIB: a 1x1 black raster is AlphaBlended at 153/255 over the dirty area, then selected pixels are restored. This removes one full-screen allocation (~31.6 MiB at 3840x2160), not a promise about total process RAM.

At native 1:1 dimensions, a single appended annotation with identical old prefix is rendered once; only its conservative ink rectangle is copied into the existing plain DIB. Other changes use safe full reconstruction while keeping same-sized DIB handles. Scaled/letterboxed demo views use the safe fallback. Live strokes/moves/resizes invalidate old/new bounds; hover uses conservative panel/tooltip damage, selection drags still fully repaint. Back DC is clipped to rcPaint and only rcPaint is presented. Chrome holds small independent panel surfaces, never an enormous union bitmap. Arrowheads, wide text bearings and step-number ink are included in damage tests.

The glyph cache uses char + exact size key, <=1024 entries /4MiB; oversized glyphs are not retained. It remains UI-thread local. Font initialization stays deferred until selection/text needs it.

## Recording
One ScreenCapturer owns screen DC and top-down DIB throughout recording; cursor compositing respects visibility/hotspot and releases GetIconInfo bitmaps. Screen snapshots omit the cursor. Capture bytes are borrowed until the next capture, never retained by callers. MP4 converts directly BGRA->limited-range BT.601 NV12 into a reused vector; factories are resolved once. COM/DLL/startup lifetime is RAII even when lookup/setup fails.

One-frame lookahead writes previous pixels with duration equal to the next capture timestamp difference. Last-frame duration ends at stop; first sample starts at zero. MP4 copies pixels into MF-owned buffers before reuse. GIF swaps reusable RGBA frames, retains quantizer scratch allocation, and carries centisecond remainder. This fixes gaps caused by pairing wall-clock timestamps with fixed durations. Streaming encoding does not accumulate a recording in memory. Writer finalizes a partial file before renaming. Video is live desktop content, not the screenshot annotation compositor.

## Security/compatibility retained
QR is local/revision-tagged/debounced. UI result row stays inside selection; URL text explicitly opens validated HTTP(S) via ShellExecute and closes only after successful system dispatch; separate pages copy keeps editor. The row never exports. Hotkey recording keeps focus-scoped Snapshot hook, release-only fallback and conflict-safe ID reservation/rollback.

Google keeps one-response single-use inline multipart handoff, cryptographic loopback token, Host/Origin/CSP/referrer controls and bounded multiplexed incomplete connections. No automatic upload and no image/search-token logging. History/config use prior atomic writes and validated IDs. PrintWindow retains one-in-flight guard; resetting it after timeout would permit unbounded hung workers and is intentionally avoided.

Runtime/hardware limitations are explicit in README and QA. Broad Config enum migration and global language removal were deferred to preserve saved JSON compatibility and avoid unrelated final-release risk.

## Translation boundaries
- Image action: `final_image()` finishes pending edits, flattens and crops; `clipboard_image` stores the final pixels; an allowlisted `op=images` Google Translate URL is opened. Only successful shell dispatch closes the editor. No local OCR. **Manual paste remains necessary** because cross-origin file-input injection is unavailable in an arbitrary user's browser. No fake global keystrokes are synthesized.
- OCR result: `show_ocr_text` adds Translate without adding it to unrelated help/error/link dialogs. `win_translate.rs` keeps the original OCR dialog alive, hides it while the translation window is open, and restores it on close. The source copy is editable; original OCR text is unchanged.
- `translate.rs`: fixed target allowlist, UTF-8 percent encoding, 5000-character native-request bound, 256KiB response cap, strict segmented JSON parsing and stale-result predicate. Browser fallback retains the former 1800-byte URL policy, no truncation, with text-free landing + full clipboard for long input.
- `win_text_http.rs`: Windows WinHTTP, fixed HTTPS host/path, normal TLS validation, automatic proxy, redirects disabled, finite phase timeouts, bounded response loop, no keys, no auto-retries. Body is an application/x-www-form-urlencoded POST rather than text in the request URL. This is an undocumented endpoint, not a guaranteed Google API contract.
- At most one text network job per app is allowed. A monotonic request ID plus source/target comparison discards answers after edits or dialog close/reopen. Result messages do not set the global screenshot busy state. Errors go into the same preview, not a replacement of OCR text. Only target preference is saved; text, URL, request body and result are not persisted by this flow. Browser/clipboard history and Google's processing remain privacy considerations.

`--translation-preview <UTF8_FILE>` opens the actual OCR result UI with a sample text fixture. It does not perform OCR. Tests register an HTTPS sink only in an isolated Wine prefix for browser-dispatch checks. Native text-translation validation used one synthetic phrase, not user data.
