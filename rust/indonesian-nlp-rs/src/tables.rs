#![forbid(unsafe_code)]

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

pub fn re(pat: &str) -> Regex {
    // INFALLIBLE: all regex patterns in this module are compile-time static literals verified by unit tests.
    Regex::new(pat).expect("valid static regex")
}

pub fn fre(pat: &str) -> FancyRegex {
    // INFALLIBLE: all fancy-regex patterns in this module are compile-time static literals verified by unit tests.
    FancyRegex::new(pat).expect("valid static fancy-regex")
}

pub const INTENTS: &[&str] = &[
    "incident_report",
    "official_warning",
    "official_update",
    "eyewitness_report",
    "personal_experience",
    "discussion",
    "opinion",
    "personal_vent",
    "question",
    "commercial_listing",
    "advertisement",
    "spam",
    "educational",
    "historical",
    "satire",
    "rumor",
    "denial_or_debunk",
    "unrelated",
    "unknown",
];

pub const PRIORITY_ORDER: &[&str] = &[
    "spam",
    "commercial_listing",
    "advertisement",
    "personal_vent",
    "educational",
    "historical",
    "satire",
    "denial_or_debunk",
    "official_warning",
    "question",
    "rumor",
    "eyewitness_report",
    "incident_report",
    "official_update",
    "personal_experience",
    "discussion",
    "opinion",
    "unrelated",
    "unknown",
];

pub const GEO_EVENT_LOCATION: &str = "event_location";
pub const GEO_AUTHOR_CURRENT: &str = "author_current_location";
pub const GEO_AUTHOR_HOMETOWN: &str = "author_hometown";
pub const GEO_READER_LOCATION: &str = "reader_location";
pub const GEO_MENTIONED_IN_PASSING: &str = "mentioned_in_passing";
pub const GEO_NEWS_SOURCE: &str = "news_source_location";
pub const GEO_HISTORICAL: &str = "historical_location";
pub const GEO_AMBIGUOUS: &str = "ambiguous_entity";
pub const GEO_UNKNOWN: &str = "unknown";

pub static SLANG_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("ngga", "tidak"),
        ("nggak", "tidak"),
        ("engga", "tidak"),
        ("enggak", "tidak"),
        ("gak", "tidak"),
        ("ga", "tidak"),
        ("gk", "tidak"),
        ("kaga", "tidak"),
        ("kagak", "tidak"),
        ("ndak", "tidak"),
        ("ora", "tidak"),
        ("teu", "tidak"),
        ("tdk", "tidak"),
        ("tak", "tidak"),
        ("blm", "belum"),
        ("acan", "belum"),
        ("durung", "belum"),
        ("bkn", "bukan"),
        ("gw", "saya"),
        ("gue", "saya"),
        ("gua", "saya"),
        ("aku", "saya"),
        ("aq", "saya"),
        ("sy", "saya"),
        ("lu", "kamu"),
        ("lo", "kamu"),
        ("loe", "kamu"),
        ("elu", "kamu"),
        ("org", "orang"),
        ("wrg", "warga"),
        ("rmh", "rumah"),
        ("omah", "rumah"),
        ("imah", "rumah"),
        ("dln", "jalan"),
        ("dalan", "jalan"),
        ("jln", "jalan"),
        ("udh", "sudah"),
        ("udah", "sudah"),
        ("sdh", "sudah"),
        ("wis", "sudah"),
        ("geus", "sudah"),
        ("skrg", "sekarang"),
        ("skg", "sekarang"),
        ("saiki", "sekarang"),
        ("ayeuna", "sekarang"),
        ("kmrn", "kemarin"),
        ("kmaren", "kemarin"),
        ("wingi", "kemarin"),
        ("kamari", "kemarin"),
        ("biyen", "dulu"),
        ("baheula", "dulu"),
        ("td", "tadi"),
        ("mlm", "malam"),
        ("yg", "yang"),
        ("dgn", "dengan"),
        ("dlm", "dalam"),
        ("utk", "untuk"),
        ("u", "untuk"),
        ("krn", "karena"),
        ("karna", "karena"),
        ("bgt", "banget"),
        ("bngt", "banget"),
        ("cuma", "hanya"),
        ("cuman", "hanya"),
        ("cm", "hanya"),
        ("cmn", "hanya"),
        ("bener", "benar"),
        ("beneran", "benar"),
        ("emg", "memang"),
        ("emang", "memang"),
        ("jd", "jadi"),
        ("jgn", "jangan"),
        ("msh", "masih"),
        ("bbrp", "beberapa"),
        ("gmn", "bagaimana"),
        ("gimana", "bagaimana"),
        ("knp", "kenapa"),
        ("dmn", "dimana"),
        ("kpn", "kapan"),
        ("brp", "berapa"),
        ("sblm", "sebelum"),
        ("stlh", "setelah"),
        ("bnyk", "banyak"),
        ("sm", "sama"),
        ("dr", "dari"),
        ("klo", "kalau"),
        ("kl", "kalau"),
        ("klw", "kalau"),
        ("ujan", "hujan"),
        ("udan", "hujan"),
        ("kebakar", "terbakar"),
        ("kobongan", "kebakaran"),
        ("kahuruan", "kebakaran"),
        ("kejebak", "terjebak"),
        ("kerasa", "terasa"),
        ("keseret", "terseret"),
        ("lindhu", "gempa"),
        ("caah", "banjir"),
        ("banyu", "air"),
        ("cai", "air"),
        ("caina", "airnya"),
        ("ambrol", "ambruk"),
        ("mlebu", "masuk"),
        ("asup", "masuk"),
        ("munggah", "naik"),
        ("cepet", "cepat"),
        ("cepet2", "cepat"),
        ("flooding", "banjir"),
        ("flooded", "terendam banjir"),
        ("earthquake", "gempa"),
        ("landslide", "longsor"),
        ("wildfire", "karhutla"),
        ("collapsed", "ambruk"),
        ("collapse", "ambruk"),
        ("evacuation", "evakuasi"),
        ("evacuated", "dievakuasi"),
        ("explosion", "ledakan"),
        ("exploded", "meledak"),
        ("shooting", "penembakan"),
        ("robbery", "perampokan"),
        ("blackout", "listrik padam"),
    ])
});

