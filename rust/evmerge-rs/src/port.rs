//! The Rust port of `src/evmerge.js`.

#![forbid(unsafe_code)]

mod pinned;

use crate::js::{
    as_number, is_undefined, number, number_to_string, sort, string_units, undefined, utf16_string,
};
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

const TSTOP_RAW: &str = "yang dan di ke dari untuk pada dengan adalah ini itu ada tidak akan juga telah sudah bisa lebih sangat para dia kami kita mereka sebuah seorang suatu saat ketika setelah sebelum hingga oleh karena maka jika hari minggu senin selasa rabu kamis jumat sabtu minggu januari februari maret april mei juni juli agustus september oktober november desember breaking news terbaru update foto video live hasil wib wit daring online pemerintah menteri presiden indonesia daerah warga polisi damkar bpbd dishub pasar korban evakuasi massa jumlah ribu juta ratusan ribuan jutaan miliar triliun total resmi viral heboh terduga diduga kini paling orang persen masyarakat pihak oknum pelaku tersangka ibu bapak kali pemkot polda polres kejaksaan kemensos kemenkeu kemendagri";
const EV_GENERIC_EXTRA: &str = "kebakaran karhutla banjir gempa guncang kecelakaan tabrak tabrakan tawuran longsor demo unjuk rasa massa damkar pemadam waspada peringatan tertimpa menimpa luka tewas tewaskan meninggal korban evakuasi ditangkap terbakar meluas amblas rob genangan macet terpampang terkunci terdampak erupsi letusan penembakan pembakaran penangkapan pembunuhan pencurian perampokan penyelundupan penggerebekan penemuan ledakan bentrok kerusuhan penganiayaan penipuan korupsi narkoba sabu senjata bom kebocoran pencemaran kekeringan rumah jalan gedung kantor sekolah kampus pasar warung toko pabrik gudang terminal stasiun bandara pelabuhan jembatan sungai desa kampung kelurahan kecamatan kota kabupaten provinsi tol ruas lahan hutan hektare hektar mobil motor truk bus angkot kereta kapal pesawat helikopter warga pelaku tersangka siswa mahasiswa guru petani nelayan buruh pemuda pria wanita orang masyarakat keluarga remaja pemerintah pemkot pemkab gubernur bupati walikota menteri presiden dpr dprd polisi polda polres tni bpbd bnpb dinas badan komisi kementerian pemprov pemda satpol dishub puskesmas rumahsakit kalteng kaltim kalsel kalbar kaltara jateng jatim jabar sumut sumbar sumsel kepri sulut sulteng sulsel sultra sulbar malut ntb ntt diy";

static TSTOP: LazyLock<HashSet<&'static str>> = LazyLock::new(|| TSTOP_RAW.split(' ').collect());
static EV_GENERIC: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    TSTOP_RAW
        .split(' ')
        .chain(EV_GENERIC_EXTRA.split(' '))
        .collect()
});
static UNDEF: LazyLock<Value> = LazyLock::new(undefined);
static RE: LazyLock<[Regex; 7]> = LazyLock::new(|| {
    [
        r"\s+-\s+[^-]{2,45}$",
        r"(?i-u)\b(?:gempa|gempabumi|tektonik|sesar|aftershock|susulan)\b",
        r"(?i-u)\b(?:erupsi|letusan|awan panas|vulkanik)\b",
        r"(?i-u)\b(?:banjir|longsor|genangan)\b",
        r"(?i-u)\b(?:video|foto|komentar|opini|analisis|kolom|kata|menurut|sebut|harap|minta|imbau|dukung|kecam|respons|tanggapi|program|inovasi|sosialisasi|edukasi|kesiapsiagaan|mitigasi|pencegahan|peringatan|tinjau|kunjungi|resmikan|launching|pelatihan|simulasi)\b",
        r"(?i-u)\b(?:kebakaran|banjir|gempa|erupsi|letusan|longsor|tabrakan|kecelakaan|pembakaran|penembakan|tewas|tewaskan|meninggal|luka|korban|evakuasi|terbakar|ambruk|tertimpa|terendam|hanyut|hilang|diselamatkan|ditangkap)\b",
        r"(?i)^\s*(?-u:(?:video|foto)\b)",
    ]
    // INFALLIBLE: all 7 patterns are static valid regex literals
    .map(|p| Regex::new(p).expect("valid static regex"))
});

