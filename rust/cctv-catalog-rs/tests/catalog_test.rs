use cctv_catalog_rs::port::service::{
    CamItem, CctvCatalog, CctvCatalogOptions, dedupe, id_region, sanitize_fb, stream_key,
};
use futures_util::FutureExt;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_id_region_lookup() {
    assert_eq!(id_region(-6.21, 106.85), "Kota Jakarta");
    assert_eq!(id_region(-7.8, 110.36), "Kota Yogyakarta");
    assert_eq!(id_region(-6.24, 106.99), "Kota Bekasi");
    assert_eq!(id_region(-8.65, 115.21), "Kota Denpasar");
    assert_eq!(id_region(-8.5, 117.5), "Nusa Tenggara");
    assert_eq!(id_region(0.0, 0.0), "Lainnya");
}

#[test]
fn test_stream_keying_and_dedupe() {
    let k1 = stream_key("https://pantaucctv.com/api/stream?id=CAM01&foo=bar");
    assert_eq!(k1, "pc:cam01");

    let k2 = stream_key("https://dpuprkotang.info/live?src=streamA");
    assert_eq!(k2, "kt:streama");

    let k3 = stream_key("https://cctv.molecool.id/slug-abc/index.m3u8?token=xyz");
    assert_eq!(k3, "mc:slug-abc");

    let k4 = stream_key("https://example.com/hls/cam1/index.m3u8");
    let k5 = stream_key("https://example.com/hls/cam1/mono.m3u8");
    assert_eq!(k4, k5);

    let cams = vec![
        CamItem::new("Cam 1", "https://example.com/hls/cam1/index.m3u8"),
        CamItem::new("Cam 1 Dupe", "https://example.com/hls/cam1/mono.m3u8"),
        CamItem::new("Cam 2", "https://example.com/hls/cam2/index.m3u8"),
    ];

    let deduped = dedupe(&cams);
    assert_eq!(deduped.len(), 2);
    assert_eq!(deduped[0].n, "Cam 1");
    assert_eq!(deduped[1].n, "Cam 2");
}

#[test]
fn test_sanitize_fallback_streams() {
    let mut cam = CamItem::new(
        "Simpang Thamrin",
        "https://example.com/stream/thamrin/index.m3u8",
    );
    cam.fb = Some(json!([
        "https://example.com/stream/thamrin/mono.m3u8", // matches own stream key
        "https://other.com/streams/simpangthamrin_backup.m3u8", // contains normalized name
        "https://random.com/irrelevant_cam.m3u8"        // should be stripped
    ]));

    let cleaned = sanitize_fb(cam);
    let fbs = cleaned.fb.expect("fallback exists");
    let arr = fbs.as_array().expect("array of fallbacks");
    assert_eq!(arr.len(), 2);
    assert!(arr[0].as_str().unwrap().contains("thamrin"));
    assert!(arr[1].as_str().unwrap().contains("simpangthamrin"));
}

