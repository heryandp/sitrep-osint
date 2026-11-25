use market_rs::port::service::{HttpResponse, MarketService};
use market_rs::port::{cnbc_symbol_for, downsample_closes};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};

#[test]
fn test_downsample_closes() {
    let arr = json!([null, 0, -5, null, 100, 105, 110]);
    let ds = downsample_closes(arr, Some(90.0)).expect("downsample");
    assert_eq!(ds, json!([100, 105, 110]));

    let long_vec: Vec<f64> = (0..300).map(|i| 5000.0 + i as f64).collect();
    let long_arr = Value::Array(long_vec.into_iter().map(Value::from).collect());
    let ds_long = downsample_closes(long_arr, Some(60.0)).expect("downsample long");
    let arr_out = ds_long.as_array().expect("array");
    assert_eq!(arr_out.len(), 60);
    assert_eq!(arr_out[0], json!(5000));
    assert_eq!(arr_out[59], json!(5299));
}

#[test]
fn test_cnbc_symbol_for() {
    assert_eq!(
        cnbc_symbol_for(json!("IDX:COMPOSITE")).unwrap(),
        json!(".JKSE")
    );
    assert_eq!(cnbc_symbol_for(json!("^JKSE")).unwrap(), json!(".JKSE"));
    assert_eq!(
        cnbc_symbol_for(json!("IDX:BBCA")).unwrap(),
        json!("BBCA-ID")
    );
    assert_eq!(
        cnbc_symbol_for(json!("FX_IDC:USDIDR")).unwrap(),
        json!("IDR=")
    );
    assert_eq!(
        cnbc_symbol_for(json!("COMEX:ALI1!")).unwrap(),
        json!("@AL.1")
    );
    assert_eq!(cnbc_symbol_for(json!("CBOT:ZR1!")).unwrap(), json!("@RR.1"));
    assert_eq!(cnbc_symbol_for(json!("CME:DC1!")).unwrap(), json!("@DA.1"));
    assert_eq!(
        cnbc_symbol_for(json!("BINANCE:BTCUSDT")).unwrap(),
        json!("BTC.CM=")
    );
    assert_eq!(cnbc_symbol_for(json!("^GSPC")).unwrap(), json!(".SPX"));
    assert_eq!(cnbc_symbol_for(json!("000001.SS")).unwrap(), json!(".SSEC"));
}

