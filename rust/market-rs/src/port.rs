//! The Rust port of `src/market.js`.

#![forbid(unsafe_code)]

#[path = "service.rs"]
pub mod service;

fn is_falsy(v: &serde_json::Value) -> bool {
    if v.is_null() || crate::js::is_undefined(v) {
        return true;
    }
    if let Some(b) = v.as_bool() {
        return !b;
    }
    if let Some(n) = crate::js::as_number(v) {
        return n == 0.0 || n.is_nan();
    }
    if let Some(s) = v.as_str() {
        return s.is_empty();
    }
    false
}

fn to_js_number(v: &serde_json::Value) -> Option<f64> {
    if let Some(n) = crate::js::as_number(v) {
        return Some(n);
    }
    match v {
        serde_json::Value::Bool(true) => Some(1.0),
        serde_json::Value::Bool(false) | serde_json::Value::Null => Some(0.0),
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Some(0.0)
            } else {
                trimmed.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

/// Port of `downsampleCloses` (`src/market.js:40`).
pub fn downsample_closes(
    arr: serde_json::Value,
    max_points: Option<f64>,
) -> Result<serde_json::Value, String> {
    let clean: Vec<f64> = if is_falsy(&arr) {
        Vec::new()
    } else if let serde_json::Value::Array(items) = &arr {
        items
            .iter()
            .filter_map(|x| {
                let n = to_js_number(x)?;
                (n.is_finite() && n > 0.0).then_some(n)
            })
            .collect()
    } else {
        return Err("(arr || []).map is not a function".to_string());
    };

    let max_pts = max_points.unwrap_or(90.0);
    if (clean.len() as f64) <= max_pts {
        return Ok(serde_json::Value::Array(
            clean.into_iter().map(crate::js::number).collect(),
        ));
    }

    let mut out = Vec::new();
    let step = (clean.len() as f64 - 1.0) / (max_pts - 1.0);
    let limit = max_pts - 1.0;
    if limit > 0.0 && limit.is_finite() {
        let mut i = 0.0;
        while i < limit {
            let idx = (i * step).round() as usize;
            if let Some(&val) = clean.get(idx) {
                out.push(crate::js::number(val));
            } else {
                out.push(crate::js::undefined());
            }
            i += 1.0;
        }
    }
    if clean.is_empty() {
        out.push(crate::js::undefined());
    } else {
        out.push(crate::js::number(clean[clean.len() - 1]));
    }

    Ok(serde_json::Value::Array(out))
}

/// Port of `cnbcSymbolFor` (`src/market.js:50`).
pub fn cnbc_symbol_for(k: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_falsy(&k) {
        return Ok(serde_json::Value::String(String::new()));
    }
    let Some(s) = k.as_str() else {
        return Err("k.startsWith is not a function".to_string());
    };
    if s == "IDX:COMPOSITE" || s == "^JKSE" {
        return Ok(serde_json::Value::String(".JKSE".to_string()));
    }
    if let Some(rest) = s.strip_prefix("IDX:") {
        return Ok(serde_json::Value::String(format!("{rest}-ID")));
    }
    if s == "FX_IDC:USDIDR" || s == "IDR=X" {
        return Ok(serde_json::Value::String("IDR=".to_string()));
    }
    let mapped = match s {
        "COMEX:GC1!" => "@GC.1",
        "COMEX:SI1!" => "@SI.1",
        "NYMEX:CL1!" => "@CL.1",
        "NYMEX:NG1!" => "@NG.1",
        "COMEX:HG1!" => "@HG.1",
        "COMEX:ALI1!" => "@AL.1",
        "NYMEX:HO1!" => "@HO.1",
        "CBOT:ZR1!" => "@RR.1",
        "CBOT:ZW1!" => "@W.1",
        "CBOT:ZC1!" => "@C.1",
        "CBOT:ZS1!" => "@S.1",
        "CME:DC1!" => "@DA.1",
        "ICEUS:KC1!" => "@KC.1",
        "ICEUS:SB1!" => "@SB.1",
        "ICEUS:CC1!" => "@CC.1",
        "BINANCE:BTCUSDT" => "BTC.CM=",
        "BINANCE:USDTIDR" => "USDTIDR=",
        "BINANCE:ETHUSDT" => "ETH.CM=",
        "BINANCE:SOLUSDT" => "SOL.CM=",
        "BINANCE:BNBUSDT" => "BNB.CM=",
        "BINANCE:XRPUSDT" => "XRP.CM=",
        "^GSPC" => ".SPX",
        "^NDX" => ".NDX",
        "^DJI" => ".DJI",
        "^RUT" => ".RUT",
        "^N225" => ".N225",
        "^KS11" => ".KS11",
        "^HSI" => ".HSI",
        "^TWII" => ".TWII",
        "^GDAXI" => ".GDAXI",
        "^FTSE" => ".FTSE",
        "^FCHI" => ".FCHI",
        "^SSMI" => ".SSMI",
        "^AXJO" => ".AXJO",
        "^BSESN" => ".BSESN",
        "^NSEI" => ".NSEI",
        "^STI" => ".STI",
        "000001.SS" => ".SSEC",
        "^BVSP" => ".BVSP",
        _ => s,
    };
    Ok(serde_json::Value::String(mapped.to_string()))
}
