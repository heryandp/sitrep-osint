use scrypt::{Params as ScryptParams, scrypt};
use security_rs::port::service::{
    AuthConfig, AuthManager, AuthMode, RateLimiter, RateRule, RequestAuthContext, Role,
    assert_public_url_with, encode_hex, hash_password, parse_users_entries, timing_equal_str,
    verify_password_async, verify_password_js_compat, verify_password_sync,
};
use security_rs::port::{is_private_ip_str, parse_public_url, rate_key_str};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[tokio::test]
async fn scrypt_roundtrip_and_node_js_compat() {
    // Reference hash produced directly by Node.js `src/security.js` `hashPassword("admin-pass-999")`
    let node_hash = "scrypt$eab89103610a91516ebeae541ed49d85$97cf2060cdb1032219fc6a2674fbaf895f5c9e08039b48389d832c1981a134082365beccd9d097af3e1f2806332f5fea9b44da5c7b13387d5ab794c94792c6ba";
    assert!(verify_password_js_compat("admin-pass-999", node_hash));
    assert!(!verify_password_js_compat("wrong-pass", node_hash));
    assert!(verify_password_async("admin-pass-999".into(), node_hash.into()).await);

    // Rust-generated hash roundtrips cleanly and produces unique random salts
    let h1 = hash_password("pajak@DJP#2026").expect("hash_password succeeds");
    let h2 = hash_password("pajak@DJP#2026").expect("hash_password succeeds");
    assert_ne!(h1, h2, "each hash must use a fresh random salt");
    assert!(h1.starts_with("scrypt$"));
    assert_eq!(h1.split('$').count(), 3);
    assert!(verify_password_sync("pajak@DJP#2026", &h1));
    assert!(verify_password_sync("pajak@DJP#2026", &h2));
    assert!(!verify_password_sync("wrong", &h1));

    // Extended 6-part CT 104 `.env` format: `scrypt$<N>$<r>$<p>$<salt>$<hash>`
    let salt = [0x11u8; 16];
    let params = ScryptParams::new(14, 8, 1, 64).expect("valid scrypt params");
    let mut dk = [0u8; 64];
    scrypt(b"ct104-secret", &salt, &params, &mut dk).expect("scrypt derives");
    let six_part = format!("scrypt$16384$8$1${}${}", encode_hex(&salt), encode_hex(&dk));
    assert!(verify_password_sync("ct104-secret", &six_part));
    assert!(!verify_password_sync("ct104-wrong", &six_part));
    // Strict JS compat rejects 6-part format to match src/security.js:909
    assert!(!verify_password_js_compat("ct104-secret", &six_part));
}

#[test]
fn parse_ct104_users_spec() {
    let prehashed = hash_password("admin-secret").expect("hash");
    let spec = format!(
        "viewer1:viewer:view-pass, op1:operator:op-pass; admin1:admin:{prehashed}\n\
         legacy_admin:legacy-pass, bad user!:admin:x, :admin:y, empty_pw:viewer:"
    );
    let entries = parse_users_entries(&spec).expect("parse_users_entries");
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].username, "viewer1");
    assert_eq!(entries[0].role, Role::Viewer);
    assert!(verify_password_sync("view-pass", &entries[0].hash));

    assert_eq!(entries[1].username, "op1");
    assert_eq!(entries[1].role, Role::Operator);
    assert!(verify_password_sync("op-pass", &entries[1].hash));

    assert_eq!(entries[2].username, "admin1");
    assert_eq!(entries[2].role, Role::Admin);
    assert_eq!(entries[2].hash, prehashed);

    assert_eq!(entries[3].username, "legacy_admin");
    assert_eq!(entries[3].role, Role::Admin);
    assert!(verify_password_sync("legacy-pass", &entries[3].hash));
}

