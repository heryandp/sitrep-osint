//! Core 6-dimension tax clustering engine (Rust service layer for `pajak-cluster-rs`).

#![forbid(unsafe_code)]

use crate::js;
use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

#[allow(dead_code)]
#[path = "../../geocode-rs/src/service.rs"]
mod geo_service;

#[derive(Debug, Clone, Copy)]
pub struct ThemeDef {
    pub id: &'static str,
    pub label: &'static str,
    pub short: &'static str,
    pub desc: &'static str,
    pub color: &'static str,
    pub light_color: &'static str,
}

pub const PAJAK_THEMES: [ThemeDef; 6] = [
    ThemeDef {
        id: "ideologi",
        label: "Ideologi",
        short: "Ideologi",
        desc: "Keadilan fiskal, nasionalisme pajak, gotong royong, dan narasi boikot/anti-pajak",
        color: "#fb7185",
        light_color: "#be123c",
    },
    ThemeDef {
        id: "politik",
        label: "Politik",
        short: "Politik",
        desc: "Kebijakan DPR/Komisi XI, sikap fraksi/partai, Menkeu, RUU/UU HPP, dan protes kebijakan tarif",
        color: "#60a5fa",
        light_color: "#1d4ed8",
    },
    ThemeDef {
        id: "sosbud",
        label: "Sosial dan Budaya",
        short: "Sosial & Budaya",
        desc: "Kepatuhan sukarela, literasi/Tax Center, kepercayaan publik, UMKM, layanan KPP, dan persepsi warga",
        color: "#fbbf24",
        light_color: "#9a4208",
    },
    ThemeDef {
        id: "ekonomi",
        label: "Ekonomi",
        short: "Ekonomi",
        desc: "Penerimaan pajak, tax ratio, dinamisasi setoran, APBN, insentif fiskal, PPN/PPh, dan dampak industri",
        color: "#34d399",
        light_color: "#047857",
    },
    ThemeDef {
        id: "hukum",
        label: "Hukum dan HAM",
        short: "Hukum & HAM",
        desc: "Pidana pajak, PPATK, sita/perampasan aset, gijzeling, Pengadilan Pajak, korupsi/OTT, dan hak wajib pajak",
        color: "#c084fc",
        light_color: "#6d28d9",
    },
    ThemeDef {
        id: "teknologi",
        label: "Teknologi",
        short: "Teknologi",
        desc: "Coretax DJP, gangguan/error sistem, e-Faktur, e-Bupot, NIK\u{2013}NPWP 16 digit, PJAP, dan keamanan siber pajak",
        color: "#22d3ee",
        light_color: "#0b637c",
    },
];

fn compile_re(pat: &str) -> Regex {
    // INFALLIBLE: all patterns are compile-time constant valid regexes.
    Regex::new(pat).expect("static regex")
}

struct Patterns {
    tax: Regex,
    tax_noise: Regex,
    tax_nasional: Regex,
    tax_daerah: Regex,
    core_central: Regex,
    theme_re: [(Regex, Regex); 6],
    fallback_tech: Regex,
    fallback_sosbud: Regex,
    urgent_crit: Regex,
    urgent_warn: Regex,
    url: Regex,
    mention: Regex,
    non_alnum: Regex,
    ws: Regex,
    stop: HashSet<&'static str>,
}

