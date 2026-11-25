//! The Rust port of `src/social.js`.

#![forbid(unsafe_code)]
#![allow(
    dead_code,
    unused_imports,
    clippy::duplicate_mod,
    clippy::collapsible_if,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

#[path = "../../indonesian-nlp-rs/src/analysis.rs"]
#[allow(dead_code)]
pub mod analysis;

#[path = "../../indonesian-nlp-rs/src/frames.rs"]
#[allow(dead_code)]
pub mod frames;

#[path = "../../geocode-rs/src/service.rs"]
#[allow(dead_code)]
pub mod geo_service;

#[path = "../../indonesian-nlp-rs/src/tables.rs"]
#[allow(dead_code)]
pub mod tables;

#[path = "topics_data.rs"]
pub mod topics_data;

use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

pub use tables::{
    ABBR_DOT_RE, CEREMONY_RE, FOREIGN_MARKER_RE, GEO_AUTHOR_CURRENT, GEO_AUTHOR_HOMETOWN,
    GEO_EVENT_LOCATION, GEO_HISTORICAL, GEO_MENTIONED_IN_PASSING, GEO_READER_LOCATION,
    HAS_CURRENT_TIME_RE, HIST_CLEAN_1_RE, HIST_CLEAN_2_RE, HIST_NOSTALGIA_RE, HISTORICAL_YEAR_RE,
    LOCAL_MACRO_RE, LOCAL_ORG_RE, URL_RE, WS_RE, fre, metaphor_blocks_topic_str, re,
};

pub static ASSESS_HISTORICAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(dulu|jaman dulu|zaman dulu|tempo dulu|sejarah|mengenang|kilas balik|tahun lalu|bulan lalu|pekan lalu|kemarin|last (?:year|month|week)|historical|anniversary|orde baru|orba|soeharto|suharto|perang dunia|world war|penjajahan|kolonial|(?:jaman|zaman|waktu|pas|saat|era|awal|masa|suasana|situasi|musim|periode|ketika|selama)\s+(?:covid(?:-?19)?|pandemi|lockdown|korona|corona)|(?:2019|2020|2021|2022)(?-u:\b)[^.!?\n]{0,40}(?-u:\b)(?:covid|pandemi|lockdown))(?-u:\b)",
    )
});

pub(crate) fn now_ms() -> f64 {
    std::env::var("RUST_ANYTHING_NOW_MS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .map_or_else(
            || {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
                    .floor()
            },
            |ms| ms as f64,
        )
}

pub(crate) fn current_year_utc() -> f64 {
    let ms = now_ms();
    let z = (ms / 86_400_000.0).floor() as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + i64::from(m <= 2)) as f64
}

pub(crate) fn is_historical_str(span_raw: &str, span_or_empty: &str, year: f64) -> bool {
    let has_current_time = HAS_CURRENT_TIME_RE.is_match(span_raw);
    let c1 = HIST_CLEAN_1_RE.replace_all(span_or_empty, " ");
    let cleaned = HIST_CLEAN_2_RE.replace_all(&c1, " ");
    if has_current_time && !HIST_NOSTALGIA_RE.is_match(span_raw) {
        return false;
    }
    if ASSESS_HISTORICAL_RE.is_match(&cleaned) {
        return true;
    }
    if let Some(caps) = HISTORICAL_YEAR_RE.captures(&cleaned)
        && let Some(m) = caps.get(1)
        && let Ok(y) = m.as_str().parse::<f64>()
    {
        return y < year;
    }
    false
}

#[path = "../../assessment-rs/src/service.rs"]
pub mod assessment_service;
pub use assessment_service::{AssessOptions, TopicSpec, assess_item};

fn js_str_or_empty(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(false) => String::new(),
        Value::Bool(true) => "true".to_string(),
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f == 0.0 || f.is_nan() {
                    String::new()
                } else {
                    n.to_string()
                }
            } else {
                n.to_string()
            }
        }
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Array(a) => js_str_or_empty(&Value::Array(a.clone())),
                Value::Object(_) => "[object Object]".to_string(),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

// Regex constants
static SOC_NONLATIN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\u{0E00}-\u{0E7F}\u{0E80}-\u{0EFF}\u{3040}-\u{30FF}\u{3400}-\u{4DBF}\u{4E00}-\u{9FFF}\u{AC00}-\u{D7AF}\u{0400}-\u{04FF}\u{0600}-\u{06FF}\u{0900}-\u{097F}\u{10A0}-\u{10FF}]").unwrap()
});
static SOC_VI_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ]").unwrap()
});
static SOC_SCRIPT_RES: LazyLock<Vec<(&'static str, Regex, usize)>> = LazyLock::new(|| {
    vec![
        ("kana", Regex::new(r"[぀-ヿ]").unwrap(), 3),
        ("hangul", Regex::new(r"[가-힯ᄀ-ᅟ]").unwrap(), 3),
        ("thai", Regex::new(r"[฀-๿]").unwrap(), 4),
        ("arabic", Regex::new(r"[؀-ۿݐ-ݿﭐ-﷿ﹰ-﻿]").unwrap(), 4),
        ("cyrillic", Regex::new(r"[Ѐ-ӿ]").unwrap(), 4),
        ("devanagari", Regex::new(r"[ऀ-ॿ]").unwrap(), 4),
        ("armenian", Regex::new(r"[԰-֏]").unwrap(), 4),
        ("hanzi", Regex::new(r"[㐀-䶿一-鿿]").unwrap(), 3),
    ]
});
static HANGUL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[가-힯ᄀ-ᅟ]").unwrap());
static KANA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[぀-ヿ]").unwrap());

static HTTP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://\S+").unwrap());
static AT_MENTION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"@[\w]+").unwrap());
static LATIN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-zÀ-ɏ]").unwrap());
static ASCII_LATIN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z]").unwrap());

static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#[\p{L}\p{N}_]+").unwrap());
static HASH_FULLWIDTH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"＃").unwrap());
static CASHTAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$[\p{L}\p{N}]{2,}").unwrap());
static CRYPTO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(crypto|kripto|bitcoin|btc|ethereum|airdrop|trading|cuan|investasi|signal|token|wallet)\b").unwrap()
});

static ASTRO_NON_ALNUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\p{L}\p{N} ]").unwrap());
static WS_MULTI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