pub static GEO_ABBR_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("jkt", "jakarta"),
        ("jaksel", "jakarta selatan"),
        ("jaktim", "jakarta timur"),
        ("jakbar", "jakarta barat"),
        ("jakut", "jakarta utara"),
        ("jakpus", "jakarta pusat"),
        ("bdg", "bandung"),
        ("sby", "surabaya"),
        ("smg", "semarang"),
        ("bks", "bekasi"),
        ("bgr", "bogor"),
        ("tgr", "tangerang"),
        ("tangsel", "tangerang selatan"),
        ("mks", "makassar"),
        ("mdn", "medan"),
        ("dps", "denpasar"),
        ("plg", "palembang"),
        ("jogja", "yogyakarta"),
        ("yogya", "yogyakarta"),
    ])
});

pub static HASHTAG_LEXICON: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut v = vec![
        "jakarta selatan",
        "jakarta timur",
        "jakarta barat",
        "jakarta utara",
        "jakarta pusat",
        "tangerang selatan",
        "bandung",
        "surabaya",
        "semarang",
        "makassar",
        "yogyakarta",
        "palembang",
        "denpasar",
        "balikpapan",
        "banjarmasin",
        "pontianak",
        "pekanbaru",
        "samarinda",
        "manado",
        "jayapura",
        "mataram",
        "kupang",
        "kendari",
        "palu",
        "jakarta",
        "bekasi",
        "depok",
        "bogor",
        "tangerang",
        "medan",
        "aceh",
        "bali",
        "jatim",
        "jateng",
        "jabar",
        "sumut",
        "sumbar",
        "sumsel",
        "kaltim",
        "kalbar",
        "kalsel",
        "sulsel",
        "papua",
        "garut",
        "cianjur",
        "sukabumi",
        "cirebon",
        "malang",
        "kebumen",
        "lampung",
        "riau",
        "jambi",
        "banten",
        "solo",
        "jogja",
        "ntb",
        "ntt",
        "kebakaranhutan",
        "banjirbandang",
        "putingbeliung",
        "tangguljebol",
        "unjukrasa",
        "aksimassa",
        "anginkencang",
        "serangansiber",
        "kebocorandata",
        "keracunanmassal",
        "coretax",
        "ditjenpajak",
        "direktoratjenderalpajak",
        "kringpajak",
        "laporpajak",
        "laporspt",
        "fakturpajak",
        "efaktur",
        "ebupot",
        "pajakkuat",
        "kanwildjp",
        "kebakaran",
        "karhutla",
        "banjir",
        "longsor",
        "gempa",
        "tsunami",
        "erupsi",
        "kecelakaan",
        "tabrakan",
        "ledakan",
        "ambruk",
        "penembakan",
        "pencurian",
        "perampokan",
        "begal",
        "demo",
        "ricuh",
        "bentrok",
        "wabah",
        "keracunan",
        "listrik",
        "padam",
        "ransomware",
        "peretasan",
        "phishing",
        "loker",
        "dijual",
        "disewa",
        "promo",
        "diskon",
        "info",
        "update",
        "berita",
        "waspada",
        "siaga",
        "darurat",
        "hujan",
        "badai",
        "cuaca",
        "bmkg",
        "bpbd",
        "bnpb",
        "damkar",
        "timur",
        "barat",
        "selatan",
        "utara",
        "pusat",
        "hari",
        "ini",
        "terkini",
        "indonesia",
        "kota",
        "rumah",
        "warga",
    ];
    v.sort_by_key(|a| std::cmp::Reverse(a.len()));
    v
});

pub static GENERIC_GEO_TOKENS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "indonesia",
        "ri",
        "nkri",
        "nusantara",
        "nasional",
        "daerah",
        "wilayah",
        "provinsi",
        "kabupaten",
        "kota",
        "kecamatan",
        "desa",
        "kelurahan",
        "pulau",
        "jawa",
        "sumatera",
        "sumatra",
        "kalimantan",
        "sulawesi",
        "papua",
        "maluku",
        "nusa tenggara",
        "jabodetabek",
        "jabar",
        "jateng",
        "jatim",
        "dkijakarta",
        "dki jakarta",
        "jogja",
        "yogya",
        "jkt",
        "jaksel",
        "jaktim",
        "jakbar",
        "jakut",
        "jakpus",
        "bdg",
        "sby",
        "smg",
        "bks",
        "bgr",
        "tgr",
        "tangsel",
        "dps",
        "mks",
        "mdn",
    ])
});

pub static EMOJI_CUES: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    vec![
        ("alert", re("[🚨⚠️🆘📢❗‼️🔴🟠]")),
        ("flood", re("[🌊🌧️⛈️☔]")),
        ("fire", re("[🔥🚒]")),
        ("mockery", re("[😂🤣🤡🙄😜😏]")),
        ("commercial", re("[🛒💰💵💸🏷️✅📦🏠🔑🏡]")),
        ("emotional", re("[😭💔😞😔😩😫🥺😢]")),
        ("fandom", re("[💜❤️💕💖✨🎤🎟️]")),
    ]
});