type Member = (Value, Vec<String>);

fn ok(x: f64) -> bool {
    x != 0.0 && !x.is_nan()
}

fn pick(xs: &[f64]) -> f64 {
    xs.iter().copied().find(|&x| ok(x)).unwrap_or(0.0)
}

fn fmm(a: f64, b: f64, max: bool) -> f64 {
    match (a.is_nan() || b.is_nan(), max) {
        (true, _) => f64::NAN,
        (_, true) => a.max(b),
        _ => a.min(b),
    }
}

fn plain_obj(v: &Value) -> Option<&Map<String, Value>> {
    (!is_undefined(v) && as_number(v).is_none() && string_units(v).is_none())
        .then(|| v.as_object())
        .flatten()
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        _ if is_undefined(v) => false,
        _ if let Some(n) = as_number(v) => ok(n),
        _ if let Some(u) = string_units(v) => !u.is_empty(),
        _ => true,
    }
}

fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if x.is_null() || is_undefined(x) {
                    String::new()
                } else {
                    js_str(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ if is_undefined(v) => "undefined".into(),
        _ if let Some(n) = as_number(v) => number_to_string(n),
        _ if let Some(u) = string_units(v) => String::from_utf16_lossy(&u),
        _ => "[object Object]".into(),
    }
}

fn js_num(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::String(s) => match s.trim() {
            "" => 0.0,
            "Infinity" | "+Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            t => t.parse().unwrap_or(f64::NAN),
        },
        Value::Array(_) => js_num(&Value::String(js_str(v))),
        _ => as_number(v).unwrap_or(f64::NAN),
    }
}

fn prop<'a>(v: &'a Value, k: &str) -> &'a Value {
    plain_obj(v).and_then(|o| o.get(k)).unwrap_or(&UNDEF)
}

fn tprop<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    let x = prop(v, k);
    is_truthy(x).then_some(x)
}

fn first_tp<'a>(v: &'a Value, ks: &[&str]) -> Option<&'a Value> {
    ks.iter().find_map(|k| tprop(v, k))
}

fn ts_of(v: &Value) -> f64 {
    tprop(v, "ts").map_or(0.0, js_num)
}

fn set_has(v: &[Value], item: &Value) -> bool {
    v.iter().any(|a| match (as_number(a), as_number(item)) {
        (Some(x), Some(y)) => (x.is_nan() && y.is_nan()) || x == y,
        _ => a == item,
    })
}

fn set_add(v: &mut Vec<Value>, item: Value) {
    if !set_has(v, &item) {
        v.push(item);
    }
}

fn spread(v: &Value) -> Map<String, Value> {
    if let Some(o) = plain_obj(v) {
        return o.clone();
    }
    let items: Vec<Value> = match v {
        Value::Array(a) => a.clone(),
        _ => string_units(v).map_or_else(Vec::new, |u| {
            u.into_iter().map(|c| utf16_string(&[c])).collect()
        }),
    };
    items
        .into_iter()
        .enumerate()
        .map(|(i, x)| (i.to_string(), x))
        .collect()
}

fn iter_or_empty(v: &Value, label: &str) -> Result<Vec<Value>, String> {
    if !is_truthy(v) {
        return Ok(Vec::new());
    }
    if let Value::Array(a) = v {
        return Ok(a.clone());
    }
    if let Some(u) = string_units(v) {
        let (mut out, mut i) = (Vec::new(), 0);
        while i < u.len() {
            let hi = (0xD800..=0xDBFF).contains(&u[i]);
            let lo = u.get(i + 1).is_some_and(|c| (0xDC00..=0xDFFF).contains(c));
            let step = usize::from(hi && lo) + 1;
            out.push(utf16_string(&u[i..i + step]));
            i += step;
        }
        return Ok(out);
    }
    Err(format!("({label} || []) is not iterable"))
}

