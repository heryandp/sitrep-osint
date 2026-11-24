use serde_json::json;
use tomtom_rs::port::service::{LatLon, TomTomService};
use tomtom_rs::port::{haversine, round_grid, round_grid_str, tt_host_ok};

#[test]
fn test_haversine_known_distances() {
    // Jakarta (-6.2088, 106.8456) to Surabaya (-7.2575, 112.7521)
    let a = json!({ "lat": -6.2088, "lon": 106.8456 });
    let b = json!({ "lat": -7.2575, "lon": 112.7521 });
    let res = haversine(a, b).unwrap();
    let km = res.as_f64().unwrap();
    assert!((km - 662.57).abs() < 0.1);

    // Identical points should have 0 distance
    let p0 = json!({ "lat": 0.0, "lon": 0.0 });
    let res0 = haversine(p0.clone(), p0).unwrap();
    assert_eq!(res0.as_f64().unwrap(), 0.0);
}

#[test]
fn test_haversine_error_cases() {
    assert!(haversine(json!(null), json!({ "lat": 1.0, "lon": 1.0 })).is_err());
    assert!(haversine(json!({ "lat": 1.0, "lon": 1.0 }), json!(null)).is_err());
}

#[test]
fn test_round_grid() {
    let r1 = round_grid(json!(-6.2088), json!(0.01)).unwrap();
    assert_eq!(r1.as_f64().unwrap(), -6.21);

    let r2 = round_grid(json!(106.8456), json!(0.05)).unwrap();
    assert!((r2.as_f64().unwrap() - 106.85).abs() < 1e-6);
}

#[test]
fn test_round_grid_str() {
    let s1 = round_grid_str(json!(-6.2088), json!(0.01), Some(2.0)).unwrap();
    assert_eq!(s1.as_str().unwrap(), "-6.21");

    let s2 = round_grid_str(json!(106.8456), json!(0.05), Some(4.0)).unwrap();
    assert_eq!(s2.as_str().unwrap(), "106.8500");

    let s_default = round_grid_str(json!(1.0), json!(1.0), None).unwrap();
    assert_eq!(s_default.as_str().unwrap(), "1.00");

    // Out of range should fail
    assert!(round_grid_str(json!(1.0), json!(1.0), Some(-1.0)).is_err());
    assert!(round_grid_str(json!(1.0), json!(1.0), Some(101.0)).is_err());
    assert!(round_grid_str(json!(1.0), json!(1.0), Some(f64::INFINITY)).is_err());
}

#[test]
fn test_tt_host_ok() {
    assert!(
        tt_host_ok(json!("https://api.tomtom.com/traffic/services/4/flow"))
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        tt_host_ok(json!("https://a.api.tomtom.com/tile"))
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        !tt_host_ok(json!("https://evil.com/fake"))
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(!tt_host_ok(json!("not a url")).unwrap().as_bool().unwrap());
    assert!(!tt_host_ok(json!(null)).unwrap().as_bool().unwrap());
}

#[tokio::test]
async fn test_tomtom_service_no_key() {
    let svc = TomTomService::new(None);
    assert!(!svc.tt_has());

    let flow = svc.flow(-6.2088, 106.8456, Some(10.0)).await;
    assert_eq!(flow["ok"], false);
    assert!(
        flow["err"]
            .as_str()
            .unwrap()
            .contains("belum dikonfigurasi")
    );

    let inc = svc.incidents("106.8,-6.2,107.0,-6.0", None).await;
    assert_eq!(inc["ok"], false);

    let route = svc
        .route(
            LatLon {
                lat: -6.2,
                lon: 106.8,
            },
            LatLon {
                lat: -6.3,
                lon: 106.9,
            },
            None,
        )
        .await;
    assert_eq!(route["ok"], false);

    let tile_invalid_coords = svc.tile("flow", "22", "0", "0", None).await;
    assert_eq!(tile_invalid_coords.status, 400);

    let tile_no_key = svc.tile("flow", "10", "100", "100", None).await;
    assert_eq!(tile_no_key.status, 503);
}

#[tokio::test]
async fn test_tomtom_service_tile_cache_limits() {
    let svc = TomTomService::new(Some("dummy_key".into()));
    assert!(svc.tt_has());
    assert_eq!(svc.tile_count().await, 0);
    assert_eq!(svc.tile_bytes().await, 0);
}
