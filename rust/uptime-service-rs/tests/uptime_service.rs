use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use uptime_service_rs::port::{
    DJP_PORTALS, FetchFn, ProbeResponse, UptimeService, normalize_check_result,
};

#[test]
fn monitors_all_4_required_official_portals() {
    let domains: Vec<&str> = DJP_PORTALS.iter().map(|p| p.domain.as_ref()).collect();
    assert_eq!(
        domains,
        vec![
            "pajak.go.id",
            "coretaxdjp.pajak.go.id",
            "djponline.pajak.go.id",
            "kemenkeu.go.id",
        ]
    );
}

#[tokio::test]
async fn classifies_up_slow_down_with_caching_and_history() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_ref = calls.clone();
    let fetcher: FetchFn = Arc::new(move |url: String| {
        let calls = calls_ref.clone();
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            let dom = url.split("domain=").nth(1).unwrap_or("");
            let body = match dom {
                "pajak.go.id" => json!({
                    "success": true, "domain": "pajak.go.id", "status": "up",
                    "http_code": 200, "response_time_ms": 2731, "ip": "103.28.106.146"
                }),
                "coretaxdjp.pajak.go.id" => json!({
                    "success": true, "domain": "coretaxdjp.pajak.go.id", "status": "up",
                    "http_code": 200, "response_time_ms": 775, "ip": "43.242.132.12"
                }),
                "djponline.pajak.go.id" => json!({
                    "success": true, "domain": "djponline.pajak.go.id", "status": "down",
                    "http_code": null, "response_time_ms": null, "error": "SSL_ERROR_SYSCALL"
                }),
                "kemenkeu.go.id" => json!({
                    "success": true, "domain": "kemenkeu.go.id", "status": "up",
                    "http_code": 200, "response_time_ms": 1120, "ip": "103.165.248.88"
                }),
                _ => json!({ "status": "down" }),
            };
            Ok(ProbeResponse {
                ok: true,
                status: 200,
                body,
            })
        })
    });

    let svc = UptimeService::with_options(
        DJP_PORTALS.to_vec(),
        60_000,
        fetcher,
        Arc::new(|| 1_700_000_000_000),
    );

    let r1 = svc.check_all(false).await;
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert!(r1.ok);
    assert_eq!(r1.up_count, 3);
    assert_eq!(r1.slow_count, 1);
    assert_eq!(r1.down_count, 1);
    assert_eq!(
        r1.items.iter().find(|x| x.id == "pajak").unwrap().state,
        "slow"
    );
    assert_eq!(
        r1.items.iter().find(|x| x.id == "coretax").unwrap().state,
        "up"
    );
    assert_eq!(
        r1.items.iter().find(|x| x.id == "djponline").unwrap().state,
        "down"
    );
    assert_eq!(
        r1.items.iter().find(|x| x.id == "kemenkeu").unwrap().state,
        "up"
    );

    let r2 = svc.check_all(false).await;
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert!(r2.cached);

    let r3 = svc.check_all(true).await;
    assert_eq!(calls.load(Ordering::SeqCst), 8);
    assert!(!r3.cached);
    assert_eq!(
        r3.items
            .iter()
            .find(|x| x.id == "coretax")
            .unwrap()
            .history
            .len(),
        2
    );
}

#[tokio::test]
async fn caps_history_at_12_and_handles_probe_errors() {
    let fetcher: FetchFn = Arc::new(|url: String| {
        Box::pin(async move {
            if url.contains("pajak.go.id") && !url.contains("coretax") && !url.contains("djponline")
            {
                Ok(ProbeResponse {
                    ok: false,
                    status: 503,
                    body: Value::Null,
                })
            } else if url.contains("coretax") {
                Err("E".repeat(200))
            } else {
                Ok(ProbeResponse {
                    ok: true,
                    status: 200,
                    body: json!({
                        "status": "up",
                        "http_code": 200,
                        "response_time_ms": 120.6,
                        "checked_at": "2024-05-10T12:34:56.789Z"
                    }),
                })
            }
        })
    });
    let svc = UptimeService::with_options(
        DJP_PORTALS.to_vec(),
        90_000,
        fetcher,
        Arc::new(|| 1_700_000_000_000),
    );
    let mut last = None;
    for _ in 0..15 {
        last = Some(svc.check_all(true).await);
    }
    let res = last.unwrap();
    let pajak = res.items.iter().find(|x| x.id == "pajak").unwrap();
    assert_eq!(pajak.history.len(), 12);
    assert_eq!(pajak.probe_error, Some(true));
    assert_eq!(pajak.error.as_deref(), Some("isdownapi HTTP 503"));

    let coretax = res.items.iter().find(|x| x.id == "coretax").unwrap();
    assert_eq!(coretax.probe_error, Some(true));
    assert_eq!(coretax.error.as_ref().map(String::len), Some(120));

    let kemenkeu = res.items.iter().find(|x| x.id == "kemenkeu").unwrap();
    assert_eq!(kemenkeu.response_ms, Some(121));
    assert_eq!(kemenkeu.checked_at, 1_715_344_496_789);
}

#[tokio::test]
async fn coalesces_concurrent_in_flight_checks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_ref = calls.clone();
    let fetcher: FetchFn = Arc::new(move |_url: String| {
        let calls = calls_ref.clone();
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(25)).await;
            Ok(ProbeResponse {
                ok: true,
                status: 200,
                body: json!({ "status": "up", "http_code": 200, "response_time_ms": 100 }),
            })
        })
    });
    let svc = Arc::new(UptimeService::with_options(
        DJP_PORTALS.to_vec(),
        90_000,
        fetcher,
        Arc::new(|| 1_700_000_000_000),
    ));
    let (a, b) = tokio::join!(svc.check_all(false), svc.check_all(false));
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert_eq!(a.up_count, 4);
    assert_eq!(b.up_count, 4);
}

#[test]
fn normalizes_dates_and_truncates_errors_to_140_chars() {
    let portal = json!({
        "id": "pajak",
        "name": "Pajak.go.id",
        "domain": "pajak.go.id",
        "url": "https://www.pajak.go.id"
    });
    let res = normalize_check_result(
        portal.clone(),
        json!({
            "status": "UP",
            "http_code": "200",
            "response_time_ms": 2499.6,
            "error": "X".repeat(200),
            "checked_at": "2024-05-10T12:34:56+07:00"
        }),
        json!(1_700_000_000_000_i64),
    )
    .unwrap();
    assert_eq!(res["status"], "up");
    assert_eq!(res["state"], "slow");
    assert_eq!(res["httpCode"], 200);
    assert_eq!(res["responseMs"], 2500);
    assert_eq!(res["error"].as_str().unwrap().len(), 140);
    assert_eq!(res["checkedAt"], 1_715_319_296_000_i64);

    // Invalid date and epoch 0 fall back to `now`
    let fallback = normalize_check_result(
        portal,
        json!({ "status": "down", "checked_at": "1970-01-01T00:00:00.000Z" }),
        json!(1_700_000_000_000_i64),
    )
    .unwrap();
    assert_eq!(fallback["checkedAt"], 1_700_000_000_000_i64);
}
