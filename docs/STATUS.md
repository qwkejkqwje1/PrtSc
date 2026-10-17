# PrtSc 1.1.0 — scope and acceptance

Extends the final 1.0 feature scope with explicit browser translation; unsigned Windows x64 release. Not a claim of bug-free or hardware-certified software.

Implemented and regression-tested: unfinished-action export/undo, small and multiline text bounds, text Esc/shortcut behavior, Multiply highlighter, persistent render/capture buffers, append-only cache updates, partial draw/move damage, bounded glyph caching, timestamp lookahead, optional recording cursor, native control styles, OCR/QR vector icons, extended Blur/Pixelate menu/hotkeys, provided exact ICO, dialog extraction and explicit native toolbar enum.

Existing QR link/copy separation, region handles, per-tool colors, pen-first layout, configurable physical hotkeys and secure Google handoff retained.

Verified here: Linux tests, Windows test EXEs/GUI under Wine/Xvfb, intercepted real Chromium upload, live GIF cursor ON/OFF. Not verified on real Windows hardware: MP4 codec output, WinRT OCR languages, real browser navigation/Lens results, multi-monitor/mixed DPI, system notifications, Explorer reuse, GPU behavior. Native MF setup fails in this Wine environment with0xD0000005; GIF is the tested recording option.

No new live hosting acceptance; previous Catbox/Litterbox HTTP412 remains unresolved. User-Agent version was corrected without spoofing browser identity or promising a WAF workaround. PrintWindow is cooperative, not protected-content bypass.

Not included: inline palette/thickness popup (existing color picker preserved), a full Config enum/schema rewrite, total removal of global language state, wholesale window-state redesign, audio, Linux/macOS app, Google Keep, DXGI, performance/resource guarantees. These are not required to fix the confirmed correctness/performance issues and would increase final-release risk.

## Translation in 1.1
Local Windows OCR reads the final flattened/cropped selection. An editable preview is shown before any browser navigation. The target language is remembered; recognized text and translation URLs are not written to settings/history by this path. The button follows Google search in both toolbar layouts; Ctrl+Shift+T opens the same action. The OCR icon uses scan lines, not the Text tool T.

Short text is percent-encoded into a fixed HTTPS Google Translate URL only after confirmation. Encoded URLs are limited to 1800 bytes with no truncation. Copy and open preserves long text in the clipboard and opens a text-free landing page for manual paste. Text in URLs can remain in browser history; clipboard history can retain copied content. Neither option makes online translation private from Google. No key, hidden HTTP API or screenshot upload is used for translation.

README has been replaced with English/Russian/Simplified Chinese descriptions and real UI screenshots of synthetic examples. This does not add Chinese app localization. The supplied black-and-white ICO remains unchanged.
