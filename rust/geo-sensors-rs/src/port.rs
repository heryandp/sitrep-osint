//! The Rust port of `src/geo-sensors.js`.

#![forbid(unsafe_code)]

#[path = "service.rs"]
pub mod service;

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

pub static VOLCANO_COORDS: LazyLock<HashMap<&'static str, (f64, f64)>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("Anak Krakatau", (-6.10, 105.42));
    m.insert("Merapi", (-7.54, 110.44));
    m.insert("Semeru", (-8.11, 112.92));
    m.insert("Ibu", (1.49, 127.63));
    m.insert("Dukono", (1.69, 127.88));
    m.insert("Sangeang Api", (-8.20, 119.07));
    m.insert("Bromo", (-7.94, 112.95));
    m.insert("Agung", (-8.34, 115.51));
    m.insert("Sinabung", (3.17, 98.39));
    m.insert("Kerinci", (-1.70, 101.26));
    m.insert("Dempo", (-4.02, 103.13));
    m.insert("Papandayan", (-7.32, 107.73));
    m.insert("Guntur", (-7.25, 107.85));
    m.insert("Batu Tara", (-7.79, 123.58));
    m.insert("Karangetang", (2.78, 125.41));
    m.insert("Lokon", (1.37, 124.83));
    m.insert("Kelimutu", (-8.77, 121.82));
    m.insert("Egon", (-8.68, 122.45));
    m.insert("Lewotobi", (-8.54, 122.77));
    m.insert("Lewotolo", (-8.27, 123.50));
    m.insert("Paluweh", (-8.32, 121.78));
    m.insert("Ili Lewotolok", (-8.27, 123.50));
    m.insert("Sirung", (-8.49, 124.13));
    m.insert("Namlegung", (-8.07, 123.40));
    m.insert("Iya", (-8.89, 121.63));
    m.insert("Batur", (-8.24, 115.38));
    m.insert("Gamalama", (0.80, 127.33));
    m.insert("Dieng", (-7.20, 109.90));
    m.insert("Slamet", (-7.24, 109.21));
    m.insert("Lawu", (-7.63, 111.19));
    m.insert("Merbabu", (-7.45, 110.44));
    m.insert("Kelud", (-7.93, 112.31));
    m.insert("Raung", (-8.13, 114.06));
    m.insert("Argopuro", (-7.77, 113.58));
    m.insert("Ijen", (-8.06, 114.24));
    m.insert("Marapi", (-0.38, 100.47));
    m.insert("Soputan", (1.11, 124.73));
    m.insert("Ruang", (2.30, 125.37));
    m.insert("Tambora", (-8.25, 118.0));
    m.insert("Rinjani", (-8.41, 116.46));
    m.insert("Tangkuban Parahu", (-6.76, 107.61));
    m.insert("Galunggung", (-7.25, 108.06));
    m.insert("Ili Boleng", (-8.344, 123.252));
    m.insert("Ile Werung", (-8.532, 123.573));
    m.insert("Inerie", (-8.875, 120.95));
    m.insert("Awu", (3.667, 125.500));
    m.insert("Banda Api", (-4.525, 129.871));
    m.insert("Talang", (-0.978, 100.679));
    m.insert("Ciremai", (-6.892, 108.40));
    m.insert("Gede", (-6.78, 106.98));
    m.insert("Salak", (-6.716, 106.734));
    m.insert("Sumbing", (-7.385, 110.073));
    m
});

static RE_MAGMA_MARK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Level\s+(IV|III|II|I)\s*\((Awas|Siaga|Waspada|Normal)\)").expect("valid regex")
});

static RE_MAGMA_TD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<td>\s*([^<>]+?)\s*-\s*([^<>]+?)\s*<a").expect("valid regex"));

static RE_CDATA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<!\[CDATA\[([\s\S]*?)\]\]>").expect("valid regex"));