static SOC_FOREIGN_WORDS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:tenho|jogar|jugar|voltar|estoy|avec|pour|dans|nicht|quoi|merci|obrigado|vuelta|habra|habrá|dado|povo|assusta|infidelite|infidélité|nouveau|nouvelle|variant\s+du|is\s+het|gaat\s+het|hebben\s+we|het\s+kabinet|uit\s+het|staat\s+op\s+vallen|gevallen|klappen|gestapt|nog\s+niet|nog\s+een|vvd|d66|pvv|tweede\s+kamer|niet\s+gevallen|zijn\s+er|waarom|wanneer|misschien|eigenlijk|gewoon|altijd|nooit|iemand|niemand|echt\s+niet|por\s+que|porque|pero|como\s+estas|tambien|también|mucho|todos|tiene|tienen|hacer|puede|pueden|cuando|donde|dónde|quien|quién|estos|estas|nosotros|ustedes|ellos|ellas|muito|voce|você|nao|não|entao|então|tambem|também|ainda|agora|sempre|nunca|ninguem|ninguém|alguem|alguém|c'est|qu'est|est-ce|sont|mais|plus|tout|tous|faire|peut|comme|cette|votre|notre|leurs|aussi|meme|même|jamais|toujours|personne|quelqu'un|aber|oder|wenn|weil|kein|keine|haben|werden|wurde|können|müssen|sagen|machen|dieser|diese|dieses|immer|wieder|heute|gestern|morgen|ang\s+mga|mga\s+tao|naman|talaga|bakit|paano|ganito|ganyan|kasi|para\s+sa|hindi\s+ko|wala\s+na|meron)\b").unwrap()
});
static SOC_FOREIGN_DIACRITIC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"[¿¡ñÑçÇßãõÃÕåøæÅØÆłńśźżŁŃŚŹŻčřšžČŘŠŽğışĞİŞ]|[áéíóúàèìòùâêîôûäöüëïÁÉÍÓÚÀÈÌÒÙÂÊÎÔÛÄÖÜËÏ]",
    )
    .unwrap()
});
static SOC_ID_UNAMBIGUOUS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:yang|yg|untuk|utk|dengan|dgn|sudah|udah|udh|belum|blm|tidak|bukan|ga|gak|nggak|engga|kagak|gue|gw|gua|aku|kami|kita|warga|orang|org|rumah|rmh|jalan|jln|banget|bgt|sih|dong|deh|kok|jir|anjir|njir|bjir|wkwk\w*|jujurly|kalo|kalau|karena|krn|tapi|tp|terus|trs|jadi|jd|masih|msh|lagi|lg|baru|mau|pengen|bisa|bikin|pakai|pake|kemarin|besok)\b").unwrap()
});
static SOC_LOAN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:caf[eé]s?|r[eé]sum[eé]s?|na[iï]ve|fianc[eé]e?s?|clich[eé]s?|d[eé]j[aà]\s*vu|voil[aà]|ol[eé]|[\u{00fc}U]ber)\b").unwrap()
});
static SOC_LOCAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(indonesia|\bwni\b|jakarta|jabar|jateng|jatim|jawa|sumatera|sumatra|kalimantan|sulawesi|papua|bali\b|aceh|riau|banten|bekasi|bogor|depok|tangerang|bandung|surabaya|medan|makassar|semarang|yogya|bpbd|bnpb|bmkg|polres|polda|polsek|kodim|damkar|pemkot|pemkab|kabupaten|provinsi|prabowo|gibran|\bdpr\b|dprd|kapolri|polri|\bdjp\b|ditjen pajak|direktorat jenderal pajak|kemenkeu|coretax|kring pajak|kanwil djp|\bkpp\b)").unwrap()
});
static SOC_FOREIGN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(amerika|united states|\busa\b|americans?|american|british|canadian|australian|japanese|chinese|korean|french|german|russian|indian|ukrainian|israeli|palestinian|malaysian|singaporean|filipino|vietnamese|brazilian|mexican|european|texas|california|new york|brasil|brazil|filipina|thailand|malaysia|korea|jepang|japan|tiongkok|china|inggris|britain|prancis|perancis|paris|france|jerman|germany|australia|india|pakistan|iran|israel|palestina|palestine|ukraina|ukraine|rusia|russia|gaza|turki|turkey|meksiko|mexico|kanada|canada|vietnam|chile|argentina|spanyol|spain|portugal|italia|italy|belanda|netherlands|den haag|s-gravenhage|tweede kamer|eerste kamer|vvd|cda|d66|pvv|fvd|nsc|gl-pvda|groenlinks|pvda|nederland|nederlands|holland|amsterdam|rotterdam|utrecht|eindhoven|groningen|kabinet rutte|kabinet schoof|dick schoof|geert wilders|swiss|swedia|sweden|norwegia|norway|denmark|yunani|greece|eropa|europe|nepal|london|oyo|osun|nigeria|afrika|trump|poland|polandia|polonia|krakow|warsaw|warszawa|lublin|czechia|ceko|prague|praha|austria|vienna|wina|finland|finlandia|helsinki|singapura|singapore|hong kong|taipei|tokyo|seoul|beijing|manila|bangkok|hanoi|kualalumpur|kathmandu|moskwa|moscow|berlin|rome|roma|madrid|amsterdam|brussels|zurich|stockholm|oslo|copenhagen|dublin|budapest|bucharest|sofia|belgrade|zagreb|vilnius|riga|tallinn|minsk|kiev|kyiv|istanbul|ankara|tehran|baghdad|beirut|riyadh|cairo|tunis|algiers|rabat|tripoli|khartoum|nairobi|kampala|kigali|lusaka|harare|pretoria|mogadishu|bamako|dakar|abuja|lagos|accra|kinshasa|sao paulo|rio de janeiro|brasilia|buenos aires|bogota|caracas|la paz|asuncion|montevideo|kingston|havana|santo domingo|paramaribo|managua|tegucigalpa|san salvador|san jose|guatemala city|mexico city|sucre|suriah|syria|irak|iraq|mesir|egypt|libya|libia|maroko|morocco|aljazair|algeria|tunisia|sudan|ethiopia|kenya|ghana|myanmar|kamboja|cambodia|laos|srilanka|sri lanka|bangladesh|lebanon|yordania|jordan|yaman|yemen|oman|qatar|uea|hungaria|hungary|rumania|romania|bulgaria|serbia|kroasia|croatia|slowakia|slovakia|slovenia|belgia|belgium|irlandia|ireland|estonia|latvia|lithuania|belarus|mongolia|maladewa|maldives|timor leste|selandia baru|new zealand|kuba|cuba|jamaika|jamaica|haiti|nikaragua|kosta rika|costa rica|kolombia|colombia|venezuela|bolivia|ekuador|ecuador|peru|paraguay|uruguay|congo|rwanda|uganda|tanzania|zambia|zimbabwe|mozambique|angola|namibia|botswana|lesotho|madagascar|mauritius|somalia|eritrea|djibouti|afghanistan|bhutan|brunei|albania|armenia|azerbaijan|cyprus|iceland|kosovo|luxembourg|malta|moldova|montenegro|san marino|vatikan|vatican|monako|monaco|georgia|jordan|panama|senegal|cameroon|gambia|guinea|sierra leone|liberia|mauritania|burkina faso|shah alam|selangor|bukit jelutong|subang jaya|petaling jaya|putrajaya|klang|penang|johor bahru|british columbia|tahsis|democrats|republicans|lebuhraya|plusline|llmtrafik|korang|macam ni|banjir tak|banjir ke|kesiannya|memandu|tanpa halangan banjir|terowong smart|m'sia|msia|sejuk beku|bekalan makanan|berbaki|penyokong|rombak kabinet|bubarkan parlimen|bubar parlimen|parlimen|timbalan perdana menteri|pakatan harapan|perikatan nasional|barisan nasional|umno|menteri besar|ketua menteri|dewan rakyat|dewan negara|kerajaan perpaduan|kerajaan madani|malaysia madani|varian baharu|baharu|dikesan|sejauh mana bahayanya|siakap keli|thesiakapkeli|banjir kilat|jabatan bomba|polis diraja|kawasan terjejas|hospital kerajaan|kementerian kesihatan)\b").unwrap()
});

