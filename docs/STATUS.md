# 0.8 acceptance status

Implemented: pen-first/default with a pencil icon; no Select/V button; eight-way selection-edge resize; outside drag starts new area; per-tool icon colors and persistence; Move beside Picker; leftmost Lens; separate one-click OCR and QR; split in-selection QR URL-open-and-close / copy-and-stay targets; larger size badge; selection-attached Edge/Compact styles without monitor docking; press-to-record configurable global hotkey with focus-scoped Snapshot interception and KEYUP support with conflict rollback; optional visible cursor in GIF/MP4 capture; original tiger EXE/tray/window artwork; nonblocking preconnect-safe one-response inline Google upload and standard ShellExecuteW.

Retained: local screenshot editor, clipboard/export, RU/EN, fixed opaque redaction, pin/topmost, native GIF, native Media Foundation MP4 code, optional history, native offline OCR, image-host requests and links, saved-file Shell reveal.

Validation in Linux/Wine only; see QA. Not accepted on native Windows hardware: actual default-browser startup and live Lens results/latency, native MP4 output/orientation/codecs, all mixed-DPI/multi-monitor/flicker cases, notification delivery and reused Explorer exact-selection behavior. WinRT OCR needs Windows OCR runtimes/languages. Prior Wine MP4 setup failed. Prior Catbox/Litterbox returned HTTP412; no new successful live-hosting/Imgur acceptance. Cooperative PrintWindow does not guarantee protected Telegram capture. No audio, Google Keep upload, Linux/macOS, DXGI, or certified extreme latency/RAM/CPU targets.

This is a test-build iteration, not a claim of full original-spec completion.