/// Port of `createGeoSensors` (`src/geo-sensors.js:86`).
pub fn create_geo_sensors(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    if arg0.is_null() {
        return Err(
            "TypeError: Cannot destructure property 'crypto' of 'object null' as it is null."
                .into(),
        );
    }
    Ok(serde_json::json!({
        "flights": { "$function": "flights" },
        "disaster": { "$function": "disaster" },
        "disCache": { "at": 0, "data": Value::Null },
        "weather": { "$function": "weather" },
        "radar": { "$function": "radar" },
        "tmaPintuAir": { "$function": "tmaPintuAir" },
        "petabencanaFloods": { "$function": "petabencanaFloods" },
        "fires": { "$function": "fires" },
        "sipongi": { "$function": "sipongi" },
        "bmkgModelRun": { "$function": "bmkgModelRun" },
        "satelliteProducts": { "$function": "satelliteProducts" },
        "windGrid": { "$function": "windGrid" },
        "alerts": { "$function": "alerts" },
        "ships": { "$function": "ships" },
        "shipMap": { "$map": [] },
        "stopAis": { "$function": "stopAis" },
        "isAisLive": { "$function": "isAisLive" },
        "transit": { "$function": "transit" }
    }))
}

/// Port of `parseMagma` (`src/geo-sensors.js:22`).
pub fn parse_magma(html: serde_json::Value) -> Result<serde_json::Value, String> {
    if html.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'matchAll')".into());
    }
    let Some(s) = html.as_str() else {
        return Err(
            "TypeError: html.matchAll is not a function or its return value is not iterable".into(),
        );
    };

    let marks: Vec<_> = RE_MAGMA_MARK.captures_iter(s).collect();
    let mut out = Vec::new();
    for (i, cap) in marks.iter().enumerate() {
        let Some(m0) = cap.get(0) else {
            continue;
        };
        let start = m0.start();
        let end = if i + 1 < marks.len() {
            marks[i + 1].get(0).map_or(s.len(), |m| m.start())
        } else {
            s.len()
        };
        let sec = &s[start..end];
        let level_num = &cap[1];
        let status = &cap[2];
        for m in RE_MAGMA_TD.captures_iter(sec) {
            let Some(n_match) = m.get(1) else {
                continue;
            };
            let Some(l_match) = m.get(2) else {
                continue;
            };
            let name = n_match.as_str().trim();
            let loc = l_match.as_str().trim();
            let coords = VOLCANO_COORDS.get(name);
            out.push(serde_json::json!({
                "name": name,
                "loc": loc,
                "level": format!("LEVEL {level_num}"),
                "status": status,
                "lat": coords.map(|c| serde_json::json!(c.0)).unwrap_or(Value::Null),
                "lon": coords.map(|c| serde_json::json!(c.1)).unwrap_or(Value::Null),
            }));
        }
    }
    Ok(Value::Array(out))
}

/// Port of `csvLine` (`src/geo-sensors.js:37`).
pub fn csv_line(line: serde_json::Value) -> Result<serde_json::Value, String> {
    if line.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'split')".into());
    }
    let Some(s) = line.as_str() else {
        return Err("TypeError: line.split is not a function".into());
    };
    let parts: Vec<Value> = s
        .split(',')
        .map(|v| Value::String(v.trim().to_string()))
        .collect();
    Ok(Value::Array(parts))
}