static SOC_LISTING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(rumah|propert\w*|tanah|kavling|ruko|rukan|kost|kosan|apartemen|villa|cluster|dagung|bengkel|gudang|kios|petak|showroom|motogarage|otomotif|motor second|mobil second|sewa mobil|rental mobil|sparepart|bengkel modifikasi|civic|avanza|brio|innova|xpander|ertiga|vario|nmax|beat\s+\d{4}|matic\s+\d{4})").unwrap()
});
static SOC_LISTING_DEAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(dijual|mau jual|jual cepat|disewakan|disewa|menjual|menyewakan|over\s*kredit|kpr|dp\s?(nol|0)?\b|cash\s?(keras|cepat|bertahap)|nego|furnish|siap (huni|tempati|pakai|lanjut)|pajak\s+hidup|surat\s+lengkap|strategis|prime location|turun harga|ngotak|pinang|worth+|hemat\s+\d|buruan|mumpung|bonus\s+(car|cash)|cashback|payment plan|cicilan|booking fee|unit (tinggal|tersedia|ready)|hubungi (wa|whatsapp)|pre[- ]?order|ready stock|bebas request|request (warna|striping|konsep|modifikasi|custom)|modifikasi sesuai|custom (livery|striping|wrap)|garansi\b|quality check|diserahterimakan|serah terima)").unwrap()
});
static SOC_NOISE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(kebakaran jenggot|kebakaran (harga|promo|diskon|obral)|tersulut api (amarah|cemburu|emosi|dendam)|terbakar (api )?(amarah|cemburu|emosi|semangat|rindu|api asmara)|(?:membakar|sulut) (semangat|jiwa|emosi|amarah|nyali)|bakar (duit|uang|kalori|lemak)|gempa (politik|kabinet|pasar|saham|hati|rasa|jantung|lokal)|baby\s+gempa|boboiboy\s+gempa|baju\s+gempa|kaos\s+gempa|goyang\s+gempa|joget\s+gempa|lagu\s+gempa|bikin\s+gempa|mengguncang (jagat|dunia|medsos|panggung|publik|politik|parlemen|bursa|klasemen|pasar)|guncangan\s+(?:jiwa|batin|emosi|politik|ekonomi|keuangan)|(?:post[- ]?)?(?:blue|blauwe)[- ]?tsunami|ocean\s+tsunami|surfing\s+(?:a\s+)?tsunami|tsunami (phk|impor|produk|barang|utang|inflasi|suara|politik|protes|biru|vonis)|badai (krisis|resesi|phk|inflasi|keuangan|kebangkrutan|pemecatan|kehidupan|asmara|pasti berlalu|kritik|protes|kecaman|hujatan|komentar)|angin segar (bagi|untuk)|ledakan (kasus|pasien|dbd|wabah|penduduk|populasi|inflasi|harga|daya beli|pengangguran|utang|kemiskinan|emosi|amarah|tawa|tangis|protes|cuan)|perang (tarif|dagang|harga|diskon|promo|bintang|opini|narasi|argumen|tagar|buzzer|medsos|urat syaraf|dingin|kata[- ]kata)|saling serang (opini|argumen|narasi|cuitan|komentar)|adu jotos (anggota (dpr|dprd|dewan|parlemen)|politisi)|(?:dibanjiri|kebanjiran|banjir|membanjiri) (order|orderan|diskon|promo|pujian|notif|notifikasi|air\s?mata|ingus|berita|informasi|data|inbox|email|surel|wa|whatsapp|komen|komentar|dm|like|followers?|rezeki|cuan|berkah|hadiah|bonus|kado|tawaran|pesanan|chat|ucapan|kritik|kecaman|hujatan|turis|wisatawan|penonton|pelamar|pendaftar|peserta|gol|impor|produk|barang|tekstil|beras|tka|aktivitas|serangan\s+siber|serangan\s+cyber|\bpc\b|photocards?|merch|lightstick|fancam|konten|content|update|jadwal|kerjaan|tugas)|satu\s+album\s+banjir|\bdpr\s+(?:ian|live|cream|artic|rem|regime|squad|gang|crew|member)\b|\bdream\s+perfect\s+regime\b|\b(?:carat|army|blink|nctzen|briize|shotties|engene|moa|fearnot|midzy|neverland|stay|once|reveluv|sone|exo-?l|atinys?|monbebe|insomnia)\b[^.\n]{0,60}\b(?:album|\bpc\b|photocards?|comeback|fancam|bias|lightstick)\b|\bbem\b[^.\n]{0,80}\b(?:kode|css|html|modifier|element|frontend|styling|component)\b|\b(?:jogar|main|game|gaming|crashed|tome|grabs?|pick\s*ups?|crawled|dcing|perks?|killer|survivor|hooked|gen|pallets?|dead\s+by\s+daylight)\b[^.\n]{0,80}\bdbd\b|\bdbd\b[^.\n]{0,80}\b(?:jogar|main|game|gaming|crashed|tome|grabs?|pick\s*ups?|crawled|dcing|perks?|killer|survivor|hooked|gen|pallets?)\b|\bcrowcaster\.com\b|\bR[1-9]\s+#\d+\b[^.\n]{0,60}\$\d+\.\d+|\bot[4-9]\b|\bot1[0-3]\b|member\s+kangen|pembangkit\s+listrik\s+tenaga\s+kangen|(?:kyk|kaya|kayak|seperti|ibarat)\s+(?:nunggu|menunggu\s+)?(?:orang\s+hamil|pengumuman\s+snbt|snbt|sbmptn)|\b(?:nadia\s+omara|ku\s+racuni|bujuran\s+keracunan)\b|(?:suruh|disuruh)\s+jadi\s+anjing\s+rabies|\bjam\s+aja\s+di\s*korupsi\b|(?:anjir+|anjg|wkwk+|hahaha+)\b[^.\n]{0,40}\b(?:rabies|keracunan|korupsi)\b|\b(?:rabies|keracunan|korupsi)\b[^.\n]{0,40}\b(?:wkwk+|hahaha+|cringe)\b|\bblt\b[^.\n]{0,50}(?:sandwich|grilled\s+cheese|bacon|lettuce|tomato|bread|toast|burger|deli|mayo|lunch)|(?:sandwich|grilled\s+cheese|bacon|lettuce|tomato|bread|toast|burger|deli|mayo)\b[^.\n]{0,50}\bblt\b|(hati|dada|semangat|jiwa|emosi|cemburu|amarah)[^.\n]{0,12}terbakar|tenggelam dalam|hanyut dalam|terjun (bebas )?(ke dunia|ke politik|payung)|panggung\s+terbakar|terbakar\s+(?:panggung|semangat)|tabrakan\s+(?:logika|jadwal|agenda|kepentingan|nalar|argumen|ideologi)|menabrak\s+(?:aturan|logika|nalar|hukum|undang[- ]undang|konstitusi|etika|norma)|tergelincir\s+(?:lidah|dari\s+puncak|klasemen|posisi|jabatan)|jembatan\s+cinta\s+ambruk|keracunan\s+(?:cinta|drakor|kpop|belanja|olshop|tiktok|shopee|konten)|wabah\s+(?:judi|judol|korupsi|kebencian|cinta|rindu|hoaks|kebodohan|fitnah)|racun\s+(?:shopee|tiktok|medsos|belanja|dunia)|ditembak\s+mati\s+rasa|mati\s+rasa\s+hatiku|nembak\s+wifi|begal\s+(?:rekening|saldo|kuota|data|pulsa)|pembunuhan\s+karakter|sinopsis\s+film|review\s+film|trailer\s+film|rekomendasi\s+film|adegan\s+film|nonton\s+(?:bareng|nobar|film|movie|bioskop|tv|sinetron|drakor|anime|konser|netflix|youtube|yt|fancam|trailer|teaser|video|konten|rekaman|pertandingan|bola|live|eps\b|episode|season|ch\b|chapter)|(?:lagi|mau|abis|habis|pengen|sambil|ikut|diajak)\s+nonton|nonton(?:nya)?\s+di\s+(?:bioskop|netflix|cinema|cgv|xxi|cinepolis|vidio|wetv|viu|iqiyi|bstation|youtube|yt)|tiket\s+(?:nonton|konser)|adegan\s+(?:film|sinetron|drakor)|drakor|kpop|k-pop|fancam|webtoon|anime|episode|\beps\b|chapter|concert|ditembak\s+duluan|ditembak\s+(?:sama\s+)?(?:gebetan|doi|crush|cewek|cowok|temen|teman|orang)|nembak\s+(?:gebetan|doi|crush)|(?:ga|gak|nggak|belum|lagi\s+(?:ga|gak|nggak))\s+mau\s+pacaran|curhat\s+(?:pacaran|percintaan|gebetan)|nyatain\s+perasaan|nyatakan\s+perasaan|confess\s+(?:ke\s+)?crush|makan\s+malem\s+bareng[^.\n]{0,50}(?:didn['’]?t\s+want|feel\s+some\s+type|perasaan|baper)|obrolan[^.\n]{0,40}minta\s+maaf[^.\n]{0,40}pacaran|mantan\s+rp|anak\s+rp|pacar\s+rl|ngedrama\s+(?:kecelakaan|tabrakan|sakit|meninggal)|(?:setaun|setahun|\d+\s*(?:tahun|thn|bln|bulan))\s+pacaran[^.\n]{0,40}(?:putusin|diputusin|putus)|\b(?:mau\s+)?(?:gw|gua|gue|aku)\s+putusin\b)").unwrap()
});
static SOC_SPAM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(vote for|follow\s+[\w.@-]+\s*[+.&]?\s*follow|like\s*(dan|&|\+)\s*(follow|rt)|rt\s*(dan|&|\+)\s*follow|dm\s+(untuk|di|ke)\s*(order|harga|promo|info|respon|detail|ticket|tiket)|dm\s+ke\s+@|link\s+(di|ada di)\s+bio|cek\s+(link\s+)?di\s+bio|order\s+(via|melalui|di)\s+(wa|whatsapp|dm|bio|link)|order\s+(di|via)\s+bio|s\.shopee\.co\.id|shp\.ee|tokopedia\.link|(?:diskusiin|konsultasi)\s+project\s+kamu|(?:via\s+)?(?:dm|wa|whatsapp)\s+(?:atau\s+(?:whatsapp|wa|dm)\s+)?di\s+bio|linktree|lynk\.?id|beacons\.?ai|taplink|bio\.?link|kasih\s+follow|follow\s+balik|jangan\s+lupa\s+follow|\bfyp\b|foryou|new\s+followers?|boost\s+followers?|giveaway|gifted|berhadiah|klaim\s+(hadiah|reward|bonus)|winner\s+dm|pemenang\s+(dm|di\s+(comment|komentar))|tag\s+(teman|orang|friend)|sebutkan\s+\d+\s*teman|tag\s+\d+\s*teman|(like|comment|share|follow)\s*(dan|&|\+)\s*(like|comment|share|follow|rt)|\b\d{1,2}\s*\/\s*(thread|bagian|part|lanjutan)|thread\s*\/|🧵|lanjut\s+di\s+thread|baca\s+sampai\s+habis|qris[^.\n]{0,40}order|order[^.\n]{0,40}qris|binary\s+option|robot\s+trading|signal\s+(trading|emas|crypto|kripto|saham)|rekomendasi\s+(saham|crypto|kripto)|cuan\s+tiap\s+hari|airdrop|seed\s+phrase|private\s+key|withdraw\s+(cepat|instan|kilat)|(?:\bbutuh\b|\bjasa\b|\bcair\s+cepat|\btanpa\s+bi\s+checking|\blunasi|\bhapus\s+data|\bgestun)\s+pinjol|pinjol\s+(?:cair\s+cepat|mudah\s+cair|legal\s+tanpa|bunga\s+rendah)|(?:tawaran|promo|ajakan|gabung)\s+investasi\s+bodong|\bwtb\b|\bwts\b|jasa titip|\bjastip\b|jasa war|war tiket|titip (tiket|beli)|tetap\s+cep[ae]t\s+dan\s+terkoordinasi\s+dengan\s+baik|dikawal\s+penuh\s+pemerintah|#penanganankarhutla|ngenalin\s+(?:temen|teman)\s+aku|kenalan\s+(?:yuk|dong)|cari\s+(?:jodoh|pasangan|pacar|suami|istri|kriteria)|kriteria\s*:\s*\d|cv\s+taaruf|biodata\s+taaruf|\btaaruf\b|siap\s+nikah|biro\s+jodoh|teman\s+kencan|cari\s+(?:cowok|cewek)|kriteria\s+(?:pasangan|calon|suami|istri)|tidak\s+merokok[^.\n]{0,40}tidak\s+judi|adakah\s+rekomendasi|butuh\s+rekomendasi|info\s+rekomendasi|spill\s+rekomendasi|minta\s+rekomendasi|(?:ada\s+|adakah\s+)?rekomendasi\s+(?:jasa|tempat|toko|salon|bengkel|laundry|klinik|dokter|cafe|kost|kosan)|jasa\s+(?:cleaning|kebersihan|bersih[- ]bersih|sedot\s+wc|service\s+ac|servis\s+ac|pindahan|laundry|cuci\s+sepatu|poles|makeup|mua|catering|katering|setrika|massage|pijat|joki)|cleaning\s+(?:rumah|kantor|apartment|apartemen)|bukan\s+perorangan[^.\n]{0,40}dramanya|joki\s+(?:tugas|skripsi|makalah|turnitin|kuis|ujian)|jual\s+(?:akun|follower|saldo)|sewa\s+akun|\b(?:janda|bispak|open\s*bo|michat|bokep|sange|colmek|coli|booking\s*cewek)\b|\bcrowcaster\.com\b|\bR[1-9]\s+#\d+\b[^.\n]{0,60}\$\d+\.\d+|\bot[4-9]\b|\bot1[0-3]\b|member\s+kangen|pembangkit\s+listrik\s+tenaga\s+kangen|(?:kyk|kaya|kayak|seperti|ibarat)\s+(?:nunggu|menunggu\s+)?(?:orang\s+hamil|pengumuman\s+snbt|snbt|sbmptn)|\b(?:nadia\s+omara|ku\s+racuni|bujuran\s+keracunan)\b|(?:suruh|disuruh)\s+jadi\s+anjing\s+rabies|\bjam\s+aja\s+di\s*korupsi\b|(?:anjir+|anjg|wkwk+|hahaha+)\b[^.\n]{0,40}\b(?:rabies|keracunan|korupsi)\b|\b(?:rabies|keracunan|korupsi)\b[^.\n]{0,40}\b(?:wkwk+|hahaha+|cringe)\b|ditembak\s+duluan|ditembak\s+(?:sama\s+)?(?:gebetan|doi|crush|cewek|cowok|temen|teman|orang)|nembak\s+(?:gebetan|doi|crush)|(?:ga|gak|nggak|belum|lagi\s+(?:ga|gak|nggak))\s+mau\s+pacaran|curhat\s+(?:pacaran|percintaan|gebetan)|nyatain\s+perasaan|nyatakan\s+perasaan|confess\s+(?:ke\s+)?crush|makan\s+malem\s+bareng[^.\n]{0,50}(?:didn['’]?t\s+want|feel\s+some\s+type|perasaan|baper)|obrolan[^.\n]{0,40}minta\s+maaf[^.\n]{0,40}pacaran|mantan\s+rp|anak\s+rp|pacar\s+rl|ngedrama\s+(?:kecelakaan|tabrakan|sakit|meninggal)|(?:setaun|setahun|\d+\s*(?:tahun|thn|bln|bulan))\s+pacaran[^.\n]{0,40}(?:putusin|diputusin|putus)|\b(?:mau\s+)?(?:gw|gua|gue|aku)\s+putusin\b)").unwrap()
});

