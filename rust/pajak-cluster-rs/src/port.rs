//! The Rust port of `src/pajak-cluster.js`.

#![forbid(unsafe_code)]

mod pinned;
#[path = "service.rs"]
pub mod service;

use serde_json::Value;

/// Port of `isTaxText` (`src/pajak-cluster.js:88`).
pub fn is_tax_text(text: Value) -> Result<Value, String> {
    let s = service::js_string_or_empty(&text);
    Ok(Value::Bool(service::is_tax_text_str(&s)))
}

/// Port of `classifyTaxScope` (`src/pajak-cluster.js:94`).
pub fn classify_tax_scope(text: Value) -> Result<Value, String> {
    let s = service::js_string_or_empty(&text);
    Ok(Value::String(service::classify_tax_scope_str(&s).into()))
}

/// Port of `classifyPajakThemes` (`src/pajak-cluster.js:108`).
pub fn classify_pajak_themes(text: Value) -> Result<Value, String> {
    let s = service::js_string_or_empty(&text);
    Ok(service::classify_pajak_themes_str(&s).to_value())
}

/// Port of `buildPajakClusters` (`src/pajak-cluster.js:263`).
pub fn build_pajak_clusters(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() {
        return Err("Cannot read properties of null (reading 'newsItems')".into());
    }
    let obj = if crate::js::is_undefined(&arg0) {
        None
    } else {
        arg0.as_object()
    };
    let get = |k: &str| {
        obj.and_then(|m| m.get(k))
            .filter(|v| !crate::js::is_undefined(v))
    };

    let news_items: &[Value] = get("newsItems")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let social_posts: &[Value] = get("socialPosts")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let now = match get("now") {
        Some(v) => crate::js::as_number(v).unwrap_or_else(|| {
            if v.is_null() {
                0.0
            } else {
                v.as_f64().unwrap_or(f64::NAN)
            }
        }),
        None => pinned::now_ms(),
    };
    let win = get("win").and_then(Value::as_str).unwrap_or("24h");
    let theme = get("theme").and_then(Value::as_str).unwrap_or("");
    let scope = get("scope").and_then(Value::as_str).unwrap_or("");

    Ok(service::build_pajak_clusters_value(
        news_items,
        social_posts,
        now,
        win,
        theme,
        scope,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exposes_exact_6_dimensions_in_required_order() {
        let labels: Vec<&str> = service::PAJAK_THEMES.iter().map(|t| t.label).collect();
        assert_eq!(
            labels,
            vec![
                "Ideologi",
                "Politik",
                "Sosial dan Budaya",
                "Ekonomi",
                "Hukum dan HAM",
                "Teknologi",
            ]
        );
    }

    #[test]
    fn classifies_representative_indonesian_tax_items_into_6_dimensions() {
        let samples = [
            (
                "Seruan boikot pajak dan debat keadilan fiskal Pasal 23A untuk rakyat",
                "ideologi",
            ),
            (
                "Komisi XI DPR dan Menkeu bahas PMK 71/2026 serta RUU Badan Penerimaan Negara",
                "politik",
            ),
            (
                "Kanwil DJP Kalselteng gandeng kampus bentuk Tax Center dan perkuat literasi pajak UMKM",
                "sosbud",
            ),
            (
                "DJP Kepri: Sektor industri topang penerimaan pajak hingga 56% dan target tax ratio APBN",
                "ekonomi",
            ),
            (
                "PPATK temukan dugaan pidana pajak dan faktur fiktif Rp122,9 triliun, DJP jelaskan sita aset",
                "hukum",
            ),
            (
                "Coretax DJP error gagal login OTP dan e-Faktur tidak bisa unggah faktur pajak hari ini",
                "teknologi",
            ),
        ];
        for (text, expected) in samples {
            let cls = service::classify_pajak_themes_str(text);
            assert_eq!(cls.primary, expected, "text: {text}");
        }
    }

    #[test]
    fn clusters_cross_source_coretax_outage_as_cross_verified_and_kritis() {
        let now = 1_700_000_000_000.0;
        let res = build_pajak_clusters(json!({
            "now": now,
            "win": "24h",
            "newsItems": [
                {
                    "title": "Wajib Pajak Keluhkan Coretax Error Gagal Login dan e-Faktur di Jakarta",
                    "desc": "Gangguan akses login Coretax dan penerbitan e-Faktur terjadi sejak pagi.",
                    "url": "https://example.com/coretax-error-1",
                    "pub": "DDTCNews",
                    "cat": "pajak",
                    "ts": now - 10.0 * 60_000.0
                },
                {
                    "title": "DJP Kepri Catat Penerimaan Pajak Industri Tembus 56 Persen",
                    "desc": "Sektor manufaktur mendominasi setoran pajak daerah.",
                    "url": "https://example.com/kepri-pajak",
                    "pub": "ANTARA Kepri",
                    "cat": "pajak",
                    "ts": now - 30.0 * 60_000.0
                },
                {
                    "title": "Pajak jadian makan bareng temen habis nonton drakor",
                    "url": "https://example.com/noise",
                    "pub": "Spam",
                    "ts": now - 5.0 * 60_000.0
                }
            ],
            "socialPosts": [
                {
                    "text": "Coretax error lagi hari ini di Jakarta, gagal login terus saat mau buat e-Faktur",
                    "url": "https://threads.net/@wp_jkt/post/1",
                    "username": "wp_jkt",
                    "from": "Threads",
                    "ts": now - 8.0 * 60_000.0,
                    "assessment": { "topics": ["coretax"] }
                }
            ]
        }))
        .unwrap();

        assert_eq!(res["ok"], true);
        assert_eq!(res["totalItems"], 3);
        assert_eq!(res["totalNews"], 2);
        assert_eq!(res["totalSocial"], 1);
        assert_eq!(res["themes"].as_array().unwrap().len(), 6);

        let clusters = res["clusters"].as_array().unwrap();
        let tech = clusters
            .iter()
            .find(|c| c["theme"] == "teknologi")
            .expect("teknologi cluster");
        assert_eq!(tech["crossVerified"], true);
        assert_eq!(tech["urgency"], "kritis");
        assert_eq!(tech["newsCount"], 1);
        assert_eq!(tech["socialCount"], 1);
        assert!(!tech["place"].as_str().unwrap_or("").is_empty());
    }

    #[test]
    fn separates_nasional_vs_daerah_and_prioritizes_nasional() {
        assert_eq!(
            service::classify_tax_scope_str("DJP Kepri: Penerimaan PPh dan PPN naik 12%"),
            "nasional"
        );
        assert_eq!(
            service::classify_tax_scope_str("Coretax error saat unggah e-Faktur di KPP Pratama"),
            "nasional"
        );
        assert_eq!(
            service::classify_tax_scope_str(
                "Pemprov DKI Beri Keringanan Pokok Pajak Kendaraan Bermotor di Samsat"
            ),
            "daerah"
        );
        assert_eq!(
            service::classify_tax_scope_str(
                "Bapenda Kejar Target PAD dari PBB-P2 dan Pajak Restoran PBJT"
            ),
            "daerah"
        );

        let now = 1_700_000_000_000.0;
        let news = json!([
            {
                "title": "Pemprov DKI Beri Pemutihan Pajak Kendaraan Bermotor Samsat",
                "url": "https://a/1",
                "pub": "MediaA",
                "cat": "pajak",
                "ts": now - 1000.0
            },
            {
                "title": "DJP Raup Penerimaan Pajak Rp898 Miliar dari Program PSA",
                "url": "https://a/2",
                "pub": "DDTCNews",
                "cat": "pajak",
                "ts": now - 2000.0
            }
        ]);
        let both = build_pajak_clusters(json!({ "now": now, "newsItems": news })).unwrap();
        assert_eq!(both["nasionalClusters"], 1);
        assert_eq!(both["daerahClusters"], 1);
        assert_eq!(both["clusters"][0]["taxScope"], "nasional");

        let nas_only =
            build_pajak_clusters(json!({ "now": now, "scope": "nasional", "newsItems": news }))
                .unwrap();
        assert_eq!(nas_only["totalClusters"], 1);
        assert_eq!(nas_only["clusters"][0]["taxScope"], "nasional");
    }
}