/// Port of `parseCsv` (`src/geo-sensors.js:41`).
pub fn parse_csv(text: serde_json::Value) -> Result<serde_json::Value, String> {
    if text.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'trim')".into());
    }
    let Some(s) = text.as_str() else {
        return Err("TypeError: text.trim is not a function".into());
    };

    let trimmed = s.trim();
    let lines: Vec<&str> = trimmed
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() < 2 {
        return Ok(serde_json::json!([]));
    }

    let head: Vec<String> = lines[0]
        .split(',')
        .map(|h| h.trim().to_lowercase())
        .collect();
    let idx = |name: &str| {
        head.iter()
            .position(|h| h == name)
            .map(|i| i as isize)
            .unwrap_or(-1)
    };
    let i_lat = idx("latitude");
    let i_lon = idx("longitude");
    let i_date = idx("acq_date");
    let i_time = idx("acq_time");
    let i_sat = idx("satellite");
    let i_instr = idx("instrument");
    let i_conf = idx("confidence");
    let i_frp = idx("frp");
    let i_day = idx("daynight");

    if i_lat < 0 || i_lon < 0 || i_date < 0 || i_time < 0 {
        return Ok(serde_json::json!([]));
    }
    let i_lat = i_lat as usize;
    let i_lon = i_lon as usize;
    let i_date = i_date as usize;
    let i_time = i_time as usize;

    let mut out = Vec::new();
    for line in &lines[1..] {
        let c: Vec<&str> = line.split(',').map(|v| v.trim()).collect();
        let lat = c.get(i_lat).and_then(|v| v.parse::<f64>().ok());
        let lon = c.get(i_lon).and_then(|v| v.parse::<f64>().ok());
        let (lat, lon) = match (lat, lon) {
            (Some(lat), Some(lon)) if lat.is_finite() && lon.is_finite() => (lat, lon),
            _ => continue,
        };

        let raw_time = c.get(i_time).copied().unwrap_or("0");
        let raw_time = if raw_time.is_empty() { "0" } else { raw_time };
        let hhmm = if raw_time.len() < 4 {
            format!("{:0>4}", raw_time)
        } else {
            raw_time.to_string()
        };
        let acq_date = c.get(i_date).copied().unwrap_or("");
        let iso = format!("{acq_date}T{}:{}:00Z", &hhmm[0..2], &hhmm[2..4]);
        let ts = chrono::DateTime::parse_from_rfc3339(&iso)
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0);
        let sat = if i_sat >= 0 {
            c.get(i_sat as usize).copied().unwrap_or("")
        } else {
            ""
        };
        let instr = if i_instr >= 0 {
            c.get(i_instr as usize).copied().unwrap_or("")
        } else {
            ""
        };
        let conf = if i_conf >= 0 {
            c.get(i_conf as usize).copied().unwrap_or("")
        } else {
            ""
        };
        let frp_val = if i_frp >= 0 {
            c.get(i_frp as usize)
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v != 0.0)
        } else {
            None
        };
        let day = if i_day >= 0 {
            c.get(i_day as usize).copied().unwrap_or("")
        } else {
            ""
        };

        out.push(serde_json::json!({
            "lat": lat,
            "lon": lon,
            "time": ts,
            "acqDate": acq_date,
            "acqTime": hhmm,
            "satellite": sat,
            "instrument": instr,
            "confidence": conf,
            "frp": frp_val.map(|f| serde_json::json!(f)).unwrap_or(Value::Null),
            "daynight": day,
        }));
    }
    Ok(Value::Array(out))
}

fn js_to_cap_tag(tag: &Value) -> String {
    match tag {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a.iter().map(js_to_cap_tag).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// Port of `parseCapField` (`src/geo-sensors.js:64`).
pub fn parse_cap_field(
    xml: serde_json::Value,
    tag: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if xml.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'match')".into());
    }
    let Some(xml_str) = xml.as_str() else {
        return Err("TypeError: xml.match is not a function".into());
    };

    let tag_str = js_to_cap_tag(&tag);
    let pattern = format!(r"(?is)<{tag_str}[^>]*>(.*?)</{tag_str}>");
    let Ok(re) = Regex::new(&pattern) else {
        return Ok(Value::String(String::new()));
    };

    if let Some(m) = re.captures(xml_str) {
        let content = m.get(1).map_or("", |c| c.as_str());
        let replaced = RE_CDATA.replace_all(content, "$1");
        Ok(Value::String(replaced.trim().to_string()))
    } else {
        Ok(Value::String(String::new()))
    }
}

/// Port of `parsePolygon` (`src/geo-sensors.js:69`).
pub fn parse_polygon(text: serde_json::Value) -> Result<serde_json::Value, String> {
    if text.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'trim')".into());
    }
    let Some(s) = text.as_str() else {
        return Err("TypeError: text.trim is not a function".into());
    };

    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(serde_json::json!([]));
    }

    let mut pts = Vec::new();
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    for t in tokens {
        let parts: Vec<&str> = t.split(',').collect();
        if parts.len() == 2
            && let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
            && a.is_finite()
            && b.is_finite()
        {
            pts.push(serde_json::json!([a, b]));
        }
    }

    if pts.is_empty() {
        let nums: Vec<f64> = trimmed
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|tok| !tok.is_empty())
            .filter_map(|tok| tok.parse::<f64>().ok())
            .filter(|n| n.is_finite())
            .collect();
        let mut i = 0;
        while i + 1 < nums.len() {
            pts.push(serde_json::json!([nums[i], nums[i + 1]]));
            i += 2;
        }
    }
    Ok(Value::Array(pts))
}
