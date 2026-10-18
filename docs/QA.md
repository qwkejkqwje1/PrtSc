# PrtSc 1.2.0 — validation

## Automated regression

**100 Rust tests passed on Linux and 100 passed as Windows x64 test EXEs under Wine.** Builds used Cargo.lock; no runtime dependency was added. Nine new tests cover request escaping and bounds, segmented Unicode responses, malformed/oversized responses, fixed image UI URL, stale-result rejection and ASCII resource description. The 91 prior tests also passed. See `qa/test-summary.json` and test logs.

The test runner uses only executables listed by `cargo test --no-run`, not every EXE in the dependency directory (which can include the ordinary tray application).

## Native translation checks

The actual Windows release EXE was run under Wine/Xvfb, using a UTF-8 fixture in place of WinRT OCR. This is a real OCR-result/translation UI test, **not a claim that Windows OCR was tested under Wine**.

- OCR result offers Translate; no request or browser launch occurs merely by opening the result.
- The real WinHTTP client sent the synthetic phrase `Hello world` to Google and received `Привет, мир`. Source text remained unchanged, and Copy translation returned the exact result.
- Browser fallback used an HTTPS test sink registered only inside the isolated Wine prefix. It preserved both text windows and sent the expected short-text URL.
- Long text disabled native translation and fell back to the full clipboard text plus a URL without text. No truncation. Blank input disabled both actions.
- Closing the translation window restored the original OCR text, unchanged. Only target preference was saved; no source/result text was written to settings by this path.
- Main image action did not invoke local OCR: it copied the final 888×436 image, preserved the opaque redaction, opened the official Google Translate Images URL and closed the editor after successful shell dispatch.
- The user must paste the image in the browser. **Automatic image upload and a completed browser image translation were not implemented or validated.** A direct Lens upload probe did not retain translation-mode parameters; an ordinary search was not silently substituted for translation.
- VerQueryValueW read the compiled EXE's FileDescription as exactly `PrtSc - native screenshots`. This fixes the UTF-8/legacy-codepage resource mojibake seen in task listings. Actual Windows Task Manager rendering remains a Windows-hardware acceptance item.

Evidence: `qa/native-translation12.json`, `assets/screenshots/translation.png`, `assets/screenshots/ocr.png`. The text translation screenshot shows a real Google response to a synthetic phrase, with fixture input rather than recognized user data.

Service-error responses and stale-result combinations have deterministic unit coverage. Live service availability is not guaranteed; a forced HTTP 429/403 and all GUI failure variants were not tested against Google. The client has no automatic retry, normal TLS checks, disabled redirects, phase timeouts, 5000-character input cap and a 256KiB response cap. The endpoint is undocumented and may change.

## Google image-search regression

Six intercepted headless Chromium runs passed: original PNG and fast JPEG, eight idle preconnections, correct multipart image field/filename/dimensions, no Referrer, and a simulated redirect. No external Google traffic was used for this regression. Synthetic warm-browser local page-to-POST timings were roughly 36–95ms, **not live Google latency**. Search remains separate from image translation. See `qa/google-browser07.json`.

## Privacy and remaining limits

Only the synthetic test phrase was used in the live text request; no user screenshot or OCR text was sent in native tests. Recognized text is sent only after an explicit Translate click. Image pixels leave the app only when pasted/uploaded on Google's page. Browser and clipboard history may retain content; Google's processing is online. No text/translation cache is persisted by this feature.

Inherited release validation is described in [QA-1.1-history.md](QA-1.1-history.md) and [QA-1.0-history.md](QA-1.0-history.md). Old screenshots/results are historical, not all GUI scenarios rerun in 1.2.

Still requires real Windows acceptance: installed WinRT OCR languages, native MP4/Media Foundation output, mixed DPI/multi-monitor, system notifications, Explorer/browser reuse and Google image clipboard paste. MP4 setup fails under this Wine runtime; GIF was verified in 1.0. Windows N may need Media Feature Pack. Builds are unsigned. Historical Catbox/Litterbox HTTP 412 remains unresolved. PrintWindow is cooperative, not guaranteed protected-content bypass.