static SOC_SEA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(kapal|laut|selat|ombak|nelayan|feri|perahu|sungai|perairan|samudra|ocean|boat|ship|tpi|dermaga)").unwrap()
});
static SOC_MOTOR_MARK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)grand\s*prix|grandprix|formula\s*1|\bf1\b|kualifikasi|qualifying|sprint\s+race|pole\s+position|podium|parc\s*ferme|safety\s*car|safetycar|red\s+flag|bendera\s+(?:merah|kuning)|pit\s+(?:stop|lane)|push-?lap|out-?lap|drs\s*zone|ers\s*zone|oversteer|understeer|lock-?up|downforce|aerodinamis|steering\s+wheel|cockpit|\bsirkuit\b|scuderia|red\s+bull(?:\s+racing)?|mercedes-amg|aston\s+martin|alpine\s+f1|haas\s+f1|racing\s+bulls|sauber|cadillac\s+f1|\bmclaren\b|balap\s+(?:motor|mobil|formula|f1|grup)").unwrap()
});

static SOC_BOILERPLATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:bmkg\s+disclaimer|disclaimer|prakirawan|pemutakhiran|episentrum|magnitudo|kedalaman|kedlmn|tenggara|barat\s+laut|barat\s+daya|timur\s+laut|timur\s+daya|kab|kec|wib|wita|wit|mmi|info\s+gempa|gempa\s+magnitudo|tidak\s+berpotensi)\b").unwrap()
});

static COMPILED_TOPICS: LazyLock<Vec<TopicSpec>> = LazyLock::new(|| {
    topics_data::STATIC_TOPICS
        .iter()
        .map(|t| TopicSpec {
            id: t.id.to_string(),
            pos: Regex::new(&format!("(?i){}", t.pos)).unwrap(),
            strong: t.strong.map(|s| Regex::new(&format!("(?i){}", s)).unwrap()),
            ctx: t.ctx.map(|s| Regex::new(&format!("(?i){}", s)).unwrap()),
            neg: t.neg.map(|s| Regex::new(&format!("(?i){}", s)).unwrap()),
            adhoc: false,
        })
        .collect()
});

macro_rules! word_set {
    ($s:expr) => {
        LazyLock::new(|| $s.split_whitespace().collect())
    };
}

