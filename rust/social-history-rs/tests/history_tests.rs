use serde_json::json;
use social_history_rs::port::{key_of, service::SocialHistory, snapshot, tag, ts_of};

#[test]
fn test_key_of_url_and_code() {
    let post_url = json!({
        "url": "https://x.com/user/status/12345",
        "text": "Banjir bandang",
    });
    assert_eq!(
        key_of(post_url).unwrap(),
        json!("https://x.com/user/status/12345")
    );

    let post_code = json!({
        "code": "TH12345",
        "text": "Gempa bumi",
    });
    assert_eq!(key_of(post_code).unwrap(), json!("TH12345"));
}

#[test]
fn test_key_of_hash_distinct_posts() {
    let text = "Banjir melanda Jakarta";
    let p1 = json!({ "text": text, "created_at": "2026-11-01T01:00:00Z", "author": "alice" });
    let p2 = json!({ "text": text, "created_at": "2026-11-02T02:00:00Z", "author": "bob" });
    let k1 = key_of(p1).unwrap();
    let k2 = key_of(p2).unwrap();
    assert_ne!(k1, k2);
    assert!(k1.as_str().unwrap().starts_with(text));
    assert!(k2.as_str().unwrap().starts_with(text));
}

#[test]
fn test_key_of_null() {
    assert!(key_of(json!(null)).is_err());
}

#[test]
fn test_ts_of() {
    assert_eq!(
        ts_of(json!({ "ts": 1700000000000_i64 })).unwrap(),
        json!(1700000000000.0)
    );
    assert_eq!(
        ts_of(json!({ "created_at": "2026-11-01T01:00:00Z" })).unwrap(),
        json!(1793494800000.0)
    );
    assert_eq!(ts_of(json!({})).unwrap(), json!(0));
    assert!(ts_of(json!(null)).is_err());
}

#[test]
fn test_tag_age_and_filter() {
    let now = 1795572224629_f64;
    let old_post = json!({
        "text": "Banjir di Jakarta",
        "ts": now - 100_000_000.0,
    });
    assert_eq!(
        tag(old_post, None, Some(json!(now)), None).unwrap(),
        json!(false)
    );

    let future_post = json!({
        "text": "Banjir di Jakarta",
        "ts": now + 10_000.0,
    });
    assert_eq!(
        tag(future_post, None, Some(json!(now)), None).unwrap(),
        json!(false)
    );
}

#[test]
fn test_tag_handle_filter() {
    let now = 1795572224629_f64;
    let post = json!({
        "text": "Berita terkini banjir kilat",
        "handle": "@malaysiakini",
        "ts": now - 1000.0,
    });
    assert_eq!(
        tag(post, None, Some(json!(now)), None).unwrap(),
        json!(false)
    );
}

#[test]
fn test_tag_valid_incident() {
    let now = 1795572224629_f64;
    let post = json!({
        "url": "https://x.com/infobmkg/status/101",
        "handle": "@infobmkg",
        "user": "infobmkg",
        "text": "BMKG keluarkan peringatan hujan ekstrem disertai angin kencang untuk wilayah Jakarta Selatan",
        "created_at": "2026-11-25T01:48:44.164Z",
        "ts": 1795571324164_i64,
    });
    let meta = json!({ "recordDiagnostics": true });
    assert_eq!(
        tag(post, Some(json!([])), Some(json!(now)), Some(meta)).unwrap(),
        json!(true)
    );
}

#[test]
fn test_snapshot_ordering() {
    let p1 = json!({ "id": 1, "ts": 1000.0 });
    let p2 = json!({ "id": 2, "ts": 3000.0 });
    let p3 = json!({ "id": 3, "ts": 2000.0 });
    let map = json!([p1, p2, p3]);

    let snap = snapshot(map, Some(json!(5000.0))).unwrap();
    assert_eq!(snap["ok"], true);
    assert_eq!(snap["snapshotAt"], 5000.0);
    let posts = snap["posts"].as_array().unwrap();
    assert_eq!(posts[0]["id"], 2);
    assert_eq!(posts[1]["id"], 3);
    assert_eq!(posts[2]["id"], 1);
}

#[tokio::test]
async fn test_social_history_service_ring_buffer() {
    let history = SocialHistory::with_capacity(3);
    assert_eq!(history.len().await, 0);

    for i in 1..=5 {
        let post = json!({
            "url": format!("https://x.com/status/{i}"),
            "text": format!("Laporan banjir ke-{i} di Jakarta"),
            "ts": 1700000000000_i64 + i * 1000,
        });
        let res = history.insert(post).await;
        assert!(res.is_ok());
    }

    assert_eq!(history.len().await, 3);
    assert!(!history.has("https://x.com/status/1").await);
    assert!(!history.has("https://x.com/status/2").await);
    assert!(history.has("https://x.com/status/3").await);
    assert!(history.has("https://x.com/status/4").await);
    assert!(history.has("https://x.com/status/5").await);

    let snap = history.snapshot(Some(1700000010000.0)).await;
    let posts = snap["posts"].as_array().unwrap();
    assert_eq!(posts.len(), 3);
    assert_eq!(posts[0]["url"], "https://x.com/status/5");
    assert_eq!(posts[1]["url"], "https://x.com/status/4");
    assert_eq!(posts[2]["url"], "https://x.com/status/3");
}

#[tokio::test]
async fn test_social_history_service_concurrency() {
    use std::sync::Arc;
    let history = Arc::new(SocialHistory::with_capacity(100));

    let mut handles = Vec::new();
    for i in 0..10 {
        let h = Arc::clone(&history);
        handles.push(tokio::spawn(async move {
            for j in 0..10 {
                let id = i * 10 + j;
                let post = json!({
                    "url": format!("https://x.com/user/status/{id}"),
                    "text": format!("Tweet kejadian ke-{id}"),
                    "ts": 1700000000000_i64 + id,
                });
                h.insert(post).await.unwrap();
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(history.len().await, 100);
}
