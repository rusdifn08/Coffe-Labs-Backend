use coffe_labs_backend::modules::barista_toolkit::services::object_storage_stream::{
    mime_for_key, parse_byte_range, validate_object_key,
};

#[test]
fn test_object_storage_mime_types_for_hls_and_mp4() {
    assert_eq!(mime_for_key("coffeelabs/lms/bab1/sub_1_1_bcc.m3u8"), "application/vnd.apple.mpegurl");
    assert_eq!(mime_for_key("coffeelabs/lms/bab1/sub_1_1_bcc_000.ts"), "video/mp2t");
    assert_eq!(mime_for_key("coffeelabs/lms/bab1/sub_1_1_bcc.mp4"), "video/mp4");
    assert_eq!(mime_for_key("coffeelabs/lms/bab1/sub_1_1_bcc.vtt"), "text/vtt; charset=utf-8");
    assert_eq!(mime_for_key("coffeelabs/lms/bab1/sub_1_1_bcc_poster.png"), "image/png");
}

#[test]
fn test_http_range_header_parser() {
    assert_eq!(parse_byte_range("bytes=0-1023", 10_000), Some((0, 1023)));
    assert_eq!(parse_byte_range("bytes=500-", 10_000), Some((500, 9999)));
    assert_eq!(parse_byte_range("bytes=-500", 10_000), Some((9500, 9999)));
    assert_eq!(parse_byte_range("bytes=9000-20000", 10_000), Some((9000, 9999)));
    assert_eq!(parse_byte_range("bytes=10000-10500", 10_000), None);
    assert_eq!(parse_byte_range("bytes=500-100", 10_000), None);
    assert_eq!(parse_byte_range("invalid", 10_000), None);
}

#[test]
fn test_object_key_path_traversal_blocked() {
    assert!(validate_object_key("coffeelabs/lms/bab1/sub_1_1_bcc.m3u8").is_ok());
    assert!(validate_object_key("../.env").is_err());
    assert!(validate_object_key("coffeelabs\\lms\\sub.mp4").is_err());
    assert!(validate_object_key("").is_err());
}