static OFFICIAL_HANDLES: LazyLock<HashSet<&'static str>> = word_set!(
    "ditjenpajakri kring_pajak kemenkeuri infobmkg bnpb_indonesia basarnas damkardki \
     tmcpoldametro halo_polisi divhumas_polri puspentni kemkesri bssn_ri pvmbg_ pvmbg \
     bpbd_dki bpbdjabar bpbdjatim bpbdjateng pln_123 infobps kemenhub151 komdigi_id kemenkominfo"
);
static NEWS_OUTLET_HANDLES: LazyLock<HashSet<&'static str>> = word_set!(
    "detikcom kompascom kompastv cnnindonesia cnbcindonesia antaranews liputan6 tempodotco \
     tribunnews tribunjogja republikaonline merdekadotcom sindonews okezonecom jawapos \
     inewsdotid tvonenews metrotv elshinta elshinta90fm suarasurabayamedia kumparancom \
     vivanews bisniscom kontannews rri_co_id beritasatu idntimes tirtoid vice_id ddtcnews ortaxid"
);
static OFFICIAL_HANDLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:info)?(?:bmkg|bnpb|bpbd|basarnas|pvmbg|bssn|damkar|polres|polda|polsek|tmcpolda|divhumas|puspen|kemkes|kemenkes|kemenhub|komdigi|pemprov|pemkot|pemkab|ditjenpajak|kring_?pajak|kemenkeu|kanwildjp|kpp)(?:[._-]|$)"
    ).unwrap()
});
static NEWS_HANDLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:detik|kompas|cnnind|cnbcind|antara|liputan6|tempo|tribun|republika|merdeka|sindo|okezone|jawapos|inews|tvone|metrotv|elshinta|suarasurabaya|kumparan|beritasatu|rri)(?:com|tv|id|news|dotco|fm|media)?$"
    ).unwrap()
});
static RE_RT_CHECK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^[\s\u{FEFF}]*rt[\s\u{FEFF}]+@").unwrap());
static REPLY_START_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*@\w+").unwrap());
static SOC_KW_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z][a-z'-]{3,}").unwrap());
static SOC_TSTOP: LazyLock<HashSet<&'static str>> = word_set!(
    "yang dan di ke dari untuk pada dengan adalah ini itu ada tidak akan juga telah sudah bisa lebih sangat para dia kami kita mereka sebuah seorang suatu saat ketika setelah sebelum hingga oleh karena maka jika hari minggu senin selasa rabu kamis jumat sabtu januari februari maret april mei juni juli agustus september oktober november desember breaking news terbaru update foto video live hasil wib wit daring online pemerintah menteri presiden indonesia daerah warga polisi damkar bpbd dishub pasar korban evakuasi massa jumlah ribu juta ratusan ribuan jutaan miliar triliun total resmi viral heboh terduga diduga kini paling orang persen masyarakat pihak oknum pelaku tersangka ibu bapak kali pemkot polda polres kejaksaan kemensos kemenkeu kemendagri detik cnn tempo liputan6 republika tribun cnbc antara jawapos okezone kumparan suara merdeka sindonews jpnn tvri inews media news kompas viva metrotvnews rmol infopublik jabarprov korankaltim tvone merahputih poskota beritasatu gridoto wowkeren bola bolaoverza mediaindonesia faktualnews berita foto video gallery wawancara investigasi khusus daerah nasional internasional ekonomi hiburan selebriti olahraga dishub cumplik the and for with that this from have has are was were will would can could should not but all any more than then when what which who how its our your their his her"
);

fn clean_handle(post: &Value) -> String {
    let raw = ["handle", "username", "user", "author"]
        .iter()
        .find_map(|k| post.get(k).and_then(Value::as_str));
    let s = raw.unwrap_or("").to_lowercase();
    s.trim().strip_prefix('@').unwrap_or(&s).trim().to_string()
}

fn classify_provenance_val(post: &Value, assessment: Option<&Value>) -> Value {
    let handle = clean_handle(post);
    let a = assessment
        .filter(|v| !v.is_null())
        .or_else(|| post.get("assessment").filter(|v| !v.is_null()));
    let text = post.get("text").and_then(Value::as_str).unwrap_or("");

    let (source_type, reason) = if !handle.is_empty()
        && (OFFICIAL_HANDLES.contains(handle.as_str()) || OFFICIAL_HANDLE_RE.is_match(&handle))
    {
        ("official_agency", "verified_official_handle")
    } else if !handle.is_empty()
        && (NEWS_OUTLET_HANDLES.contains(handle.as_str()) || NEWS_HANDLE_RE.is_match(&handle))
    {
        ("news_outlet", "recognized_news_handle")
    } else if a.and_then(|av| av.get("intent")).and_then(Value::as_str) == Some("eyewitness_report")
        && !RE_RT_CHECK.is_match(text)
    {
        ("eyewitness", "firsthand_eyewitness_markers")
    } else if a
        .and_then(|av| av.get("event"))
        .and_then(|ev| ev.get("sourceType"))
        .and_then(Value::as_str)
        == Some("eyewitness")
        && !RE_RT_CHECK.is_match(text)
    {
        ("eyewitness", "eyewitness_event_framing")
    } else {
        ("anonymous", "unverified_social_account")
    };

    let weight = match source_type {
        "official_agency" => 1.0,
        "news_outlet" => 0.85,
        "eyewitness" => 0.72,
        _ => 0.35,
    };

    json!({
        "sourceType": source_type,
        "authorityWeight": weight,
        "handle": if handle.is_empty() { String::new() } else { format!("@{handle}") },
        "reason": reason,
        "engagementExcludedFromCredibility": true,
    })
}

fn soc_is_motorsport(t: &str) -> bool {
    let mut matches = HashSet::new();
    for m in SOC_MOTOR_MARK_RE.find_iter(t) {
        matches.insert(m.as_str().to_lowercase());
    }
    matches.len() >= 2
}

fn soc_fix_events(mut res: Value) -> Value {
    if let Some(findings) = res.get_mut("findings").and_then(Value::as_array_mut) {
        for f in findings.iter_mut() {
            let ev = f.get("evidence").and_then(Value::as_str).unwrap_or("");
            let topic = f.get("topic").and_then(Value::as_str).unwrap_or("");
            let ftype = f.get("type").and_then(Value::as_str).unwrap_or("");
            if topic == "insiden" && ftype == "maritime_accident" && !SOC_SEA_RE.is_match(ev) {
                f["type"] = json!("kecelakaan");
            } else if topic == "kecelakaan"
                && (ev.contains("tenggelam") || ev.contains("karam") || ev.contains("hanyut"))
                && SOC_SEA_RE.is_match(ev)
            {
                f["type"] = json!("maritime_accident");
            } else if topic == "wabah" {
                f["type"] = json!("health_incident");
                if f.get("category").is_none_or(|c| c.is_null()) {
                    f["category"] = json!("kesehatan");
                }
            }
        }
    }
    if let Some(events) = res.get_mut("events").and_then(Value::as_array_mut) {
        for f in events.iter_mut() {
            let ev = f.get("evidence").and_then(Value::as_str).unwrap_or("");
            let topic = f.get("topic").and_then(Value::as_str).unwrap_or("");
            let ftype = f.get("type").and_then(Value::as_str).unwrap_or("");
            if topic == "insiden" && ftype == "maritime_accident" && !SOC_SEA_RE.is_match(ev) {
                f["type"] = json!("kecelakaan");
            } else if topic == "kecelakaan"
                && (ev.contains("tenggelam") || ev.contains("karam") || ev.contains("hanyut"))
                && SOC_SEA_RE.is_match(ev)
            {
                f["type"] = json!("maritime_accident");
            } else if topic == "wabah" {
                f["type"] = json!("health_incident");
                if f.get("category").is_none_or(|c| c.is_null()) {
                    f["category"] = json!("kesehatan");
                }
            }
        }
    }
    res
}

/// Port of `socAssess` (`src/social.js:419`).
pub fn soc_assess(
    text: serde_json::Value,
    extra: serde_json::Value,
    context: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let t = js_str_or_empty(&text);
    let listing = SOC_LISTING_RE.is_match(&t) && SOC_LISTING_DEAL.is_match(&t);
    let is_motor = soc_is_motorsport(&t);

    let mut topics: Vec<TopicSpec> = COMPILED_TOPICS
        .iter()
        .filter(|tp| !is_motor || tp.id != "kecelakaan")
        .cloned()
        .collect();

    if let Value::Array(extra_arr) = extra {
        for item in extra_arr {
            if let Some(obj) = item.as_object() {
                let id = obj
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if id.is_empty() {
                    continue;
                }
                let pos_re = if let Some(p) = obj.get("pos").and_then(Value::as_str) {
                    Regex::new(&format!("(?i){p}")).unwrap_or_else(|_| Regex::new("$^").unwrap())
                } else {
                    Regex::new(&format!(r"(?i)\b{}\b", regex::escape(&id)))
                        .unwrap_or_else(|_| Regex::new("$^").unwrap())
                };
                topics.push(TopicSpec {
                    id,
                    pos: pos_re,
                    strong: None,
                    ctx: None,
                    neg: None,
                    adhoc: true,
                });
            }
        }
    }

    let opts = AssessOptions {
        year: None,
        listing,
        spam: Some(SOC_SPAM_RE.clone()),
        noise: Some(SOC_NOISE_RE.clone()),
    };

    let mut res = soc_fix_events(assess_item(&t, &topics, &opts));
    let prov = classify_provenance_val(&context, Some(&res));
    res["provenance"] = prov;
    Ok(res)
}

/// Port of `socTopics` (`src/social.js:436`).
pub fn soc_topics(
    text: serde_json::Value,
    extra: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let res = soc_assess(text, extra, Value::Null)?;
    Ok(res.get("topics").cloned().unwrap_or(json!([])))
}