pub static METAPHOR_FRAMES: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    vec![
        (
            "metaphor_banjir",
            re(
                r"(?i)(?-u:\b)(?:dibanjiri|membanjiri)(?-u:\b)|(?-u:\b)(?:banjir|kebanjiran)\s+(?:order|orderan|cuan|rezeki|berkah|hadiah|bonus|promo|diskon|ucapan|pujian|penghargaan|kritik|kecaman|hujatan|komentar|aduan|laporan|air\s*mata|gol|darah|impor|produk|turis|wisatawan|penonton|pelamar|pendaftar|peserta|serangan\s+siber|serangan\s+cyber|pesan|chat|dm|email|notif|konten|content|update|jadwal|kerjaan|tugas)(?-u:\b)",
            ),
        ),
        (
            "metaphor_ledakan",
            re(
                r"(?i)(?-u:\b)(?:harga|tarif|biaya|ongkos|sewa|tiket(?:\s+pesawat|\s+kereta|\s+konser)?|rumah|properti|penduduk|populasi|kasus|pasien|penderita|order|orderan|penjualan|omzet|saham|profit|bisnis|cuan|inflasi|utang|kemiskinan|emosi|amarah|tawa|tangis|popularitas|traffic|trafik|subscribers?|followers?|viewers?|komentar)(?-u:\b)[^.!?\n]{0,55}(?-u:\b)(?:meledak|ledakan)(?-u:\b)|(?-u:\b)(?:ledakan|meledak)(?-u:\b)[^.!?\n]{0,40}(?-u:\b)(?:harga|tarif|tiket|penduduk|populasi|kasus|pasien|order|orderan|penjualan|omzet|bisnis|popularitas|tawa|tangis|emosi|amarah|di\s+pasaran)(?-u:\b)",
            ),
        ),
        (
            "metaphor_panas",
            re(
                r"(?i)(?-u:\b)(?:politik|suasana|debat|opini|diskusi|persaingan|laga|pertandingan|derby|hati|kepala|makanan|kopi|teh|soto|bakso|cuaca)(?-u:\b)[^.!?\n]{0,20}(?-u:\b)(?:panas|memanas)(?-u:\b)|(?-u:\b)(?:panas|memanas)(?-u:\b)[^.!?\n]{0,20}(?-u:\b)(?:politik|persaingan|debat|hati|asam|lambung)(?-u:\b)",
            ),
        ),
        (
            "metaphor_jatuh",
            re(
                r"(?i)(?-u:\b)(?:harga|saham|ihsg|rupiah|kurs|nilai|poin|peringkat|klasemen|mental|wibawa|reputasi|cinta|hati|tempo|miskin)(?-u:\b)[^.!?\n]{0,20}(?-u:\b)(?:jatuh|ambruk|anjlok|runtuh)(?-u:\b)|(?-u:\b)(?:jatuh|ambruk|anjlok|runtuh)(?-u:\b)[^.!?\n]{0,20}(?-u:\b)(?:cinta|tempo|miskin|hati|jiwa|perasaan|logika|kurs|saham|ihsg|rupiah|harga|rezim|koalisi|suara)(?-u:\b)",
            ),
        ),
        (
            "metaphor_bakar",
            re(
                r"(?i)(?-u:\b)(?:kebakaran\s+jenggot|kebakaran\s+(?:harga|promo|diskon|obral)|(?:ihsg|saham|portofolio|porto|market|bursa|kripto|crypto)\s+kebakaran|terbakar\s+(?:api\s+)?(?:amarah|cemburu|emosi|semangat|rindu|asmara)|membakar\s+(?:semangat|duit|uang|kalori|lemak)|panggung\s+terbakar)(?-u:\b)|(?-u:\b)(?:ayam|nasi|ikan|sate|roti|steak|jagung|bakso|kuliner|masak|resep|wajan)(?-u:\b)[^.!?\n]{0,20}(?-u:\b)(?:bakar|terbakar)(?-u:\b)",
            ),
        ),
        (
            "metaphor_tenggelam",
            re(
                r"(?i)(?-u:\b)(?:tenggelam|hanyut)\s+(?:dalam|oleh)\s+(?:kenangan|lamunan|duka|kesedihan|nostalgia|alunan|suasana|musik|lagu|emosi|perasaan|senyuman|tatapan|cinta|pesona|dosa)(?-u:\b)|(?-u:\b)tenggelam\s+di\s+dasar\s+klasemen(?-u:\b)",
            ),
        ),
        (
            "metaphor_bencana_politik",
            re(
                r"(?i)(?-u:\b)(?:tsunami|gempa|badai|longsor)\s+(?:biru|politik|kabinet|protes|suara|pemilu|dukungan|saham|cuan|phk|vonis|kritik|kecaman|hujatan|asmara|cinta|kehidupan|hidup|cedera|pasti\s+berlalu)(?-u:\b)|(?-u:\b)(?:guncangan|mengguncang)\s+(?:politik|kabinet|ekonomi|jiwa|batin|jagat|dunia\s+maya|netizen|medsos|publik|parlemen|panggung)(?-u:\b)",
            ),
        ),
        (
            "metaphor_crime",
            re(
                r"(?i)(?-u:\b)(?:pencuri|maling|rampok|begal)(?-u:\b)[^.!?\n]{0,18}(?-u:\b)(?:hati|cinta|perhatian|senyum|pandangan|poin|start|rekening|saldo|data|partai|konstitusi)(?-u:\b)|(?-u:\b)(?:dibunuh|membunuh)(?-u:\b)[^.!?\n]{0,18}(?-u:\b)(?:rasa|rindu|sepi|waktu|kebosanan|karakter|mimpi|harapan|cinta)(?-u:\b)|(?-u:\b)(?:ditembak|menembak)(?-u:\b)[^.!?\n]{0,25}(?-u:\b)(?:gebetan|doi|crush|pacar|pacaran|hati|cinta|mati\s+rasa|wifi|sim|gawang)(?-u:\b)",
            ),
        ),
        (
            "metaphor_siber_or_wabah",
            re(
                r"(?i)(?-u:\b)(?:humans?|manusia|jiwa|otak|hati)(?-u:\b)[^.!?\n]{0,30}(?-u:\b)(?:malware|virus)(?-u:\b)|(?-u:\b)wabah\s+(?:judi|judol|korupsi|kebencian|cinta|rindu|hoaks|fitnah|drakor|k-?pop|tiktok)(?-u:\b)|(?-u:\b)racun\s+(?:tiktok|shopee|ig|belanja|skincare|cinta|asmara)(?-u:\b)",
            ),
        ),
        (
            "metaphor_diamankan_noncrime",
            re(
                r"(?i)(?-u:\b)(?:data|akun|aset|uang|dokumen|file|arsip|berkas|tiket|kursi|juara|poin|kemenangan|medali)(?-u:\b)[^.!?\n]{0,18}(?-u:\b)(?:diamankan)(?-u:\b)",
            ),
        ),
        (
            "metaphor_bocor_nonincident",
            re(
                r"(?i)(?-u:\b)(?:atap|genteng|ban|ember|pipa\s+air|rahasia|soal\s+ujian|kunci\s+jawaban|skor|bocoran\s+film)(?-u:\b)[^.!?\n]{0,15}(?-u:\b)(?:bocor|kebocoran)(?-u:\b)|(?-u:\b)(?:bocor)\s+(?:halus|alusan)(?-u:\b)",
            ),
        ),
        (
            "nonincident_sport_entertainment",
            re(
                r"(?i)(?-u:\b)(?:dead\s+by\s+daylight|main\s+dbd|play\s+dbd|jogar\s+dbd|grand\s+prix|formula\s+1|safety\s+car|sinopsis\s+film|review\s+film|trailer\s+film|cdrama|drachin|jdrama|boboiboy|baby\s+gempa|baju\s+gempa|kaos\s+gempa|goyang\s+gempa|joget\s+gempa|sid\s+bukan\s+pahlawan|bermahkotakan\s+duri|emyu|munyuk|bobotoh\s+persib|mantan\s+rp|pacar\s+rl)(?-u:\b)|(?-u:\b)(?:game|gaming|crashed|tome|grabs?|pick\s*ups?|crawled|dcing|perks?|killer|survivor|hooked|gen|pallets?)(?-u:\b)[^.!?\n]{0,80}(?-u:\b)dbd(?-u:\b)|(?-u:\b)dbd(?-u:\b)[^.!?\n]{0,80}(?-u:\b)(?:game|gaming|crashed|tome|grabs?|pick\s*ups?|crawled|dcing|perks?|killer|survivor|hooked|gen|pallets?)(?-u:\b)",
            ),
        ),
        (
            "metaphor_bentrok_jadwal",
            re(
                r"(?i)(?-u:\b)(?:bentrok|tabrakan)\s+(?:sama\s+|dengan\s+)?(?:jadwal|agenda|kuliah|matkul|mata\s+kuliah|jam\s+kuliah|kelas|meeting|rapat|acara|praktikum|waktu|ujian|kalender|zoom)(?-u:\b)",
            ),
        ),
        (
            "metaphor_revolusi_komersial",
            re(
                r"(?i)(?-u:\b)revolusi\s+(?:skincare|kecantikan|wajah|kulit|glowing|jerawat|industri(?:\s+4\.0)?|mental|teknologi|digital|kebiasaan|gaya\s+hidup|tubuh|diet|perawatan|estetika)(?-u:\b)",
            ),
        ),
        (
            "metaphor_affective_mismatch",
            re(
                r"(?i)(?-u:\b)(?:banjir|gempa|tsunami|erupsi|kebakaran|longsor|tanah\s+longsor|ambruk|runtuh|kecelakaan|ledakan|penembakan|wabah|keracunan)(?-u:\b)[^.!?\n]{0,60}(?-u:\b)(?:wkwk+|wkkwk+|wkwkwk+|hahaha+|hehehe+|xixi+|ngakak|kocak|bjir\s+kocak|anjir\s+kocak|giveaway|saldo\s+dana|shopee\s+haul)(?-u:\b)",
            ),
        ),
    ]
});

