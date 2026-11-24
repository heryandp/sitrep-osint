use std::collections::HashSet;

use serde_json::json;
use xpool_rs::port::{XAccount, XAccountStatus, XPool, build_x_accounts, format_wib_time};

#[test]
fn build_x_accounts_parses_and_deduplicates() {
    let accs = XAccount::collect(
        Some(&json!({
            "X_AUTH_TOKEN": "tok1, tok2",
            "X_CT0": "ct1, ct2",
            "X_AUTH_TOKEN_2": "tok2",
            "X_CT0_2": "ct2",
            "X_AUTH_TOKEN_3": "tok3",
            "X_CT0_3": "ct3",
            "X_AUTH_TOKEN_4": "missing_ct0"
        })),
        Some(&json!({
            "accounts": [
                { "auth_token": "tok4", "ct0": "ct4" },
                { "auth_token": "tok1", "ct0": "ct1" }
            ]
        })),
    )
    .expect("should parse accounts");

    assert_eq!(accs.len(), 4);
    assert_eq!(
        accs.iter()
            .map(|a| a.auth_token.as_str())
            .collect::<Vec<_>>(),
        vec!["tok1", "tok2", "tok3", "tok4"]
    );
    assert_eq!(
        accs.iter().map(|a| a.ct0.as_str()).collect::<Vec<_>>(),
        vec!["ct1", "ct2", "ct3", "ct4"]
    );

    let val = build_x_accounts(json!({ "X_AUTH_TOKEN": "t1", "X_CT0": "c1" }), json!({}))
        .expect("json value");
    assert_eq!(val[0]["id"], 1);
    assert_eq!(val[0]["cooldownUntil"], 0);
    assert_eq!(val[0]["disabledUntil"], 0);
}

#[test]
fn xpool_round_robin_and_rate_limit_failover() {
    let accs = XAccount::collect(
        Some(&json!({
            "X_AUTH_TOKEN": "a1", "X_CT0": "c1",
            "X_AUTH_TOKEN_2": "a2", "X_CT0_2": "c2"
        })),
        None,
    )
    .expect("accounts");
    let mut pool = XPool::new(accs);
    let now = 1_700_000_000_000_i64;

    assert_eq!(pool.size(), 2);
    assert!(!pool.is_empty());
    assert!(pool.has());
    assert!(!pool.all_cooling(now));

    let p1 = pool.pick(now, None).expect("p1").clone();
    let p2 = pool.pick(now, None).expect("p2").clone();
    let p3 = pool.pick(now, None).expect("p3").clone();
    assert_eq!(p1.auth_token, "a1");
    assert_eq!(p2.auth_token, "a2");
    assert_eq!(p3.auth_token, "a1");

    pool.mark_rate_limit(p1.id, now + 600_000, now);
    assert!(!pool.all_cooling(now));
    assert_eq!(pool.pick(now, None).expect("p2a").auth_token, "a2");
    assert_eq!(pool.pick(now, None).expect("p2b").auth_token, "a2");

    let exclude = HashSet::from([p2.id]);
    assert!(pool.pick(now, Some(&exclude)).is_none());

    pool.mark_rate_limit(p2.id, now + 120_000, now);
    assert!(pool.all_cooling(now));
    assert_eq!(pool.earliest_reset(now), now + 120_000);
    assert!(pool.pick(now, None).is_none());
    assert_eq!(
        pool.pick(now + 120_001, None)
            .expect("recovered")
            .auth_token,
        "a2"
    );
}

#[test]
fn xpool_isolates_auth_errors_and_omits_secrets_from_status() {
    let accs = XAccount::collect(
        Some(&json!({
            "X_AUTH_TOKEN": "secret-tok-1", "X_CT0": "secret-ct-1",
            "X_AUTH_TOKEN_2": "secret-tok-2", "X_CT0_2": "secret-ct-2"
        })),
        None,
    )
    .expect("accounts");
    let mut pool = XPool::new(accs);
    let now = 1_700_000_000_000_i64;

    let first = pool.pick(now, None).expect("first").clone();
    pool.mark_auth_error(first.id, now);
    assert!(!pool.all_cooling(now));
    assert_eq!(
        pool.pick(now, None).expect("second").auth_token,
        "secret-tok-2"
    );

    let st = pool.status(now);
    assert_eq!(
        st[0],
        XAccountStatus {
            id: 1,
            healthy: false,
            wait_sec: 1800
        }
    );
    assert_eq!(
        st[1],
        XAccountStatus {
            id: 2,
            healthy: true,
            wait_sec: 0
        }
    );
    let st_json = serde_json::to_string(&st).expect("serialize status");
    assert!(!st_json.contains("secret-tok"));
    assert!(!st_json.contains("secret-ct"));
}

#[test]
fn xpool_empty_and_default_rate_limit_fallback() {
    let mut empty = XPool::default();
    let now = 1_700_000_000_000_i64;
    assert_eq!(empty.size(), 0);
    assert!(empty.is_empty());
    assert!(!empty.has());
    assert!(!empty.all_cooling(now));
    assert_eq!(empty.earliest_reset(now), now);
    assert!(empty.pick(now, None).is_none());
    assert!(empty.status(now).is_empty());

    let accs = XAccount::collect(Some(&json!({ "X_AUTH_TOKEN": "a1", "X_CT0": "c1" })), None)
        .expect("accounts");
    let mut pool = XPool::new(accs);
    pool.mark_rate_limit(1, 0, now);
    assert!(pool.all_cooling(now));
    assert_eq!(pool.earliest_reset(now), now + 5 * 60_000);
    assert_eq!(pool.status(now)[0].wait_sec, 300);
}

#[test]
fn format_wib_time_deterministic_utc_plus_7() {
    assert_eq!(XPool::format_wib_time(1_794_290_040_000.0), "12:54");
    assert_eq!(XPool::format_wib_time(1_794_330_300_000.0), "00:05");
    assert_eq!(
        format_wib_time(json!(1_794_290_040_000_i64)).expect("ok"),
        json!("12:54")
    );
}
