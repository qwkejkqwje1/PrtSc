//! Bounded HTTPS POST. Default TLS validation, no redirects or automatic retries.
use super::*;
use luma_shot::translate;
use windows_sys::Win32::Networking::WinHttp::*;
struct Http(*mut std::ffi::c_void);
impl Drop for Http {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                WinHttpCloseHandle(self.0);
            }
        }
    }
}
pub(super) fn translate(text: &str, target: &str) -> Result<String, String> {
    let body = translate::request_body(text, target).map_err(str::to_string)?;
    unsafe {
        let session = Http(WinHttpOpen(
            wide(concat!("PrtSc/", env!("CARGO_PKG_VERSION"))).as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            null(),
            null(),
            0,
        ));
        if session.0.is_null() {
            return Err("Cannot initialize translation connection; use the browser.".into());
        }
        WinHttpSetTimeouts(session.0, 5000, 5000, 8000, 8000);
        let conn = Http(WinHttpConnect(
            session.0,
            wide("translate.googleapis.com").as_ptr(),
            443,
            0,
        ));
        if conn.0.is_null() {
            return Err("Cannot connect to Google; use the browser.".into());
        }
        let request = Http(WinHttpOpenRequest(
            conn.0,
            wide("POST").as_ptr(),
            wide("/translate_a/single").as_ptr(),
            null(),
            null(),
            null(),
            WINHTTP_FLAG_SECURE,
        ));
        if request.0.is_null() {
            return Err("Cannot create translation request; use the browser.".into());
        }
        let policy = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
        if WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_REDIRECT_POLICY,
            &policy as *const _ as _,
            std::mem::size_of_val(&policy) as u32,
        ) == 0
        {
            return Err("Cannot apply translation redirect policy".into());
        }
        let headers=wide("Content-Type: application/x-www-form-urlencoded; charset=UTF-8\r\nAccept: application/json\r\n");
        if WinHttpSendRequest(
            request.0,
            headers.as_ptr(),
            headers.len() as u32 - 1,
            body.as_ptr() as _,
            body.len() as u32,
            body.len() as u32,
            0,
        ) == 0
            || WinHttpReceiveResponse(request.0, null_mut()) == 0
        {
            return Err("Google translation is unavailable or timed out. Original text is preserved; use the browser.".into());
        }
        let (mut status, mut n) = (0u32, 4u32);
        if WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            null(),
            &mut status as *mut _ as _,
            &mut n,
            null_mut(),
        ) == 0
        {
            return Err("Cannot read Google response status".into());
        }
        if status != 200 {
            return Err(match status {
            403|429=>format!("Google limited this unofficial service (HTTP {status}). No retries; use the browser."),
            _=>format!("Google translation returned HTTP {status}. Original text is preserved; use the browser.")
        });
        }
        let started = Instant::now();
        let mut response = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            if started.elapsed() > std::time::Duration::from_secs(20) {
                return Err("Translation response timed out; use the browser.".into());
            }
            let mut n = 0;
            if WinHttpReadData(request.0, buf.as_mut_ptr() as _, buf.len() as u32, &mut n) == 0 {
                return Err("Cannot read Google translation; use the browser.".into());
            }
            if n == 0 {
                break;
            }
            if response.len() + n as usize > translate::MAX_RESPONSE_BYTES {
                return Err("Translation response exceeds limit".into());
            }
            response.extend_from_slice(&buf[..n as usize]);
        }
        translate::parse_response(&response).map_err(str::to_string)
    }
}