/// Port of `socAdhoc` (`src/social.js:467`).
pub fn soc_adhoc(phrases: serde_json::Value) -> Result<serde_json::Value, String> {
    let arr = match &phrases {
        Value::Array(a) => a,
        Value::Null => &[][..],
        _ => {
            if let Value::Number(n) = &phrases {
                if n.as_f64() == Some(0.0) {
                    return Ok(json!([]));
                }
            }
            return Err("(phrases || []).map is not a function".to_string());
        }
    };

    static SOC_ADHOC_GENERIC: LazyLock<HashSet<&'static str>> = word_set!(
        "rumah warga orang anak pria wanita ibu bapak mobil motor jalan tanah lahan gedung kantor desa kota hari bulan tahun waktu tempat kerja loker jasa harga biaya uang pasar toko produk barang sekolah kampus kuliah teman keluarga makan minum"
    );

    let mut out = Vec::new();
    for ph_val in arr {
        let s = js_str_or_empty(ph_val);
        let s = s.replace(['"', '\'', '\r', '\n', '\t'], " ");
        let s = WS_MULTI_RE.replace_all(&s, " ");
        let ph = s.trim();
        if ph.len() < 3 {
            continue;
        }
        let low = ph.to_lowercase();
        if SOC_ADHOC_GENERIC.contains(low.as_str()) && !ph.contains(' ') {
            continue;
        }
        if SOC_BOILERPLATE_RE.is_match(&low) || SOC_NOISE_RE.is_match(&low) {
            continue;
        }
        if analysis::is_generic_location_phrase_str(&low) {
            continue;
        }
        out.push(ph.to_string());
        if out.len() == 4 {
            break;
        }
    }

    let res: Vec<Value> = out
        .into_iter()
        .map(|ph| {
            json!({
                "id": ph.to_lowercase(),
                "adhoc": true,
                "local": false,
                "pos": {},
                "x": [format!("\"{}\" -dijual -kpr -rumah -sewa -loker -promo -giveaway -drakor -kpop -filter:replies", ph)],
                "th": [ph],
            })
        })
        .collect();
    Ok(Value::Array(res))
}