static PATS: LazyLock<Patterns> = LazyLock::new(|| {
    Patterns {
    tax: compile_re(
        r"(?i)(?:(?-u:\b)coretax(?-u:\b)|core\s*tax|(?-u:\b)djp(?-u:\b)|ditjen\s+pajak|direktorat\s+jenderal\s+pajak|kring\s+pajak|kanwil\s+djp|(?-u:\b)kpp\s+(?:pratama|madya|wajib\s+pajak|khusus|bumn|pma)(?-u:\b)|kantor\s+pelayanan\s+pajak|perpajakan|wajib\s+pajak|(?:uang|bayar|tarif|kenaikan|aturan|kebijakan|bebas|insentif|setor|lapor|kepatuhan|literasi|penerimaan|setoran|tunggakan|restitusi|pemeriksaan|penagihan|pidana|pengemplang|pengampunan|keringanan|relaksasi|sengketa|boikot|tolak|keadilan)\s+pajak|tax\s+(?:amnesty|ratio|center|holiday|allowance)|rasio\s+pajak|faktur\s+pajak|e-?faktur|e-?bupot|e-?filing|e-?billing|(?-u:\b)spt\s+(?:tahunan|masa|pph|ppn)(?-u:\b)|lapor\s+spt|pelaporan\s+spt|(?-u:\b)npwp(?-u:\b)|nik\s*(?:jadi|menjadi)?\s*npwp|(?-u:\b)ppn\s*(?:[0-9]{1,2}%|bm)?(?-u:\b)|(?-u:\b)ppnbm(?-u:\b)|(?-u:\b)pph\s*(?:pasal\s*[0-9]+|21|22|23|25|26|29|final|badan|orang\s+pribadi)?(?-u:\b)|pajak\s+(?:penghasilan|pertambahan\s+nilai|bumi\s+dan\s+bangunan|kendaraan|daerah|karbon|digital|kripto|natura|umkm|dividen|progresif|impor|ekspor|hiburan|warisan)|ter\s+pph|pengadilan\s+pajak|konsultan\s+pajak|fiskus|gijzeling|penyanderaan\s+pajak|sita\s+aset\s+pajak|keuangan\s+negara|(?-u:\b)apbn(?-u:\b)|defisit\s+apbn|surplus\s+apbn|penerimaan\s+negara|belanja\s+negara|utang\s+negara|(?-u:\b)pnbp(?-u:\b)|(?-u:\b)djbc(?-u:\b)|bea\s+(?:dan\s+)?cukai|(?-u:\b)dja(?-u:\b)|(?-u:\b)djppr(?-u:\b)|kemenkeu|kementerian\s+keuangan|(?-u:\b)menkeu(?-u:\b)|menteri\s+keuangan|kebijakan\s+fiskal|surat\s+berharga\s+negara|(?-u:\b)sbn(?-u:\b)|(?-u:\b)pmk\s*(?:no\.?|nomor)?\s*[0-9]+)",
    ),
    tax_noise: compile_re(
        r"(?i)(?:pajak\s+(?:jadian|nikah|jodoh|ganteng|cantik|makan\s+bareng)|judi\s+online|slot\s+gacor|drakor|k-?pop)",
    ),
    tax_nasional: compile_re(
        r"(?i)(?:(?-u:\b)djp(?-u:\b)|ditjen\s+pajak|direktorat\s+jenderal\s+pajak|dirjen\s+pajak|kanwil\s+djp|(?-u:\b)kpp\s+(?:pratama|madya|wajib\s+pajak|khusus|bumn|pma)(?-u:\b)|kantor\s+pelayanan\s+pajak|kring\s+pajak|(?-u:\b)coretax(?-u:\b)|core\s*tax|djp\s+online|e-?faktur|e-?bupot|e-?filing|e-?billing|faktur\s+pajak|(?-u:\b)spt\s+(?:tahunan|masa|pph|ppn)(?-u:\b)|lapor\s+spt|pelaporan\s+spt|(?-u:\b)npwp(?-u:\b)|nik\s*(?:jadi|menjadi)?\s*npwp|(?-u:\b)ppn\s*(?:[0-9]{1,2}%|bm)?(?-u:\b)|(?-u:\b)ppnbm(?-u:\b)|(?-u:\b)pph\s*(?:pasal\s*[0-9]+|21|22|23|25|26|29|final|badan|orang\s+pribadi)?(?-u:\b)|ter\s+pph|bea\s+meterai|pajak\s+penghasilan|pajak\s+pertambahan\s+nilai|pajak\s+(?:digital|kripto|karbon|natura|dividen|impor|ekspor|pusat)|kemenkeu|kementerian\s+keuangan|(?-u:\b)menkeu(?-u:\b)|menteri\s+keuangan|keuangan\s+negara|(?-u:\b)apbn(?-u:\b)|defisit\s+apbn|surplus\s+apbn|penerimaan\s+negara|belanja\s+negara|utang\s+negara|(?-u:\b)pnbp(?-u:\b)|(?-u:\b)djbc(?-u:\b)|bea\s+(?:dan\s+)?cukai|(?-u:\b)dja(?-u:\b)|(?-u:\b)djppr(?-u:\b)|kebijakan\s+fiskal|surat\s+berharga\s+negara|(?-u:\b)sbn(?-u:\b)|uu\s+hpp|(?-u:\b)pmk\s*(?:no\.?|nomor)?\s*[0-9]+|tax\s+(?:ratio|amnesty|holiday|allowance)|rasio\s+pajak|pengadilan\s+pajak|badan\s+penerimaan\s+negara|restitusi\s+pajak|pidana\s+pajak|faktur\s+fiktif|gijzeling|penyanderaan\s+pajak|program\s+psa|dinamisasi|komisi\s+xi|ppatk|ddtc|ortax)",
    ),
    tax_daerah: compile_re(
        r"(?i)(?:pajak\s+daerah|pajak\s+kendaraan(?:\s+bermotor)?|(?-u:\b)pkb(?-u:\b)|(?-u:\b)bbnkb(?-u:\b)|pemutihan\s+pajak|keringanan\s+(?:pokok\s+)?pajak\s+kendaraan|(?-u:\b)samsat(?-u:\b)|(?-u:\b)bapenda(?-u:\b)|(?-u:\b)dispenda(?-u:\b)|(?-u:\b)bappenda(?-u:\b)|(?-u:\b)bpkad(?-u:\b)|(?-u:\b)bphtb(?-u:\b)|(?-u:\b)pbb[- ]?p2(?-u:\b)|pajak\s+bumi\s+dan\s+bangunan|pajak\s+(?:hotel|restoran|hiburan|parkir|reklame|air\s+tanah|penerangan\s+jalan|sarang\s+burung\s+walet|mineral\s+bukan\s+logam|mblb|pbjt)|(?-u:\b)pbjt(?-u:\b)|pendapatan\s+asli\s+daerah|(?-u:\b)pad(?-u:\b)|perda\s+pajak|pemprov|pemkab|pemkot|pemda|gubernur|bupati|wali\s*kota|opsen\s+(?:pkb|bbnkb|pajak))",
    ),
    core_central: compile_re(
        r"(?i)(?-u:\b)(?:djp|ditjen\s+pajak|direktorat\s+jenderal\s+pajak|coretax|core\s*tax|pph|ppn|spt|npwp|e-?faktur|e-?bupot|kemenkeu|menkeu|apbn|tax\s+ratio|pengadilan\s+pajak|kanwil\s+djp|kpp\s+(?:pratama|madya))(?-u:\b)",
    ),
    theme_re: [
        (
            compile_re(
                r"(?i)(?:keadilan\s+(?:pajak|fiskal)|boikot\s+pajak|tolak\s+bayar\s+pajak|mogok\s+bayar\s+pajak|anti[- ]?pajak|bela\s+negara|pancasila|pasal\s+23\s*a|konstitusi|kedaulatan\s+fiskal|gotong\s+royong|pajak\s+kuat\s+indonesia\s+maju|pajak\s+untuk\s+rakyat|ketimpangan\s+(?:pajak|fiskal)|pajak\s+berkeadilan|ke\s*mana\s+(?:sebenarnya\s+)?uang\s+pajak)",
            ),
            compile_re(
                r"(?i)(?:ideologi|nasionalisme|kedaulatan|keadilan|pemerataan|rakyat\s+kecil|kaum\s+kaya|orang\s+kaya|konglomerat|oligarki|ketimpangan|solidaritas|kontribusi\s+untuk\s+negara|pembangunan\s+bangsa)",
            ),
        ),
        (
            compile_re(
                r"(?i)(?:(?-u:\b)dpr(?-u:\b)|(?-u:\b)dprd(?-u:\b)|komisi\s+xi|fraksi|partai|parlemen|senayan|presiden|wapres|menteri\s+keuangan|(?-u:\b)menkeu(?-u:\b)|dirjen\s+pajak|badan\s+penerimaan\s+negara|(?-u:\b)bpn(?-u:\b)|uu\s+hpp|ruu\s+pajak|perppu|(?-u:\b)pmk\s*(?:no\.?|nomor)?\s*[0-9]+|unjuk\s+rasa|demo\s+tolak|aksi\s+tolak|protes\s+(?:ppn|pajak)|kebijakan\s+fiskal|pemprov|gubernur|bupati|wali\s*kota)",
            ),
            compile_re(
                r"(?i)(?:kebijakan|regulasi|aturan\s+pajak|pemerintah|kabinet|istana|koalisi|oposisi|pilkada|pemilu|apbd|perda\s+pajak|keputusan\s+menteri|dpr\s+ri|kritik\s+kebijakan|polemik)",
            ),
        ),
        (
            compile_re(
                r"(?i)(?:tax\s+center|literasi\s+pajak|edukasi\s+pajak|sosialisasi\s+(?:pajak|pmk|coretax|spt)|kepatuhan\s+(?:pajak|wajib\s+pajak)|kepercayaan\s+publik|pelayanan\s+(?:pajak|kpp)|kring\s+pajak|antrean\s+(?:kpp|wajib\s+pajak)|konsultan\s+pajak|ikpi|kampus|universitas|mahasiswa|sdgs|gaya\s+hidup\s+(?:pegawai|pejabat)\s+pajak|kelas\s+menengah|kesadaran\s+pajak|budaya\s+(?:bayar\s+)?pajak)",
            ),
            compile_re(
                r"(?i)(?:masyarakat|warga|publik|umkm|pekerja|karyawan|buruh|komunitas|mahasiswa|pelajar|keluhan\s+wajib\s+pajak|wajib\s+pajak|sinergi|pelatihan|seminar|webinar|lapor\s+spt|spt\s+tahunan|keringanan\s+(?:pokok\s+)?pajak|pemutihan\s+pajak)",
            ),
        ),
        (
            compile_re(
                r"(?i)(?:penerimaan\s+pajak|setoran\s+pajak|realisasi\s+pajak|target\s+pajak|tax\s+ratio|rasio\s+pajak|shortfall|surplus\s+apbn|defisit\s+apbn|dinamisasi|restitusi\s+pajak|insentif\s+pajak|tax\s+holiday|tax\s+allowance|relaksasi\s+pajak|tarif\s+(?:ppn|pph)|ppn\s*12%|pph\s*(?:21|22|23|25|29|final|badan)|ter\s+pph|sektor\s+industri|daya\s+beli|pertumbuhan\s+ekonomi)",
            ),
            compile_re(
                r"(?i)(?:triliun|miliar|apbn|ekonomi|inflasi|investasi|ekspor|impor|emiten|dividen|saham|industri|manufaktur|perdagangan|bisnis|pengusaha|korporasi|kripto|aset\s+kripto|komoditas|tambang|sawit|nikel|batu\s+bara|bea\s+cukai|program\s+psa)",
            ),
        ),
        (
            compile_re(
                r"(?i)(?:pidana\s+pajak|ppatk|penggelapan\s+pajak|pengemplang\s+pajak|faktur\s+(?:pajak\s+)?fiktif|pengadilan\s+pajak|sengketa\s+pajak|perampasan\s+aset|harta\s+(?:bisa\s+)?dirampas|sita\s+aset|penyitaan\s+aset|gijzeling|penyanderaan\s+pajak|pemeriksaan\s+pajak|penagihan\s+pajak|tunggakan\s+pajak|suap\s+pajak|korupsi\s+pajak|ott\s+(?:pegawai|pejabat)\s+pajak|hak\s+wajib\s+pajak|uu\s+pdp|privasi\s+data|penipuan\s+(?:mengatasnamakan\s+)?djp)",
            ),
            compile_re(
                r"(?i)(?:hukum|pidana|tersangka|terdakwa|vonis|putusan|kasasi|kejaksaan|kpk|polri|polisi|penyidikan|sanksi|denda\s+pajak|pemblokiran\s+rekening|blokir\s+rekening|cekal|pencekalan|ham(?-u:\b)|hak\s+asasi|keberatan|banding|peninjauan\s+kembali|penipuan|modus|phishing)",
            ),
        ),
        (
            compile_re(
                r"(?i)(?:(?-u:\b)coretax(?-u:\b)|core\s*tax|e-?faktur|e-?bupot|e-?filing|e-?billing|djp\s+online|nik\s*(?:jadi|menjadi)?\s*npwp|npwp\s*16\s*digit|pjap|pajak\s+digital|apk\s+palsu\s+(?:djp|pajak|coretax)|kebocoran\s+data\s+(?:npwp|pajak|djp)|peretasan\s+(?:djp|coretax|pajak)|sistem\s+(?:coretax|perpajakan|djp))",
            ),
            compile_re(
                r"(?i)(?:aplikasi|sistem|server|website|situs|portal|login|otp|error|eror|down|gangguan|lemot|lambat|timeout|bug|crash|maintenance|pemeliharaan|digital|digitalisasi|otomasi|api(?-u:\b)|cloud|siber|cyber|ransomware|phishing|malware|peretasan|data\s+breach|enkripsi|sertifikat\s+elektronik)",
            ),
        ),
    ],
    fallback_tech: compile_re(r"(?i)coretax|e-?faktur|e-?bupot|e-?filing|e-?billing|npwp"),
    fallback_sosbud: compile_re(r"(?i)kpp|kanwil|wajib\s+pajak|spt"),
    urgent_crit: compile_re(
        r"(?i)(?:coretax\s+(?:error|eror|down|lumpuh|gangguan|gagal|crash)|(?:error|eror|down|lumpuh|gangguan)\s+coretax|pidana\s+pajak|penggelapan\s+pajak|faktur\s+fiktif|ott(?-u:\b)|suap\s+pajak|korupsi\s+pajak|boikot\s+pajak|tolak\s+bayar\s+pajak|demo\s+tolak|unjuk\s+rasa|kebocoran\s+data|peretasan|apk\s+palsu|phishing)",
    ),
    urgent_warn: compile_re(
        r"(?i)(?:gangguan|kendala|keluhan|antrean|tunggakan|sita\s+aset|dirampas|perampasan\s+aset|gijzeling|penyanderaan|sengketa|restitusi|pemeriksaan|penagihan|denda|sanksi|tarif\s+naik|kenaikan\s+ppn|ppn\s*12%|shortfall|protes|polemik)",
    ),
    url: compile_re(r"https?://\S+"),
    mention: compile_re(r"@[A-Za-z0-9_.]+"),
    non_alnum: compile_re(r"[^\p{L}\p{N}]+"),
    ws: compile_re(r"\s+"),
    stop: concat!(
        "yang dan di ke dari untuk pada dengan adalah ini itu ada tidak akan juga telah sudah bisa lebih sangat ",
        "para oleh hari kini tadi baru saja info update berita kabar terkini viral breaking news ",
        "pajak perpajakan djp ditjen direktorat jenderal kementerian keuangan kemenkeu indonesia nasional ",
        "wajib tahun bulan hari ini sebut tegas minta dorong siap terus jadi bagi soal terkait melalui hingga"
    )
    .split(' ')
    .collect(),
}
});