#[tokio::test]
async fn test_cctv_catalog_cold_start_memo_and_persistence() {
    let tmp_dir = std::env::temp_dir().join(format!(
        "sitrep-cctv-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&tmp_dir);
    let bpjt_hits = Arc::new(AtomicUsize::new(0));
    let bekasi_hits = Arc::new(AtomicUsize::new(0));
    let jogja_hits = Arc::new(AtomicUsize::new(0));

    let bpjt_h = Arc::clone(&bpjt_hits);
    let bekasi_h = Arc::clone(&bekasi_hits);
    let jogja_h = Arc::clone(&jogja_hits);

    let fetch_text = Arc::new(move |url: &str, _ms: u64| {
        let u = url.to_string();
        let bpjt_h = Arc::clone(&bpjt_h);
        let bekasi_h = Arc::clone(&bekasi_h);
        let jogja_h = Arc::clone(&jogja_h);

        async move {
            if u.contains("bpjt.pu.go.id") {
                bpjt_h.fetch_add(1, Ordering::SeqCst);
                Ok("const allStreams = {\"ruas\":[{\"streamhls\":\"https://bpjt.pu.go.id/cam1/index.m3u8\",\"status\":\"online\",\"lat\":-6.21,\"lon\":106.85,\"nama_km\":\"KM 10\",\"nama_ruas\":\"Jagorawi\"}]};\n".to_string())
            } else if u.contains("bekasikota.go.id") {
                bekasi_h.fetch_add(1, Ordering::SeqCst);
                Ok("[{\"name\":\"Simpang_Kayuringin\",\"path\":\"kayuringin/index.m3u8\"}]".to_string())
            } else if u.contains("jogjakota.go.id") {
                jogja_h.fetch_add(1, Ordering::SeqCst);
                Ok("[{\"cctv_title\":\"Malioboro\",\"cctv_link\":\"https://cctv.jogjakota.go.id/malio/index.m3u8\",\"cctv_latitude\":-7.79,\"cctv_longitude\":110.36}]".to_string())
            } else {
                Ok("".to_string())
            }
        }.boxed()
    });

    let opts = CctvCatalogOptions {
        hist_dir: Some(tmp_dir.clone()),
        persist_hist: true,
        user_agent: "sitrep-test/1.0".to_string(),
        fetch_text: Some(fetch_text),
        probe_url: None,
        reported_dead: Some(Arc::new(|_| false)),
        detect_ffmpeg: Some(Arc::new(|| async { false }.boxed())),
        is_draining: Some(Arc::new(|| false)),
        cam_list: Some(Arc::new(Vec::new)),
    };

    let cat1 = CctvCatalog::new(opts);
    let r1 = cat1.catalog().await;
    assert!(r1.ok);
    assert!(
        r1.cams.iter().any(|c| c.s.contains("bpjt.pu.go.id")),
        "cold start must include BPJT"
    );
    assert!(
        r1.cams.iter().any(|c| c.s.contains("jogjakota.go.id")),
        "cold start must include Jogja"
    );
    assert!(
        r1.cams.iter().any(|c| c.s.contains("bekasikota.go.id")),
        "cold start must include Bekasi"
    );

    let prev_bekasi = bekasi_hits.load(Ordering::SeqCst);
    let prev_jogja = jogja_hits.load(Ordering::SeqCst);

    // Call catalog again - memoized sources should not be refetched
    let _r1_repeat = cat1.catalog().await;
    assert_eq!(
        bekasi_hits.load(Ordering::SeqCst),
        prev_bekasi,
        "bekasi must be memoized"
    );
    assert_eq!(
        jogja_hits.load(Ordering::SeqCst),
        prev_jogja,
        "jogja must be memoized"
    );

    // Persist to history
    cat1.save_cctv_hist(true).expect("save history file");
    assert!(
        tmp_dir.join("cctv-history.json").exists(),
        "history file must exist"
    );

    // Second catalog boot with failing upstream
    let failing_fetch = Arc::new(|_url: &str, _ms: u64| {
        async move { Err("Upstream unreachable".to_string()) }.boxed()
    });

    let opts2 = CctvCatalogOptions {
        hist_dir: Some(tmp_dir.clone()),
        persist_hist: true,
        user_agent: "sitrep-test/1.0".to_string(),
        fetch_text: Some(failing_fetch),
        probe_url: None,
        reported_dead: Some(Arc::new(|_| false)),
        detect_ffmpeg: Some(Arc::new(|| async { false }.boxed())),
        is_draining: Some(Arc::new(|| false)),
        cam_list: Some(Arc::new(Vec::new)),
    };

    let cat2 = CctvCatalog::new(opts2);
    let r2 = cat2.catalog().await;
    assert!(r2.ok);
    assert_eq!(
        r2.cams.len(),
        r1.cams.len(),
        "second boot with failing upstream must fallback to persisted history"
    );

    let _ = std::fs::remove_dir_all(&tmp_dir);
}
