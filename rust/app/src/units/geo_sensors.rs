//! Glue from Axum front door to `geo-sensors-rs`.

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use geo_sensors_rs::port::service::GeoSensors;
use serde_json::json;

static SENSORS: LazyLock<GeoSensors> = LazyLock::new(GeoSensors::new);

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn json_resp(status: StatusCode, cache_control: &'static str, body: String) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, cache_control),
            (header::REFERRER_POLICY, "same-origin"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response()
}

pub async fn handle(route: &str, _params: HashMap<String, String>, _req: Request) -> Response {
    match route {
        "GET /api/flights" => {
            let items = SENSORS.flights().await;
            let val = json!({ "ok": true, "items": items, "at": now_ms() });
            json_resp(
                StatusCode::OK,
                "public, max-age=5, s-maxage=12, stale-while-revalidate=30",
                val.to_string(),
            )
        }
        "GET /api/ships" => {
            let items = SENSORS.ships().await;
            let live = SENSORS.is_ais_live().await;
            let n = SENSORS.ship_map.read().await.len();
            let val = json!({ "ok": true, "items": items, "n": n, "live": live, "at": now_ms() });
            json_resp(
                StatusCode::OK,
                "public, max-age=5, s-maxage=12, stale-while-revalidate=30",
                val.to_string(),
            )
        }
        "GET /api/transit" => {
            let items = SENSORS.transit().await;
            let val = json!({ "ok": true, "items": items, "at": now_ms() });
            json_resp(
                StatusCode::OK,
                "public, max-age=5, s-maxage=12, stale-while-revalidate=30",
                val.to_string(),
            )
        }
        "GET /api/disaster" => {
            let data = SENSORS.disaster().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=15, s-maxage=60, stale-while-revalidate=120",
                data.to_string(),
            )
        }
        "GET /api/weather" => {
            let data = SENSORS.weather().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=120, stale-while-revalidate=300",
                data.to_string(),
            )
        }
        "GET /api/wind" => {
            let data = SENSORS.wind_grid().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=120, stale-while-revalidate=300",
                data.to_string(),
            )
        }
        "GET /api/fires" => {
            let data = SENSORS.fires().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=120, stale-while-revalidate=300",
                data.to_string(),
            )
        }
        "GET /api/sipongi" => {
            let data = SENSORS.sipongi().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=120, stale-while-revalidate=300",
                data.to_string(),
            )
        }
        "GET /api/satellite" => {
            let data = SENSORS.satellite_products().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=120, stale-while-revalidate=300",
                data.to_string(),
            )
        }
        "GET /api/bmkg-modelrun" => {
            let data = SENSORS.bmkg_model_run().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=120, stale-while-revalidate=300",
                data.to_string(),
            )
        }
        "GET /api/alerts" => {
            let data = SENSORS.alerts().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=120, stale-while-revalidate=300",
                data.to_string(),
            )
        }
        "GET /api/radar" => {
            let data = SENSORS.radar().await;
            json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=120, stale-while-revalidate=300",
                data.to_string(),
            )
        }
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
