use bytes::Bytes;
use cctv_gateway_rs::port::service::{
    CctvGateway, ConcurrencyLane, GatewayOptions, LruCache, RumEvent, TokenBucket,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[tokio::test]
async fn test_concurrency_lane_limits() {
    let lane = Arc::new(ConcurrencyLane::new(3, 2, 10));
    assert_eq!(lane.max_active(), 3);

    let cur = Arc::new(AtomicUsize::new(0));
    let max_cur = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for i in 0..6 {
        let l = Arc::clone(&lane);
        let c = Arc::clone(&cur);
        let mc = Arc::clone(&max_cur);
        let host = if i % 2 == 0 { "host-a" } else { "host-b" };
        handles.push(tokio::spawn(async move {
            let _ = l
                .run(host, async {
                    let prev = c.fetch_add(1, Ordering::SeqCst);
                    let now = prev + 1;
                    mc.fetch_max(now, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    c.fetch_sub(1, Ordering::SeqCst);
                })
                .await;
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    assert!(max_cur.load(Ordering::SeqCst) <= 3);
    assert_eq!(lane.stats().active, 0);
}

#[test]
fn test_token_bucket_rate_limiter() {
    let mut b = TokenBucket::new(2.0, 4.0);
    let mut t = 1000u64;

    for _ in 0..4 {
        assert!(b.take("1.2.3.4", 1.0, t));
    }
    // Exhausted
    assert!(!b.take("1.2.3.4", 1.0, t));

    // After 1 second (1000 ms), 2 tokens added
    t += 1000;
    assert!(b.take("1.2.3.4", 1.0, t));
    assert!(b.take("1.2.3.4", 1.0, t));
    assert!(!b.take("1.2.3.4", 1.0, t));

    // Charging bytes
    b.charge("1.2.3.4", 100.0, t);
    assert!(!b.ok("1.2.3.4", t));
}

#[test]
fn test_lru_cache_byte_cap_and_ttl() {
    let mut c = LruCache::<i32>::new(100);
    let mut t = 1000u64;

    c.set("a".to_string(), 1, 60, 1000, t);
    c.set("b".to_string(), 2, 60, 1000, t);

    // "a" must be evicted because 60 + 60 > 100
    assert_eq!(c.get("a", t), None);
    assert_eq!(c.get("b", t), Some(2));
    assert_eq!(c.stats().bytes, 60);

    // TTL expiry after 2000 ms
    t += 2000;
    assert_eq!(c.get("b", t), None);
    assert_eq!(c.stats().bytes, 0);
}

#[test]
fn test_canonical_key_generation() {
    // pantaucctv with volatile nonce query param t
    let p1 = "https://restreamer.pantaucctv.com/cam1/index.m3u8?t=1000";
    let p2 = "https://restreamer.pantaucctv.com/cam1/index.m3u8?t=2000";
    assert_eq!(
        CctvGateway::canonical_key(p1),
        CctvGateway::canonical_key(p2)
    );
    assert!(
        CctvGateway::canonical_key(p1).starts_with("pc:restreamer.pantaucctv.com/cam1/index.m3u8")
    );

    // molecool
    let m1 = "https://cctv.molecool.id/slug-a/index.m3u8?token=tok1";
    let m2 = "https://cctv.molecool.id/slug-a/index.m3u8?token=tok2";
    assert_eq!(
        CctvGateway::canonical_key(m1),
        CctvGateway::canonical_key(m2)
    );
    assert_eq!(CctvGateway::canonical_key(m1), "mc:/slug-a/index.m3u8");

    // dpuprkotang
    let k1 = "https://cctv.dpuprkotang.info/api/stream.mp4?src=Sarakan&t=100";
    let k2 = "https://cctv.dpuprkotang.info/api/frame.jpeg?src=Sarakan&t=200";
    assert_ne!(
        CctvGateway::canonical_key(k1),
        CctvGateway::canonical_key(k2)
    );
    assert_eq!(CctvGateway::canonical_key(k1), "kt:/api/stream.mp4:sarakan");
    assert_eq!(CctvGateway::canonical_key(k2), "kt:/api/frame.jpeg:sarakan");

    // tangerangkab isolates per ULID
    let tk1 = "https://cctv-dishub.tangerangkab.go.id/storage/video/01aaa/thumbnail.jpg";
    let tk2 = "https://cctv-dishub.tangerangkab.go.id/storage/video/01bbb/thumbnail.jpg";
    assert_eq!(CctvGateway::canonical_key(tk1), "tk:01aaa");
    assert_eq!(CctvGateway::canonical_key(tk2), "tk:01bbb");
    assert_ne!(
        CctvGateway::canonical_key(tk1),
        CctvGateway::canonical_key(tk2)
    );

    // baliprov isolates per CCTV ID
    let bp1 = "https://baliprov.go.id/cctv/cam01/index.m3u8";
    let bp2 = "https://baliprov.go.id/cctv/cam02/index.m3u8";
    assert_eq!(CctvGateway::canonical_key(bp1), "bali:cam01");
    assert_eq!(CctvGateway::canonical_key(bp2), "bali:cam02");
}

#[test]
fn test_resolve_direct_snap_url() {
    let u1 = "https://cctv.dpuprkotang.info/api/stream.mp4?src=Sarakan&t=100&v=1";
    assert_eq!(
        CctvGateway::resolve_direct_snap_url(u1),
        Some("https://cctv.dpuprkotang.info/api/frame.jpeg?src=Sarakan".to_string())
    );

    let u2 = "https://cam.example/preview.jpeg?t=99";
    assert_eq!(
        CctvGateway::resolve_direct_snap_url(u2),
        Some("https://cam.example/preview.jpeg".to_string())
    );

    let u3 = "https://dsdajakarta.id/live/cam.jpg";
    assert_eq!(
        CctvGateway::resolve_direct_snap_url(u3),
        Some("https://dsdajakarta.id/live/cam.jpg".to_string())
    );

    let u4 = "https://cam.example/live.m3u8";
    assert_eq!(CctvGateway::resolve_direct_snap_url(u4), None);
}

#[test]
fn test_dead_cam_reporting() {
    let gw = CctvGateway::new(GatewayOptions::default());
    let ok_url = "https://cam.example/ok.m3u8";

    // Foreign URL rejected
    assert!(
        !gw.report("https://evil.example/cam", false, "1.1.1.1", |u| u
            == ok_url)
    );

    // 1 report not enough
    assert!(gw.report(ok_url, false, "1.1.1.1", |u| u == ok_url));
    assert!(gw.report(ok_url, false, "1.1.1.1", |u| u == ok_url));
    assert!(!gw.reported_dead(ok_url));

    // Second distinct IP triggers dead
    assert!(gw.report(ok_url, false, "2.2.2.2", |u| u == ok_url));
    assert!(gw.reported_dead(ok_url));

    // Recovery report clears dead status
    assert!(gw.report(ok_url, true, "3.3.3.3", |u| u == ok_url));
    assert!(!gw.reported_dead(ok_url));
}

#[test]
fn test_rum_telemetry_and_quantiles() {
    let gw = CctvGateway::new(GatewayOptions::default());
    let ev = vec![
        RumEvent {
            h: "a.example".to_string(),
            t: Some(1000.0),
            f: None,
            s: None,
        },
        RumEvent {
            h: "a.example".to_string(),
            t: Some(3000.0),
            f: None,
            s: None,
        },
        RumEvent {
            h: "a.example".to_string(),
            t: Some(9000.0),
            f: None,
            s: None,
        },
        RumEvent {
            h: "a.example".to_string(),
            t: None,
            f: Some(true),
            s: None,
        },
        RumEvent {
            h: "<script>".to_string(),
            t: Some(5.0),
            f: None,
            s: None,
        },
        RumEvent {
            h: "a.example".to_string(),
            t: Some(-1.0),
            f: None,
            s: None,
        },
        RumEvent {
            h: "a.example".to_string(),
            t: None,
            f: None,
            s: Some(2.0),
        },
    ];

    let ingested = gw.rum_ingest(&ev, "9.9.9.9", |h| h == "a.example");
    assert_eq!(ingested, 6);

    let stats = gw.rum_stats();
    let a = stats.get("a.example").expect("host a.example present");
    assert_eq!(a.ok, 3);
    assert_eq!(a.fail, 1);
    assert_eq!(a.stalls, 2);
    assert_eq!(a.ttff_p50, Some(3000));
    assert_eq!(a.ttff_p95, Some(9000));
}

#[tokio::test]
async fn test_upstream_coalescing() {
    let gw = Arc::new(CctvGateway::new(GatewayOptions::default()));
    let upstream_calls = Arc::new(AtomicUsize::new(0));

    let key = "https://cam.example/stream.m3u8";
    let mut handles = Vec::new();

    for _ in 0..20 {
        let gw_clone = Arc::clone(&gw);
        let calls = Arc::clone(&upstream_calls);
        handles.push(tokio::spawn(async move {
            gw_clone
                .coalesced_fetch(key, || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    Ok(Bytes::from_static(b"#EXTM3U\nseg1.ts\n"))
                })
                .await
        }));
    }

    for h in handles {
        let res = h.await.expect("task completed");
        assert_eq!(res, Ok(Bytes::from_static(b"#EXTM3U\nseg1.ts\n")));
    }

    assert_eq!(upstream_calls.load(Ordering::SeqCst), 1);
    assert_eq!(gw.stats().upstream, 1);
    assert_eq!(gw.stats().coalesced, 19);
}

#[test]
fn test_rewrite_playlist() {
    let pl =
        "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:4.0,\nseg1.ts\n";
    let furl = "https://cam.example/live/master.m3u8";
    let px = |u: &str| format!("/hls?u={}", u);

    let rewritten = CctvGateway::rewrite_playlist(pl, furl, px);
    assert!(rewritten.contains(r#"#EXT-X-MAP:URI="/hls?u=https://cam.example/live/init.mp4""#));
    assert!(rewritten.contains("/hls?u=https://cam.example/live/seg1.ts"));
}

#[test]
fn test_media_content_type_sanitization() {
    assert_eq!(
        CctvGateway::sanitize_media_ct("text/plain", "https://x.example/test.ts"),
        "video/mp2t"
    );
    assert_eq!(
        CctvGateway::sanitize_media_ct("application/octet-stream", "https://x.example/chunk.m4s"),
        "video/iso.segment"
    );
    assert_eq!(
        CctvGateway::sanitize_media_ct(
            "text/html; charset=utf-8",
            "https://x.example/malicious.bin"
        ),
        "application/octet-stream"
    );
}

#[test]
fn test_mjpeg_frame_extraction() {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"--boundary\r\nContent-Type: image/jpeg\r\n\r\n");
    let jpeg_start = payload.len();
    payload.extend_from_slice(&[0xff, 0xd8]);
    payload.resize(jpeg_start + 1200, 0x42);
    payload.extend_from_slice(&[0xff, 0xd9]);
    payload.extend_from_slice(b"\r\n--boundary\r\n");

    let frame = CctvGateway::mjpeg_extract_frame(&payload);
    assert!(frame.is_some());
    let f = frame.unwrap();
    assert_eq!(&f[0..2], &[0xff, 0xd8]);
    assert_eq!(&f[f.len() - 2..], &[0xff, 0xd9]);
    assert!(f.len() >= 1024);
}
