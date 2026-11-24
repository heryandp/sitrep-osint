use exa_summary_rs::port::{
    build_fallback_summary, extract_key_entities, is_valid_public_http_url,
    parse_exa_summary_object, service::*,
};
use serde_json::json;

#[test]
fn test_is_valid_public_http_url() {
    assert_eq!(
        is_valid_public_http_url(json!("https://news.ddtc.co.id/berita/coretax")).unwrap(),
        json!(true)
    );
    assert_eq!(
        is_valid_public_http_url(json!("http://127.0.0.1:8787/api/x")).unwrap(),
        json!(false)
    );
    assert_eq!(
        is_valid_public_http_url(json!("http://localhost/secret")).unwrap(),
        json!(false)
    );
    assert_eq!(
        is_valid_public_http_url(json!("http://192.168.1.1/admin")).unwrap(),
        json!(false)
    );
    assert_eq!(
        is_valid_public_http_url(json!("file:///etc/passwd")).unwrap(),
        json!(false)
    );
    assert_eq!(
        is_valid_public_http_url(json!("javascript:alert(1)")).unwrap(),
        json!(false)
    );
    assert_eq!(
        is_valid_public_http_url(json!("https://user:pass@example.com")).unwrap(),
        json!(false)
    );
}

#[test]
fn test_extract_key_entities() {
    let text = json!(
        "DJP mencatat penerimaan Coretax tembus Rp60,1 triliun atau naik 8,5% sesuai APBN dan PMK No. 12/2024."
    );
    let res = extract_key_entities(text).unwrap();
    let arr = res.as_array().unwrap();
    assert!(arr.iter().any(|x| x.as_str() == Some("Rp60,1 triliun")));
    assert!(arr.iter().any(|x| x.as_str() == Some("8,5%")));
    assert!(arr.iter().any(|x| x.as_str() == Some("Coretax")));
    assert!(arr.iter().any(|x| x.as_str() == Some("DJP")));
    assert!(arr.iter().any(|x| x.as_str() == Some("APBN")));
    assert!(arr.iter().any(|x| x.as_str().unwrap().starts_with("PMK")));
    assert!(arr.len() <= 6);
}

#[test]
fn test_build_fallback_summary() {
    let opts = json!({
        "title": "Setoran Pajak Digital Naik",
        "desc": "Direktorat Jenderal Pajak mencatat pertumbuhan penerimaan secara stabil di awal kuartal.",
        "themeLabel": "Kepatuhan",
        "urls": ["https://news.ddtc.co.id/pajak"],
        "reason": "local_extractive",
    });
    let res = build_fallback_summary(opts).unwrap();
    assert_eq!(res["ok"], json!(true));
    assert_eq!(res["mode"], json!("fallback"));
    assert_eq!(res["fallbackReason"], json!("local_extractive"));
    assert!(res["dampak_djp"].as_str().unwrap().contains("Kepatuhan"));
    assert_eq!(
        res["sources"][0]["title"],
        json!("Setoran Pajak Digital Naik")
    );

    // Null argument should fail with JS TypeError
    let null_res = build_fallback_summary(json!(null));
    assert!(null_res.is_err());
    assert!(
        null_res
            .unwrap_err()
            .contains("Cannot read properties of null")
    );
}

#[test]
fn test_parse_exa_summary_object() {
    let raw = json!({
        "ringkasan": "Coretax dioptimalkan.",
        "dampak_djp": "Meningkatkan stabilitas.",
        "entitas_kunci": ["Coretax", "DJP"],
        "atensi": "Pantau terus.",
    });
    let parsed = parse_exa_summary_object(raw, json!("Title"), json!("Desc")).unwrap();
    assert_eq!(parsed["ringkasan"], json!("Coretax dioptimalkan."));
    assert_eq!(parsed["entitas_kunci"], json!(["Coretax", "DJP"]));

    // String JSON input
    let str_json = json!(
        "{\"ringkasan\":\"OK\",\"dampak_djp\":\"Dampak\",\"entitas_kunci\":[\"APBN\"],\"atensi\":\"Atensi\"}"
    );
    let parsed_str = parse_exa_summary_object(str_json, json!("Title"), json!("Desc")).unwrap();
    assert_eq!(parsed_str["ringkasan"], json!("OK"));
    assert_eq!(parsed_str["entitas_kunci"], json!(["APBN"]));
}

#[test]
fn test_wib_date_key_and_hash() {
    let ts = 1711238400000_i64; // arbitrary timestamp
    let key = wib_date_key(ts);
    assert_eq!(key.len(), 10);
    assert_eq!(key.chars().filter(|c| *c == '-').count(), 2);

    let h = hash_key("https://news.ddtc.co.id/coretax");
    assert_eq!(h.len(), 20);
}

#[tokio::test]
async fn test_exa_summary_service_lifecycle() {
    let tmp_dir = std::env::temp_dir().join(format!("sitrep-exa-test-{}", now_millis()));
    let cfg = ServiceConfig {
        hist_dir: tmp_dir.clone(),
        api_key: String::new(), // unconfigured
        daily_cap: 5,
        ttl_ms: 60_000,
        brief_ttl_ms: 60_000,
        persist: true,
    };

    let svc = ExaSummaryService::new(cfg);
    let status = svc.status().await;
    assert!(status.ok);
    assert!(!status.configured);
    assert_eq!(status.daily_cap, 5);
    assert_eq!(status.daily_used, 0);

    // summarize should fallback cleanly when key is missing
    let sum = svc
        .summarize(
            "https://finance.detik.com/pajak-digital",
            "Setoran Pajak Digital Tembus Rp60,1 Triliun",
            "Direktorat Jenderal Pajak mencatat penerimaan pajak usaha ekonomi digital mencapai Rp60,1 triliun hingga akhir kuartal.",
            false,
            "Penerimaan",
            Some(1711238400000),
        )
        .await
        .unwrap();

    assert_eq!(sum["ok"], json!(true));
    assert_eq!(sum["mode"], json!("fallback"));
    assert_eq!(sum["fallbackReason"], json!("no_exa_key"));
    assert_eq!(sum["exaConfigured"], json!(false));

    // executive brief fallback
    let brief = svc
        .brief("Coretax & SPT", Some(1711238400000))
        .await
        .unwrap();
    assert_eq!(brief["ok"], json!(true));
    assert_eq!(brief["mode"], json!("fallback"));
    assert_eq!(brief["fallbackReason"], json!("no_exa_key"));

    let _ = std::fs::remove_dir_all(&tmp_dir);
}