/// Port of `socKeepX` (`src/social.js:489`).
pub fn soc_keep_x(p: serde_json::Value) -> Result<serde_json::Value, String> {
    if p.is_null() {
        return Err("Cannot read properties of null (reading 'text')".to_string());
    }
    let text = p.get("text").and_then(Value::as_str).unwrap_or("");
    let likes = p
        .get("likes")
        .or_else(|| p.get("like_count"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let topics_val = p
        .get("topics")
        .or_else(|| p.get("assessment").and_then(|a| a.get("topics")));
    let topics: Vec<&str> = topics_val
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let assessment = p.get("assessment");
    let signal = assessment
        .and_then(|a| a.get("signalEligible"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if let Some(a) = assessment {
        if a.get("version").and_then(Value::as_str) == Some(assessment_service::VERSION)
            && a.get("feedEligible").and_then(Value::as_bool) == Some(false)
        {
            return Ok(Value::Bool(false));
        }
    }

    let name = p.get("name").and_then(Value::as_str).unwrap_or("");
    if !signal
        && !name.is_empty()
        && (HANGUL_RE.is_match(name) || KANA_RE.is_match(name))
        && !topics.is_empty()
        && topics
            .iter()
            .all(|t| matches!(*t, "pendidikan" | "hukum" | "bansos"))
    {
        return Ok(Value::Bool(false));
    }

    if REPLY_START_RE.is_match(text) && likes < 10.0 && topics.is_empty() && !signal {
        return Ok(Value::Bool(false));
    }

    let mentions_count = AT_MENTION_RE.find_iter(text).count();
    if mentions_count >= 3 && likes < 20.0 && topics.is_empty() && !signal {
        return Ok(Value::Bool(false));
    }

    if !signal && SOC_LISTING_RE.is_match(text) && SOC_LISTING_DEAL.is_match(text) {
        return Ok(Value::Bool(false));
    }

    Ok(Value::Bool(true))
}

/// Port of `socKeepTh` (`src/social.js:504`).
pub fn soc_keep_th(p: serde_json::Value) -> Result<serde_json::Value, String> {
    if p.is_null() {
        return Err("Cannot read properties of null (reading 'assessment')".to_string());
    }
    let assessment = p.get("assessment");
    if let Some(a) = assessment {
        if a.get("version").and_then(Value::as_str) == Some(assessment_service::VERSION)
            && a.get("feedEligible").and_then(Value::as_bool) == Some(false)
        {
            return Ok(Value::Bool(false));
        }
    }

    let is_reply = p.get("is_reply").and_then(Value::as_bool).unwrap_or(false);
    let like_count = p.get("like_count").and_then(Value::as_f64).unwrap_or(0.0);
    let topics_len = p
        .get("topics")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    if is_reply && like_count < 5.0 && topics_len == 0 {
        return Ok(Value::Bool(false));
    }

    let signal = assessment
        .and_then(|a| a.get("signalEligible"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let text = p.get("text").and_then(Value::as_str).unwrap_or("");
    if !signal && SOC_LISTING_RE.is_match(text) && SOC_LISTING_DEAL.is_match(text) {
        return Ok(Value::Bool(false));
    }

    Ok(Value::Bool(true))
}

/// Port of `socBalance` (`src/social.js:511`).
pub fn soc_balance(
    arr: serde_json::Value,
    max: serde_json::Value,
    user_of: serde_json::Value,
    per_topic: Option<f64>,
    per_user: Option<f64>,
    user_cap: Option<f64>,
) -> Result<serde_json::Value, String> {
    let max_val = match &max {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::Bool(true) => 1.0,
        Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };
    let arr_vec = match &arr {
        Value::Array(a) => a,
        _ => return Ok(json!([])),
    };

    if max_val <= 0.0 || max_val.is_nan() || arr_vec.is_empty() {
        return Ok(json!([]));
    }

    // In JS: userOf(p) throws if userOf is not a function
    if !user_of.is_string() {
        return Err("userOf is not a function".to_string());
    }

    let per_topic = per_topic.unwrap_or(0.22);
    let per_user = per_user.unwrap_or(3.0);
    let user_cap = user_cap.unwrap_or(10.0);

    let max_len = max_val as usize;
    let cap_t = ((max_val.min(arr_vec.len() as f64) * per_topic).ceil() as usize).max(8);
    let mut tc: HashMap<String, usize> = HashMap::new();
    let mut uc: HashMap<String, usize> = HashMap::new();
    let mut ug: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Value> = Vec::new();

    let is_sig = |p: &Value| {
        p.get("assessment")
            .and_then(|a| a.get("signalEligible"))
            .and_then(Value::as_bool)
            == Some(true)
    };
    let is_rep = |p: &Value| {
        is_sig(p)
            || p.get("assessment")
                .and_then(|a| a.get("reportType"))
                .and_then(Value::as_str)
                == Some("reported")
    };

    let user_prop = user_of.as_str().unwrap_or("user");
    let get_user = |p: &Value| -> String {
        p.get(user_prop)
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_lowercase()
    };

    for pass in 0..3 {
        for p in arr_vec {
            let matches_pass = match pass {
                0 => is_sig(p),
                1 => !is_sig(p) && is_rep(p),
                _ => !is_sig(p) && !is_rep(p),
            };
            if !matches_pass {
                continue;
            }

            let tps: Vec<String> = p
                .get("topics")
                .and_then(Value::as_array)
                .map(|a| {
                    let mut s = HashSet::new();
                    a.iter()
                        .filter_map(Value::as_str)
                        .filter(|x| s.insert(*x))
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let u = get_user(p);

            if tps.is_empty()
                || tps.iter().any(|t| {
                    *tc.get(t).unwrap_or(&0) >= cap_t
                        || *uc.get(&format!("{u}|{t}")).unwrap_or(&0) >= per_user as usize
                })
            {
                continue;
            }
            if !is_sig(p) && user_cap > 0.0 && *ug.get(&u).unwrap_or(&0) >= user_cap as usize {
                continue;
            }

            for t in &tps {
                *tc.entry(t.clone()).or_insert(0) += 1;
                *uc.entry(format!("{u}|{t}")).or_insert(0) += 1;
            }
            if !is_sig(p) {
                *ug.entry(u).or_insert(0) += 1;
            }
            out.push(p.clone());
            if out.len() >= max_len {
                return Ok(Value::Array(out));
            }
        }
    }

    Ok(Value::Array(out))
}

/// Port of `socQueries` (`src/social.js:531`).
pub fn soc_queries(
    kind: serde_json::Value,
    extra: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out: Vec<Value> = Vec::new();

    // Check if extra has length
    let extra_has_len = match &extra {
        Value::Array(a) => !a.is_empty(),
        Value::String(s) => !s.is_empty(),
        _ => false,
    };

    // First list: static topics
    let kind_str = match &kind {
        Value::String(s) => s.as_str(),
        Value::Number(n) if n.as_f64() == Some(0.0) => "0",
        _ => "",
    };

    for tp in topics_data::STATIC_TOPICS {
        let queries: &[&str] = match kind_str {
            "x" => tp.x,
            "th" => tp.th,
            _ => &[],
        };
        for q in queries {
            out.push(json!({ "t": tp.id, "q": q }));
        }
    }

    // Now extra items
    if extra_has_len {
        match extra {
            Value::Array(arr) => {
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let id = obj.get("id").and_then(Value::as_str);
                        let qs = obj.get(kind_str).and_then(Value::as_array);
                        if let Some(q_arr) = qs {
                            for q in q_arr {
                                if let Some(qs_str) = q.as_str() {
                                    out.push(json!({ "t": id, "q": qs_str }));
                                }
                            }
                        }
                    } else if let Some(s) = item.as_str() {
                        if kind_str == "0" && !s.is_empty() {
                            let first_char = &s[..1];
                            out.push(json!({
                                "t": { "$undefined": true },
                                "q": first_char,
                            }));
                        }
                    }
                }
            }
            Value::String(s) => {
                // When extra is a string (e.g. "x") and kind is 0, in JS ("x")[0] === "x", ("x").id is undefined
                if kind_str == "0" && !s.is_empty() {
                    let first_char = &s[..1];
                    out.push(json!({
                        "t": { "$undefined": true },
                        "q": first_char,
                    }));
                }
            }
            _ => {}
        }
    }

    Ok(Value::Array(out))
}

/// Port of `socHot` (`src/social.js:539`).
pub fn soc_hot(
    posts: serde_json::Value,
    now: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let posts_arr = match &posts {
        Value::Array(a) => a,
        Value::String(_) => {
            return Ok(json!({ "stats": {}, "hot": [] }));
        }
        _ => return Err("posts is not iterable".to_string()),
    };

    let now_ms_val = match now {
        Some(Value::Number(n)) => n.as_f64().unwrap_or_else(now_ms),
        Some(Value::Null) => 0.0,
        _ => now_ms(),
    };

    let w_ms: f64 = 90.0 * 60.0 * 1000.0;
    let b_ms: f64 = 24.0 * 3600.0 * 1000.0;
    let windows = (b_ms / w_ms).ceil() as usize;

    struct Entry {
        x: usize,
        th: usize,
        rx: HashMap<String, f64>,
        rt: HashMap<String, f64>,
        buckets: HashMap<usize, HashMap<String, f64>>,
        has_weighted: bool,
    }

    let mut st: HashMap<String, Entry> = HashMap::new();

    for p in posts_arr {
        let ts = p.get("ts").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let age = now_ms_val - ts;
        if !ts.is_finite() || ts <= 0.0 || age < 0.0 || age > b_ms {
            continue;
        }
        let platform = p.get("platform").and_then(Value::as_str).unwrap_or("");
        if platform != "x" && platform != "th" {
            continue;
        }

        let assessment = p.get("assessment");
        let has_assess = assessment.is_some_and(|a| a.is_object());
        if let Some(a) = assessment.filter(|a| a.is_object()) {
            if a.get("feedEligible").and_then(Value::as_bool) == Some(false) {
                continue;
            }
        }

        let mut w = 1.0;
        if let Some(a) = assessment.filter(|a| a.is_object()) {
            let is_ctx = a.get("feedLane").and_then(Value::as_str) == Some("situational_context")
                || a.get("reportType").and_then(Value::as_str) == Some("situational_context");
            let sig_w = if a.get("signalEligible").and_then(Value::as_bool) == Some(true) {
                1.5
            } else if is_ctx {
                0.5
            } else {
                1.0
            };
            let raw_prov_w = a.get("provenance").and_then(|pr| {
                pr.get("authorityWeight")
                    .or_else(|| pr.get("weight"))
                    .and_then(Value::as_f64)
            });
            let prov_w = match raw_prov_w {
                Some(pw) if pw.is_finite() => pw.clamp(0.6, 1.5),
                _ => 1.0,
            };
            w = sig_w * prov_w;
            if a.get("reportType").and_then(Value::as_str) == Some("reported")
                && a.get("signalEligible").and_then(Value::as_bool) != Some(true)
            {
                w *= 0.85;
            }
        }

        let author = p
            .get("user")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let author = author.strip_prefix('@').unwrap_or(&author).to_string();

        let topics = p
            .get("topics")
            .and_then(Value::as_array)
            .map(|arr| {
                let mut s = HashSet::new();
                arr.iter()
                    .filter_map(Value::as_str)
                    .filter(|t| s.insert(*t))
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for t in topics {
            let e = st.entry(t).or_insert_with(|| Entry {
                x: 0,
                th: 0,
                rx: HashMap::new(),
                rt: HashMap::new(),
                buckets: HashMap::new(),
                has_weighted: false,
            });

            if platform == "x" {
                e.x += 1;
            } else {
                e.th += 1;
            }
            if has_assess {
                e.has_weighted = true;
            }
            if author.is_empty() {
                continue;
            }

            if age < w_ms {
                let m = if platform == "x" {
                    &mut e.rx
                } else {
                    &mut e.rt
                };
                let old = m.get(&author).copied().unwrap_or(0.0);
                m.insert(author.clone(), old.max(w));
            } else {
                let bucket = (age / w_ms).floor() as usize;
                let users = e.buckets.entry(bucket).or_default();
                let k = format!("{platform}:{author}");
                let old = users.get(&k).copied().unwrap_or(0.0);
                users.insert(k, old.max(w));
            }
        }
    }

    let mut stats = Map::new();
    let mut hot = Vec::new();

    for (t, e) in st {
        stats.insert(t.clone(), json!({ "x": e.x, "th": e.th }));
        let users = e.rx.len() + e.rt.len();
        let both = !e.rx.is_empty() && !e.rt.is_empty();
        if users < 3 {
            continue;
        }

        let w_recent: f64 = e.rx.values().sum::<f64>() + e.rt.values().sum::<f64>();
        let w_base: f64 = e.buckets.values().map(|m| m.values().sum::<f64>()).sum();
        let base_rate = w_base / ((windows - 1) as f64);
        let cross_mult = if e.has_weighted && both { 1.15 } else { 1.0 };
        let score = (w_recent * cross_mult) / (1.0 + base_rate);
        let score_rounded = (score * 100.0).round() / 100.0;

        hot.push(json!({
            "id": t,
            "users": users,
            "both": both,
            "score": score_rounded,
            "coverage": "sampled",
            "unit": "platform_accounts",
        }));
    }

    hot.sort_by(|a, b| {
        let sa = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let sb = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        sb.total_cmp(&sa)
    });
    hot.truncate(6);

    Ok(json!({
        "stats": stats,
        "hot": hot,
    }))
}

/// Port of `socNonLatin` (`src/social.js:28`).
pub fn soc_non_latin(t: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_str_or_empty(&t);
    let s = HTTP_RE.replace_all(&s, " ");
    let s = AT_MENTION_RE.replace_all(&s, " ");

    for (_name, re, min) in SOC_SCRIPT_RES.iter() {
        if re.find_iter(&s).count() >= *min {
            return Ok(Value::Bool(true));
        }
    }

    let nl = SOC_NONLATIN_RE.find_iter(&s).count();
    let lat = LATIN_RE.find_iter(&s).count();
    Ok(Value::Bool(nl >= 20 && nl > lat))
}

/// Port of `socVi` (`src/social.js:39`).
pub fn soc_vi(t: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_str_or_empty(&t);
    let s = HTTP_RE.replace_all(&s, " ");
    let s = AT_MENTION_RE.replace_all(&s, " ");

    let vi = SOC_VI_RE.find_iter(&s).count();
    let lat = ASCII_LATIN_RE.find_iter(&s).count();

    let cond1 = vi >= 15 && vi * 4 >= lat;
    let cond2 = vi >= 8 && vi * 3 >= lat;
    Ok(Value::Bool(cond1 || cond2))
}

/// Port of `socHashJunk` (`src/social.js:46`).
pub fn soc_hash_junk(t: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_str_or_empty(&t);
    let s = HTTP_RE.replace_all(&s, " ");

    let tags: Vec<&str> = TAG_RE.find_iter(&s).map(|m| m.as_str()).collect();
    let fullwidth_count = HASH_FULLWIDTH_RE.find_iter(&s).count();
    let n = tags.len() + fullwidth_count;
    let cashtags = CASHTAG_RE.find_iter(&s).count();

    if n > 10 {
        return Ok(Value::Bool(true));
    }
    if cashtags >= 3 && CRYPTO_RE.is_match(&s) {
        return Ok(Value::Bool(true));
    }

    let s_len = s.encode_utf16().count();
    let chars: usize = tags.iter().map(|h| h.encode_utf16().count()).sum();
    let ratio = (chars as f64) / (s_len.max(1) as f64);

    if n >= 8 && ratio > 0.28 {
        return Ok(Value::Bool(true));
    }
    if n >= 5 && s_len <= 80 && ratio > 0.30 {
        return Ok(Value::Bool(true));
    }
    Ok(Value::Bool(s_len > 60 && ratio > 0.4))
}

/// Port of `astroturfSig` (`src/social.js:60`).
pub fn astroturf_sig(t: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_str_or_empty(&t).to_lowercase();
    let s = HTTP_RE.replace_all(&s, " ");
    let s = AT_MENTION_RE.replace_all(&s, " ");
    let s = TAG_RE.replace_all(&s, " ");
    let s = ASTRO_NON_ALNUM.replace_all(&s, " ");
    let s = WS_MULTI_RE.replace_all(&s, " ");
    let trimmed = s.trim();
    let u = crate::utf16::units(trimmed);
    Ok(Value::String(crate::utf16::slice(&u, 0, 160)))
}

fn detect_whatlang(clean: &str) -> &'static str {
    if clean.chars().count() < 10 {
        return "und";
    }
    match whatlang::detect(clean) {
        Some(info) if info.is_reliable() => match info.lang() {
            whatlang::Lang::Nob => "nor",
            other => other.code(),
        },
        _ => "und",
    }
}

/// Port of `socForeignLang` (`src/social.js:74`).
pub fn soc_foreign_lang(t: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_str_or_empty(&t);
    let s = HTTP_RE.replace_all(&s, " ");
    let s = AT_MENTION_RE.replace_all(&s, " ");
    let s = s.trim();

    let id_unambiguous_count = SOC_ID_UNAMBIGUOUS_RE.find_iter(s).count();
    let has_local_entity = SOC_LOCAL_RE.is_match(s);
    let has_id_context = has_local_entity || id_unambiguous_count > 0;

    if SOC_FOREIGN_WORDS_RE.is_match(s) && !has_local_entity && id_unambiguous_count < 2 {
        return Ok(Value::Bool(true));
    }

    let s_no_loan = SOC_LOAN_RE.replace_all(s, " ");
    if SOC_FOREIGN_DIACRITIC_RE.is_match(&s_no_loan)
        && !has_local_entity
        && id_unambiguous_count < 2
    {
        return Ok(Value::Bool(true));
    }

    if has_id_context {
        return Ok(Value::Bool(false));
    }
    if s.len() < 20 {
        return Ok(Value::Bool(false));
    }

    let lang = detect_whatlang(s);
    static SOC_LANG_OK: LazyLock<HashSet<&'static str>> =
        word_set!("ind zlm eng und sun min jav mad ban bug mak ace");

    let is_ok = SOC_LANG_OK.contains(lang);
    Ok(Value::Bool(!is_ok))
}

/// Port of `socKeywordMine` (`src/social.js:690`).
pub fn soc_keyword_mine(
    posts: serde_json::Value,
    topic_id: serde_json::Value,
    arg2: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if arg2.is_null() {
        return Err("Cannot read properties of null (reading 'now')".to_string());
    }

    let id = js_str_or_empty(&topic_id).to_lowercase();
    let topic_def = topics_data::STATIC_TOPICS.iter().find(|t| t.id == id);
    let Some(topic) = topic_def else {
        return Ok(json!([]));
    };

    let hay = format!("{} {}", topic.th.join(" "), topic.x.join(" ")).to_lowercase();

    let now_val = arg2
        .get("now")
        .and_then(Value::as_f64)
        .unwrap_or_else(now_ms);
    let window_ms = arg2
        .get("windowMs")
        .and_then(Value::as_f64)
        .unwrap_or(6.0 * 3600.0 * 1000.0);
    let max_terms = arg2
        .get("maxTerms")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(8);

    let posts_arr = match &posts {
        Value::Array(a) => a,
        _ => return Ok(json!([])),
    };

    let mut df: HashMap<String, usize> = HashMap::new();
    let mut scanned = 0;

    for p in posts_arr {
        if scanned >= 400 {
            break;
        }
        let ts = p.get("ts").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let age = now_val - ts;
        if !ts.is_finite() || ts <= 0.0 || age < 0.0 || age > window_ms {
            continue;
        }

        let tps: Vec<String> = p
            .get("topics")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_lowercase())
                    .collect()
            })
            .unwrap_or_default();
        let q = p
            .get("q")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();

        if !tps.contains(&id) && q != id {
            continue;
        }
        scanned += 1;

        let text = p.get("text").and_then(Value::as_str).unwrap_or("");
        let no_http = HTTP_RE.replace_all(text, " ");
        let clean_text = AT_MENTION_RE.replace_all(&no_http, " ");

        if soc_hash_junk(json!(clean_text.as_ref())).is_ok_and(|v| v == Value::Bool(true))
            || soc_non_latin(json!(clean_text.as_ref())).is_ok_and(|v| v == Value::Bool(true))
            || soc_vi(json!(clean_text.as_ref())).is_ok_and(|v| v == Value::Bool(true))
        {
            continue;
        }

        let lower = clean_text.to_lowercase();
        let mut seen = HashSet::new();
        for m in SOC_KW_RE.find_iter(&lower) {
            let term = m.as_str().trim_matches(&['\'', '-'][..]);
            if term.len() < 4 || !seen.insert(term.to_string()) {
                continue;
            }
            if SOC_TSTOP.contains(term) || SOC_FOREIGN_RE.is_match(term) {
                continue;
            }
            if hay.contains(term) {
                continue;
            }
            *df.entry(term.to_string()).or_insert(0) += 1;
        }
    }

    let need = 3.max((scanned as f64 * 0.01).ceil() as usize);
    let mut candidates: Vec<(String, usize)> = df.into_iter().filter(|(_, n)| *n >= need).collect();
    candidates.sort_by_key(|a| std::cmp::Reverse(a.1));
    candidates.truncate(max_terms);

    let res: Vec<Value> = candidates
        .into_iter()
        .map(|(term, df_n)| json!({ "term": term, "df": df_n }))
        .collect();
    Ok(Value::Array(res))
}

/// Port of `explainPost` (`src/social.js:438`).
pub fn explain_post(
    text: serde_json::Value,
    extra: serde_json::Value,
    context: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let t_len = js_str_or_empty(&text).encode_utf16().count();
    let res = soc_assess(text, extra, context)?;

    let clauses = res
        .get("layerTrace")
        .and_then(|lt| lt.get("normalization"))
        .and_then(|n| n.get("clauses"))
        .cloned()
        .unwrap_or(json!(0));

    Ok(json!({
        "classifierVersion": res.get("version").cloned().unwrap_or(Value::Null),
        "version": res.get("version").cloned().unwrap_or(Value::Null),
        "textLength": t_len,
        "normalizedText": res.get("normalizedText").cloned().unwrap_or(Value::Null),
        "contentHash": res.get("contentHash").cloned().unwrap_or(Value::Null),
        "intent": res.get("intent").cloned().unwrap_or(Value::Null),
        "feedLane": res.get("feedLane").cloned().unwrap_or(Value::Null),
        "signalEligible": res.get("signalEligible").cloned().unwrap_or(Value::Null),
        "feedEligible": res.get("feedEligible").cloned().unwrap_or(Value::Null),
        "confidence": res.get("confidence").cloned().unwrap_or(Value::Null),
        "classification": res.get("classification").cloned().unwrap_or(Value::Null),
        "geographicContext": res.get("geographicContext").cloned().unwrap_or(Value::Null),
        "event": res.get("event").cloned().unwrap_or(Value::Null),
        "extractedEvents": res.get("extractedEvents").cloned().unwrap_or(Value::Null),
        "provenance": res.get("provenance").cloned().unwrap_or(Value::Null),
        "topics": res.get("topics").cloned().unwrap_or(Value::Null),
        "reasons": res.get("reasons").cloned().unwrap_or(Value::Null),
        "rejectedReasons": res.get("rejectedReasons").cloned().unwrap_or(Value::Null),
        "clauses": clauses,
        "layerTrace": res.get("layerTrace").cloned().unwrap_or(Value::Null),
    }))
}