#[tokio::test]
async fn auth_manager_session_rbac_cookies_and_revocation() {
    let admin_hash = hash_password("admin-pass-999").expect("hash");
    let users_spec =
        format!("viewer1:viewer:view-pass,op1:operator:op-pass,admin1:admin:{admin_hash}");

    // Short secret rejected in session mode
    let short_err = AuthManager::new(AuthConfig {
        users_spec: users_spec.clone(),
        auth_secret: "too-short".into(),
        ..Default::default()
    });
    assert!(short_err.is_err());

    let auth = AuthManager::new(AuthConfig {
        api_token: Some("headless-token-xyz".into()),
        explicit_token: true,
        users_spec,
        auth_secret: "test-session-signing-secret-xyz-1234".into(),
        session_ttl_sec: 3600,
        trust_proxy: true,
        token_role: Role::Operator,
        ..Default::default()
    })
    .expect("valid AuthManager");

    assert_eq!(auth.mode(), AuthMode::Session);
    assert!(auth.user_mode());
    assert!(!auth.public_mode());

    // Wrong password rejected (and unknown user rejected without panicking)
    assert!(auth.login("viewer1", "bad", false).await.is_none());
    assert!(auth.login("ghost", "bad", false).await.is_none());

    // Successful login produces HttpOnly, SameSite=Lax, Secure (when HTTPS) cookie
    let login = auth
        .login("admin1", "admin-pass-999", true)
        .await
        .expect("admin login");
    assert_eq!(login.principal.username, "admin1");
    assert_eq!(login.principal.role, Role::Admin);
    assert!(login.set_cookie.contains("sitrep_sess="));
    assert!(login.set_cookie.contains("HttpOnly"));
    assert!(login.set_cookie.contains("SameSite=Lax"));
    assert!(login.set_cookie.contains("Secure"));

    // Authenticate via cookie
    let cookie_pair = format!("sitrep_sess={}", login.token);
    let authed = auth
        .authenticate(&RequestAuthContext {
            cookie_header: Some(&cookie_pair),
            ..Default::default()
        })
        .expect("cookie authenticates");
    assert_eq!(authed.username, "admin1");
    assert_eq!(authed.role, Role::Admin);
    assert!(auth.has_role(Some(&authed), Role::Viewer));
    assert!(auth.has_role(Some(&authed), Role::Operator));
    assert!(auth.has_role(Some(&authed), Role::Admin));

    // HTML meta injection never leaks API token in session mode
    let html = auth.inject_html_meta("<html><head><title>x</title></head></html>", Some(&authed));
    assert!(html.contains(r#"<meta name="sitrep-user" content="admin1:admin">"#));
    assert!(!html.contains("headless-token-xyz"));

    // Tampered HMAC signature rejected
    let tampered = format!("{}X", login.token);
    assert!(auth.verify_session_token(&tampered).is_none());

    // Logout revokes token & returns Max-Age=0 clearing cookie
    let clear_cookie = auth.logout(&login.token, true);
    assert!(clear_cookie.contains("Max-Age=0"));
    assert!(auth.verify_session_token(&login.token).is_none());

    // Revocation survives past 1050 revocations (audit-p0 invariant)
    let mut early = Vec::new();
    for i in 0..1050 {
        let (tok, _, _) = auth.sign_session("op1", Role::Operator);
        auth.revoke_token(&tok);
        if i < 5 {
            early.push(tok);
        }
    }
    for tok in &early {
        assert!(auth.verify_session_token(tok).is_none());
    }
    let (fresh, _, _) = auth.sign_session("op1", Role::Operator);
    let op_p = auth.verify_session_token(&fresh).expect("fresh valid");
    assert!(auth.has_role(Some(&op_p), Role::Operator));
    assert!(!auth.has_role(Some(&op_p), Role::Admin));

    // Explicit token fallback works in session mode
    let tok_p = auth
        .authenticate(&RequestAuthContext {
            token_header: Some("headless-token-xyz"),
            ..Default::default()
        })
        .expect("explicit token valid");
    assert_eq!(tok_p.role, Role::Operator);
    assert_eq!(tok_p.via, "token");
}

#[test]
fn auth_manager_public_and_token_modes() {
    let pub_auth = AuthManager::new(AuthConfig {
        public_deploy: true,
        api_token: Some("secret-tok".into()),
        explicit_token: true,
        token_role: Role::Admin,
        ..Default::default()
    })
    .expect("public auth");
    assert_eq!(pub_auth.mode(), AuthMode::Public);
    let anon = pub_auth
        .authenticate(&RequestAuthContext::default())
        .expect("public viewer");
    assert_eq!(anon.role, Role::Viewer);
    assert_eq!(anon.via, "public");
    assert!(
        pub_auth
            .inject_html_meta("<head></head>", Some(&anon))
            .contains(r#"<meta name="sitrep-public" content="1">"#)
    );

    let tok_auth = AuthManager::new(AuthConfig {
        api_token: Some("local-tok-123".into()),
        explicit_token: true,
        ..Default::default()
    })
    .expect("token auth");
    assert_eq!(tok_auth.mode(), AuthMode::Token);
    assert!(
        tok_auth
            .authenticate(&RequestAuthContext::default())
            .is_none()
    );
    let local = tok_auth
        .authenticate(&RequestAuthContext {
            token_header: Some("local-tok-123"),
            ..Default::default()
        })
        .expect("local token auth");
    assert_eq!(local.role, Role::Admin);
    assert!(timing_equal_str("abc", "abc"));
    assert!(!timing_equal_str("abc", "abd"));
}

#[tokio::test]
async fn ssrf_ipv4_ipv6_and_async_dns_blocking() {
    let private_ips = [
        "0.0.0.0",
        "10.0.0.1",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.0.0.1",
        "192.0.2.1",
        "192.168.1.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "::",
        "::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "::ffff:a9fe:a9fe",
        "::ffff:0:0:10.0.0.1",
        "64:ff9b::7f00:1",
        "64:ff9b:1::1",
        "2002:7f00:0001::",
        "2001::1",
        "2001:db8::1",
        "100::1",
        "fc00::1",
        "fd12:3456:789a::1",
        "fe80::1",
        "fec0::1",
        "ff02::1",
        "not-an-ip",
    ];
    for ip in private_ips {
        assert!(is_private_ip_str(ip), "expected private: {ip}");
    }
    for ip in [
        "8.8.8.8",
        "1.1.1.1",
        "2606:4700:4700::1111",
        "2001:4860:4860::8888",
        "2002:0808:0808::1",
        "64:ff9b::8.8.8.8",
    ] {
        assert!(!is_private_ip_str(ip), "expected public: {ip}");
    }

    for bad in [
        "http://127.0.0.1/secret",
        "http://0177.0.0.1/secret",
        "http://0x7f000001/secret",
        "http://2130706433/secret",
        "http://169.254.169.254/latest/meta-data/",
        "http://[::1]/admin",
        "http://[::ffff:7f00:1]/admin",
        "https://localhost./admin",
        "http://metadata.google.internal./computeMetadata/v1/",
        "http://user:pass@8.8.8.8/",
        "file:///etc/passwd",
    ] {
        assert!(parse_public_url(bad).is_none(), "expected reject: {bad}");
    }
    let ok_u = parse_public_url("https://example.com/berita?id=1").expect("public url");
    assert_eq!(ok_u.host_str(), Some("example.com"));

    // Async DNS resolver returning private IP must be rejected
    let priv_res = assert_public_url_with("https://evil-rebind.example/a", |_, _| async {
        Ok(vec![IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))])
    })
    .await;
    assert!(priv_res.is_err());

    // Mixed public + private IPv6 ULA must be rejected
    let mixed_res = assert_public_url_with("https://mixed.example/a", |_, _| async {
        Ok(vec![
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            IpAddr::V6("fd00::1".parse::<Ipv6Addr>().expect("v6")),
        ])
    })
    .await;
    assert!(mixed_res.is_err());

    // Public DNS result succeeds
    let ok_res = assert_public_url_with("https://public.example/a", |_, _| async {
        Ok(vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))])
    })
    .await
    .expect("public DNS allowed");
    assert_eq!(ok_res.host, "public.example");
    assert_eq!(
        ok_res.addrs,
        vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))]
    );
}