pub fn break_norm_str(s: &str) -> String {
    let lo = s.to_lowercase();
    let kept: String = RE[0]
        .replace(&lo, "")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .collect();
    kept.split(' ')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn norm_place(v: &Value) -> String {
    if !is_truthy(v) {
        return String::new();
    }
    let s = js_str(v).to_lowercase();
    let p: Vec<&str> = s
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    p.join(" ")
}

fn tokens(title: &str, pk: &str, min: usize, stop: &HashSet<&str>) -> Vec<String> {
    let pt: HashSet<&str> = pk.split(' ').collect();
    let mut out = Vec::new();
    for w in break_norm_str(title).split(' ') {
        if w.len() >= min && !stop.contains(w) && !pt.contains(w) && !out.iter().any(|x| x == w) {
            out.push(w.to_owned());
        }
    }
    out
}

pub fn ev_rare(title: &str, pk: &str) -> Vec<String> {
    tokens(title, pk, 5, &EV_GENERIC)
}

pub fn hazard_of_str(t: &str) -> &'static str {
    const NAMES: [&str; 3] = ["gempa", "erupsi", "flood"];
    (0..3)
        .find_map(|i| RE[i + 1].is_match(t).then_some(NAMES[i]))
        .unwrap_or("")
}

fn rep_score_val(p: &Value) -> i32 {
    let t = first_tp(p, &["title", "name"])
        .map(js_str)
        .unwrap_or_default();
    2 * i32::from(RE[5].is_match(&t))
        - 3 * i32::from(RE[4].is_match(&t))
        - 2 * i32::from(RE[6].is_match(&t))
}

fn hash_units(it: impl IntoIterator<Item = u16>) -> String {
    let mut h = it.into_iter().fold(5381u32, |h, u| {
        h.wrapping_mul(33).wrapping_add(u32::from(u))
    });
    if h == 0 {
        return "0".into();
    }
    let mut d = Vec::with_capacity(8);
    while h > 0 {
        d.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[(h % 36) as usize] as char);
        h /= 36;
    }
    d.into_iter().rev().collect()
}

/// Port of `breakNorm` (`src/evmerge.js:3`).
pub fn break_norm(t: Value) -> Result<Value, String> {
    Ok(Value::String(break_norm_str(&js_str(&t))))
}

/// Port of `hazardOf` (`src/evmerge.js:37`).
pub fn hazard_of(t: Value) -> Result<Value, String> {
    Ok(Value::String(hazard_of_str(&js_str(&t)).to_owned()))
}

/// Port of `repScore` (`src/evmerge.js:42`).
pub fn rep_score(p: Value) -> Result<Value, String> {
    Ok(Value::from(rep_score_val(&p)))
}

/// Port of `evHash` (`src/evmerge.js:51`).
pub fn ev_hash(s: Value) -> Result<Value, String> {
    if s.is_null() || is_undefined(&s) {
        let k = if s.is_null() { "null" } else { "undefined" };
        return Err(format!("Cannot read properties of {k} (reading 'length')"));
    }
    if let Some(u) = string_units(&s) {
        return Ok(Value::String(hash_units(u)));
    }
    let bad = match &s {
        Value::Array(a) => !a.is_empty(),
        _ if plain_obj(&s).is_some() => js_num(prop(&s, "length")) > 0.0,
        _ => false,
    };
    if bad {
        Err("s.charCodeAt is not a function".into())
    } else {
        Ok(Value::String(hash_units([])))
    }
}

fn cmp_pin(a: &Value, b: &Value) -> f64 {
    let sd = rep_score_val(b) - rep_score_val(a);
    if sd != 0 {
        f64::from(sd)
    } else {
        ts_of(b) - ts_of(a)
    }
}

fn prev_ok(pp: &Value) -> bool {
    is_truthy(pp) && tprop(pp, "eid").is_some() && prop(pp, "kind").as_str() != Some("node")
}

