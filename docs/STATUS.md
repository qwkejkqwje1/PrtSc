# PrtSc 1.2.0 — scope and acceptance

Extends the final 1.0 feature scope with explicit browser translation; unsigned Windows x64 release. Not a claim of bug-free or hardware-certified software.

Implemented and regression-tested: unfinished-action export/undo, small and multiline text bounds, text Esc/shortcut behavior, Multiply highlighter, persistent render/capture buffers, append-only cache updates, partial draw/move damage, bounded glyph caching, timestamp lookahead, optional recording cursor, native control styles, OCR/QR vector icons, extended Blur/Pixelate menu/hotkeys, provided exact ICO, dialog extraction and explicit native toolbar enum.

Existing QR link/copy separation, region handles, per-tool colors, pen-first layout, configurable physical hotkeys and secure Google handoff retained.

Verified here: Linux tests, Windows test EXEs/GUI under Wine/Xvfb, intercepted real Chromium upload, live GIF cursor ON/OFF. Not verified on real Windows hardware: MP4 codec output, WinRT OCR languages, real browser navigation/Lens results, multi-monitor/mixed DPI, system notifications, Explorer reuse, GPU behavior. Native MF setup fails in this Wine environment with0xD0000005; GIF is the tested recording option.

No new live hosting acceptance; previous Catbox/Litterbox HTTP412 remains unresolved. User-Agent version was corrected without spoofing browser identity or promising a WAF workaround. PrintWindow is cooperative, not protected-content bypass.

Not included: inline palette/thickness popup (existing color picker preserved), a full Config enum/schema rewrite, total removal of global language state, wholesale window-state redesign, audio, Linux/macOS app, Google Keep, DXGI, performance/resource guarantees. These are not required to fix the confirmed correctness/performance issues and would increase final-release risk.

## Translation in 1.2
The main button after search now copies the final image to the system clipboard and opens the official Google Translate Images UI. The user must paste it there (Ctrl+V / Paste an image). This is deliberately not advertised as automatic upload or automatic switch of an ordinary Lens search. It bypasses local OCR. The existing Google Lens search button is unchanged.

Only OCR result windows offer Translate. A separate native window retains editable source, target language and read-only result. An explicit click sends a bounded HTTPS form POST to the undocumented Google text endpoint. No browser, API key or extra runtime is needed for successful in-app translation. Errors preserve source and offer explicit browser fallback; long fallback input is copied for manual paste rather than truncated into a URL. No automatic retries or automatic text transmission.

The compiled FileDescription is now ASCII, fixing UTF-8/legacy resource-decoder mojibake in process/task listings. The provided ICO remains unchanged.

An actual WinHTTP request from the Windows EXE under Wine returned «Привет, мир» for synthetic «Hello world». This proves that tested request, not future service availability, translation quality or real Windows OCR acceptance.