pub fn metaphor_blocks_topic_str(metaphor_ids: &[String], topic_id: &str) -> bool {
    for id in metaphor_ids {
        let blocks = match id.as_str() {
            "metaphor_ledakan"
            | "metaphor_affective_mismatch"
            | "nonincident_sport_entertainment" => true,
            "metaphor_banjir" => topic_id == "banjir",
            "metaphor_panas" => matches!(
                topic_id,
                "badai" | "kebakaran" | "karhutla" | "gempa" | "politik" | "konflik" | "transport"
            ),
            "metaphor_jatuh" => matches!(topic_id, "kecelakaan" | "insiden" | "banjir" | "gempa"),
            "metaphor_bakar" => matches!(topic_id, "kebakaran" | "karhutla"),
            "metaphor_tenggelam" => matches!(topic_id, "banjir" | "insiden" | "kecelakaan"),
            "metaphor_bencana_politik" => {
                matches!(topic_id, "gempa" | "badai" | "banjir" | "insiden" | "phk")
            }
            "metaphor_crime" => matches!(
                topic_id,
                "pencurian" | "penembakan" | "kriminal" | "penangkapan"
            ),
            "metaphor_siber_or_wabah" => matches!(
                topic_id,
                "ransomware" | "serangan siber" | "wabah" | "kesehatan"
            ),
            "metaphor_diamankan_noncrime" => {
                matches!(topic_id, "penangkapan" | "kriminal" | "pencurian")
            }
            "metaphor_bocor_nonincident" => matches!(topic_id, "serangan siber" | "ransomware"),
            "metaphor_bentrok_jadwal" => matches!(topic_id, "demo" | "insiden"),
            "metaphor_revolusi_komersial" => matches!(topic_id, "demo" | "politik"),
            _ => false,
        };
        if blocks {
            return true;
        }
    }
    false
}

// Core regexes
pub static NUM_ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| re(r"&#(x[0-9a-fA-F]+|[0-9]+);?"));
pub static NAMED_ENTITY_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)&(lt|gt|quot|apos|nbsp|mdash|ndash|hellip);"));
pub static AMP_ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)&amp;"));

pub static LEADING_HASH_RE: LazyLock<Regex> = LazyLock::new(|| re(r"^#+"));
pub static UNDERSCORE_RE: LazyLock<Regex> = LazyLock::new(|| re(r"_+"));
pub static CAMEL_RE: LazyLock<Regex> = LazyLock::new(|| re(r"([a-z\p{Ll}])([A-Z\p{Lu}])"));
pub static LETTER_DIGIT_RE: LazyLock<Regex> = LazyLock::new(|| re(r"([A-Za-z\p{L}])([0-9])"));
pub static DIGIT_LETTER_RE: LazyLock<Regex> = LazyLock::new(|| re(r"([0-9])([A-Za-z\p{L}])"));
pub static WS_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\s+"));

pub static URL_RE: LazyLock<Regex> = LazyLock::new(|| re(r"https?://\S+"));
pub static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| re(r"@[A-Za-z0-9_.]+"));
pub static HASHTAG_RE: LazyLock<Regex> = LazyLock::new(|| re(r"#[\p{L}\p{N}_]+"));
pub static WORD_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| re(r"[\p{L}\p{N}_'-]+"));
pub static JALAN_JALAN_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?-u:\b)jalan-jalan(?-u:\b)"));
pub static LETTER_RE: LazyLock<Regex> = LazyLock::new(|| re(r"^\p{L}$"));

pub static REDUPL_LEISURE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(jalan-jalan|makan-makan|senyum-senyum|foto-foto|nonton-nonton|main-main|bohong-bohongan|pura-pura|tiba-tiba|hati-hati|kira-kira|masing-masing|oleh-oleh|gara-gara|sia-sia|sama-sama|lama-lama|diam-diam)(?-u:\b)",
    )
});
pub static REDUPL_COLLECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:rumah-rumah|warga-warga|gedung-gedung|ruko-ruko|kendaraan-kendaraan|mobil-mobil|motor-motor|pohon-pohon|korban-korban|bangunan-bangunan|kios-kios|toko-toko|tiang-tiang|atap-atap|dinding-dinding|berjatuhan|berserakan|berhamburan|beterbangan|bertumbangan)(?-u:\b)",
    )
});