fn hwin(h: &str) -> f64 {
    if h == "flood" { 21.6e6 } else { 43.2e6 }
}

fn match_prev<'a>(
    prev_list: &'a [Value],
    idxs: &[usize],
    used: &mut Vec<Value>,
    rare_u: &[String],
    ctx: (&str, &str, &str, &str, f64),
) -> Option<&'a Value> {
    let (cat, pk, rep_bn, hz_rep, newest) = ctx;
    let (haz_cat, nt, win) = (
        cat == "disaster" || cat == "flood",
        ok(newest),
        hwin(hz_rep),
    );
    for &pi in idxs {
        let pp = &prev_list[pi];
        let pp_eid = prop(pp, "eid");
        if set_has(used, pp_eid) {
            continue;
        }
        let (pt, dt) = (
            tprop(pp, "ts").is_some(),
            (newest - js_num(prop(pp, "ts"))).abs(),
        );
        if nt && pt && dt > 86.4e6 {
            continue;
        }
        let pp_title = js_str(prop(pp, "title"));
        let pr: Vec<Value> = match prop(pp, "rt") {
            Value::Array(a) if !a.is_empty() => a.iter().fold(Vec::new(), |mut v, e| {
                set_add(&mut v, e.clone());
                v
            }),
            _ => ev_rare(&pp_title, pk)
                .into_iter()
                .map(Value::String)
                .collect(),
        };
        let sh = rare_u
            .iter()
            .filter(|t| set_has(&pr, &Value::String((*t).clone())))
            .count();
        let exact = rare_u.is_empty() && pr.is_empty() && break_norm_str(&pp_title) == rep_bn;
        let single = rare_u.len() == 1 && pr.len() == 1 && sh == 1 && (!nt || !pt || dt <= 21.6e6);
        let haz = haz_cat
            && !hz_rep.is_empty()
            && hz_rep == hazard_of_str(&pp_title)
            && (!nt || !pt || dt <= win);
        if sh >= 2 || single || exact || haz {
            set_add(used, pp_eid.clone());
            return Some(pp);
        }
    }
    None
}

