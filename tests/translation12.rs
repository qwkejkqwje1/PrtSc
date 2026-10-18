use luma_shot::translate::{self, *};
#[test]
fn post_body_escapes_injection_and_unicode() {
    let b = String::from_utf8(request_body("Hello &tl=xx\n你好 🐱", "ru").unwrap()).unwrap();
    assert!(b.starts_with("client=gtx&sl=auto&tl=ru&dt=t&q="));
    assert_eq!(b.matches("&tl=").count(), 1);
    assert!(b.contains("%26tl%3Dxx"));
    assert!(b.contains("%E4%BD%A0"));
    assert!(!b.contains('\n'));
}
#[test]
fn no_silent_truncation_or_invalid_requests() {
    assert!(request_body("", "ru").is_err());
    assert!(request_body("a\0b", "ru").is_err());
    assert!(request_body("a", "bad&tl=xx").is_err());
    assert!(request_body(&"中".repeat(MAX_TEXT_CHARS), "en").is_ok());
    assert!(request_body(&"a".repeat(MAX_TEXT_CHARS + 1), "en").is_err());
}
#[test]
fn joins_all_translation_segments_in_order() {
    let v = br#"[[["Hello\n","A",null],["world & <tag>","B",null]],null,"ru"]"#;
    assert_eq!(parse_response(v).unwrap(), "Hello\nworld & <tag>");
    assert_eq!(
        parse_response("[[[\"你好，世界\",\"Hello\"]],null,\"en\"]".as_bytes()).unwrap(),
        "你好，世界"
    );
}
#[test]
fn unexpected_responses_are_not_partial_success() {
    for b in [
        "<html>blocked</html>",
        "{}",
        "null",
        "[]",
        "[[]]",
        "[[[null]]]",
        "[[[\"ok\"],null]]",
        "[[[\"\"]]]",
    ] {
        assert!(parse_response(b.as_bytes()).is_err(), "{b}");
    }
}
#[test]
fn oversized_response_rejected() {
    assert!(parse_response(&vec![b' '; MAX_RESPONSE_BYTES + 1]).is_err());
}
#[test]
fn image_translation_opens_images_not_lens_or_ocr() {
    for target in TARGETS {
        let u = image_url(target).unwrap();
        assert!(u.starts_with("https://translate.google.com/"));
        assert!(u.ends_with("&op=images"));
        assert!(!u.contains("text="));
        assert!(!u.contains("lens"));
    }
    assert!(image_url("ru&text=bad").is_err());
}
#[test]
fn resource_description_is_encoding_independent() {
    let rc = include_str!("../assets/windows.rc");
    let line = rc.lines().find(|l| l.contains("FileDescription")).unwrap();
    assert!(line.is_ascii());
    assert!(line.contains("PrtSc - native screenshots"));
}
#[test]
fn experimental_post_has_no_key_or_text_in_path() {
    let body = translate::request_body("private test text", "en").unwrap();
    assert!(String::from_utf8(body)
        .unwrap()
        .contains("q=private%20test%20text"));
    let code = include_str!("../src/platform/win_text_http.rs");
    assert!(code.contains("WINHTTP_OPTION_REDIRECT_POLICY_NEVER"));
    assert!(code.contains("/translate_a/single"));
    assert!(!code.contains("IGNORE_CERT"));
}
#[test]
fn stale_translation_never_replaces_new_text_or_dialog() {
    assert!(result_matches(3, 3, "source", "ru", "source", "ru"));
    assert!(!result_matches(3, 0, "source", "ru", "source", "ru"));
    assert!(!result_matches(3, 4, "source", "ru", "source", "ru"));
    assert!(!result_matches(3, 3, "source", "ru", "edited", "ru"));
    assert!(!result_matches(3, 3, "source", "ru", "source", "en"));
    assert!(!result_matches(0, 0, "source", "ru", "source", "ru"));
}