pub static ABBR_DOT_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    fre(
        r"(?i)\b(PT|CV|UD|Tb|Tbk|Jl|Jln|Dr|Prof|Ir|H|Hj|Bp|Bpk|Sdr|Ny|Tn|Jend|Letjen|Mayjen|Brigjen|Kol|Letkol|Mayor|Kapt|Lettu|Letda|Kombes|AKBP|Kompol|AKP|Iptu|Ipda|Bripka|Brigadir|Briptu|Bripda|Pol|TNI|Kec|Kab|Prov|Kel|Ds|RT|RW|KM|Km|No|Rp|St|Mt)\.(?=\s+)",
    )
});
pub static SPLIT_CLAUSE_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    fre(
        r"(?i)\n+|(?<=[!?;])\s+|(?<=\.)\s+(?=[A-Za-z\x{00C0}-\x{024F}0-9])|,\s+(?=(?:tapi|tetapi|namun|melainkan)\b)",
    )
});
pub static DEFANG_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)(?-u:\b)(?:https?|hxxps?|ftp)://\S+|(?-u:\b)[A-Za-z0-9_.-]+\[\.\][a-z]{2,}\S*")
});

pub const NEG_WORDS: &str = "(?:tidak|bukan|belum|tanpa|nihil|nol|anti|bebas)";
pub static NEG_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:tidak|bukan|belum|tanpa|nihil|nol)\s+(?:ada\s+|lagi\s+|pernah\s+|terjadi\s+|mengalami\s+|akan\s+|bakal\s+)?([a-z0-9_]+(?:\s+[a-z0-9_]+)?)",
    )
});
pub static NEG_POST_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(gempa|tsunami|erupsi|kebakaran|banjir|kebanjiran|longsor|kecelakaan|ledakan|penembakan|peretasan|kebocoran|serangan|wabah|keracunan|korban|demo|bentrok)(?-u:\b)[^.!?\n]{0,36}(?-u:\b)(?:tidak terjadi|tidak benar|tidak ada|belum ada|nihil|aman terkendali|sudah surut|hoaks|hoax|palsu|dibantah|bohong|bukan)(?-u:\b)",
    )
});
pub static FREE_OF_DISASTER_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:bebas|anti|aman|tanpa|jauh dari)\s+(?:banjir|kebanjiran|macet|kebakaran|gempa|longsor|badai)(?-u:\b)",
    )
});
pub static CORRECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)(?-u:\b)bukan\s+([^,;.\n]+?)\s*,\s*(?:tapi|tetapi|melainkan|cuma|hanya)\s+([^.!?\n]+)")
});

pub static DENIAL_ANY_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:hoaks?|kabar palsu|berita palsu|false claim|misinformation|disinformasi|isu hoax|video lama|foto lama|dibantah)(?-u:\b)",
    )
});
pub static HISTORICAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:dulu|jaman dulu|zaman dulu|tempo dulu|sejarah|mengenang|kilas balik|nostalgia|ingat waktu|tahun lalu|bulan lalu|pekan lalu|minggu lalu|last year|last month|last week|historical|anniversary|orde baru|orba|soeharto|suharto|perang dunia|world war|penjajahan|kolonial|beberapa tahun silam|(?:jaman|zaman|waktu|pas|saat|era|awal|masa|suasana|situasi|musim|periode|ketika|selama)\s+(?:covid(?:-?19)?|pandemi|lockdown|korona|corona)|(?:2019|2020|2021|2022)(?-u:\b)[^.!?\n]{0,40}(?-u:\b)(?:covid|pandemi|lockdown))(?-u:\b)",
    )
});
pub static HISTORICAL_YEAR_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)(?-u:\b)(?:tahun|pada|sejak|era)\s+((?:19|20)[0-9]{2})(?-u:\b)"));
pub static HAS_CURRENT_TIME_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:hari\s+ini|pagi\s+ini|siang\s+ini|sore\s+ini|malam\s+ini|saat\s+ini|sekarang|barusan|baru\s+saja|pukul\s+[0-9]{1,2}[:.][0-9]{2}|[0-9]{1,2}[:.][0-9]{2}\s*wib)(?-u:\b)",
    )
});
pub static HIST_CLEAN_1_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:yang\s+)?(?:sudah\s+)?(?:berjalan|berlangsung|ada|hadir|digelar|diselenggarakan|dimulai)\s+sejak\s+(?:tahun\s+)?(?:19|20)[0-9]{2}(?-u:\b)",
    )
});
pub static HIST_CLEAN_2_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:dibangun|didirikan|diresmikan|dibuka|dibeli|dijual|berdiri|lulus|menikah)\s+(?:pada\s+)?(?:tahun\s+(?:19|20)[0-9]{2}|(?:19|20)[0-9]{2}|tahun\s+lalu|bulan\s+lalu|dulu)(?-u:\b)",
    )
});
pub static HIST_NOSTALGIA_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:mengenang|kilas\s+balik|nostalgia|sejarah|tempo\s+dulu|jaman\s+dulu|zaman\s+dulu)(?-u:\b)",
    )
});