pub fn theme_by_id(id: &str) -> Option<&'static ThemeDef> {
    PAJAK_THEMES.iter().find(|t| t.id == id)
}

pub fn is_tax_text_str(text: &str) -> bool {
    let s = text.trim();
    if s.is_empty() || PATS.tax_noise.is_match(s) {
        return false;
    }
    PATS.tax.is_match(s)
}

pub fn classify_tax_scope_str(text: &str) -> &'static str {
    let n = PATS.tax_nasional.find_iter(text).count();
    let d = PATS.tax_daerah.find_iter(text).count();
    if d > 0 && n == 0 {
        return "daerah";
    }
    if d > 0 && n > 0 {
        if PATS.core_central.is_match(text) {
            return "nasional";
        }
        return if d > n { "daerah" } else { "nasional" };
    }
    "nasional"
}

#[derive(Debug, Clone)]
pub struct ThemeClassification {
    pub primary: &'static str,
    pub secondary: Vec<&'static str>,
    pub scores: Vec<(&'static str, f64)>,
}

impl ThemeClassification {
    pub fn to_value(&self) -> Value {
        let mut score_map = Map::new();
        for &(id, sc) in &self.scores {
            let rounded = format!("{sc:.2}").parse::<f64>().unwrap_or(sc);
            score_map.insert(id.into(), js::number(rounded));
        }
        json!({
            "primary": self.primary,
            "secondary": self.secondary,
            "scores": Value::Object(score_map),
        })
    }
}

pub fn classify_pajak_themes_str(text: &str) -> ThemeClassification {
    let mut scores: Vec<(&'static str, f64)> = Vec::new();
    for (idx, t) in PAJAK_THEMES.iter().enumerate() {
        let (strong_re, base_re) = &PATS.theme_re[idx];
        let mut sc = 0.0_f64;
        let strong_hits = strong_re.find_iter(text).count().min(3);
        let base_hits = base_re.find_iter(text).count().min(3);
        if strong_hits > 0 {
            sc += (strong_hits as f64) * 3.0;
        }
        if base_hits > 0 {
            sc += (base_hits as f64) * 1.25;
        }
        if sc > 0.0 {
            scores.push((t.id, sc));
        }
    }
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    if scores.is_empty() {
        let primary = if PATS.fallback_tech.is_match(text) {
            "teknologi"
        } else if PATS.fallback_sosbud.is_match(text) {
            "sosbud"
        } else {
            "ekonomi"
        };
        return ThemeClassification {
            primary,
            secondary: Vec::new(),
            scores: vec![(primary, 1.0)],
        };
    }
    let primary = scores[0].0;
    let secondary = scores[1..]
        .iter()
        .filter(|x| x.1 >= 2.0)
        .take(2)
        .map(|x| x.0)
        .collect();
    ThemeClassification {
        primary,
        secondary,
        scores,
    }
}

fn tokenize(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let s1 = PATS.url.replace_all(&lower, " ");
    let s2 = PATS.mention.replace_all(&s1, " ");
    let s3 = PATS.non_alnum.replace_all(&s2, " ");
    let norm = s3.trim();
    if norm.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for w in norm.split_whitespace() {
        if w.encode_utf16().count() < 3
            || w.bytes().all(|b| b.is_ascii_digit())
            || PATS.stop.contains(w)
            || !seen.insert(w.to_string())
        {
            continue;
        }
        out.push(w.to_string());
    }
    out
}

fn token_overlap(a: &[String], b: &[String]) -> (usize, f64) {
    if a.is_empty() || b.is_empty() {
        return (0, 0.0);
    }
    let set_a: HashSet<&str> = a.iter().map(String::as_str).collect();
    let mut shared = 0usize;
    for w in b {
        if set_a.contains(w.as_str()) {
            shared += 1;
        }
    }
    let union = set_a.len() + b.len() - shared;
    let jaccard = if union > 0 {
        shared as f64 / union as f64
    } else {
        0.0
    };
    (shared, jaccard)
}

fn window_cutoff(win: &str, now: f64) -> f64 {
    if win == "2h" {
        return now - 2.0 * 3_600_000.0;
    }
    if win == "today" {
        let wib = now + 7.0 * 3_600_000.0;
        if !wib.is_finite() || wib.abs() > 8.64e15 {
            return f64::NAN;
        }
        let day_ms = 86_400_000.0;
        return (wib / day_ms).floor() * day_ms - 7.0 * 3_600_000.0;
    }
    now - 24.0 * 3_600_000.0
}

fn utf16_slice_prefix(s: &str, max_units: usize) -> String {
    let mut count = 0usize;
    let mut end = s.len();
    for (idx, ch) in s.char_indices() {
        let u = ch.len_utf16();
        if count + u > max_units {
            end = idx;
            break;
        }
        count += u;
    }
    s[..end].to_string()
}

pub fn js_truthy(v: &Value) -> bool {
    if js::is_undefined(v) {
        return false;
    }
    if let Some(n) = js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

pub fn js_to_string(v: &Value) -> String {
    if js::is_undefined(v) {
        return "undefined".into();
    }
    if let Some(n) = js::as_number(v) {
        return js::number_to_string(n);
    }
    if let Some(units) = js::string_units(v) {
        return String::from_utf16_lossy(&units);
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let mut out = String::new();
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if !item.is_null() && !js::is_undefined(item) {
                    out.push_str(&js_to_string(item));
                }
            }
            out
        }
        Value::Object(_) => "[object Object]".into(),
    }
}

pub fn js_string_or_empty(v: &Value) -> String {
    if js_truthy(v) {
        js_to_string(v)
    } else {
        String::new()
    }
}

fn parse_js_number_str(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    if t.eq_ignore_ascii_case("nan")
        || t.eq_ignore_ascii_case("inf")
        || t.eq_ignore_ascii_case("-inf")
        || t.eq_ignore_ascii_case("+inf")
        || t.eq_ignore_ascii_case("infinity")
        || t.eq_ignore_ascii_case("-infinity")
        || t.eq_ignore_ascii_case("+infinity")
    {
        return f64::NAN;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(oct) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return u64::from_str_radix(oct, 8)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(bin) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u64::from_str_radix(bin, 2)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn js_to_number(v: Option<&Value>) -> f64 {
    let Some(v) = v else {
        return f64::NAN;
    };
    if js::is_undefined(v) {
        return f64::NAN;
    }
    if let Some(n) = js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_js_number_str(s),
        Value::Array(arr) => match arr.as_slice() {
            [] => 0.0,
            [only] if only.is_null() || js::is_undefined(only) => 0.0,
            [only] => js_to_number(Some(only)),
            _ => f64::NAN,
        },
        Value::Object(_) => f64::NAN,
    }
}

fn parse_js_date_ms(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return f64::NAN;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(t) {
        return dt.timestamp_millis() as f64;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(t) {
        return dt.timestamp_millis() as f64;
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d")
        && let Some(dt) = d.and_hms_opt(0, 0, 0)
    {
        return dt.and_utc().timestamp_millis() as f64;
    }
    f64::NAN
}

fn num_or_zero(n: f64) -> f64 {
    if n.is_nan() || n == 0.0 { 0.0 } else { n }
}

fn has_topic(arr: &[Value]) -> bool {
    arr.iter()
        .any(|v| v.as_str().is_some_and(|s| s == "pajak" || s == "coretax"))
}

struct GeoLoc {
    name: String,
    prov: String,
    la: Option<f64>,
    lo: Option<f64>,
}

fn locate_from_geo(text: &str) -> GeoLoc {
    if let Some(val) = geo_service::locate_text(text)
        && let Some(obj) = val.as_object()
    {
        let name = obj
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let prov = obj
            .get("prov")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let la = obj.get("la").and_then(js::as_number);
        let lo = obj.get("lo").and_then(js::as_number);
        return GeoLoc { name, prov, la, lo };
    }
    GeoLoc {
        name: String::new(),
        prov: String::new(),
        la: None,
        lo: None,
    }
}

#[derive(Clone)]
struct NormItem {
    is_news: bool,
    platform: String,
    title: String,
    desc: String,
    url: Value,
    src: String,
    handle: String,
    author: String,
    provenance: Value,
    ts: f64,
    theme: &'static str,
    secondary_themes: Vec<&'static str>,
    tax_scope: &'static str,
    tokens: Vec<String>,
    place: String,
    prov: String,
    la: Option<f64>,
    lo: Option<f64>,
    urgent_crit: bool,
    urgent_warn: bool,
}

fn normalize_items(
    news_items: &[Value],
    social_posts: &[Value],
    cutoff: f64,
    now: f64,
) -> Vec<NormItem> {
    let mut out = Vec::new();
    let mut seen_key: HashSet<String> = HashSet::new();

    for it in news_items {
        let Some(obj) = it.as_object() else {
            continue;
        };
        let Some(title_val) = obj.get("title") else {
            continue;
        };
        if !js_truthy(title_val) {
            continue;
        }
        let ts = num_or_zero(js_to_number(obj.get("ts")));
        if ts != 0.0 && (ts < cutoff || ts > now + 10.0 * 60_000.0) {
            continue;
        }
        let title_raw = js_to_string(title_val);
        let desc_raw = obj.get("desc").map(js_string_or_empty).unwrap_or_default();
        let text = if !desc_raw.is_empty() {
            format!("{title_raw} {desc_raw}")
        } else {
            title_raw.clone()
        };
        let tagged_tax = obj.get("cat").and_then(Value::as_str) == Some("pajak")
            || obj
                .get("assessment")
                .and_then(Value::as_object)
                .and_then(|a| a.get("topics"))
                .and_then(Value::as_array)
                .is_some_and(|arr| has_topic(arr));
        if !tagged_tax && !is_tax_text_str(&text) {
            continue;
        }
        if PATS.tax_noise.is_match(&text) {
            continue;
        }
        let url_or_title = obj
            .get("url")
            .filter(|v| js_truthy(v))
            .map(js_to_string)
            .unwrap_or_else(|| title_raw.clone());
        let dedupe_key = format!("n:{}", url_or_title.trim().to_lowercase());
        if !seen_key.insert(dedupe_key) {
            continue;
        }

        let cls = classify_pajak_themes_str(&text);
        let tax_scope = classify_tax_scope_str(&text);
        let has_explicit_loc = obj.get("la").is_some_and(js_truthy)
            && obj.get("lo").is_some_and(js_truthy)
            && obj.get("name").is_some_and(js_truthy);
        let loc = if has_explicit_loc {
            GeoLoc {
                name: obj.get("name").map(js_to_string).unwrap_or_default(),
                prov: obj.get("prov").map(js_string_or_empty).unwrap_or_default(),
                la: obj.get("la").and_then(js::as_number),
                lo: obj.get("lo").and_then(js::as_number),
            }
        } else {
            locate_from_geo(&title_raw)
        };
        let src = obj
            .get("pub")
            .filter(|v| js_truthy(v))
            .or_else(|| obj.get("src").filter(|v| js_truthy(v)))
            .map(js_to_string)
            .unwrap_or_else(|| "Media".into())
            .trim()
            .to_string();
        let url = obj
            .get("url")
            .filter(|v| js_truthy(v))
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()));

        out.push(NormItem {
            is_news: true,
            platform: String::new(),
            title: title_raw.trim().to_string(),
            desc: desc_raw.trim().to_string(),
            url,
            src,
            handle: String::new(),
            author: String::new(),
            provenance: Value::Null,
            ts: if ts != 0.0 { ts } else { now },
            theme: cls.primary,
            secondary_themes: cls.secondary,
            tax_scope,
            tokens: tokenize(&title_raw),
            place: loc.name,
            prov: loc.prov,
            la: loc.la,
            lo: loc.lo,
            urgent_crit: PATS.urgent_crit.is_match(&text),
            urgent_warn: PATS.urgent_warn.is_match(&text),
        });
    }

    for p in social_posts {
        let Some(obj) = p.as_object() else {
            continue;
        };
        let txt_val = obj
            .get("text")
            .filter(|v| js_truthy(v))
            .or_else(|| obj.get("title").filter(|v| js_truthy(v)));
        let txt = txt_val
            .map(js_to_string)
            .unwrap_or_default()
            .trim()
            .to_string();
        if txt.is_empty() {
            continue;
        }
        let mut ts = num_or_zero(js_to_number(obj.get("ts")));
        if ts == 0.0
            && let Some(ca) = obj.get("created_at").filter(|v| js_truthy(v))
        {
            ts = num_or_zero(parse_js_date_ms(&js_to_string(ca)));
        }
        if ts != 0.0 && (ts < cutoff || ts > now + 10.0 * 60_000.0) {
            continue;
        }
        let tagged_tax = obj
            .get("assessment")
            .and_then(Value::as_object)
            .and_then(|a| a.get("topics"))
            .and_then(Value::as_array)
            .or_else(|| obj.get("topics").and_then(Value::as_array))
            .is_some_and(|arr| has_topic(arr));
        if !tagged_tax && !is_tax_text_str(&txt) {
            continue;
        }
        if PATS.tax_noise.is_match(&txt) {
            continue;
        }
        let handle_raw = obj
            .get("username")
            .filter(|v| js_truthy(v))
            .or_else(|| obj.get("handle").filter(|v| js_truthy(v)))
            .or_else(|| obj.get("user").filter(|v| js_truthy(v)))
            .map(js_to_string)
            .unwrap_or_default();
        let handle = handle_raw
            .strip_prefix('@')
            .unwrap_or(&handle_raw)
            .trim()
            .to_string();
        let dedupe_part = obj
            .get("url")
            .filter(|v| js_truthy(v))
            .or_else(|| obj.get("code").filter(|v| js_truthy(v)))
            .map(js_to_string)
            .unwrap_or_else(|| format!("{handle}:{}", utf16_slice_prefix(&txt, 80)));
        let dedupe_key = format!("s:{}", dedupe_part.to_lowercase());
        if !seen_key.insert(dedupe_key) {
            continue;
        }

        let cls = classify_pajak_themes_str(&txt);
        let tax_scope = classify_tax_scope_str(&txt);
        let loc = locate_from_geo(&txt);
        let platform = obj
            .get("from")
            .filter(|v| js_truthy(v))
            .map(js_to_string)
            .unwrap_or_else(|| {
                if obj.get("code").is_some_and(js_truthy) {
                    "Threads".into()
                } else {
                    "X".into()
                }
            });
        let collapsed = PATS.ws.replace_all(&txt, " ");
        let title = utf16_slice_prefix(&collapsed, 220);
        let src = if !handle.is_empty() {
            format!("@{handle}")
        } else {
            obj.get("from")
                .filter(|v| js_truthy(v))
                .map(js_to_string)
                .unwrap_or_else(|| "Sosial".into())
        };
        let author = obj
            .get("name")
            .filter(|v| js_truthy(v))
            .or_else(|| obj.get("author").filter(|v| js_truthy(v)))
            .map(js_to_string)
            .unwrap_or_else(|| {
                if !handle.is_empty() {
                    handle.clone()
                } else {
                    "Warganet".into()
                }
            })
            .trim()
            .to_string();
        let provenance = match obj
            .get("assessment")
            .and_then(Value::as_object)
            .and_then(|a| a.get("provenance"))
            .filter(|v| js_truthy(v))
        {
            Some(prov_obj) => prov_obj
                .as_object()
                .and_then(|m| m.get("sourceType"))
                .cloned()
                .unwrap_or_else(js::undefined),
            None => Value::String("anonymous".into()),
        };
        let url = obj
            .get("url")
            .filter(|v| js_truthy(v))
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()));

        out.push(NormItem {
            is_news: false,
            platform,
            title,
            desc: txt.clone(),
            url,
            src,
            handle,
            author,
            provenance,
            ts: if ts != 0.0 { ts } else { now },
            theme: cls.primary,
            secondary_themes: cls.secondary,
            tax_scope,
            tokens: tokenize(&txt),
            place: loc.name,
            prov: loc.prov,
            la: loc.la,
            lo: loc.lo,
            urgent_crit: PATS.urgent_crit.is_match(&txt),
            urgent_warn: PATS.urgent_warn.is_match(&txt),
        });
    }