#[tokio::test]
async fn test_service_cnbc_fallback_and_cache_retention() {
    let tmp_dir = std::env::temp_dir().join(format!("mkt-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);

    let cnbc_calls = Arc::new(AtomicUsize::new(0));
    let fail_mode = Arc::new(AtomicBool::new(false));

    let cnbc_calls_clone = cnbc_calls.clone();
    let fail_mode_clone = fail_mode.clone();

    let fetcher = Arc::new(
        move |url: String,
              _body: Option<String>,
              _hdrs: Option<HashMap<String, String>>,
              _to: u64| {
            let cnbc_calls = cnbc_calls_clone.clone();
            let fail_mode = fail_mode_clone.clone();
            Box::pin(async move {
                if url.contains("finance.yahoo.com") {
                    return Ok(HttpResponse::new(429, "Too Many Requests"));
                }
                if url.contains("ts-api.cnbc.com") {
                    cnbc_calls.fetch_add(1, Ordering::SeqCst);
                    if fail_mode.load(Ordering::SeqCst) {
                        return Ok(HttpResponse::new(503, "Service Unavailable"));
                    }
                    let body = json!({
                        "barData": {
                            "priceBars": [
                                { "close": "5010.5" },
                                { "close": "5040.2" },
                                { "close": "5088.1" },
                                { "close": "5125.7" }
                            ]
                        }
                    });
                    return Ok(HttpResponse::new(
                        200,
                        serde_json::to_string(&body).unwrap(),
                    ));
                }
                Ok(HttpResponse::new(404, "Not Found"))
            }) as market_rs::port::service::BoxFuture<Result<HttpResponse, String>>
        },
    );

    let svc = MarketService::new(&tmp_dir, "test-agent", Some(fetcher), None);

    let list = vec![
        "IDX:COMPOSITE".to_string(),
        "IDX:BBCA".to_string(),
        "COMEX:GC1!".to_string(),
    ];
    let out = svc.closes(&list).await;
    assert_eq!(out.get("ok"), Some(&json!(true)));
    let closes_map = out.get("closes").unwrap().as_object().unwrap();
    assert_eq!(
        closes_map.get("IDX:COMPOSITE").unwrap(),
        &json!([5010.5, 5040.2, 5088.1, 5125.7])
    );
    assert_eq!(
        closes_map.get("IDX:BBCA").unwrap(),
        &json!([5010.5, 5040.2, 5088.1, 5125.7])
    );
    assert_eq!(
        closes_map.get("COMEX:GC1!").unwrap(),
        &json!([5010.5, 5040.2, 5088.1, 5125.7])
    );

    // Stale valid series retention test
    {
        let mut memo = svc.close_memo.lock().await;
        if let Some(entry) = memo.get_mut("IDX:COMPOSITE") {
            entry.at = svc.now_ms() - 20 * 60_000;
        }
    }
    fail_mode.store(true, Ordering::SeqCst);
    let fallback_out = svc.fetch_symbol_closes("IDX:COMPOSITE").await;
    assert_eq!(
        fallback_out.len(),
        4,
        "Must retain stale valid series instead of poisoning with []"
    );

    // Cold test
    let cold = svc.fetch_symbol_closes("IDX:COLDTEST").await;
    assert!(cold.is_empty());
    {
        let memo = svc.close_memo.lock().await;
        let cold_meta = memo.get("IDX:COLDTEST").unwrap();
        assert_eq!(cold_meta.at, 0);
        assert!(cold_meta.fail_at > 0);
    }

    // Recover after fail_at expires
    {
        let mut memo = svc.close_memo.lock().await;
        if let Some(meta) = memo.get_mut("IDX:COLDTEST") {
            meta.fail_at = svc.now_ms() - 60_000;
        }
    }
    fail_mode.store(false, Ordering::SeqCst);
    let recovered = svc.fetch_symbol_closes("IDX:COLDTEST").await;
    assert_eq!(recovered.len(), 4);
    assert!(cnbc_calls.load(Ordering::SeqCst) >= 4);

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test]
async fn test_service_binance_fallback() {
    let tmp_dir = std::env::temp_dir().join(format!("mkt-test-bin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);

    let fetcher = Arc::new(
        move |url: String,
              _body: Option<String>,
              _hdrs: Option<HashMap<String, String>>,
              _to: u64| {
            Box::pin(async move {
                if url.contains("ts-api.cnbc.com") || url.contains("finance.yahoo.com") {
                    return Ok(HttpResponse::new(503, "Service Unavailable"));
                }
                if url.contains("api.binance.com/api/v3/klines") {
                    let klines = json!([
                        [0, "70000", "71000", "69000", "70500.5"],
                        [1, "70500", "72000", "70000", "71850.25"]
                    ]);
                    return Ok(HttpResponse::new(
                        200,
                        serde_json::to_string(&klines).unwrap(),
                    ));
                }
                Ok(HttpResponse::new(404, "Not Found"))
            }) as market_rs::port::service::BoxFuture<Result<HttpResponse, String>>
        },
    );

    let svc = MarketService::new(&tmp_dir, "test-agent", Some(fetcher), None);
    let cl = svc.fetch_symbol_closes("BINANCE:BTCUSDT").await;
    assert_eq!(cl, vec![70500.5, 71850.25]);

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test]
async fn test_service_market_and_world_index() {
    let tmp_dir = std::env::temp_dir().join(format!("mkt-test-mkt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let now = Arc::new(AtomicI64::new(1_795_564_800_000));
    let clock = now.clone();
    let gold_mode = Arc::new(AtomicUsize::new(0));
    let mode = gold_mode.clone();

    let fetcher = Arc::new(
        move |url: String,
              body: Option<String>,
              _hdrs: Option<HashMap<String, String>>,
              _to: u64| {
            let mode = mode.load(Ordering::SeqCst);
            Box::pin(async move {
                if url.contains("logam-mulia-api.iamutaki.workers.dev") {
                    let history = url.ends_with("/history?weight=1&length=10");
                    if mode == 1 || (mode == 2 && history) {
                        return Ok(HttpResponse::new(503, "Service Unavailable"));
                    }
                    let data = if history {
                        json!([
                            {"recordedDate":"2026-11-25","sellPrice":2490000,"materialType":"Antam 2026"},
                            {"recordedDate":"2026-11-25","sellPrice":999999,"materialType":"Antam 2026"},
                            {"recordedDate":"2026-11-24","sellPrice":"2485000","materialType":"Antam 2026"},
                            {"recordedDate":"2026-11-23","sellPrice":2400000,"materialType":"Antam 2025"}
                        ])
                    } else if mode == 2 {
                        json!([{"weight": 1, "sellPrice": 2500000}])
                    } else {
                        json!([
                            {"weight":1,"sellPrice":2300000,"materialType":"Antam 2025"},
                            {"weight":10,"sellPrice":24900000,"materialType":"Antam 2026"},
                            {"weight":"1","sellPrice":"2490000","buybackPrice":2200000,"recordedDate":"2026-11-25","materialType":"Antam 2026"}
                        ])
                    };
                    return Ok(HttpResponse::new(
                        200,
                        json!({"success":true,"data":data}).to_string(),
                    ));
                }
                if url.contains("finance.yahoo.com") {
                    return Ok(HttpResponse::new(429, "Too Many Requests"));
                }
                if url.contains("scanner.tradingview.com/indonesia/scan") {
                    let b = body.unwrap_or_default();
                    if b.contains("IDX:COMPOSITE") {
                        let data = json!({
                            "data": [{ "s": "IDX:COMPOSITE", "d": ["COMPOSITE", 5125.72, 1.02, 51.95, 0] }]
                        });
                        return Ok(HttpResponse::new(
                            200,
                            serde_json::to_string(&data).unwrap(),
                        ));
                    }
                    let data = json!({
                        "data": [{ "s": "IDX:BBCA", "d": ["BBCA", 5300.0, 0.95, 50.0, 1000000] }]
                    });
                    return Ok(HttpResponse::new(
                        200,
                        serde_json::to_string(&data).unwrap(),
                    ));
                }
                if url.contains("scanner.tradingview.com/forex/scan") {
                    let data = json!({
                        "data": [{ "s": "FX_IDC:USDIDR", "d": ["USDIDR", 18030.0, -0.45, -81.0, 0] }]
                    });
                    return Ok(HttpResponse::new(
                        200,
                        serde_json::to_string(&data).unwrap(),
                    ));
                }
                if url.contains("scanner.tradingview.com/futures/scan")
                    || url.contains("scanner.tradingview.com/crypto/scan")
                {
                    let data = json!({
                        "data": [{ "s": "COMEX:GC1!", "d": ["GC1!", 4042.0, 0.5, 20.0, 0] }]
                    });
                    return Ok(HttpResponse::new(
                        200,
                        serde_json::to_string(&data).unwrap(),
                    ));
                }
                if url.contains("scanner.tradingview.com/global/scan") {
                    let data = json!({
                        "data": [{ "s": "SP:SPX", "d": ["SPX", 7520.93, -0.38, -29.01, 0] }]
                    });
                    return Ok(HttpResponse::new(
                        200,
                        serde_json::to_string(&data).unwrap(),
                    ));
                }
                if url.contains("floatrates.com") {
                    let data = json!({
                        "idr": { "rate": 18030.0, "date": "2026-11-24" }
                    });
                    return Ok(HttpResponse::new(
                        200,
                        serde_json::to_string(&data).unwrap(),
                    ));
                }
                if url.contains("ts-api.cnbc.com") {
                    let data = json!({
                        "barData": {
                            "priceBars": [
                                { "close": "5050" },
                                { "close": "5090" },
                                { "close": "5125.72" }
                            ]
                        }
                    });
                    return Ok(HttpResponse::new(
                        200,
                        serde_json::to_string(&data).unwrap(),
                    ));
                }
                Ok(HttpResponse::new(404, "Not Found"))
            }) as market_rs::port::service::BoxFuture<Result<HttpResponse, String>>
        },
    );

    let svc = MarketService::new(
        &tmp_dir,
        "test-agent",
        Some(fetcher.clone()),
        Some(Arc::new(move || clock.load(Ordering::SeqCst))),
    );
    let mkt = svc.market().await.expect("market");
    assert_eq!(mkt.get("ok"), Some(&json!(true)));
    let gold = svc.gold().await;
    assert_eq!(mkt["gold"], gold);
    assert_eq!(gold["price"].as_f64(), Some(2490000.0));
    assert_eq!(gold["buyback"].as_f64(), Some(2200000.0));
    assert_eq!(gold["chg"].as_f64(), Some(5000.0));
    assert_eq!(gold["pct"], json!(0.2));
    assert_eq!(gold["closes"], json!([2485000.0, 2490000.0]));
    assert_eq!(
        gold["tiers"][0],
        json!({"weight":1.0,"sell":2490000.0,"unit":"gr","buyback":2200000.0})
    );
    assert!(
        mkt["forex"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|it| { it["sym"] == "EMAS:ANTAM" && it["price"] == gold["price"] })
    );

    gold_mode.store(2, Ordering::SeqCst);
    assert_eq!(svc.gold().await, gold, "fresh cache must skip refresh");
    now.fetch_add(15 * 60_000, Ordering::SeqCst);
    gold_mode.store(1, Ordering::SeqCst);
    assert_eq!(
        svc.gold().await,
        gold,
        "keep stale quote on upstream failure"
    );
    let cold = MarketService::new(&tmp_dir, "test-agent", Some(fetcher), None);
    assert_eq!(
        cold.gold().await,
        json!({"ok":false,"err":"gold fetch 503"})
    );
    gold_mode.store(2, Ordering::SeqCst);
    let refreshed = svc.gold().await;
    assert_eq!(refreshed["closes"], json!([2500000.0, 2500000.0]));
    assert_eq!(refreshed["tiers"], json!([{"weight":1.0,"sell":2500000.0}]));
    assert_eq!(refreshed["date"], "2026-11-25");

    let ihsg = mkt.get("ihsg").unwrap();
    assert!(
        ihsg.get("closes")
            .and_then(|c| c.as_array())
            .map(|a| a.len() >= 2)
            .unwrap_or(false),
        "market() must return ihsg.closes directly"
    );

    let world = mkt.get("world").unwrap().as_array().unwrap();
    assert!(
        !world.is_empty(),
        "market() must populate world indices via TradingView + CNBC under Yahoo 429"
    );
    assert!(world.iter().any(|w| {
        w.get("sym") == Some(&json!("^GSPC"))
            && w.get("closes")
                .and_then(|c| c.as_array())
                .map(|a| a.len() >= 2)
                .unwrap_or(false)
    }));

    let _ = std::fs::remove_dir_all(&tmp_dir);
}