pub static QUESTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\?|^\s*(?:apakah|apa benar|benarkah|beneran|adakah|ada yang tahu|ada yang tau|ada yang dengar|ada yang lihat|ada info|ada kabar|info dong|tanya dong|spill|gimana|bagaimana|berapa|dimana|di mana|kapan|siapa|anyone know|is it true|is there)(?-u:\b)|(?-u:\b)(?:tidak sih|nggak sih|nggak ya|beneran nggak|valid nggak|hoaks nggak|banjir nggak|aman nggak|macet nggak)(?-u:\b)",
    )
});
pub static HEARSAY_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:katanya|kabarnya|katanye|rumornya|rumor|rumour|denger-denger|dengar-dengar|isu beredar|konon|katanya sih)(?-u:\b)",
    )
});
pub static CONDITIONAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:andaikan|seandainya|bagaimana jika|misalkan|kalau nanti|jika nanti|bakal|akan terjadi jika)(?-u:\b)",
    )
});
pub static SATIRE_IRONY_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:bebas banjir,?\s*katanya|mantap,\s*(?:jakarta|kota|negara)[^.!?\n]{0,40}(?:tenggelam|banjir|macet|rusak)\s+lagi|prestasi\s+(?:banjir|macet)|selamat menikmati\s+(?:banjir|macet)|macetnya bikin [^.!?\n]{0,20}mati berdiri)(?-u:\b)",
    )
});
pub static EDUCATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:apa itu|pengertian|tutorial|cara (?:mencegah|mengatasi|menghapus|menghindari|menyelamatkan)|tips|edukasi|simulasi|latihan|mitigasi|kesiapsiagaan|tas bencana|go[- ]?bag|sosialisasi|webinar|seminar|workshop|rakor|what is|how to|exercise|simulation)(?-u:\b)",
    )
});
pub static CEREMONY_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:upacara\s+(?:peringatan|tabur bunga|hari|bendera|hut|penghormatan)|tabur bunga|hari pahlawan|dirgahayu|hut\s+(?:tni|ri|bhayangkara)|ziarah\s+(?:nasional|makam)|penghargaan|apresiasi|kunjungan kerja|kunker|launching|peresmian|resmikan|museum\s+militer|lomba|parade|wisuda|silaturahmi)(?-u:\b)",
    )
});
pub static RELIGIOUS_FATWA_STRONG_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:dihukumi|hukum(?:nya)?\s+(?:islam|syariat|syara['’]?|fiqih|fikih|halal|haram|wajib|sunnah|makruh|mubah|bid['’]?ah|kafir|murtad|munafik|syirik)|masuk\s+(?:neraka|surga)|ahli\s+(?:neraka|surga)|siksa\s+(?:kubur|neraka)|azab\s+(?:allah|kubur)|dosa\s+besar|ampunan\s+dosa|fatwa|mufti|fiqih|fikih|hadits|hadis|ayat\s+al[- ]?qur['’]?an|tafsir|ceramah|kajian\s+(?:islam|sunnah|salaf|ustadz)|khutbah|khotbah|pahala\s+(?:puasa|salat|sholat|shalat|ibadah))(?-u:\b)",
    )
});
pub static RELIGIOUS_PRACTICE_ALL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:salat|sholat|shalat)(?:\s+(?:jumat|jum['’]?at|wajib|sunnah|sunah|fardhu|subuh|dzuhur|ashar|maghrib|isya|tarawih|ied|idul|qadha|jama|qashar|berjamaah|tahajud|dhuha))?(?-u:\b)|(?-u:\b)(?:munafik|murtad|kafir|syirik|bid['’]?ah|wudhu|tayammum|rakaat|makmum|muadzin|puasa\s+(?:ramadhan|sunnah|qadha|senin\s+kamis)|zakat\s+(?:fitrah|mal)|umrah|manasik)(?-u:\b)",
    )
});
pub static OFFICIAL_AGENCY_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:bmkg|bnpb|bpbd|pvmbg|basarnas|bssn|kemenkes|kemkes|polri|polda|polres|polsek|tmc\s*polda|damkar|pln|kpu|bawaslu|komdigi|kominfo|pemprov|pemkot|pemkab|knkt|kpk|dishub)(?-u:\b)",
    )
});
pub static OFFICIAL_WARNING_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:peringatan\s+(?:dini|cuaca|tsunami|banjir|longsor|erupsi|gelombang|angin|bmkg)|(?:keluarkan|mengeluarkan|rilis|merilis|terbitkan|menerbitkan)\s+peringatan|warning|early warning|(?:naikkan|tingkatkan|tetapkan)\s+status[^.!?\n]{0,35}(?-u:\b)(?:awas|siaga|waspada)|status\s+(?:awas|siaga)|diminta\s+(?:untuk\s+)?evakuasi|waspada\s+(?:tsunami|banjir|longsor|cuaca|erupsi|gelombang|angin|hujan)|potensi\s+(?:hujan\s+(?:ekstrem|lebat)|banjir|tsunami|angin\s+kencang|gelombang\s+tinggi)|hujan\s+ekstrem|siaga\s+[123]|siapkan\s+pengamanan|menyiapkan\s+pengamanan|(?:di)?siagakan|(?:di)?kerahkan|antisipasi\s+(?:demo|unjuk rasa|aksi|banjir))(?-u:\b)",
    )
});
pub static OFFICIAL_UPDATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:melaporkan|mencatat|mengungkap|menyatakan|mengonfirmasi|konfirmasi|berdasarkan laporan|update\s+(?:terkini|resmi)|data\s+sementara|tim\s+gabungan)(?-u:\b)",
    )
});
pub static EYEWITNESS_FRAMING_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:rumah saya|depan rumah|belakang rumah|samping rumah|kosan saya|kamar saya|kantor saya|jalanan depan|gang saya|kampung saya|wilayah saya|daerah saya|lingkungan saya|air(?:\s+selokan|\s+kali|\s+hujan)?\s+(?:sudah|mulai)\s+(?:naik\s+)?masuk|air masuk|terasa banget|terasa kencang|kerasa banget|bergoyang|bergetar|barusan|baru saja|saat ini|sekarang lagi|tolong|kejebak|terjebak|lihat langsung|dengar suara|asap tebal|listrik mati|mati lampu)(?-u:\b)",
    )
});
pub static COMMERCIAL_LISTING_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:dijual|mau\s+jual|jual\s+(?:cepat|mobil|motor|honda|toyota|civic|brio|avanza|innova|xpander|ertiga|vario|nmax)|disewakan|disewa|over\s*kredit|for\s*sale|for\s*rent|open\s*rent|looking\s*to\s*rent|(?-u:\b)wts(?-u:\b)|(?-u:\b)wtb(?-u:\b)|(?-u:\b)wtt(?-u:\b)|loker|lowongan\s+kerja|membutuhkan\s+(?:karyawan|staff|cook|barista|admin|driver|kurir)|kirim\s+cv|lamaran|kpr|dp\s*(?:0|nol)|cicilan|booking\s*fee|nego|siap\s+(?:huni|pakai)|pajak\s+hidup|surat\s+lengkap|lt\s*[0-9]+|lb\s*[0-9]+|shm|hgb|tahun\s+pembelian|transmisi\s*:?\s*(?:matic|manual)|stnk|(?-u:\b)km\s*:?\s*[0-9]|showroom|ready\s*stock|open\s*order|pre[- ]?order|jastip|jasa\s+titip|war\s+tiket|sewa\s+(?:raket|kamera|iphone|samsung|kost|kosan|kontrakan|apartemen|ruko|armybomb|army\s*bomb|lightstick)|rekomendasi\s+(?:kost|kosan|kontrakan|cafe|kafe|kuliner|makanan|cakes?|salon|dokter|klinik|tempat|bengkel|laundry|hotel|villa)|recommendation\s+(?:for\s+)?(?:cakes?|food|cafe|kost|hotel|place))(?-u:\b)",
    )
});
pub static ADVERTISEMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:promo|diskon|cashback|flash\s*sale|gratis\s+ongkir|bagi[- ]?bagi\s+(?:gratis|kopi|americano|minuman|makanan|voucher|hadiah)|gratis\s+(?:buat|untuk)\s+(?:warga|mahasiswa|kalian)|link\s+(?:ada\s+)?di\s+bio|cek\s+(?:link\s+)?di\s+bio|order\s+(?:via|di)\s+(?:wa|whatsapp|dm|bio)|dm\s+(?:untuk|for)\s+(?:order|harga|price|rent|booking)|s\.shopee\.co\.id|shp\.ee|tokopedia\.link|(?:diskusiin|konsultasi)\s+project\s+kamu|(?:via\s+)?(?:dm|whatsapp|wa)\s+(?:atau\s+(?:whatsapp|wa|dm)\s+)?di\s+bio|grand\s+opening|fanmeeting\s+tour|world\s+tour|asia\s+tour|beli\s+tiket|tiket\s+(?:konser|nonton|film|bioskop))(?-u:\b)",
    )
});
pub static PERSONAL_VENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:capek\s+banget|lelah\s+banget|pengen\s+resign|mau\s+resign|ingin\s+resign|muak\s+sama|stres\s+(?:kerja|banget|hidup)|ga\s+kuat\s+(?:lagi|di|hidup)|tidak\s+kuat\s+(?:lagi|di|hidup)|pindah\s+(?:aja|saja)\s+ke|capek\s+kerja|gaji\s+kecil|macetnya\s+bikin|bikin\s+(?:emosi|stres|gila|mati\s+berdiri)|badan\s+lemas|mager|gabut|galau|patah\s+hati|curhat|butuh\s+temen\s+cuddle|temen\s+cuddle)(?-u:\b)",
    )
});
pub static OPINION_PRAISE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:selalu\s+punya\s+tempat\s+di\s+hati|kota\s+(?:terindah|terbaik|favorit|kenangan)|i\s+love\s+|cinta\s+banget\s+sama|kangen\s+banget\s+sama|rindu\s+(?:kota|suasana|jakarta|bandung|jogja|surabaya)|menurut\s+(?:saya|pribadi)|pendapat\s+(?:saya|pribadi)|mengkhianati\s+demokrasi|wacana\s+(?:pemilihan|pilkada|kebijakan))(?-u:\b)",
    )
});
pub static SPAM_SCAM_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:follow\s+[A-Za-z0-9_.@-]+\s*[+.&]?\s*follow|like\s*(?:dan|&|\+)\s*(?:follow|rt)|rt\s*(?:dan|&|\+)\s*follow|boost\s+followers?|giveaway|klaim\s+(?:hadiah|reward|bonus)|airdrop|seed\s+phrase|private\s+key|robot\s+trading|binary\s+option|joki\s+(?:tugas|skripsi|makalah|ujian)|cari\s+(?:jodoh|pasangan|pacar|suami|istri)|cv\s+taaruf|biodata\s+taaruf|kriteria\s*:\s*1\.|jilmek|open\s*bo|vcs|bispak|janda(?-u:\b)[^.\n]{0,30}(?-u:\b)(?:jakarta|bo|vcs|open|michat)|michat|bokep|sange|colmek|crowcaster\.com|slot\s+gacor|togel|judol|gestun\s+pinjol)(?-u:\b)|(?-u:\b)R[1-9]\s+#[0-9]+(?-u:\b)[^.\n]{0,60}\$[0-9]+\.[0-9]+",
    )
});
pub static READER_ACTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:saya|aku|gue|gw|kami|kita)\s+(?:(?:lagi|sedang|posisi|masih)\s+(?:di|dari)\s+[^,.!?\n]{2,30}[,\s]+)?(?:baca|membaca|membacanya|nonton|menonton|lihat\s+(?:berita|kabar|postingan)|melihat\s+(?:berita|kabar)|dengar\s+(?:berita|kabar)|mendengar\s+(?:berita|kabar)|pantau|memantau|scroll|liat|nemu)(?-u:\b)",
    )
});