/// Port of `mergeEvents` (`src/evmerge.js:53`).
pub fn merge_events(out: Value, prev_pins: Value, opts: Value) -> Result<Value, String> {
    let prev_list = iter_or_empty(&prev_pins, "prevPins")?;
    let mut prev_by_key: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, pp) in prev_list.iter().enumerate() {
        let nv = prop(pp, "name");
        if prev_ok(pp) && string_units(nv).is_some() {
            let k = format!("{}|{}", js_str(prop(pp, "kind")), norm_place(nv));
            prev_by_key.entry(k).or_default().push(idx);
        }
    }

    let mut group_idx: HashMap<String, usize> = HashMap::new();
    let mut groups: Vec<(String, Vec<Member>)> = Vec::new();
    for p in iter_or_empty(&out, "out")? {
        if p.is_null() || is_undefined(&p) {
            let k = if p.is_null() { "null" } else { "undefined" };
            return Err(format!("Cannot read properties of {k} (reading 'name')"));
        }
        let pk = norm_place(prop(&p, "name"));
        let key = format!("{}|{}", js_str(prop(&p, "kind")), pk);
        let rs = ev_rare(&js_str(prop(&p, "title")), &pk);
        let gi = *group_idx.entry(key.clone()).or_insert_with(|| {
            groups.push((key, Vec::new()));
            groups.len() - 1
        });
        groups[gi].1.push((p, rs));
    }

    let mut group_df: HashMap<String, usize> = HashMap::new();
    for (_, ms) in &groups {
        let seen: HashSet<&str> = ms
            .iter()
            .flat_map(|(_, r)| r.iter().map(String::as_str))
            .collect();
        seen.into_iter()
            .for_each(|t| *group_df.entry(t.to_owned()).or_insert(0) += 1);
    }

    let now = as_number(prop(&opts, "now"))
        .filter(|n| n.is_finite())
        .unwrap_or_else(pinned::now_ms);
    let retain_ms = Some(js_num(prop(&opts, "retainPrevMs")))
        .filter(|&n| n > 0.0)
        .unwrap_or(0.0);
    let (mut merged, mut used) = (Vec::new(), Vec::new());

    for (key, mut ms) in groups {
        let cut = key.find('|').unwrap_or(0);
        let (cat, pk) = (&key[..cut], &key[cut + 1..]);
        let haz_cat = cat == "disaster" || cat == "flood";
        sort(&mut ms, |a, b| cmp_pin(&a.0, &b.0));
        let hz: Vec<&str> = ms
            .iter()
            .map(|(p, _)| hazard_of_str(&js_str(prop(p, "title"))))
            .collect();
        let mut df: HashMap<&str, usize> = HashMap::new();
        for (_, r) in &ms {
            r.iter()
                .for_each(|t| *df.entry(t.as_str()).or_insert(0) += 1);
        }
        let df_max = 2usize.max(((ms.len() as f64) * 0.3).floor() as usize);
        let eff: Vec<HashSet<&str>> = ms
            .iter()
            .map(|(_, r)| {
                r.iter()
                    .map(String::as_str)
                    .filter(|&t| df[t] <= df_max || group_df.get(t).copied().unwrap_or(0) < 3)
                    .collect()
            })
            .collect();

        let mut cls: Vec<(usize, Vec<usize>)> = Vec::new();
        for i in 0..ms.len() {
            let found = cls.iter_mut().find(|(r, _)| {
                let sh = eff[*r].iter().filter(|&&t| eff[i].contains(t)).count();
                sh >= 2
                    || (haz_cat
                        && !hz[*r].is_empty()
                        && hz[*r] == hz[i]
                        && (ts_of(&ms[*r].0) - ts_of(&ms[i].0)).abs() <= hwin(hz[*r]))
            });
            if let Some(cl) = found {
                cl.1.push(i);
            } else {
                cls.push((i, vec![i]));
            }
        }

        for (_, mut idxs) in cls {
            sort(&mut idxs, |&a, &b| cmp_pin(&ms[a].0, &ms[b].0));
            let rep = &ms[idxs[0]].0;
            let rep_title = js_str(prop(rep, "title"));
            let (hz_rep, rep_bn) = (hazard_of_str(&rep_title), break_norm_str(&rep_title));
            let mut rare_u: Vec<String> = Vec::new();
            for &i in &idxs {
                for t in &ms[i].1 {
                    if !rare_u.contains(t) {
                        rare_u.push(t.clone());
                    }
                }
            }
            let newest = idxs
                .iter()
                .fold(f64::NEG_INFINITY, |acc, &i| fmm(acc, ts_of(&ms[i].0), true));
            let earliest = idxs.iter().fold(f64::INFINITY, |acc, &i| {
                fmm(
                    acc,
                    tprop(&ms[i].0, "ts").map_or_else(|| pick(&[newest]), js_num),
                    false,
                )
            });

            let pi = prev_by_key.get(&key).map(Vec::as_slice).unwrap_or(&[]);
            let mp = match_prev(
                &prev_list,
                pi,
                &mut used,
                &rare_u,
                (cat, pk, &rep_bn, hz_rep, newest),
            );
            let eid = if let Some(pp) = mp {
                prop(pp, "eid").clone()
            } else {
                let eh = |s: String| Value::String(format!("e{}", hash_units(s.encode_utf16())));
                let seed = if !rare_u.is_empty() {
                    let mut s = rare_u.clone();
                    s.sort();
                    s.join(",")
                } else {
                    let mut d = tokens(&rep_title, pk, 3, &TSTOP);
                    d.sort();
                    let head = if d.is_empty() {
                        rep_bn.clone()
                    } else {
                        d.join(",")
                    };
                    format!(
                        "{head}@{}",
                        number_to_string((pick(&[earliest]) / 43.2e6).floor())
                    )
                };
                let mut cand = eh(format!("{cat}|{pk}|{seed}"));
                if set_has(&used, &cand) {
                    cand = eh(format!("{cat}|{pk}|{rep_bn}|{}", used.len()));
                }
                set_add(&mut used, cand.clone());
                cand
            };

            let mut srcs = Vec::new();
            let mut add_src = |v: &Value| {
                if is_truthy(v) {
                    set_add(&mut srcs, v.clone());
                }
            };
            if let Some(pp) = mp {
                match prop(pp, "sources") {
                    Value::Array(a) => a.iter().for_each(&mut add_src),
                    _ => add_src(prop(pp, "src")),
                }
            }
            idxs.iter().for_each(|&i| add_src(prop(&ms[i].0, "src")));

            let first_seen = mp.map_or_else(
                || pick(&[earliest, newest]),
                |pp| {
                    let rhs = first_tp(pp, &["firstSeenTs", "reportedTs", "ts"])
                        .map_or_else(|| pick(&[earliest, now]), js_num);
                    fmm(pick(&[earliest, newest, now]), rhs, false)
                },
            );
            let prev_n = mp.and_then(|pp| tprop(pp, "n")).map_or(1.0, js_num);
            let total_n = fmm(idxs.len().max(srcs.len()) as f64, prev_n, true);
            let prev_ts = mp.and_then(|pp| tprop(pp, "ts")).map_or(0.0, js_num);
            let rt = Value::Array(rare_u.into_iter().map(Value::String).collect());
            let src_v = if srcs.is_empty() {
                undefined()
            } else {
                Value::Array(srcs)
            };
            let rts = tprop(rep, "ts").cloned().unwrap_or_else(|| Value::from(0));

            let mut item = spread(rep);
            item.extend([
                ("eid".into(), eid),
                ("rt".into(), rt),
                ("sources".into(), src_v),
                ("firstSeenTs".into(), number(first_seen)),
                ("reportedTs".into(), rts),
                ("lastSeenTs".into(), number(newest)),
                ("ts".into(), number(fmm(newest, prev_ts, true))),
            ]);
            if total_n > 1.0 {
                item.insert("n".into(), number(total_n));
            }
            merged.push(Value::Object(item));
        }
    }

    if retain_ms > 0.0 {
        for pp in &prev_list {
            let eid = prop(pp, "eid");
            let pts = pick(&[js_num(prop(pp, "ts"))]);
            if !prev_ok(pp)
                || is_truthy(prop(pp, "fixed"))
                || set_has(&used, eid)
                || pts <= 0.0
                || pts > now + 300_000.0
                || now - pts > retain_ms
            {
                continue;
            }
            set_add(&mut used, eid.clone());
            let mut kept = spread(pp);
            let fs = first_tp(pp, &["firstSeenTs", "reportedTs"])
                .cloned()
                .unwrap_or_else(|| number(pts));
            let ls = tprop(pp, "lastSeenTs")
                .cloned()
                .unwrap_or_else(|| number(pts));
            kept.extend([
                ("carried".into(), Value::Bool(true)),
                ("firstSeenTs".into(), fs),
                ("lastSeenTs".into(), ls),
            ]);
            merged.push(Value::Object(kept));
        }
    }
    Ok(Value::Array(merged))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn helpers_unit() {
        assert_eq!(
            break_norm_str("Kebakaran Rumah! - Kompas.com"),
            "kebakaran rumah"
        );
        assert_eq!(hazard_of_str("Gempa tektonik"), "gempa");
        assert_eq!(hazard_of_str("awan panas"), "erupsi");
        assert_eq!(hazard_of_str("Banjir"), "flood");
        assert_eq!(
            rep_score(json!({"title": "Video: Komentar soal kebakaran"})).unwrap(),
            json!(-3)
        );
        assert_eq!(ev_hash(json!("x")).unwrap(), json!("3t3x"));
        assert_eq!(
            ev_rare("Kebakaran rumah sariwati melati di Bandung", "bandung"),
            vec!["sariwati", "melati"]
        );
    }
}
