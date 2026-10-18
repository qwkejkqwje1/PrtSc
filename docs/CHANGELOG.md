# PrtSc 1.2.0

- Split image translation from OCR: the main action copies the final screenshot and opens Google Translate Images for manual paste. No local OCR in this path; no claim of automatic upload.
- Added Translate to OCR result windows: optional in-app keyless Google text translation, with source/target/result, explicit consent by button, bounded background HTTPS POST and browser fallback.
- Preserved source on failure/close; stale responses after editing or reopening are discarded. No automatic retries or text persistence.
- Fixed FileDescription resource encoding: `PrtSc - native screenshots` in compiled EXE.
- Added parser, request-policy, image-URL, stale-result and resource regression tests.

---

# PrtSc 1.1.0

- Added keyless browser translation after local OCR, with an editable preview, nine target languages, explicit confirmation and long-text manual-paste fallback. Ctrl+Shift+T; separate action immediately after the magnifier.
- OCR now has a scan-line icon distinct from the Text tool. Preserved the user-provided application icon.
- Replaced README with English, Russian and Simplified Chinese descriptions and current screenshots. Refreshed About copy.
- Added URL policy/config/layout tests and native preview/dispatch checks. No new runtime dependency.
- Google Lens handoff is unchanged and regression-tested with intercepted Chromium requests.

---

# 1.0.0 — final feature scope

- Commit active draw/move/resize before export; undo-safe gesture completion.
- Text metric bounds, multiline/14px live text, visible caret/box, Esc commits without closing, Ctrl+C/S exports while typing.
- Multiply highlighter preserves black text; Blur/Pixelate reachable through More and B/X.
- Reuse DIB/back/capture buffers; remove dim frame; append annotation cache, clipped damage, bounded glyph cache.
- Reuse NV12/quantizer scratch; direct BGRA conversion; cache MF factories; continuous sample durations with lookahead and final flush.
- Common Controls v6 and white labels/checkboxes; vector OCR/QR icons.
- User-provided eight-resolution ICO embedded unchanged, old generated tiger artwork removed.
- Extract native dialogs; shared text/damage/clock modules; typed explicit native toolbar; stateless legacy dock compatibility.
- Correct product/manifest/User-Agent version. Preserve old settings aliases and security properties.
- 85 tests on Linux and Windows test EXEs under Wine plus GUI/GIF/QR/intercepted-browser checks. Native Windows acceptance limits documented.

---
## Historical changes

# 0.8.0
- Redrawn broad-faced snarling tiger with rounded ears, clear stripes, narrowed eyes and long fangs.
- QR result inside selected region near QR action. URL click opens via default browser and closes editor on successful system dispatch; copy button keeps editor open. Row omitted in too-small regions; UI row never exported.
- PrintScreen shortcut recording handles release-only events and temporarily intercepts Snapshot while the shortcut field has focus; no capture/OS screenshot action during recording, hook removed on blur/destroy.
- 70 automated cases plus synthetic native GUI acceptance under Wine; native Windows hardware remains unverified.

# 0.7.0
- QR URL text opens directly; separate overlapping-pages button copies, with green confirmation and editor left open. Plain text copies directly.
- Original snarling tiger icon in EXE, inherited shortcuts, windows and tray; optical 16/20px and standard 24–256px artwork.
- Larger 22px region-size badge with 14px pixel digits.
- Capture shortcut records physical key combinations; Tab/Escape navigation and old registered shortcut suppression while editing; atomic save/conflict rollback retained.
- Nonblocking pending HTTP clients remove preconnect head-of-line delays; direct RGBA JPEG encoding; speed-optimized release; native browser base64 decoding when supported.
- 68 Linux tests and Windows test EXEs under Wine; intercepted browser upload and synthetic native GUI validation. Native Windows/live Google acceptance remains open.

# 0.6.0
- Restored one-response inline Google handoff and original standard URL dispatch; no second local image fetch.
- Dedicated QR button and inline result/copy without editor close; OCR is direct again.
- Default/first pencil; removed selection button, added eight-way region border resizing; Move beside Picker, Lens first.
- Per-tool icon coloring, smaller size badge, Edge naming, removed screen-side docking commands.
- Configurable global capture shortcut with atomic conflict rollback; optional recorded system cursor.
- Local tests and acceptance limits documented in QA/STATUS. Older entries below are historical, not current UI specifications.

# 0.5.0 — selection-attached toolbars

- Removed minimal grid. Added selection-attached Lightshot drawing rail + bottom action bar and refined two-row Compact.
- Single persistent per-tool color picker, fixed opaque redaction, explicit four-arrow move and eyedropper icons.
- Pixel size badge at top-left selection edge; no persistent status banner/QR spacing.
- OCR popup includes QR; Ctrl+T and Ctrl+Q keep direct actions.
- Native PIDL/COM saved-file reveal + Explorer fallback + last-saved menu action.
- Browser activation dispatched from GUI/COM; concurrent encoding, TCP_NODELAY, JPEG/PNG search modes, non-content diagnostics.
- Added attachment/hitbox/edge/color/search-encoding tests and real-browser intercepted JPEG/PNG multipart regression.
- Still Windows preview. No new claim of native MP4 acceptance, protected Telegram bypass, successful live hosting or native performance certification.

---

# 0.4.0
- Replace labelled toolbar with Compact / Minimal icon layouts; add pin action, header dragging and edge docking.
- Explicit QR click copies cached result immediately; preserve results across toolbar clicks; selectable text, SysLink and Open link for single safe HTTP(S) results.
- Click saved-image/video notification to open output folder; uploaded-link notification opens the URL.
- Checked PrintWindow toggle in menus, persists to settings; no guaranteed Telegram bypass.
- Top-down NV12 input fixes the explicit RGB row inversion path in MP4; native Windows end-to-end acceptance pending.
- Faster Google handoff: final-image cache reuse, worker preparation, binary PNG instead of Base64, event-driven Windows socket wait and accepted-socket race fix.
- Layout overlap, color-plane and orientation regression tests.

# 0.3.0
- Native offscreen composition + one present per paint, no startup balloon.
- RU/EN, standard/compact layout, clearer recording and upload actions.
- UTF-16 WM_CHAR surrogate handling; no replacement-character source strings.
- Optional silent notifications; default Google handoff without extra confirmation.
- Local QR quiet-zone/inverse/rescale attempts, visible QR:N result.
- Real GIF live-region recording; native Media Foundation MP4 backend (Windows acceptance pending).
- Select Litterbox / Catbox / Imgur; explicit final-selection PNG multipart upload.
- Experimental cooperative PrintWindow (not guaranteed Telegram bypass).
- New Unicode/host/GIF tests and truthful open-acceptance documentation.