pub static FOREIGN_MARKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(chiba|odesa|kharkiv|tel aviv|abu dhabi|dubai|jepang|japan|nepal|london|oyo|osun|nigeria|amerika|united states|(?-u:\b)usa(?-u:\b)|americans?|american|british|canadian|australian|japanese|chinese|korean|french|german|russian|indian|ukrainian|israeli|palestinian|malaysian|singaporean|filipino|vietnamese|brazilian|mexican|european|texas|california|new york|michigan|florida|ohio|illinois|pennsylvania|virginia|washington|boston|chicago|los angeles|brasil|brazil|filipina|philippines|thailand|malaysia|m'sia|msia|lebuhraya|sejuk beku|bekalan makanan|plusline|llmtrafik|korang|macam ni|banjir tak|banjir ke|kesiannya|memandu|tanpa halangan banjir|terowong smart|berbaki|penyokong|rombak kabinet|bubarkan parlimen|bubar parlimen|parlimen|timbalan perdana menteri|pakatan harapan|perikatan nasional|barisan nasional|umno|menteri besar|ketua menteri|dewan rakyat|dewan negara|kerajaan perpaduan|kerajaan madani|malaysia madani|varian baharu|baharu|dikesan|sejauh mana bahayanya|siakap keli|thesiakapkeli|banjir kilat|jabatan bomba|polis diraja|kawasan terjejas|hospital kerajaan|kementerian kesihatan|kelantan|pulau pinang|sabah|sarawak|terengganu|pahang|kedah|perak|selangor|putrajaya|negeri sembilan|melaka|kota kinabalu|kuching|ipoh|seremban|alor setar|kuantan|kuala terengganu|kota bharu|astro awani|awani|jbpm|pdrm|korea|china|tiongkok|inggris|britain|prancis|perancis|paris|france|eiffel|jerman|germany|australia|india|pakistan|iran|israel|palestina|palestine|ukraina|ukraine|rusia|russia|gaza|turki|turkey|meksiko|mexico|kanada|canada|vietnam|chile|argentina|spanyol|spain|portugal|italia|italy|belanda|netherlands|den haag|s-gravenhage|tweede kamer|eerste kamer|vvd|cda|d66|pvv|fvd|nsc|gl-pvda|groenlinks|pvda|nederland|nederlands|holland|amsterdam|rotterdam|utrecht|eindhoven|groningen|kabinet rutte|kabinet schoof|dick schoof|geert wilders|swiss|swedia|sweden|norwegia|norway|denmark|yunani|greece|eropa|europe|fincen|poland|polandia|polonia|krakow|warsaw|warszawa|warschau|lublin|gdansk|wroclaw|poznan|szczecin|czechia|czech|ceko|prague|praha|brno|ostrava|austria|vienna|wina|salzburg|innsbruck|finland|finlandia|helsinki|singapura|singapore|hong kong|taipei|tokyo|kyoto|osaka|seoul|pyongyang|beijing|shanghai|guangzhou|shenzhen|manila|bangkok|chiang mai|hanoi|haiphong|vientiane|phnom penh|kualalumpur|kuala lumpur|penang|johor|kathmandu|colombo|islamabad|karachi|lahore|mumbai|delhi|new delhi|chennai|kolkata|dhaka|chittagong|yangon|mandalay|ulaanbaatar|timor leste|papua new guinea|port moresby|brunei|darussalam|afghanistan|kabul|herat|suriah|syria|irak|iraq|mesir|egypt|cairo|giza|libya|libia|maroko|morocco|sudan|ethiopia|kenya|ghana|myanmar|kamboja|cambodia|laos|srilanka|sri lanka|bangladesh|lebanon|yordania|jordan|yaman|yemen|oman|qatar|uea|georgia|panama|san francisco)(?-u:\b)|(?-u:\b)tpm\s+(?:pula|kata|malaysia|dato|datuk|zahid|fadillah)",
    )
});
pub static LOCAL_ORG_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(bmkg|bnpb|bpbd|bssn|pdn|polri|pln|kpu|bawaslu|komdigi|kominfo|bank indonesia|bursa efek indonesia|ihsg|kemenkes|kemensos|kemdikbud|kemendikdasmen|kemenhub|kejaksaan|kemenkumham|kementerian keuangan|kemenkeu|direktorat jenderal pajak|ditjen pajak|djp|coretax|core tax|kring pajak|kanwil djp|kpp pratama|kpp madya|pengadilan pajak|ppatk|kementerian|pemkot|pemkab|pemprov|dishub|damkar|bpjs|taspen|bri|bni|mandiri|basarnas|knkt|kpk|polres|polda|polsek|kodim)(?-u:\b)",
    )
});
pub static LOCAL_MACRO_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:(?:di|in|wilayah|pulau|kawasan|kota[- ]kota|seluruh|berbagai)\s+(?:wilayah\s+|daerah\s+|kota\s+)?indonesia|jabodetabek|sumatera|sumatra|kalimantan|sulawesi|nusa\s+tenggara|kepulauan\s+riau|bangka\s+belitung|pantura|(?-u:\b)wni(?-u:\b)|tol\s+(?:cipularang|jagorawi|cikampek|cipali|palikanci|pejagan|cisumdawu|mbz|japek|trans\s+jawa|bali\s+mandara)|jembatan\s+suramadu|sungai\s+ciliwung|kali\s+bekasi|gunung\s+(?:semeru|merapi|marapi|bromo|sinabung|lewotobi|rinjani|agung)|(?:indonesia['’]?s?|indonesian)\s+(?:politics|government|president|vice[- ]?president|minister|cabinet|parliament|election|democracy|court|military|police|unrest|regime|oligarchy)|(?:in|of)\s+indonesia)(?-u:\b)",
    )
});