#[test]
fn rate_limiter_counters_window_reset_and_ipv6_prefix() {
    let limiter = RateLimiter::with_buckets(HashMap::from([(
        "login".into(),
        RateRule {
            limit: 3,
            window_ms: 10_000,
        },
    )]));

    // IPv6 addresses in the same /64 share one bucket
    assert_eq!(
        rate_key_str("2001:db8:85a3:1::1"),
        rate_key_str("2001:db8:85a3:1::ffff")
    );
    assert_eq!(rate_key_str("::ffff:203.0.113.5"), "203.0.113.5");

    let t0 = 1_700_000_000_000u64;
    let d1 = limiter.check_at("login", "2001:db8:85a3:1::1", false, None, t0);
    assert!(d1.allowed && d1.limit == 3 && d1.remaining == 2 && d1.retry_after_sec == 10);

    let d2 = limiter.check_at("login", "2001:db8:85a3:1::2", false, None, t0 + 1_000);
    assert!(d2.allowed && d2.remaining == 1 && d2.retry_after_sec == 9);

    let d3 = limiter.check_at("login", "2001:db8:85a3:1::3", false, None, t0 + 2_000);
    assert!(d3.allowed && d3.remaining == 0);

    let d4 = limiter.check_at("login", "2001:db8:85a3:1::4", false, None, t0 + 3_000);
    assert!(!d4.allowed && d4.remaining == 0 && d4.retry_after_sec == 7);

    // Exempt bypasses limit
    let ex = limiter.check_at("login", "2001:db8:85a3:1::4", true, None, t0 + 3_000);
    assert!(ex.allowed && ex.exempt && ex.retry_after_sec == 0);

    // After window expires, counter resets
    let d5 = limiter.check_at("login", "2001:db8:85a3:1::1", false, None, t0 + 10_000);
    assert!(d5.allowed && d5.remaining == 2);
}
