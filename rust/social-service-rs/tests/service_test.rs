use serde_json::json;
use social_service_rs::port::create_social_service;
use social_service_rs::port::service::{SocialService, XAccount, XPool};

#[test]
fn test_create_social_service_validation() {
    // null input
    let err = create_social_service(serde_json::Value::Null).unwrap_err();
    assert!(err.contains("Cannot destructure property 'fs'"));

    // invalid / missing HISTORY
    let err = create_social_service(json!({})).unwrap_err();
    assert!(err.contains("reading 'keyOf'"));

    let err = create_social_service(json!(123)).unwrap_err();
    assert!(err.contains("reading 'keyOf'"));

    // valid input with HISTORY
    let res = create_social_service(json!({
        "HISTORY": { "keyOf": "some_key" }
    }))
    .unwrap();
    assert_eq!(res.get("status").and_then(|v| v.as_str()), Some("ok"));
}

#[test]
fn test_xpool_rotation_and_cooldown() {
    let acc1 = XAccount {
        id: 1,
        auth_token: "tok1".to_string(),
        ct0: "ct0_1".to_string(),
        cooldown_until: 0,
        disabled_until: 0,
    };
    let acc2 = XAccount {
        id: 2,
        auth_token: "tok2".to_string(),
        ct0: "ct0_2".to_string(),
        cooldown_until: 0,
        disabled_until: 0,
    };

    let mut pool = XPool::new(vec![acc1, acc2]);
    assert_eq!(pool.size(), 2);
    assert!(pool.has());

    let exclude = std::collections::HashSet::new();
    let picked1 = pool.pick(1000, &exclude).unwrap();
    assert_eq!(picked1.id, 1);

    let picked2 = pool.pick(1000, &exclude).unwrap();
    assert_eq!(picked2.id, 2);

    let picked3 = pool.pick(1000, &exclude).unwrap();
    assert_eq!(picked3.id, 1);

    // mark acc1 rate limited
    pool.mark_rate_limit(1, 5000);
    assert!(!pool.all_cooling(1000));

    let picked = pool.pick(1000, &exclude).unwrap();
    assert_eq!(picked.id, 2);

    // mark acc2 auth error
    pool.mark_auth_error(2, 1000);
    assert!(pool.all_cooling(1000));
    assert!(pool.pick(1000, &exclude).is_none());

    // advance time past cooldown
    assert!(!pool.all_cooling(6000));
    let picked_after = pool.pick(6000, &exclude).unwrap();
    assert_eq!(picked_after.id, 1);
}

#[test]
fn test_astroturf_filtering() {
    let spam_text = "PEMBERITAHUAN PENTING: Segera bayar pajak kendaraan Anda sebelum denda menumpuk di kantor samsat setempat seluruh Indonesia! Informasi lebih lanjut hubungi call center.";
    let mut posts = vec![
        json!({
            "id": "1",
            "handle": "@user1",
            "text": spam_text,
            "ts": 1000000,
        }),
        json!({
            "id": "2",
            "handle": "@user2",
            "text": spam_text,
            "ts": 1000001,
        }),
        json!({
            "id": "3",
            "handle": "@user3",
            "text": spam_text,
            "ts": 1000002,
        }),
        json!({
            "id": "4",
            "handle": "@ditjenpajakri",
            "text": spam_text,
            "ts": 1000003,
        }),
    ];

    SocialService::filter_astroturf(&mut posts);

    // user1, user2, user3 should be eliminated as astroturf spam (>=3 users),
    // but @ditjenpajakri is in AUTH_ACCOUNTS so it must be preserved!
    assert_eq!(posts.len(), 1);
    assert_eq!(
        posts[0].get("handle").and_then(|v| v.as_str()),
        Some("@ditjenpajakri")
    );
}

#[tokio::test]
async fn test_threads_worker_execution() {
    let svc = SocialService::new();
    let res = svc.threads_scrape(&[], &[], None).await;
    assert!(res.is_ok(), "threads_worker failed: {:?}", res);
    let val = res.unwrap();
    assert_eq!(val.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        val.get("posts").and_then(|v| v.as_array()).map(|a| a.len()),
        Some(0)
    );
}

#[test]
fn test_social_diagnostics_and_explain() {
    let svc = SocialService::new();

    let diag = svc.api_social_diagnostics().unwrap();
    assert_eq!(diag.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        diag.get("classifierVersion").and_then(|v| v.as_str()),
        Some("2026-11-23.1")
    );

    let exp = svc
        .api_social_explain("kantor pelayanan pajak pratama jakarta buka layanan")
        .unwrap();
    assert_eq!(exp.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(exp.get("explanation").is_some());

    let xdiag = svc.api_xdiag().unwrap();
    assert_eq!(xdiag.get("host").and_then(|v| v.as_str()), Some("local"));
}

#[test]
fn test_history_merge_and_deduplication() {
    let svc = SocialService::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let p1 = json!({
        "url": "https://x.com/pajak/status/123",
        "handle": "@pajak",
        "text": "Lapor SPT tahunan tepat waktu",
        "ts": now - 1000,
    });
    let p2 = json!({
        "url": "https://x.com/pajak/status/123",
        "handle": "@pajak",
        "text": "Lapor SPT tahunan tepat waktu (update)",
        "ts": now - 500,
    });

    svc.hist_merge("x", &[p1, p2]);

    let feed = svc.api_x().unwrap();
    let posts = feed.get("posts").and_then(|v| v.as_array()).unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(
        posts[0].get("text").and_then(|v| v.as_str()),
        Some("Lapor SPT tahunan tepat waktu (update)")
    );
}