pub static READER_SEG_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    fre(
        r"(?i)\b(?:saya|kami|kita)\s+(?:lagi\s+|sedang\s+|posisi\s+)?(?:(?:di|dari)\s+([a-z][a-z\s]{2,26}?)(?:\s*,\s*|\s+)(?:baca|membaca|membacanya|nonton|menonton|lihat|melihat|dengar|mendengar|pantau)|(?:baca|membaca|membacanya|nonton|menonton|lihat|melihat|dengar|mendengar|pantau)[^.!?\n]{0,60}?\bdari\s+([a-z][a-z\s]{2,26}?)(?=[,.;!?]|$))",
    )
});
pub static AUTHOR_CUR_SEG_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    fre(
        r"(?i)\b(?:kerja|bekerja|tinggal|kuliah|ngekos|ngekost|kost|kos|domisili|menetap|pindah(?:\s+kerja|\s+tugas)?|cari\s+(?:kost|kosan|kontrakan|kerja|loker)|berangkat|based(?:\s+in)?)\s+(?:di|ke|area|daerah|wilayah|in)?\s*([a-z][a-z\s]{2,26}?)(?=[,.;!?]|$|\s+(?:pengen|mau|udah|sudah|karena|harga|yang|dan|atau))",
    )
});
pub static AUTHOR_HOME_SEG_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    fre(
        r"(?i)\b(?:asli|orang|pulang\s+kampung\s+ke|mudik\s+ke|asal)\s+([a-z][a-z\s]{2,26}?)(?=[,.;!?]|$|\s+(?:yang|di|dan|tapi))",
    )
});
pub static DATELINE_SEG_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:[A-Z][A-Z\s]{2,24})\s*[—–-]\s+"));

pub static EUROPEAN_WORDS_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:het|een|van|dat|die|voor|naar|niet|zijn|wordt|werd|worden|hebben|heeft|hadden|maar|door|over|deze|onze|jullie|tegen|zich|geen|nieuwe|moeten|mogen|toch|misschien|weer|alleen|wanneer|hoezo|waarom|vanavond|morgen|tussenformeren|verkiezingen|uitslag|regerende|regering|partij|partijen|kandidaat|coalitie|coalitieleden|meerderheid|minderheid|kiezer|stemt|stemmen|stemming|linksaf|rechtsaf|haag|den\s+haag|vvd|cda|d66|pvv|fvd|nsc|tweede\s+kamer|eerste\s+kamer|nederland|nederlands|holland|der|die|das|und|nicht|kabinett|bundestag|scholz|habeck|merz|bundeskanzler|wahl|stimmen|le|la|les|des|dans|pour|avec|macron|gouvernement|ministre|assemblee|el|los|las|del|por|para|con|gobierno|elecciones|partido|muito|voce|você|nao|não|entao|então|governo|eleicoes)(?-u:\b)",
    )
});
pub static ID_UNAMBIGUOUS_CORE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:yang|yg|untuk|utk|dengan|dgn|sudah|udah|udh|belum|blm|tidak|nggak|ngga|gak|ga|enggak|engga|kagak|kaga|bukan|gue|gw|gua|aku|kami|kita|warga|banget|bgt|dong|deh|kok|sih|jir|anjir|njir|bjir|wkwk[A-Za-z0-9_]*|kalo|kalau|karena|krn|karna|terus|trs|masih|msh|bisa|pengen|pingin|bikin|pake|pakai|kemarin|kmrn|barusan|sekarang|skrg|kenapa|knp|gimana|gmn|bagaimana|siapa|apakah|adakah|dimana|dmn|kemana|terjadi|melanda|merendam|mengungsi|tewas|korban|polisi|aparat|rusak|ambruk|roboh|jebol|padam|hujan|kebakaran|banjir|gempa|longsor|terbakar|kecelakaan|tabrakan|penembakan|pencurian|begal|prabowo|gibran|jokowi|dpr\s+ri|jakarta|bandung|surabaya|semarang|medan|makassar|bekasi|depok|tangerang|bogor|pim|pln|kemang|jaksel)(?-u:\b)",
    )
});
