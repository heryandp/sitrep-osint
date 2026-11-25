use reader_service_rs::port::service::{ReaderService, ReaderServiceConfig};
use serde_json::json;

#[tokio::test]
async fn test_ssrf_and_public_url_validation() {
    // Valid public URLs
    assert!(ReaderService::is_public_url("https://example.com/news/123"));
    assert!(ReaderService::is_public_url("http://tempo.co/read/12345"));
    assert!(ReaderService::is_public_url(
        "https://antaranews.com/berita/1"
    ));

    // Invalid / SSRF / private targets
    assert!(!ReaderService::is_public_url("http://localhost/admin"));
    assert!(!ReaderService::is_public_url(
        "http://127.0.0.1:8080/secret"
    ));
    assert!(!ReaderService::is_public_url("http://10.0.0.1/internal"));
    assert!(!ReaderService::is_public_url("http://192.168.1.1/router"));
    assert!(!ReaderService::is_public_url("http://172.16.0.5/api"));
    assert!(!ReaderService::is_public_url(
        "http://169.254.169.254/latest/meta-data/"
    ));
    assert!(!ReaderService::is_public_url("ftp://example.com/file"));
    assert!(!ReaderService::is_public_url("file:///etc/passwd"));
    assert!(!ReaderService::is_public_url(
        "https://user:pass@example.com/news"
    ));
    assert!(!ReaderService::is_public_url("not a url"));
}

#[tokio::test]
async fn test_waf_host_detection_and_learning() {
    let svc = ReaderService::new(ReaderServiceConfig::default());

    // Well-known WAF hosts
    assert!(svc.is_waf_host_url("https://tempo.co/read/123"));
    assert!(svc.is_waf_host_url("https://cnnindonesia.com/nasional/456"));
    assert!(
        svc.is_waf_host_url("https://tribratanews-resmesuji.lampung.polri.go.id/detail-post/123")
    );
    assert!(!svc.is_waf_host_url("https://example-non-waf.id/berita/1"));

    // Learned WAF host
    assert!(svc.mark_waf_host("https://example-non-waf.id/berita/1"));
    assert!(svc.is_waf_host_url("https://example-non-waf.id/berita/1"));
    assert!(svc.is_waf_host_url("https://www.example-non-waf.id/another-path"));
}

#[tokio::test]
async fn test_youtube_url_detection() {
    assert_eq!(
        ReaderService::yt_id_of("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
        Some("dQw4w9WgXcQ".to_string())
    );
    assert_eq!(
        ReaderService::yt_id_of("https://youtu.be/dQw4w9WgXcQ"),
        Some("dQw4w9WgXcQ".to_string())
    );
    assert_eq!(
        ReaderService::yt_id_of("https://youtube.com/shorts/dQw4w9WgXcQ"),
        Some("dQw4w9WgXcQ".to_string())
    );
    assert_eq!(
        ReaderService::yt_id_of("https://youtube.com/embed/dQw4w9WgXcQ"),
        Some("dQw4w9WgXcQ".to_string())
    );
    assert_eq!(
        ReaderService::yt_id_of("https://example.com/not-youtube"),
        None
    );
}

#[tokio::test]
async fn test_synthetic_head_injection() {
    let raw_html = "<html><body><article><p>Berita utama.</p></article></body></html>";
    let metadata = json!({
        "title": "Judul Penting Berita",
        "description": "Deskripsi singkat dari berita yang dilaporkan.",
        "ogImage": "https://example.com/img.jpg"
    });

    let injected = ReaderService::inject_synthetic_head(raw_html, &metadata);
    assert!(injected.contains("<title>Judul Penting Berita</title>"));
    assert!(injected.contains(r#"<meta property="og:title" content="Judul Penting Berita">"#));
    assert!(injected.contains(r#"<meta property="og:description" content="Deskripsi singkat dari berita yang dilaporkan.">"#));
    assert!(
        injected.contains(r#"<meta property="og:image" content="https://example.com/img.jpg">"#)
    );
}

#[tokio::test]
async fn test_firecrawl_cooldown_logic() {
    let svc = ReaderService::new(ReaderServiceConfig {
        firecrawl_api_key: Some("test-key".to_string()),
        ..Default::default()
    });

    assert!(svc.is_fc_enabled());
    assert!(!svc.fc_blocked("https://example.com/article"));

    // Simulating quota exhaustion (HTTP 402/401)
    svc.fc_mark_quota_exhausted();
    assert!(!svc.is_fc_enabled());
    assert!(svc.fc_blocked("https://example.com/article"));
    assert!(svc.fc_blocked("https://another.com/article"));
}

#[tokio::test]
async fn test_image_proxy_unauthorized_host() {
    let svc = ReaderService::new(ReaderServiceConfig::default());

    // Disallowed host should be rejected
    let res = svc.img("https://malicious.example.com/exploit.png").await;
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Host tidak diizinkan"));

    // Non-https should be rejected
    let res = svc.img("http://fbcdn.net/image.jpg").await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_create_reader_service_destructure_checks() {
    // Calling create_reader_service with null gives TypeError like JS
    let err_null = reader_service_rs::port::create_reader_service(serde_json::Value::Null);
    assert_eq!(
        err_null.unwrap_err(),
        "Cannot destructure property 'CFG' of 'object null' as it is null."
    );

    // Calling with empty object gives missing SEC
    let err_empty = reader_service_rs::port::create_reader_service(json!({}));
    assert_eq!(
        err_empty.unwrap_err(),
        "Cannot destructure property 'publicURL' of 'SEC' as it is undefined."
    );

    // Valid object
    let ok = reader_service_rs::port::create_reader_service(json!({
        "CFG": {},
        "SEC": { "publicURL": true }
    }));
    assert!(ok.is_ok());
}