    js::sort(&mut out, |a, b| {
        let diff = b.ts - a.ts;
        if diff != 0.0 && !diff.is_nan() {
            diff
        } else if a.is_news {
            -1.0
        } else {
            1.0
        }
    });
    out
}

struct RawCluster {
    id: String,
    theme: &'static str,
    tax_scope: &'static str,
    secondary_themes: Vec<&'static str>,
    place: String,
    prov: String,
    la: Option<f64>,
    lo: Option<f64>,
    latest_ts: f64,
    first_ts: f64,
    tokens: Vec<String>,
    items: Vec<usize>,
}

struct FinalCluster {
    theme: &'static str,
    tax_scope: &'static str,
    secondary_themes: Vec<&'static str>,
    urgency: &'static str,
    score: f64,
    latest_ts: f64,
    cross_verified: bool,
    json: Value,
}

pub fn build_pajak_clusters_value(
    news_items: &[Value],
    social_posts: &[Value],
    now: f64,
    win: &str,
    theme: &str,
    scope: &str,
) -> Value {
    let cutoff = window_cutoff(win, now);
    let items = normalize_items(news_items, social_posts, cutoff, now);
    let mut clusters: Vec<RawCluster> = Vec::new();

    for (idx, it) in items.iter().enumerate() {
        let mut best_idx: Option<usize> = None;
        let mut best_score = 0.0_f64;
        for (c_idx, cl) in clusters.iter().enumerate() {
            if cl.theme != it.theme || cl.tax_scope != it.tax_scope {
                continue;
            }
            if (cl.latest_ts - it.ts).abs() > 18.0 * 3_600_000.0 {
                continue;
            }
            let (shared, jaccard) = token_overlap(&it.tokens, &cl.tokens);
            let same_place = !it.place.is_empty()
                && !cl.place.is_empty()
                && it.place.to_lowercase() == cl.place.to_lowercase();
            let matched = shared >= 2 || jaccard >= 0.32 || (same_place && shared >= 1);
            if !matched {
                continue;
            }
            let sc = (shared as f64) * 2.0 + jaccard * 3.0 + if same_place { 1.0 } else { 0.0 };
            if sc > best_score {
                best_score = sc;
                best_idx = Some(c_idx);
            }
        }
        if let Some(bi) = best_idx {
            let best = &mut clusters[bi];
            best.items.push(idx);
            for w in &it.tokens {
                if best.tokens.len() < 24 && !best.tokens.contains(w) {
                    best.tokens.push(w.clone());
                }
            }
            if it.ts > best.latest_ts {
                best.latest_ts = it.ts;
            }
            if it.ts < best.first_ts {
                best.first_ts = it.ts;
            }
            if best.place.is_empty() && !it.place.is_empty() {
                best.place = it.place.clone();
                best.prov = it.prov.clone();
                best.la = it.la;
                best.lo = it.lo;
            }
            for &st in &it.secondary_themes {
                if st != best.theme && !best.secondary_themes.contains(&st) {
                    best.secondary_themes.push(st);
                }
            }
        } else {
            let cid = format!("pj-{}-{}", it.theme, clusters.len() + 1);
            clusters.push(RawCluster {
                id: cid,
                theme: it.theme,
                tax_scope: it.tax_scope,
                secondary_themes: it.secondary_themes.clone(),
                place: it.place.clone(),
                prov: it.prov.clone(),
                la: it.la,
                lo: it.lo,
                latest_ts: it.ts,
                first_ts: it.ts,
                tokens: it.tokens.iter().take(16).cloned().collect(),
                items: vec![idx],
            });
        }
    }

    let mut finalized: Vec<FinalCluster> = clusters
        .into_iter()
        .map(|cl| {
            let mut news_list: Vec<&NormItem> = cl
                .items
                .iter()
                .map(|&i| &items[i])
                .filter(|x| x.is_news)
                .collect();
            js::sort(&mut news_list, |a, b| b.ts - a.ts);
            let mut social_list: Vec<&NormItem> = cl
                .items
                .iter()
                .map(|&i| &items[i])
                .filter(|x| !x.is_news)
                .collect();
            js::sort(&mut social_list, |a, b| b.ts - a.ts);

            let src_set: HashSet<String> = cl
                .items
                .iter()
                .map(|&i| items[i].src.to_lowercase())
                .collect();
            let rep = news_list
                .first()
                .copied()
                .or_else(|| social_list.first().copied())
                .unwrap_or(&items[cl.items[0]]);
            let has_crit = cl.items.iter().any(|&i| items[i].urgent_crit);
            let has_warn = cl.items.iter().any(|&i| items[i].urgent_warn);
            let cross_verified = !news_list.is_empty() && !social_list.is_empty();
            let urgency = if has_crit || src_set.len() >= 4 {
                "kritis"
            } else if has_warn || cross_verified || src_set.len() >= 2 {
                "waspada"
            } else {
                "normal"
            };
            let urgency_rank = match urgency {
                "kritis" => 3.0,
                "waspada" => 2.0,
                _ => 1.0,
            };
            let scope_bonus = if cl.tax_scope == "nasional" { 60.0 } else { 0.0 };
            let score = urgency_rank * 100.0
                + scope_bonus
                + if cross_verified { 40.0 } else { 0.0 }
                + (src_set.len() as f64) * 12.0
                + (cl.items.len() as f64) * 4.0;

            let mut kw_count: HashMap<&str, usize> = HashMap::new();
            for &i in &cl.items {
                for w in &items[i].tokens {
                    *kw_count.entry(w.as_str()).or_insert(0) += 1;
                }
            }
            let mut kw_vec: Vec<(&str, usize)> = kw_count.into_iter().collect();
            kw_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            let keywords: Vec<Value> = kw_vec
                .into_iter()
                .take(5)
                .map(|(w, _)| Value::String(w.to_string()))
                .collect();

            let theme_meta = theme_by_id(cl.theme).unwrap_or(&PAJAK_THEMES[0]);
            let sec: Vec<&'static str> = cl.secondary_themes.into_iter().take(2).collect();
            let desc = if !rep.desc.is_empty() && rep.desc != rep.title {
                utf16_slice_prefix(&rep.desc, 240)
            } else {
                String::new()
            };
            let news_json: Vec<Value> = news_list
                .iter()
                .take(10)
                .map(|x| {
                    json!({
                        "title": x.title,
                        "url": x.url,
                        "src": x.src,
                        "ts": js::number(x.ts),
                        "place": x.place,
                    })
                })
                .collect();
            let social_json: Vec<Value> = social_list
                .iter()
                .take(10)
                .map(|x| {
                    json!({
                        "title": x.title,
                        "text": x.desc,
                        "url": x.url,
                        "src": x.src,
                        "handle": x.handle,
                        "author": x.author,
                        "platform": x.platform,
                        "provenance": x.provenance,
                        "ts": js::number(x.ts),
                        "place": x.place,
                    })
                })
                .collect();

            let val = json!({
                "id": cl.id,
                "theme": cl.theme,
                "themeLabel": theme_meta.label,
                "taxScope": cl.tax_scope,
                "taxScopeLabel": if cl.tax_scope == "daerah" { "Pajak Daerah" } else { "Pajak Nasional \u{00B7} DJP" },
                "secondaryThemes": sec,
                "urgency": urgency,
                "score": js::number(score),
                "title": rep.title,
                "desc": desc,
                "url": rep.url,
                "repKind": if rep.is_news { "news" } else { "social" },
                "place": cl.place,
                "prov": cl.prov,
                "la": cl.la.map(js::number).unwrap_or(Value::Null),
                "lo": cl.lo.map(js::number).unwrap_or(Value::Null),
                "latestTs": js::number(cl.latest_ts),
                "firstTs": js::number(cl.first_ts),
                "totalItems": cl.items.len(),
                "newsCount": news_list.len(),
                "socialCount": social_list.len(),
                "sourceCount": src_set.len(),
                "crossVerified": cross_verified,
                "keywords": keywords,
                "newsItems": news_json,
                "socialItems": social_json,
            });
            FinalCluster {
                theme: cl.theme,
                tax_scope: cl.tax_scope,
                secondary_themes: sec,
                urgency,
                score,
                latest_ts: cl.latest_ts,
                cross_verified,
                json: val,
            }
        })
        .collect();

    js::sort(&mut finalized, |a, b| {
        let diff = b.score - a.score;
        if diff != 0.0 && !diff.is_nan() {
            diff
        } else {
            b.latest_ts - a.latest_ts
        }
    });

    let filter_scope = if scope == "nasional" || scope == "daerah" {
        scope
    } else {
        ""
    };
    let scoped_items: Vec<&NormItem> = if !filter_scope.is_empty() {
        items
            .iter()
            .filter(|x| x.tax_scope == filter_scope)
            .collect()
    } else {
        items.iter().collect()
    };
    let scoped_clusters: Vec<&FinalCluster> = if !filter_scope.is_empty() {
        finalized
            .iter()
            .filter(|c| c.tax_scope == filter_scope)
            .collect()
    } else {
        finalized.iter().collect()
    };

    let total_items = scoped_items.len();
    let total_news = scoped_items.iter().filter(|x| x.is_news).count();
    let total_social = scoped_items.iter().filter(|x| !x.is_news).count();
    let total_nasional = items.iter().filter(|x| x.tax_scope == "nasional").count();
    let total_daerah = items.iter().filter(|x| x.tax_scope == "daerah").count();
    let nasional_clusters = finalized
        .iter()
        .filter(|c| c.tax_scope == "nasional")
        .count();
    let daerah_clusters = finalized.iter().filter(|c| c.tax_scope == "daerah").count();

    let themes: Vec<Value> = PAJAK_THEMES
        .iter()
        .map(|t| {
            let t_clusters: Vec<&FinalCluster> = scoped_clusters
                .iter()
                .copied()
                .filter(|c| c.theme == t.id)
                .collect();
            let t_items: Vec<&NormItem> = scoped_items
                .iter()
                .copied()
                .filter(|x| x.theme == t.id)
                .collect();
            let t_news = t_items.iter().filter(|x| x.is_news).count();
            let t_social = t_items.iter().filter(|x| !x.is_news).count();
            let t_nas = t_clusters
                .iter()
                .filter(|c| c.tax_scope == "nasional")
                .count();
            let t_dae = t_clusters
                .iter()
                .filter(|c| c.tax_scope == "daerah")
                .count();
            let urgent_count = t_clusters
                .iter()
                .filter(|c| c.urgency == "kritis" || c.urgency == "waspada")
                .count();

            let mut reg_order: Vec<(&str, usize)> = Vec::new();
            let mut reg_idx: HashMap<&str, usize> = HashMap::new();
            for it in &t_items {
                if !it.place.is_empty() {
                    if let Some(&pos) = reg_idx.get(it.place.as_str()) {
                        reg_order[pos].1 += 1;
                    } else {
                        reg_idx.insert(it.place.as_str(), reg_order.len());
                        reg_order.push((it.place.as_str(), 1));
                    }
                }
            }
            reg_order.sort_by_key(|a| std::cmp::Reverse(a.1));
            let top_regions: Vec<Value> = reg_order
                .into_iter()
                .take(3)
                .map(|(name, count)| json!({ "name": name, "count": count }))
                .collect();

            let share_pct = if total_items > 0 {
                js::math_round((t_items.len() as f64 / total_items as f64) * 100.0)
            } else {
                0.0
            };
            let latest_ts = if t_clusters.is_empty() {
                0.0
            } else {
                t_clusters
                    .iter()
                    .fold(f64::NEG_INFINITY, |acc, c| acc.max(c.latest_ts))
            };
            json!({
                "id": t.id,
                "label": t.label,
                "short": t.short,
                "desc": t.desc,
                "color": t.color,
                "lightColor": t.light_color,
                "totalItems": t_items.len(),
                "newsCount": t_news,
                "socialCount": t_social,
                "clusterCount": t_clusters.len(),
                "nasionalClusters": t_nas,
                "daerahClusters": t_dae,
                "urgentCount": urgent_count,
                "sharePct": js::number(share_pct),
                "latestTs": js::number(latest_ts),
                "topRegions": top_regions,
            })
        })
        .collect();

    let filter_theme = if !theme.is_empty() && theme_by_id(theme).is_some() {
        theme
    } else {
        ""
    };
    let visible_clusters: Vec<Value> = if !filter_theme.is_empty() {
        scoped_clusters
            .iter()
            .filter(|c| c.theme == filter_theme || c.secondary_themes.contains(&filter_theme))
            .take(80)
            .map(|c| c.json.clone())
            .collect()
    } else {
        scoped_clusters
            .iter()
            .take(80)
            .map(|c| c.json.clone())
            .collect()
    };

    let cross_verified_count = scoped_clusters.iter().filter(|c| c.cross_verified).count();
    let win_out = if matches!(win, "24h" | "today" | "2h") {
        win
    } else {
        "24h"
    };

    json!({
        "ok": true,
        "at": js::number(now),
        "win": win_out,
        "theme": filter_theme,
        "scope": filter_scope,
        "totalItems": total_items,
        "totalNews": total_news,
        "totalSocial": total_social,
        "totalNasional": total_nasional,
        "totalDaerah": total_daerah,
        "nasionalClusters": nasional_clusters,
        "daerahClusters": daerah_clusters,
        "totalClusters": scoped_clusters.len(),
        "crossVerifiedCount": cross_verified_count,
        "themes": themes,
        "clusters": visible_clusters,
    })
}
