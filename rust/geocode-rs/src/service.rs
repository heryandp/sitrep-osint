#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub name: String,
    pub la: f64,
    pub lo: f64,
    pub prov: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeoStats {
    pub keys: usize,
    pub kab: usize,
    pub prov: usize,
    pub kec: usize,
    pub poi: usize,
    pub villages: usize,
    pub desa: usize,
    pub kelurahan: usize,
    pub village_names: usize,
    pub ambiguous_village_names: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapLabel {
    pub id: String,
    pub name: String,
    pub la: f64,
    pub lo: f64,
    pub kind: String,
}

#[derive(Debug, Clone)]
struct MapItem {
    id: String,
    name: String,
    la: f64,
    lo: f64,
    kind: &'static str,
    prio: f64,
}
impl MapItem {
    fn new(id: String, name: String, la: f64, lo: f64, kind: &'static str, prio: f64) -> Self {
        Self {
            id,
            name,
            la,
            lo,
            kind,
            prio,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Level {
    Poi,
    Village,
    Kec,
    #[default]
    Kab,
    Region,
    Prov,
}

impl Level {
    fn score(self) -> f64 {
        match self {
            Level::Poi => 7.0,
            Level::Village => 7.2,
            Level::Kec => 6.5,
            Level::Kab => 6.0,
            Level::Region => 3.5,
            Level::Prov => 3.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct PlaceEntry {
    id: String,
    level: Level,
    kota: bool,
    alias: bool,
    name: String,
    la: f64,
    lo: f64,
    prov: String,
    prec: &'static str,
    prov_code: Option<String>,
    kab_code: Option<String>,
    kec_code: Option<String>,
    code: Option<String>,
    coordinate_source: Option<&'static str>,
    boundary_status: Option<String>,
}

impl PlaceEntry {
    fn new(
        id: &str,
        level: Level,
        name: &str,
        la: f64,
        lo: f64,
        prov: &str,
        prec: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            level,
            name: name.into(),
            la,
            lo,
            prov: prov.into(),
            prec,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
struct KecRow {
    code: String,
    name: String,
    kab_code: String,
    prov_code: String,
}

#[derive(Debug, Clone)]
enum PoiMatcher {
    Volc(Vec<&'static str>, bool),
    Phrases(Vec<Vec<&'static str>>, bool),
    Istana,
    Dpr,
    MakasarLa,
    MakasarSpan,
    GentengLa(bool),
    GentengSpan,
    CandiLa(bool),
    CandiSpan,
}

#[derive(Debug, Clone)]
struct PoiEntry {
    matcher: PoiMatcher,
    entry: PlaceEntry,
}

#[derive(Deserialize)]
struct RawGazetteer {
    prov: Vec<RawProv>,
    kab: Vec<RawKab>,
    kec: HashMap<String, Vec<String>>,
}
#[derive(Deserialize)]
struct RawProv {
    code: String,
    name: String,
    la: f64,
    lo: f64,
}
#[derive(Deserialize)]
struct RawKab {
    code: String,
    name: String,
    kota: bool,
    ibukota: String,
    la: f64,
    lo: f64,
    prov: String,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct RawBigCounts {
    places: usize,
    #[serde(rename = "Desa")]
    desa: usize,
    #[serde(rename = "Kelurahan")]
    kelurahan: usize,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct RawBig {
    counts: RawBigCounts,
    provinces: Vec<Vec<String>>,
    #[serde(rename = "kabupatenKota")]
    kabupaten_kota: Vec<Vec<String>>,
    kecamatan: Vec<Vec<String>>,
    places: Vec<(String, String, String, f64, f64, String)>,
}

struct StaticSets {
    ambig: HashSet<&'static str>,
    loc_cue: HashSet<&'static str>,
    prefix_kab: HashSet<&'static str>,
    prefix_kota: HashSet<&'static str>,
    prefix_kec: HashSet<&'static str>,
    prefix_prov: HashSet<&'static str>,
    cue_in: HashSet<&'static str>,
    cue_from: HashSet<&'static str>,
    dest_verb: HashSet<&'static str>,
    sep_ok: HashSet<&'static str>,
    ik_generic: HashSet<&'static str>,
    cue_pre: HashSet<&'static str>,
    kec_loose: HashSet<&'static str>,
    kec_stop: HashSet<&'static str>,
    village_generic_cue: HashSet<&'static str>,
    village_loc_cue: HashSet<&'static str>,
    village_dest_cue: HashSet<&'static str>,
    village_foreign: HashSet<&'static str>,
    village_stop: HashSet<&'static str>,
}

fn csv_set(s: &'static str) -> HashSet<&'static str> {
    s.split(',').filter(|x| !x.is_empty()).collect()
}

static SETS: LazyLock<StaticSets> = LazyLock::new(|| StaticSets {
    ambig: csv_set(
        "batang,batu,batu bara,puncak,metro,serang,malang,buru,pati,sula,banjar,bone,kaur,tanah laut,kota baru,seribu,jaya,raya,mamasa,lahat,medan,subang,kudus,kuningan,negara,sumber,benteng,pandan,pinang,manggar",
    ),
    loc_cue: csv_set(
        "di,ke,kota,kabupaten,kab,pemkot,pemkab,walikota,bupati,kodya,kecamatan,kec,wilayah,kapolres,polres,polresta,polrestabes,kodim,bpbd",
    ),
    prefix_kab: csv_set("kabupaten,kab,pemkab,bupati"),
    prefix_kota: csv_set("kota,kodya,pemkot,walikota,polresta,polrestabes"),
    prefix_kec: csv_set("kecamatan,kec"),
    prefix_prov: csv_set("provinsi,prov,pemprov,gubernur"),
    cue_in: csv_set("di,wilayah,sekitar,kecamatan,kec,kabupaten,kota"),
    cue_from: csv_set("dari,asal,warga,asal-usul"),
    dest_verb: csv_set(
        "dipulangkan,dikirim,dibawa,dilarikan,dirujuk,diterbangkan,diberangkatkan,dikembalikan,menuju,kembali,pulang,mudik,dipindahkan,dievakuasi,diangkut",
    ),
    sep_ok: csv_set("dan,serta,hingga,sampai,maupun,atau,juga,sekitarnya,dll,lainnya"),
    ik_generic: csv_set(
        "tanjung,kuala,muara,sungai,simpang,pulau,bukit,batu,air,pasar,kota,teluk,padang,lubuk,pangkalan,labuhan,sei,karang,sinar,baru,mekar,tebing,talang,ujung,mandiri,jaya,wonosari,bangkinang,negara,sumber,benteng,pandan,pinang,manggar,kalianda,kuningan",
    ),
    cue_pre: csv_set(
        "di,ke,dari,warga,sekitar,area,kecamatan,kec,pemkot,dishub,polres,truk,bus,angkot,mahasiswa",
    ),
    kec_loose: csv_set("di,ke,warga,wilayah,desa"),
    kec_stop: csv_set(
        "kampung baru,pasar baru,tanjung raya,sungai raya,bukit raya,kota baru,pulau besar,air besar,indonesia raya,jalan raya,cipta karya,karya jaya,sumber jaya,harapan jaya,makmur jaya,mandiri jaya,pesantren,kantor,bandara,pelabuhan,stadion,perbatasan,pedalaman,kejaksaan,lapangan,alun alun,bendungan,terminal,stasiun,pasar,perguruan,perkebunan,pertanian,perikanan,kehutanan,kesehatan,pendidikan,keamanan,pertahanan",
    ),
    village_generic_cue: csv_set("gampong,nagari,kampung,negeri,pekon,tiyuh,lembang"),
    village_loc_cue: csv_set("di,ke,wilayah,sekitar,area,lokasi"),
    village_dest_cue: csv_set("untuk,kepada"),
    village_foreign: csv_set(
        "jepang,london,kelantan,pulau pinang,sabah,sarawak,selangor,johor,pahang,terengganu,kedah,perak,melaka,negeri sembilan,empat negeri",
    ),
    village_stop: csv_set(
        "jatuh,besar,tengah,dalam,atas,bawah,depan,belakang,utara,selatan,timur,barat,pusat,indah,baru,lama,raya,jaya,mulia,makmur,harapan,damai,sejahtera,sentosa,maju,sukses,pertama,utama,ujung,hulu,hilir,kantor,lapangan,kejaksaan,pengadilan,bandara,pelabuhan,stadion,stasiun,terminal,dermaga,perbatasan,pedalaman,bendungan,waduk,pesantren,kampung,alun alun,turunan,tanjakan,tikungan,simpang,bundaran,jembatan,jalan,lorong,gang,negara,istana,parlemen,senayan,kementerian,sumber,benteng,pandan,pinang,manggar,korban,warga,polisi,tentara,aparat,petugas,tersangka,pelaku,saksi,rumah,gedung,sekolah,kampus,masjid,gereja,pura,wihara,asrama,penjara,lapas,rutan,polsek,polres,polda,kodim,koramil,puskesmas,klinik,apotek,hotel,mall,plaza,taman,pasar,toko,warung,kios,gudang,pabrik,bengkel,tambang,kebun,hutan,lahan,sawah,ladang,sungai,danau,laut,pulau,pantai,teluk,tanjung,muara,kuala,gunung,bukit,lembah,banjir,longsor,gempa,kebakaran,kecelakaan,bencana,darurat,harapan baru,kampung baru,pasar baru,kota baru,sumber baru,sumber jaya,mekar jaya,mekar sari,suka maju,suka makmur,suka damai,suka mulia,suka rame,suka karya,durian runtuh",
    ),
});

pub struct Gazetteer {
    idx: HashMap<String, Vec<PlaceEntry>>,
    kab_by_code: HashMap<String, PlaceEntry>,
    kec_idx: HashMap<String, Vec<String>>,
    big_kec_by_name: HashMap<String, Vec<KecRow>>,
    big_kec_coords: HashMap<String, (f64, f64)>,
    fix_kec_codes: HashSet<String>,
    big_kec_max_words: usize,
    village_idx: HashMap<String, Vec<PlaceEntry>>,
    village_max_words: usize,
    poi: Vec<PoiEntry>,
    poi_triggers: HashMap<&'static str, Vec<usize>>,
    map_prov: Vec<MapItem>,
    map_kab: Vec<MapItem>,
    map_kec: Vec<MapItem>,
    map_poi: Vec<MapItem>,
    village_grid: HashMap<(i32, i32), Vec<MapItem>>,
    nodes: Vec<Node>,
    stats: GeoStats,
}

static GAZETTEER: LazyLock<Result<Gazetteer, String>> = LazyLock::new(Gazetteer::load);

pub fn gazetteer() -> Result<&'static Gazetteer, String> {
    GAZETTEER.as_ref().map_err(Clone::clone)
}

pub fn norm_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true;
    let mut push = |c: char| {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    };
    if s.is_ascii() {
        for b in s.bytes() {
            push(b.to_ascii_lowercase() as char);
        }
    } else {
        for c in s.to_lowercase().nfd() {
            if !('\u{0300}'..='\u{036F}').contains(&c) {
                push(c);
            }
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

fn prov_key(p: &str) -> String {
    let k = norm_str(p);
    match k.as_str() {
        "daerah khusus ibukota jakarta" | "dki jakarta" => "dki jakarta".into(),
        "daerah istimewa yogyakarta" | "di yogyakarta" | "yogyakarta" => "di yogyakarta".into(),
        "kepulauan bangka belitung" | "bangka belitung" => "bangka belitung".into(),
        _ => k,
    }
}

fn label_fix(name: &str) -> &str {
    match name {
        "Palangkaraya" => "Palangka Raya",
        "Bau Bau" => "Baubau",
        "Kep. Siau Tagulandang Biaro" => "Sitaro",
        "Toli-Toli" => "Tolitoli",
        _ => name,
    }
}

fn strip_ci<'a>(s: &'a str, prefs: &[&str]) -> &'a str {
    let lower = s.to_ascii_lowercase();
    for pref in prefs {
        if let Some(rest) = lower.strip_prefix(pref) {
            let ws = rest.len() - rest.trim_start().len();
            if ws > 0 {
                return &s[pref.len() + ws..];
            }
        }
    }
    s
}

fn to_fixed_5(x: f64) -> f64 {
    let v = format!("{x:.5}").parse::<f64>().unwrap_or(x);
    if v == 0.0 { 0.0 } else { v }
}

fn fix_pair(v: &Value) -> Option<(f64, f64)> {
    let a = v.as_array()?;
    Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
}

fn village_type_cue(w: &str) -> Option<&'static str> {
    match w {
        "desa" => Some("desa"),
        "kelurahan" | "kel" | "kalurahan" => Some("kelurahan"),
        _ => None,
    }
}

fn poi_city(id: &str) -> Option<&'static str> {
    match id {
        "simpang-lima" | "banyumanik" | "pedurungan" | "genuk" | "tembalang" => Some("semarang"),
        "pasar-kliwon" | "jebres" | "laweyan" => Some("surakarta"),
        "sukolilo" | "wonokromo" | "rungkut" | "mulyorejo" | "gunungsari" | "siwalankerto"
        | "tandes" | "kenjeran" => Some("surabaya"),
        "klojen" | "lowokwaru" | "sukun" | "blimbing" => Some("malang"),
        "gondokusuman" | "umbulharjo" | "mantrijeron" => Some("yogyakarta"),
        _ => None,
    }
}

const KAB_ALIAS_TABLE: &str = "jakarta|Jakarta Pusat|1|Jakarta;dki|Jakarta Pusat|1|Jakarta;jogja|Yogyakarta|1|;yogya|Yogyakarta|1|;tangsel|Tangerang Selatan|1|;pangkalpinang|Pangkal Pinang|1|;tanjungpinang|Tanjung Pinang|1|;tasik|Tasikmalaya|1|;banjarbaru|Banjar Baru|1|;sitaro|Kep. Siau Tagulandang Biaro|0|;tolitoli|Toli-Toli|0|;baubau|Bau Bau|1|";
const PROV_ALIAS_TABLE: &str = "11:aceh,nanggroe aceh;12:sumatera utara,sumatra utara,sumut;13:sumatera barat,sumatra barat,sumbar;14:riau;15:jambi;16:sumatera selatan,sumatra selatan,sumsel;17:bengkulu;18:lampung;19:bangka belitung,babel,kepulauan bangka belitung,bangka,belitung;21:kepulauan riau,kepri;32:jawa barat,jabar;33:jawa tengah,jateng;34:diy,daerah istimewa yogyakarta;35:jawa timur,jatim;36:banten;51:bali;52:nusa tenggara barat,ntb;53:nusa tenggara timur,ntt;61:kalimantan barat,kalbar;62:kalimantan tengah,kalteng;63:kalimantan selatan,kalsel;64:kalimantan timur,kaltim;65:kalimantan utara,kaltara;71:sulawesi utara,sulut;72:sulawesi tengah,sulteng;73:sulawesi selatan,sulsel;74:sulawesi tenggara,sultra;75:gorontalo;76:sulawesi barat,sulbar;81:maluku;82:maluku utara,malut;91:papua;92:papua barat;93:papua selatan;94:papua tengah;95:papua pegunungan;96:papua barat daya";
const VOLC_TABLE: &str = "anak krakatau|-6.10|105.42|1;krakatau|-6.10|105.42|1;merapi|-7.54|110.44|1;semeru|-8.108|112.92|1;bromo|-7.94|112.95|1;agung|-8.34|115.51|0;sinabung|3.17|98.39|1;kerinci|-1.70|101.26|1;marapi|-0.38|100.47|1;dempo|-4.02|103.13|0;papandayan|-7.32|107.73|1;guntur|-7.25|107.85|0;tangkuban perahu|-6.76|107.61|1;galunggung|-7.25|108.06|1;slamet|-7.24|109.21|0;dieng|-7.20|109.90|0;lawu|-7.63|111.19|0;kelud|-7.93|112.31|1;raung|-8.13|114.06|1;ijen|-8.06|114.24|0;batur|-8.24|115.38|0;rinjani|-8.41|116.46|1;tambora|-8.25|118.0|1;sangeang api|-8.20|119.07|1;lewotobi laki laki|-8.54|122.77|1;lewotobi|-8.54|122.77|1;ili lewotolok|-8.27|123.50|1;lewotolok|-8.27|123.50|1;batu tara|-7.79|123.58|0;karangetang|2.78|125.41|1;lokon|1.37|124.83|1;soputan|1.11|124.73|1;ruang|2.30|125.37|0;ibu|1.49|127.63|0;dukono|1.69|127.88|1;gamalama|0.80|127.33|1;kelimutu|-8.77|121.82|1;egon|-8.68|122.45|1;merbabu|-7.45|110.44|1;paluweh|-8.32|121.78|1;sirung|-8.49|124.13|1;iya|-8.89|121.63|0;argopuro|-7.77|113.58|0;ili boleng|-8.344|123.252|1;ile werung|-8.532|123.573|1;inerie|-8.875|120.95|1;awu|3.667|125.500|0;banda api|-4.525|129.871|1;talang|-0.978|100.679|0;ciremai|-6.892|108.40|1;gede|-6.78|106.98|0;salak|-6.716|106.734|0;sumbing|-7.385|110.073|0";
const LM_PRE_TABLE: &str = "istana-wapres|Istana Wapres|-6.1814|106.8268|DKI Jakarta|istana wapres,istana wakil presiden,kantor wapres;istana-garuda|Istana Garuda IKN|-0.9634|116.7022|Kalimantan Timur|istana garuda;istana-bogor|Istana Bogor|-6.5950|106.7941|Jawa Barat|istana bogor;istana-cipanas|Istana Cipanas|-6.7369|107.0397|Jawa Barat|istana cipanas;istana-tampaksiring|Istana Tampaksiring|-8.4358|115.3094|Bali|istana tampaksiring;istana-maimun|Istana Maimun|3.5753|98.6837|Sumatera Utara|istana maimun;istana-pagaruyung|Istana Pagaruyung|-0.4712|100.6212|Sumatera Barat|istana pagaruyung,istana basa pagaruyung";
const LM_POST_TABLE: &str = "tugu-tani|Tugu Tani|-6.1834|106.8339|DKI Jakarta|tugu tani,patung pahlawan;monas|Monas|-6.1754|106.8272|DKI Jakarta|monas,patung kuda,medan merdeka,lapangan banteng,bundaran hi;mk|Mahkamah Konstitusi|-6.1751|106.8228|DKI Jakarta|gedung mk,mahkamah konstitusi;kpk|Gedung KPK|-6.2115|106.8315|DKI Jakarta|gedung kpk,kantor kpk,kpk ri;kpu|KPU RI|-6.1994|106.8322|DKI Jakarta|gedung kpu,kantor kpu,kpu ri;djp-pusat|Kantor Pusat DJP|-6.2297|106.8174|DKI Jakarta|kantor pusat djp,gedung djp,kantor djp,ditjen pajak,direktorat jenderal pajak,kring pajak;kemenkeu|Kementerian Keuangan|-6.1695|106.8364|DKI Jakarta|gedung kemenkeu,kantor kemenkeu,kementerian keuangan,kemenkeu;pengadilan-pajak|Pengadilan Pajak|-6.1712|106.8359|DKI Jakarta|pengadilan pajak,sekretariat pengadilan pajak;pelaga|Pelaga, Badung|-8.2918|115.2127|Bali|pelaga";
const CPOI_PART1: &str = "gambir|Gambir|-6.1767|106.8306|DKI Jakarta|gambir,stasiun gambir|0;tanah-abang|Pasar Tanah Abang|-6.2063|106.8178|DKI Jakarta|pasar tanah abang|0;tanah-abang-kec|Tanah Abang|-6.1857|106.8109|DKI Jakarta|tanah abang|0;cikini|Cikini|-6.1985|106.8413|DKI Jakarta|cikini|0;senen|Senen|-6.1744|106.8446|DKI Jakarta|senen,pasar senen|0;kemayoran|Kemayoran|-6.1617|106.8414|DKI Jakarta|kemayoran|0;halim|Halim Perdanakusuma|-6.2656|106.8936|DKI Jakarta|halim perdanakusuma,bandara halim,halim|1;tanjung-priok|Tanjung Priok|-6.1289|106.8708|DKI Jakarta|tanjung priok|0;pluit|Pluit|-6.1150|106.7874|DKI Jakarta|pluit|0;kalideres|Kalideres|-6.1370|106.7016|DKI Jakarta|kalideres|0;gbk|GBK|-6.2186|106.8036|DKI Jakarta|gelora bung karno,gbk|0;taman-mini|Taman Mini|-6.3026|106.8953|DKI Jakarta|taman mini|0;mangga-besar|Mangga Besar|-6.1498|106.8270|DKI Jakarta|mangga besar|0;matraman|Matraman|-6.2126|106.8598|DKI Jakarta|matraman|0;menteng|Menteng|-6.1950|106.8322|DKI Jakarta|menteng|0;tebet|Tebet|-6.2264|106.8584|DKI Jakarta|tebet|0;pancoran|Pancoran|-6.2581|106.8427|DKI Jakarta|pancoran|1;kebayoran|Kebayoran|-6.2372|106.7825|DKI Jakarta|kebayoran|0;ciracas|Ciracas|-6.3296|106.8766|DKI Jakarta|ciracas|0;duren-sawit|Duren Sawit|-6.2341|106.9192|DKI Jakarta|duren sawit|0;jagakarsa|Jagakarsa|-6.3301|106.8222|DKI Jakarta|jagakarsa|0;pondok-gede|Pondok Gede|-6.2896|106.8959|Jawa Barat|pondok gede|0;sawah-besar|Sawah Besar|-6.1607|106.8277|DKI Jakarta|sawah besar|0;johar-baru|Johar Baru|-6.1831|106.8553|DKI Jakarta|johar baru|0;gunung-sahari|Gunung Sahari|-6.1368|106.8324|DKI Jakarta|gunung sahari|0;koja|Koja|-6.1208|106.9074|DKI Jakarta|koja|0;penjaringan|Penjaringan|-6.1173|106.7674|DKI Jakarta|penjaringan|0;ancol|Ancol|-6.1281|106.8451|DKI Jakarta|ancol|0;kelapa-gading|Kelapa Gading|-6.1599|106.9025|DKI Jakarta|kelapa gading|0;cengkareng|Cengkareng|-6.1529|106.7447|DKI Jakarta|cengkareng|0;tomang|Tomang|-6.1716|106.7969|DKI Jakarta|tomang|0;grogol|Grogol|-6.1620|106.7896|DKI Jakarta|grogol|0;palmerah|Palmerah|-6.2079|106.7972|DKI Jakarta|palmerah|0;karet|Karet|-6.2135|106.8271|DKI Jakarta|karet|1;mampang|Mampang|-6.2362|106.8198|DKI Jakarta|mampang|0;kramat-jati|Kramat Jati|-6.2749|106.8625|DKI Jakarta|kramat jati|0;balekambang|Balekambang|-6.2807|106.8521|DKI Jakarta|balekambang|0;bendungan-hilir|Bendungan Hilir|-6.2150|106.8179|DKI Jakarta|bendungan hilir|0;pasar-minggu|Pasar Minggu|-6.2833|106.8449|DKI Jakarta|pasar minggu|0;kembangan|Kembangan|-6.1914|106.7406|DKI Jakarta|kembangan|1;pondok-labu|Pondok Labu|-6.3088|106.7979|DKI Jakarta|pondok labu|0;lubang-buaya|Lubang Buaya|-6.2929|106.9031|DKI Jakarta|lubang buaya|0;jatinegara|Jatinegara|-6.2149|106.8703|DKI Jakarta|jatinegara|0;bali-mester|Bali Mester, Jatinegara|-6.2135|106.8725|DKI Jakarta|bali mester|0;pulo-gadung|Pulo Gadung|-6.1911|106.8906|DKI Jakarta|pulo gadung|0;cipayung|Cipayung|-6.3294|106.9037|DKI Jakarta|cipayung|0;klender|Klender|-6.2133|106.8994|DKI Jakarta|klender|0;cakung|Cakung|-6.1897|106.9397|DKI Jakarta|cakung|0;pasar-rebo|Pasar Rebo|-6.3250|106.8534|DKI Jakarta|pasar rebo|0;kalibata|Kalibata|-6.2624|106.8403|DKI Jakarta|kalibata|0;lebak-bulus|Lebak Bulus|-6.3017|106.7797|DKI Jakarta|lebak bulus|0;pesanggrahan|Pesanggrahan|-6.2555|106.7631|DKI Jakarta|pesanggrahan|0;tanjung-barat|Tanjung Barat|-6.3080|106.8389|DKI Jakarta|tanjung barat|0;srengseng|Srengseng|-6.2065|106.7541|DKI Jakarta|srengseng|1;sukapura|Sukapura|-6.1476|106.9304|DKI Jakarta|sukapura|0";
const CPOI_PART2: &str = "ciputat|Ciputat|-6.3111|106.7184|Banten|ciputat|0;stasiun-tugu|Stasiun Tugu, Yogyakarta|-7.7897|110.3632|DI Yogyakarta|stasiun tugu|0;sardjito|RSUP Dr Sardjito|-7.7689|110.3725|DI Yogyakarta|rsup dr sardjito,sardjito|0;mantrijeron|Mantrijeron|-7.8181|110.3597|DI Yogyakarta|mantrijeron|0;gondokusuman|Gondokusuman|-7.7867|110.3810|DI Yogyakarta|gondokusuman|0;umbulharjo|Umbulharjo|-7.8144|110.3874|DI Yogyakarta|umbulharjo|0;gejayan|Gejayan|-7.7712|110.3905|DI Yogyakarta|gejayan,jalan gejayan|0;kota-gudeg|Yogyakarta|-7.7999|110.3914|DI Yogyakarta|kota gudeg|0;mlati|Mlati|-7.7384|110.3443|DI Yogyakarta|mlati|1;bandung-kota|Bandung Kota|-6.9143|107.6021|Jawa Barat|bandung kota|0;dago|Dago|-6.8750|107.6160|Jawa Barat|dago|0;cihampelas|Cihampelas|-6.8989|107.6043|Jawa Barat|cihampelas|0;gedung-sate|Gedung Sate|-6.9025|107.6188|Jawa Barat|gedung sate|0;kota-kembang|Bandung|-6.9175|107.6191|Jawa Barat|kota kembang|0;wonokromo|Wonokromo|-7.3021|112.7392|Jawa Timur|wonokromo|0;rungkut|Rungkut|-7.3197|112.7905|Jawa Timur|rungkut|0;mulyorejo|Mulyorejo|-7.2680|112.7935|Jawa Timur|mulyorejo|0;kota-pahlawan|Surabaya|-7.2575|112.7521|Jawa Timur|kota pahlawan|0;gunungsari|Gunungsari|-7.2998|112.7149|Jawa Timur|gunungsari|0;sukolilo|Sukolilo|-7.2930|112.7864|Jawa Timur|sukolilo|0;siwalankerto|Siwalankerto|-7.3356|112.7360|Jawa Timur|siwalankerto|0;tandes|Tandes|-7.2551|112.6740|Jawa Timur|tandes|0;kenjeran|Kenjeran|-7.2202|112.7716|Jawa Timur|kenjeran|0;pasar-turi|Stasiun Pasar Turi|-7.2483|112.7311|Jawa Timur|stasiun pasar turi|0;juanda|Bandara Juanda|-7.3735|112.7857|Jawa Timur|bandara juanda,juanda|0;medan-kota|Medan Kota|3.5730|98.6889|Sumatera Utara|medan kota|0;glugur|Glugur|3.6099|98.6749|Sumatera Utara|glugur|0;polonia|Polonia|3.5596|98.6659|Sumatera Utara|polonia|0;helvetia|Helvetia|3.6070|98.6305|Sumatera Utara|helvetia|0;perumnas|Perumnas|3.6952|98.6873|Sumatera Utara|perumnas|1;tamalanrea|Tamalanrea|-5.1105|119.4695|Sulawesi Selatan|tamalanrea|0;mariso|Mariso|-5.1553|119.4069|Sulawesi Selatan|mariso|0;tamalate|Tamalate|-5.1930|119.3862|Sulawesi Selatan|tamalate|0;panakkukang|Panakkukang|-5.1437|119.4536|Sulawesi Selatan|panakkukang|0;simpang-lima|Simpang Lima|-6.9893|110.4236|Jawa Tengah|simpang lima|0;banyumanik|Banyumanik|-7.0607|110.4189|Jawa Tengah|banyumanik|0;pedurungan|Pedurungan|-7.0012|110.4761|Jawa Tengah|pedurungan|0;genuk|Genuk|-6.9614|110.4784|Jawa Tengah|genuk|0;tembalang|Tembalang|-7.0601|110.4466|Jawa Tengah|tembalang|0;pasar-kliwon|Pasar Kliwon|-7.5836|110.8286|Jawa Tengah|pasar kliwon|0;jebres|Jebres|-7.5587|110.8407|Jawa Tengah|jebres|0;laweyan|Laweyan|-7.5700|110.7969|Jawa Tengah|laweyan|0;klojen|Klojen|-7.9771|112.6340|Jawa Timur|klojen|0;lowokwaru|Lowokwaru|-7.9563|112.6324|Jawa Timur|lowokwaru|0;sukun|Sukun|-7.9934|112.6199|Jawa Timur|sukun|1;blimbing|Blimbing|-7.9399|112.6449|Jawa Timur|blimbing|1;denpasar-barat|Denpasar Barat|-8.6609|115.1961|Bali|denpasar barat|0;denpasar-timur|Denpasar Timur|-8.6332|115.2449|Bali|denpasar timur|0;panjer|Panjer|-8.6842|115.2271|Bali|panjer|0;sanur|Sanur|-8.6946|115.2606|Bali|sanur|0;balikpapan-kota|Balikpapan Kota|-1.2671|116.8230|Kalimantan Timur|balikpapan kota|0;balikpapan-utara|Balikpapan Utara|-1.1536|116.8828|Kalimantan Timur|balikpapan utara|0;sepinggan|Sepinggan|-1.2431|116.9201|Kalimantan Timur|sepinggan|0;ilir-timur|Ilir Timur|-2.9762|104.7419|Sumatera Selatan|ilir timur|0;seberang-ulu|Seberang Ulu|-3.0245|104.7800|Sumatera Selatan|seberang ulu|0;kertapati|Kertapati|-3.0169|104.7511|Sumatera Selatan|kertapati|0;plaju|Plaju|-3.0104|104.8215|Sumatera Selatan|plaju|0;soekarno-hatta|Bandara Soekarno-Hatta|-6.1274|106.6522|Banten|soekarno hatta,bandara soekarno hatta|0;ngurah-rai|Bandara Ngurah Rai|-8.7420|115.1637|Bali|ngurah rai,bandara ngurah rai|0";

fn resolve_geo_dir() -> Result<PathBuf, String> {
    let mut cands = Vec::new();
    let add_anc = |out: &mut Vec<PathBuf>, mut p: Option<&Path>| {
        for _ in 0..4 {
            if let Some(d) = p {
                out.push(d.join("geo"));
                p = d.parent();
            }
        }
    };
    if let Ok(root) = std::env::var("SITREP_ROOT") {
        cands.push(PathBuf::from(root).join("geo"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        add_anc(&mut cands, Some(cwd.as_path()));
    }
    cands.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../geo"));
    if let Ok(exe) = std::env::current_exe() {
        add_anc(&mut cands, exe.parent());
    }
    cands
        .into_iter()
        .find(|d| d.join("gazetteer.json").is_file())
        .ok_or_else(|| "geo/gazetteer.json not found (set SITREP_ROOT)".into())
}

fn title_words(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cap = true;
    for c in s.chars() {
        out.push(if cap && c != ' ' {
            c.to_ascii_uppercase()
        } else {
            c
        });
        cap = c == ' ';
    }
    out
}

struct PoiBuilder {
    poi: Vec<PoiEntry>,
    triggers: HashMap<&'static str, Vec<usize>>,
}

impl PoiBuilder {
    fn push(&mut self, matcher: PoiMatcher, entry: PlaceEntry, keys: &[&'static str]) {
        let idx = self.poi.len();
        self.poi.push(PoiEntry { matcher, entry });
        for &k in keys {
            let v = self.triggers.entry(k).or_default();
            if v.last().copied() != Some(idx) {
                v.push(idx);
            }
        }
    }

    fn load_table(&mut self, table: &'static str, split_phrases: bool) {
        for rec in table.split(';') {
            let p: Vec<&'static str> = rec.split('|').collect();
            if p.len() < 6 {
                continue;
            }
            let la = p[2].parse().unwrap_or(0.0);
            let lo = p[3].parse().unwrap_or(0.0);
            let cue = p.get(6).copied() == Some("1");
            let entry = PlaceEntry::new(p[0], Level::Poi, p[1], la, lo, p[4], "poi");
            if split_phrases {
                for phrase_str in p[5].split(',') {
                    let words: Vec<&'static str> = phrase_str.split(' ').collect();
                    let trig = words.first().copied().unwrap_or("");
                    self.push(
                        PoiMatcher::Phrases(vec![words], cue),
                        entry.clone(),
                        &[trig],
                    );
                }
            } else {
                let phrases: Vec<Vec<&'static str>> =
                    p[5].split(',').map(|s| s.split(' ').collect()).collect();
                let keys: Vec<&'static str> =
                    phrases.iter().filter_map(|v| v.first().copied()).collect();
                self.push(PoiMatcher::Phrases(phrases, cue), entry, &keys);
            }
        }
    }
}

fn poi_entry(id: &str, name: &str, la: f64, lo: f64, prov: &str) -> PlaceEntry {
    PlaceEntry::new(id, Level::Poi, name, la, lo, prov, "poi")
}

fn build_pois() -> (Vec<PoiEntry>, HashMap<&'static str, Vec<usize>>) {
    let mut b = PoiBuilder {
        poi: Vec::with_capacity(210),
        triggers: HashMap::new(),
    };
    for rec in VOLC_TABLE.split(';') {
        let p: Vec<&'static str> = rec.split('|').collect();
        if p.len() < 4 {
            continue;
        }
        let words: Vec<&'static str> = p[0].split(' ').collect();
        let e = poi_entry(
            &format!("V{}", p[0]),
            &format!("Gunung {}", title_words(p[0])),
            p[1].parse().unwrap_or(0.0),
            p[2].parse().unwrap_or(0.0),
            "",
        );
        b.push(PoiMatcher::Volc(words.clone(), p[3] == "1"), e, &[words[0]]);
    }
    b.load_table(LM_PRE_TABLE, false);
    b.push(
        PoiMatcher::Istana,
        poi_entry("istana", "Istana Merdeka", -6.1702, 106.8272, "DKI Jakarta"),
        &["istana"],
    );
    b.push(
        PoiMatcher::Dpr,
        poi_entry("dpr", "DPR / MPR Senayan", -6.2185, 106.8002, "DKI Jakarta"),
        &["dpr", "mpr", "parlemen", "senayan"],
    );
    b.load_table(LM_POST_TABLE, false);
    b.load_table(CPOI_PART1, true);
    let mak = poi_entry(
        "makasar",
        "Makasar, Jakarta",
        -6.2696,
        106.8887,
        "DKI Jakarta",
    );
    b.push(PoiMatcher::MakasarLa, mak.clone(), &["makasar"]);
    b.push(PoiMatcher::MakasarSpan, mak, &["makasar"]);
    b.load_table(CPOI_PART2, true);
    let gent = poi_entry(
        "genteng",
        "Genteng, Surabaya",
        -7.2585,
        112.7443,
        "Jawa Timur",
    );
    for m in [
        PoiMatcher::GentengLa(true),
        PoiMatcher::GentengLa(false),
        PoiMatcher::GentengSpan,
    ] {
        b.push(m, gent.clone(), &["genteng"]);
    }
    let cnd = poi_entry("candi", "Candi, Semarang", -7.0107, 110.4290, "Jawa Tengah");
    for m in [
        PoiMatcher::CandiLa(true),
        PoiMatcher::CandiLa(false),
        PoiMatcher::CandiSpan,
    ] {
        b.push(m, cnd.clone(), &["candi"]);
    }
    (b.poi, b.triggers)
}

fn join_into(buf: &mut String, slice: &[&str]) {
    buf.clear();
    for (idx, &w) in slice.iter().enumerate() {
        if idx > 0 {
            buf.push(' ');
        }
        buf.push_str(w);
    }
}

struct MatchedOcc {
    e: PlaceEntry,
    pos: usize,
    n: usize,
}

#[derive(Default)]
struct Cand {
    e: PlaceEntry,
    pos: usize,
    n: Option<usize>,
    cue: f64,
    score: f64,
    key: Option<String>,
    start_token: usize,
    end_token: usize,
    match_key: String,
    specific: bool,
}
impl Cand {
    fn new(e: PlaceEntry, pos: usize, n: Option<usize>, cue: f64, key: Option<String>) -> Self {
        let score = e.level.score();
        Self {
            e,
            pos,
            n,
            cue,
            score,
            key,
            ..Self::default()
        }
    }
}

fn prev3<'a>(tk: &[&'a str], i: usize) -> (&'a str, &'a str, &'a str) {
    (
        i.checked_sub(1).map_or("", |j| tk[j]),
        i.checked_sub(2).map_or("", |j| tk[j]),
        i.checked_sub(3).map_or("", |j| tk[j]),
    )
}

fn is_dest_to(p1: &str, p2: &str, sets: &StaticSets) -> bool {
    (p1 == "ke" || p1 == "kepada") && sets.dest_verb.contains(p2)
}

fn near_sep(a_pos: usize, a_n: usize, b_pos: usize, b_n: usize) -> bool {
    a_pos.abs_diff(b_pos) <= 12 && !(a_pos < b_pos + b_n && b_pos < a_pos + a_n)
}

fn collect_near_parents(
    occ: &[MatchedOcc],
    pos: usize,
    n: usize,
) -> (HashSet<&str>, HashSet<&str>) {
    let (mut kabs, mut provs) = (HashSet::new(), HashSet::new());
    for m in occ {
        if near_sep(m.pos, m.n, pos, n) {
            if m.e.level == Level::Kab {
                kabs.insert(m.e.id.as_str());
            } else if m.e.level == Level::Prov && !m.e.alias && m.e.id.len() > 1 {
                provs.insert(&m.e.id[1..]);
            }
        }
    }
    (kabs, provs)
}

#[derive(Default)]
struct KecStore {
    order: Vec<KecRow>,
    by_code: HashMap<String, usize>,
    by_name: HashMap<String, Vec<KecRow>>,
    max_words: usize,
}

impl KecStore {
    fn insert(&mut self, code: String, name: String, kab_code: String) {
        let prov_code = code.get(..2).unwrap_or("").to_string();
        let kr = KecRow {
            code: code.clone(),
            name: name.clone(),
            kab_code,
            prov_code,
        };
        self.by_code.insert(code, self.order.len());
        self.order.push(kr.clone());
        let key = norm_str(&name);
        self.max_words = self.max_words.max(key.split(' ').count());
        self.by_name.entry(key).or_default().push(kr);
    }
}

fn put_idx(idx: &mut HashMap<String, Vec<PlaceEntry>>, key: String, e: PlaceEntry, min: usize) {
    if key.len() >= min {
        let list = idx.entry(key).or_default();
        if !list.iter().any(|x| x.id == e.id) {
            list.push(e);
        }
    }
}

fn load_json_opt<T: serde::de::DeserializeOwned + Default>(path: PathBuf) -> T {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn fix_map<'a>(fix: &'a Value, key: &str) -> Option<&'a Map<String, Value>> {
    fix.get(key).and_then(Value::as_object)
}

fn clean_prov_name(name: &str) -> String {
    name.replace("Daerah Khusus Ibukota ", "")
        .replace("Daerah Istimewa ", "DI ")
}

fn kab_display(name: &str, norm_key: &str, kota: bool, kota_names: &HashSet<String>) -> String {
    let pref = if !kota && kota_names.contains(norm_key) {
        "Kab. "
    } else {
        ""
    };
    format!("{pref}{}", label_fix(name))
}

impl Gazetteer {
    fn load() -> Result<Self, String> {
        let dir = resolve_geo_dir()?;
        let g_str = std::fs::read_to_string(dir.join("gazetteer.json"))
            .map_err(|e| format!("failed reading gazetteer.json: {e}"))?;
        let mut g: RawGazetteer =
            serde_json::from_str(&g_str).map_err(|e| format!("invalid gazetteer.json: {e}"))?;
        let big: RawBig = load_json_opt(dir.join("desa-kelurahan-big.json"));
        let fix: Value = load_json_opt(dir.join("coord-fixes.json"));

        if let Some(m) = fix_map(&fix, "kab") {
            for k in &mut g.kab {
                if let Some(p) = m.get(&k.code).and_then(fix_pair) {
                    (k.la, k.lo) = p;
                }
            }
        }
        if let Some(m) = fix_map(&fix, "prov") {
            for p in &mut g.prov {
                if let Some(pt) = m.get(&p.code).and_then(fix_pair) {
                    (p.la, p.lo) = pt;
                }
            }
        }

        let prov_by_code: HashMap<String, String> = g
            .prov
            .iter()
            .map(|p| (p.code.clone(), p.name.clone()))
            .collect();
        let kota_names: HashSet<String> = g
            .kab
            .iter()
            .filter(|k| k.kota)
            .map(|k| norm_str(&k.name))
            .collect();
        let mut idx: HashMap<String, Vec<PlaceEntry>> = HashMap::new();
        let mut kab_by_code: HashMap<String, PlaceEntry> = HashMap::new();
        let big_kab_name: HashMap<String, String> = big
            .kabupaten_kota
            .iter()
            .filter(|r| r.len() >= 2)
            .map(|r| (r[0].clone(), r[1].clone()))
            .collect();
        let mut orig_kab_norm: HashMap<String, String> = HashMap::new();

        for k in &mut g.kab {
            k.name = k.name.trim().to_string();
            let n = norm_str(&k.name);
            orig_kab_norm.insert(k.code.clone(), n.clone());
            let disp = kab_display(&k.name, &n, k.kota, &kota_names);
            let prov_n = prov_by_code.get(&k.prov).map(String::as_str).unwrap_or("");
            let mut e = PlaceEntry::new(&k.code, Level::Kab, &disp, k.la, k.lo, prov_n, "kab");
            (e.kota, e.prov_code, e.kab_code) =
                (k.kota, Some(k.prov.clone()), Some(k.code.clone()));
            kab_by_code.insert(k.code.clone(), e.clone());
            put_idx(&mut idx, n.clone(), e.clone(), 4);
            if n.contains(' ') && n.len() >= 8 {
                put_idx(&mut idx, n.replace(' ', ""), e.clone(), 4);
            }
            if let Some(rest) = n
                .strip_prefix("kepulauan ")
                .or_else(|| n.strip_prefix("pulau "))
                && rest.len() >= 5
                && rest != "seribu"
            {
                put_idx(&mut idx, rest.to_string(), e, 4);
            }
        }

        for row in &big.kabupaten_kota {
            let [code, raw, ..] = row.as_slice() else {
                continue;
            };
            let Some(mut base) = kab_by_code.get(code).cloned() else {
                continue;
            };
            let name = strip_ci(raw, &["kabupaten", "kota administrasi", "kota"]);
            let key = norm_str(name);
            if key.is_empty() || orig_kab_norm.get(code).is_some_and(|o| o == &key) {
                continue;
            }
            base.name = kab_display(name, &key, base.kota, &kota_names);
            base.alias = true;
            put_idx(&mut idx, key, base, 4);
        }

        let mut ik_count: HashMap<String, usize> = HashMap::new();
        for k in &g.kab {
            *ik_count
                .entry(norm_str(
                    k.ibukota.strip_prefix("Kota ").unwrap_or(&k.ibukota),
                ))
                .or_insert(0) += 1;
        }
        let sets = &*SETS;
        for k in &g.kab {
            let ik = norm_str(k.ibukota.strip_prefix("Kota ").unwrap_or(&k.ibukota));
            let first = ik.split(' ').next().unwrap_or("");
            if ik.len() >= 6
                && ik_count.get(&ik).copied() == Some(1)
                && !idx.contains_key(&ik)
                && !sets.ambig.contains(ik.as_str())
                && !sets.ik_generic.contains(first)
                && let Some(mut base) = kab_by_code.get(&k.code).cloned()
            {
                base.alias = true;
                put_idx(&mut idx, ik, base, 4);
            }
        }

        for rec in KAB_ALIAS_TABLE.split(';') {
            let p: Vec<&str> = rec.split('|').collect();
            if p.len() < 4 {
                continue;
            }
            let (target, kota) = (norm_str(p[1]), p[2] == "1");
            if let Some(k) = g
                .kab
                .iter()
                .find(|k| norm_str(&k.name) == target && k.kota == kota)
                && let Some(mut e) = kab_by_code.get(&k.code).cloned()
            {
                if !p[3].is_empty() {
                    e.name = p[3].into();
                }
                put_idx(&mut idx, p[0].into(), e, 3);
            }
        }

        let prov_alias_map: HashMap<&str, Vec<&str>> = PROV_ALIAS_TABLE
            .split(';')
            .filter_map(|r| r.split_once(':'))
            .map(|(c, a)| (c, a.split(',').collect()))
            .collect();
        for p in &g.prov {
            let disp = clean_prov_name(&p.name);
            let e = PlaceEntry::new(
                &format!("P{}", p.code),
                Level::Prov,
                &disp,
                p.la,
                p.lo,
                &p.name,
                "prov",
            );
            put_idx(&mut idx, norm_str(&p.name), e.clone(), 4);
            if let Some(aliases) = prov_alias_map.get(p.code.as_str()) {
                for &a in aliases {
                    let mut ae = e.clone();
                    ae.alias = true;
                    put_idx(&mut idx, a.into(), ae, 3);
                }
            }
        }

        for &(key, name, la, lo) in &[
            ("flores", "Flores", -8.65, 121.0),
            ("lombok", "Lombok", -8.65, 116.32),
            ("sumba", "Sumba", -9.7, 119.9),
            ("halmahera", "Halmahera", 0.8, 128.0),
        ] {
            let e = PlaceEntry::new(
                &format!("R{key}"),
                Level::Region,
                name,
                la,
                lo,
                "",
                "region",
            );
            put_idx(&mut idx, key.into(), e, 4);
        }

        let (poi, poi_triggers) = build_pois();
        let mut kecs = KecStore {
            max_words: 1,
            ..KecStore::default()
        };
        for row in &big.kecamatan {
            if row.len() >= 3 {
                kecs.insert(row[0].clone(), row[1].clone(), row[2].clone());
            }
        }
        let mut fix_kec_codes: HashSet<String> = HashSet::new();
        if let Some(fk) = fix_map(&fix, "kec") {
            for (code, entry) in fk {
                fix_kec_codes.insert(code.clone());
                if let Some(name) = entry
                    .as_array()
                    .and_then(|a| a.get(2))
                    .and_then(Value::as_str)
                    && !name.is_empty()
                    && !kecs.by_code.contains_key(code)
                {
                    kecs.insert(
                        code.clone(),
                        name.into(),
                        code.get(..5).unwrap_or("").into(),
                    );
                }
            }
        }

        let big_prov_codes: HashSet<String> = big
            .provinces
            .iter()
            .filter_map(|r| r.first().cloned())
            .collect();
        let fix_vil = fix_map(&fix, "village");
        let mut village_idx: HashMap<String, Vec<PlaceEntry>> = HashMap::new();
        let mut village_meta: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut village_keys_order: Vec<String> = Vec::new();
        let mut kec_village_pts: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
        let mut village_grid: HashMap<(i32, i32), Vec<MapItem>> = HashMap::new();
        let mut village_max_words = 1usize;

        for (code, raw_name, source_type, la, lo, status) in big.places {
            let v_fix = fix_vil.and_then(|m| m.get(&code)).and_then(fix_pair);
            let (v_la, v_lo) = v_fix.unwrap_or((la, lo));
            let err = || format!("BIG village hierarchy missing for PUM {code}");
            let parts: Vec<&str> = code.split('.').collect();
            if parts.len() < 3 {
                return Err(err());
            }
            let prov_code = parts[0].to_string();
            let kab_code = format!("{}.{}", parts[0], parts[1]);
            let kec_code = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
            let &kec_pos = kecs.by_code.get(&kec_code).ok_or_else(err)?;
            let base_kab = kab_by_code.get(&kab_code).ok_or_else(err)?;
            let prov_name = prov_by_code.get(&prov_code).ok_or_else(err)?;
            if !big_prov_codes.contains(&prov_code) {
                return Err(err());
            }
            let full_kab = big_kab_name
                .get(&kab_code)
                .map_or(base_kab.name.as_str(), String::as_str);
            let short_kab = strip_ci(full_kab, &["kabupaten", "kota administrasi", "kota"]);
            let kab_name = kab_display(short_kab, &norm_str(short_kab), base_kab.kota, &kota_names);
            let vtype = if source_type == "Desa" {
                "desa"
            } else {
                "kelurahan"
            };
            let key = norm_str(&raw_name);
            if key.is_empty() {
                return Err(format!("BIG village has empty normalized name: {code}"));
            }
            village_max_words = village_max_words.max(key.split(' ').count());
            kec_village_pts
                .entry(kec_code.clone())
                .or_default()
                .push((v_la, v_lo));
            let (v_pref, v_prio) = if vtype == "kelurahan" {
                ("Kel. ", 2.2)
            } else {
                ("Desa ", 2.0)
            };
            village_grid
                .entry(((v_la * 2.0).floor() as i32, (v_lo * 2.0).floor() as i32))
                .or_default()
                .push(MapItem::new(
                    format!("vil:{code}"),
                    format!("{v_pref}{raw_name}"),
                    to_fixed_5(v_la),
                    to_fixed_5(v_lo),
                    vtype,
                    v_prio,
                ));
            let mut e = PlaceEntry::new(
                &format!("B{code}"),
                Level::Village,
                &raw_name,
                v_la,
                v_lo,
                prov_name,
                vtype,
            );
            e.prov_code = Some(prov_code);
            e.kab_code = Some(kab_code);
            e.kec_code = Some(kec_code);
            e.code = Some(code);
            e.coordinate_source = Some(if v_fix.is_some() {
                "BIG village coord fix"
            } else {
                "BIG administrative polygon centroid"
            });
            e.boundary_status = Some(status);
            if !village_idx.contains_key(&key) {
                village_keys_order.push(key.clone());
            }
            village_idx.entry(key.clone()).or_default().push(e);
            village_meta
                .entry(key)
                .or_default()
                .push((kecs.order[kec_pos].name.clone(), kab_name));
        }

        let mut big_kec_coords: HashMap<String, (f64, f64)> = HashMap::new();
        for (k_code, pts) in kec_village_pts {
            let best = if pts.len() == 1 {
                pts[0]
            } else {
                let (mut las, mut los): (Vec<f64>, Vec<f64>) = pts.iter().copied().unzip();
                las.sort_by(f64::total_cmp);
                los.sort_by(f64::total_cmp);
                let (m_la, m_lo) = (las[pts.len() >> 1], los[pts.len() >> 1]);
                pts.into_iter()
                    .min_by(|a, b| {
                        let da = (a.0 - m_la).powi(2) + (a.1 - m_lo).powi(2);
                        let db = (b.0 - m_la).powi(2) + (b.1 - m_lo).powi(2);
                        da.total_cmp(&db)
                    })
                    .unwrap_or((m_la, m_lo))
            };
            big_kec_coords.insert(k_code, best);
        }
        if let Some(fk) = fix_map(&fix, "kec") {
            for (k_code, entry) in fk {
                if let Some(pt) = fix_pair(entry) {
                    big_kec_coords.insert(k_code.clone(), pt);
                }
            }
        }

        let map_prov = g
            .prov
            .iter()
            .map(|p| {
                MapItem::new(
                    format!("prov:{}", p.code),
                    clean_prov_name(&p.name),
                    p.la,
                    p.lo,
                    "prov",
                    5.0,
                )
            })
            .collect();
        let map_kab = g
            .kab
            .iter()
            .map(|k| {
                let (pref, kind, prio) = if k.kota {
                    ("Kota ", "kota", 4.2)
                } else {
                    ("Kab. ", "kab", 4.0)
                };
                let name = format!(
                    "{pref}{}",
                    strip_ci(label_fix(&k.name), &["kab.", "kabupaten", "kota"])
                );
                MapItem::new(format!("kab:{}", k.code), name, k.la, k.lo, kind, prio)
            })
            .collect();
        let map_kec = kecs
            .order
            .iter()
            .filter_map(|r| {
                let &(la, lo) = big_kec_coords.get(&r.code)?;
                Some(MapItem::new(
                    format!("kec:{}", r.code),
                    format!("Kec. {}", r.name),
                    to_fixed_5(la),
                    to_fixed_5(lo),
                    "kec",
                    3.0,
                ))
            })
            .collect();
        let mut seen_poi: HashSet<&str> = HashSet::new();
        let map_poi = poi
            .iter()
            .filter_map(|it| {
                let e = &it.entry;
                (!e.id.is_empty() && seen_poi.insert(&e.id)).then(|| {
                    let kind = if e.id.starts_with('V') {
                        "gunung"
                    } else {
                        "poi"
                    };
                    MapItem::new(
                        format!("poi:{}", e.id),
                        e.name.clone(),
                        e.la,
                        e.lo,
                        kind,
                        3.5,
                    )
                })
            })
            .collect();

        let (mut ambiguous_village_names, mut village_labels) = (0usize, HashSet::new());
        for key in village_keys_order {
            let (Some(rows), Some(meta)) = (village_idx.get_mut(&key), village_meta.get(&key))
            else {
                continue;
            };
            let multi = rows.len() > 1;
            ambiguous_village_names += usize::from(multi);
            for (row, (kec_name, kab_name)) in rows.iter_mut().zip(meta) {
                let kind = if row.prec == "desa" {
                    "Desa"
                } else {
                    "Kelurahan"
                };
                let mut label = format!("{kind} {}", row.name);
                if multi {
                    label.push_str(&format!(", Kecamatan {kec_name}, {kab_name}, {}", row.prov));
                }
                if village_labels.contains(&norm_str(&label)) {
                    label.push_str(&format!(", PUM {}", row.code.as_deref().unwrap_or("")));
                }
                village_labels.insert(norm_str(&label));
                row.name = label;
            }
        }

        let nodes = g
            .kab
            .iter()
            .filter_map(|k| kab_by_code.get(&k.code))
            .map(|e| Node {
                name: e.name.clone(),
                la: e.la,
                lo: e.lo,
                prov: e.prov.clone(),
            })
            .collect();
        let stats = GeoStats {
            keys: idx.len(),
            kab: g.kab.len(),
            prov: g.prov.len(),
            kec: g.kec.len(),
            poi: poi.len(),
            villages: big.counts.places,
            desa: big.counts.desa,
            kelurahan: big.counts.kelurahan,
            village_names: village_idx.len(),
            ambiguous_village_names,
        };
        Ok(Self {
            idx,
            kab_by_code,
            kec_idx: g.kec,
            big_kec_by_name: kecs.by_name,
            big_kec_coords,
            fix_kec_codes,
            big_kec_max_words: kecs.max_words,
            village_idx,
            village_max_words,
            poi,
            poi_triggers,
            map_prov,
            map_kab,
            map_kec,
            map_poi,
            village_grid,
            nodes,
            stats,
        })
    }

    pub fn locate(&self, title: &str) -> Option<Value> {
        let norm_text = norm_str(title);
        if norm_text.is_empty() {
            return None;
        }
        let raw_tk: Vec<&str> = norm_text.split(' ').collect();
        let tk = strip_not_place(&raw_tk);
        if tk.is_empty() {
            return None;
        }
        let sets = &*SETS;
        let mut tok_space_idx = Vec::with_capacity(tk.len());
        let mut off = 0usize;
        for &w in &tk {
            tok_space_idx.push(off);
            off += 1 + w.len();
        }

        let mut cands: Vec<Cand> = Vec::new();
        let mut poi_cands: Vec<usize> = Vec::new();
        for &tok in &tk {
            if let Some(list) = self.poi_triggers.get(tok) {
                poi_cands.extend_from_slice(list);
            }
        }
        poi_cands.sort_unstable();
        poi_cands.dedup();
        for poi_i in poi_cands {
            let item = &self.poi[poi_i];
            let Some((st, en)) = match_poi(&item.matcher, &tk) else {
                continue;
            };
            let before = if st > 0 { tk[st - 1] } else { "" };
            let mut pc = if sets.cue_in.contains(before) || before == "di" {
                1.5
            } else {
                0.0
            };
            if sets.prefix_kab.contains(before)
                || sets.prefix_kota.contains(before)
                || sets.prefix_prov.contains(before)
            {
                pc += 0.5;
            }
            let vb = if item.entry.id.starts_with('V') {
                1.0
            } else {
                0.0
            };
            let mut c = Cand::new(item.entry.clone(), tok_space_idx[st], None, pc + vb, None);
            (c.start_token, c.end_token, c.match_key, c.specific) =
                (st, en, tk[st..en].join(" "), en - st > 1);
            cands.push(c);
        }

        let (mut matched, mut matched_occ, mut used, mut key_buf) = (
            Vec::new(),
            Vec::new(),
            vec![false; tk.len()],
            String::with_capacity(48),
        );
        for i in 0..tk.len() {
            if used[i] {
                continue;
            }
            let (prev, prev2, prev3) = prev3(&tk, i);
            for n in (1..=4.min(tk.len() - i)).rev() {
                join_into(&mut key_buf, &tk[i..i + n]);
                let Some(list) = self.idx.get(&key_buf) else {
                    continue;
                };
                let next = tk.get(i + n).copied().unwrap_or("");
                let next1 = tk.get(i + 1).copied().unwrap_or("");
                let city_tail = (i + n >= tk.len()) && !cands.is_empty();
                const MEDAN_STOP: &[&str] = &[
                    "perang",
                    "pertempuran",
                    "tempur",
                    "rendah",
                    "legam",
                    "basah",
                    "lodak",
                    "laga",
                ];
                if (sets.ambig.contains(key_buf.as_str())
                    && !(sets.loc_cue.contains(prev)
                        || sets.prefix_kab.contains(prev)
                        || sets.prefix_kota.contains(prev)
                        || city_tail))
                    || (key_buf == "malaka" && prev == "selat")
                    || (key_buf == "medan" && MEDAN_STOP.contains(&next))
                    || (key_buf == "metro" && matches!(next1, "jaya" | "tv"))
                {
                    continue;
                }
                let e = pick_idx_entry(list, prev, prev2, sets);
                let mut cue = if sets.cue_in.contains(prev) || prev == "di" {
                    1.5
                } else {
                    0.0
                };
                if sets.cue_from.contains(prev) {
                    cue -= 1.5;
                }
                if sets.prefix_kab.contains(prev)
                    || sets.prefix_kota.contains(prev)
                    || sets.prefix_prov.contains(prev)
                {
                    cue += 0.5;
                }
                const FAC: &[&str] = &["rsud", "rs", "rsup", "rsu", "rumah", "puskesmas", "klinik"];
                let dest_fac = FAC.contains(&prev)
                    && (is_dest_to(prev2, prev3, sets) || sets.dest_verb.contains(prev2));
                if dest_fac
                    || is_dest_to(prev, prev2, sets)
                    || (sets.dest_verb.contains(prev) && !sets.prefix_kab.contains(prev))
                {
                    used[i..i + n].fill(true);
                    break;
                }
                matched.push(e.clone());
                matched_occ.push(MatchedOcc {
                    e: e.clone(),
                    pos: i,
                    n,
                });
                cands.push(Cand::new(e, i, Some(n), cue, Some(key_buf.clone())));
                used[i..i + n].fill(true);
                break;
            }
        }

        let kec_max = self.big_kec_max_words.max(3);
        for i in 0..tk.len() {
            let prev = if i >= 1 { tk[i - 1] } else { "" };
            let explicit = sets.prefix_kec.contains(prev);
            let loose = !explicit && sets.kec_loose.contains(prev);
            if !explicit && !loose {
                continue;
            }
            for n in (1..=kec_max.min(tk.len() - i)).rev() {
                join_into(&mut key_buf, &tk[i..i + n]);
                let big_rows = self
                    .big_kec_by_name
                    .get(&key_buf)
                    .map_or(&[][..], Vec::as_slice);
                let parents: Vec<&str> = if let Some(p) = self.kec_idx.get(&key_buf) {
                    p.iter().map(String::as_str).collect()
                } else {
                    let mut out = Vec::new();
                    for r in big_rows {
                        if !out.contains(&r.kab_code.as_str()) {
                            out.push(r.kab_code.as_str());
                        }
                    }
                    out
                };
                if parents.is_empty() {
                    continue;
                }
                if loose
                    && (parents.len() != 1
                        || (n == 1 && key_buf.len() < 9)
                        || sets.kec_stop.contains(key_buf.as_str())
                        || used[i..i + n].iter().any(|&u| u))
                {
                    break;
                }
                let pc = if parents.len() == 1 {
                    Some(parents[0])
                } else {
                    parents.iter().copied().find(|&c| {
                        let cp = self.kab_by_code.get(c).map_or("", |k| k.prov.as_str());
                        matched
                            .iter()
                            .any(|m| m.id == c || (m.level == Level::Prov && m.prov == cp))
                    })
                };
                let Some(parent) = pc.and_then(|c| self.kab_by_code.get(c)) else {
                    break;
                };
                let district: Vec<&KecRow> = big_rows
                    .iter()
                    .filter(|r| r.kab_code == parent.id)
                    .collect();
                let ke = (district.len() == 1).then(|| district[0]);
                let d_name = ke.map_or(
                    if key_buf.is_empty() {
                        "kecamatan"
                    } else {
                        &key_buf
                    },
                    |r| r.name.as_str(),
                );
                let mut e = parent.clone();
                e.coordinate_source = Some("parent kab/kota center");
                if let Some(ke) = ke {
                    (e.id, e.prov_code, e.kab_code, e.kec_code) = (
                        format!("K{}", ke.code),
                        Some(ke.prov_code.clone()),
                        Some(ke.kab_code.clone()),
                        Some(ke.code.clone()),
                    );
                    if let Some(&(la, lo)) = self.big_kec_coords.get(&ke.code) {
                        (e.la, e.lo) = (la, lo);
                        e.coordinate_source = Some(if self.fix_kec_codes.contains(&ke.code) {
                            "Kepmendagri 2025 kecamatan center"
                        } else {
                            "BIG kecamatan village medoid"
                        });
                    }
                }
                (e.level, e.name, e.prec) = (
                    Level::Kec,
                    format!("Kecamatan {d_name}, {}", parent.name),
                    "kec",
                );
                cands.push(Cand::new(e, i, None, 1.5, Some(key_buf.clone())));
                break;
            }
        }

        let mut district_mentions: Vec<(usize, usize, HashSet<String>)> = Vec::new();
        for i in 1..tk.len() {
            if !sets.prefix_kec.contains(tk[i - 1]) {
                continue;
            }
            for n in (1..=self.big_kec_max_words.min(tk.len() - i)).rev() {
                join_into(&mut key_buf, &tk[i..i + n]);
                let Some(rows) = self.big_kec_by_name.get(&key_buf) else {
                    continue;
                };
                let (kabs, provs) = collect_near_parents(&matched_occ, i, n);
                let codes: HashSet<String> = rows
                    .iter()
                    .filter(|r| {
                        (kabs.is_empty() || kabs.contains(r.kab_code.as_str()))
                            && (provs.is_empty() || provs.contains(r.prov_code.as_str()))
                    })
                    .map(|r| r.code.clone())
                    .collect();
                if !codes.is_empty() {
                    district_mentions.push((i, n, codes));
                }
                break;
            }
        }

        for i in 0..tk.len() {
            let (prev, prev2, prev3) = prev3(&tk, i);
            for n in (1..=self.village_max_words.min(tk.len() - i)).rev() {
                join_into(&mut key_buf, &tk[i..i + n]);
                let Some(all_rows) = self.village_idx.get(&key_buf) else {
                    continue;
                };
                let type_cue = village_type_cue(prev);
                let generic_cue = sets.village_generic_cue.contains(prev);
                let explicit_cue = type_cue.is_some() || generic_cue;
                if is_dest_to(prev, prev2, sets)
                    || (sets.dest_verb.contains(prev) && !sets.prefix_kab.contains(prev))
                    || (explicit_cue
                        && (sets.village_dest_cue.contains(prev2)
                            || is_dest_to(prev2, prev3, sets)))
                {
                    break;
                }
                let (kabs, provs) = collect_near_parents(&matched_occ, i, n);
                let kecs: Vec<&HashSet<String>> = district_mentions
                    .iter()
                    .filter(|m| near_sep(m.0, m.1, i, n))
                    .map(|m| &m.2)
                    .collect();
                let rows: Vec<&PlaceEntry> = all_rows
                    .iter()
                    .filter(|r| {
                        type_cue.is_none_or(|tc| r.prec == tc)
                            && (kecs.is_empty()
                                || r.kec_code
                                    .as_ref()
                                    .is_some_and(|kc| kecs.iter().any(|c| c.contains(kc))))
                            && (kabs.is_empty()
                                || r.kab_code.as_deref().is_some_and(|c| kabs.contains(c)))
                            && (provs.is_empty()
                                || r.prov_code.as_deref().is_some_and(|c| provs.contains(c)))
                    })
                    .collect();
                if rows.len() != 1 {
                    continue;
                }
                let v = rows[0];
                let kec_ctx = v
                    .kec_code
                    .as_ref()
                    .is_some_and(|kc| kecs.iter().any(|c| c.contains(kc)));
                let parent_ctx = v.kab_code.as_deref().is_some_and(|c| kabs.contains(c))
                    || v.prov_code.as_deref().is_some_and(|c| provs.contains(c));
                let loc_cue = sets.village_loc_cue.contains(prev);
                if (sets.village_foreign.contains(key_buf.as_str())
                    && type_cue.is_none()
                    && !kec_ctx)
                    || (sets.village_stop.contains(key_buf.as_str()) && type_cue.is_none())
                    || (!explicit_cue
                        && (self.idx.contains_key(&key_buf) || used[i..i + n].iter().any(|&u| u)))
                    || (!explicit_cue && !kec_ctx && !loc_cue)
                    || (generic_cue
                        && type_cue.is_none()
                        && !kec_ctx
                        && !parent_ctx
                        && !(all_rows.len() == 1 && key_buf.len() >= 6))
                    || (!explicit_cue
                        && !kec_ctx
                        && !parent_ctx
                        && !(all_rows.len() == 1
                            && loc_cue
                            && key_buf.contains(' ')
                            && key_buf.len() >= 8))
                {
                    continue;
                }
                let (v_pk, suffix) = (prov_key(&v.prov), format!(" {key_buf}"));
                if cands.iter().any(|c| {
                    c.e.level == Level::Poi
                        && c.specific
                        && (c.match_key == key_buf || c.match_key.ends_with(&suffix))
                        && (c.e.prov.is_empty() || prov_key(&c.e.prov) == v_pk)
                }) {
                    continue;
                }
                let poi_ov: Vec<&Cand> = cands
                    .iter()
                    .filter(|c| c.e.level == Level::Poi && c.start_token < i + n && c.end_token > i)
                    .collect();
                if !poi_ov.is_empty()
                    && !explicit_cue
                    && !kec_ctx
                    && (!parent_ctx
                        || poi_ov
                            .iter()
                            .any(|c| c.e.prov.is_empty() || prov_key(&c.e.prov) == v_pk))
                {
                    continue;
                }
                let mut cue = match (explicit_cue, kec_ctx, parent_ctx) {
                    (true, _, _) => 3.0,
                    (_, true, _) => 2.2,
                    (_, _, true) => 1.6,
                    _ => 0.8,
                };
                if sets.cue_from.contains(prev2) && !explicit_cue {
                    cue -= 1.5;
                }
                let mut c = Cand::new(v.clone(), i, Some(n), cue, Some(key_buf.clone()));
                (c.start_token, c.end_token) = (i, i + n);
                cands.push(c);
                break;
            }
        }

        if cands.is_empty() {
            return None;
        }
        crate::js::sort(&mut cands, |a, b| {
            let diff = (b.score + b.cue) - (a.score + a.cue);
            if diff != 0.0 {
                diff
            } else {
                a.pos as f64 - b.pos as f64
            }
        });

        for _ in 0..6 {
            let Some(top) = cands.first() else { break };
            if top.e.level != Level::Poi || top.e.id.starts_with('V') || top.e.prov.is_empty() {
                break;
            }
            let (top_pk, top_city) = (prov_key(&top.e.prov), poi_city(&top.e.id));
            let has_alt = cands.iter().skip(1).any(|c| {
                (c.e.level == Level::Kab
                    || (c.e.level == Level::Village && c.cue >= 2.0)
                    || (c.e.level == Level::Prov && c.cue > 0.0))
                    && ((!c.e.prov.is_empty() && prov_key(&c.e.prov) != top_pk)
                        || (c.e.level == Level::Kab
                            && top_city.is_some_and(|city| !norm_str(&c.e.name).contains(city))))
            });
            if !has_alt {
                break;
            }
            cands.remove(0);
        }

        let best = cands.first()?;
        let all_arr = build_all_chain(&cands, best, &tk, sets);
        Some(format_locate_result(best, all_arr))
    }

    pub(crate) fn query_map_labels_raw(
        &self,
        s: f64,
        w: f64,
        n: f64,
        e: f64,
        z_raw: f64,
        limit_raw: f64,
    ) -> Vec<MapLabel> {
        let def = |v: f64, d: f64| if v == 0.0 || v.is_nan() { d } else { v };
        let z = crate::js::math_round(def(z_raw, 5.0)).clamp(4.0, 18.0);
        let limit = def(limit_raw, 140.0).clamp(10.0, 240.0) as usize;
        if ![s, w, n, e].iter().all(|x| x.is_finite())
            || s > n
            || w > e
            || s < -90.0
            || n > 90.0
            || w < -180.0
            || e > 180.0
        {
            return Vec::new();
        }
        let (c_la, c_lo) = ((s + n) / 2.0, (w + e) / 2.0);
        let in_box = |it: &&MapItem| it.la >= s && it.la <= n && it.lo >= w && it.lo <= e;
        let mut cands: Vec<&MapItem> = Vec::new();
        let layers: &[&[MapItem]] = if z <= 6.0 {
            &[&self.map_prov]
        } else if z <= 8.0 {
            &[&self.map_prov, &self.map_kab]
        } else if z <= 12.0 {
            &[&self.map_kab, &self.map_poi, &self.map_kec]
        } else {
            &[&self.map_poi, &self.map_kec]
        };
        for layer in layers {
            cands.extend(layer.iter().filter(in_box));
        }
        if z > 12.0 {
            let (y0, y1, x0, x1) = (
                (s * 2.0).floor() as i32,
                (n * 2.0).floor() as i32,
                (w * 2.0).floor() as i32,
                (e * 2.0).floor() as i32,
            );
            if (y1 - y0 + 1) as i64 * (x1 - x0 + 1) as i64 <= 64 {
                for gy in y0..=y1 {
                    for gx in x0..=x1 {
                        if let Some(b) = self.village_grid.get(&(gy, gx)) {
                            cands.extend(b.iter().filter(in_box));
                        }
                    }
                }
            }
        }
        crate::js::sort(&mut cands, |a, b| {
            let dp = b.prio - a.prio;
            if dp != 0.0 {
                dp
            } else {
                ((a.la - c_la).powi(2) + (a.lo - c_lo).powi(2))
                    - ((b.la - c_la).powi(2) + (b.lo - c_lo).powi(2))
            }
        });
        cands
            .into_iter()
            .take(limit)
            .map(|it| MapLabel {
                id: it.id.clone(),
                name: it.name.clone(),
                la: it.la,
                lo: it.lo,
                kind: it.kind.into(),
            })
            .collect()
    }
}

fn pick_idx_entry(list: &[PlaceEntry], prev: &str, prev2: &str, sets: &StaticSets) -> PlaceEntry {
    if list.len() <= 1 {
        return list[0].clone();
    }
    let want_prov = sets.prefix_prov.contains(prev) || sets.prefix_prov.contains(prev2);
    let want_kab =
        sets.prefix_kab.contains(prev) || (prev != "kota" && sets.prefix_kab.contains(prev2));
    let want_kota = sets.prefix_kota.contains(prev)
        || (sets.prefix_kota.contains(prev2) && !sets.prefix_kab.contains(prev));
    want_prov
        .then(|| list.iter().find(|x| x.level == Level::Prov))
        .flatten()
        .or_else(|| {
            want_kab
                .then(|| list.iter().find(|x| x.level == Level::Kab && !x.kota))
                .flatten()
        })
        .or_else(|| want_kota.then(|| list.iter().find(|x| x.kota)).flatten())
        .or_else(|| list.iter().find(|x| x.kota))
        .unwrap_or(&list[0])
        .clone()
}

fn place_base_map(e: &PlaceEntry) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("name".into(), e.name.clone().into());
    m.insert("la".into(), crate::js::number(e.la));
    m.insert("lo".into(), crate::js::number(e.lo));
    m.insert("prec".into(), e.prec.into());
    m.insert("prov".into(), e.prov.clone().into());
    m
}

fn build_all_chain(cands: &[Cand], best: &Cand, tk: &[&str], sets: &StaticSets) -> Vec<Value> {
    let e = &best.e;
    if best.n.is_none()
        || !matches!(
            e.level,
            Level::Kab | Level::Prov | Level::Region | Level::Village
        )
    {
        return Vec::new();
    }
    let mut same: Vec<&Cand> = cands
        .iter()
        .filter(|c| c.n.is_some() && c.e.level == e.level && c.cue >= 0.0)
        .collect();
    crate::js::sort(&mut same, |a, b| a.pos as f64 - b.pos as f64);
    let mut seen = HashSet::new();
    let uniq: Vec<&Cand> = same
        .into_iter()
        .filter(|c| seen.insert(c.e.id.as_str()))
        .collect();
    if uniq.len() <= 1 || !uniq.iter().any(|c| c.e.id == e.id) {
        return Vec::new();
    }
    let mut chain = vec![uniq[0]];
    for &cand in &uniq[1..] {
        let prev_c = chain[chain.len() - 1];
        let start = prev_c.pos + prev_c.n.unwrap_or(0);
        let gap = if start < cand.pos {
            &tk[start..cand.pos]
        } else {
            &[]
        };
        let joined = if e.level == Level::Village {
            gap.len() <= 4
                && gap.iter().all(|&w| {
                    sets.sep_ok.contains(w)
                        || village_type_cue(w).is_some()
                        || sets.village_generic_cue.contains(w)
                })
        } else {
            gap.len() <= 2 && gap.iter().all(|&w| sets.sep_ok.contains(w))
        };
        if joined {
            chain.push(cand);
        } else {
            break;
        }
    }
    if chain.len() <= 1 || !chain.iter().any(|c| c.e.id == e.id) {
        return Vec::new();
    }
    chain
        .into_iter()
        .take(4)
        .map(|c| {
            let mut m = place_base_map(&c.e);
            if let Some(code) = &c.e.code {
                m.insert("code".into(), code.clone().into());
            }
            Value::Object(m)
        })
        .collect()
}

fn format_locate_result(best: &Cand, all_arr: Vec<Value>) -> Value {
    let e = &best.e;
    let mut out = place_base_map(e);
    out.insert(
        "via".into(),
        best.key.clone().unwrap_or_else(|| e.id.clone()).into(),
    );
    out.insert("all".into(), Value::Array(all_arr));
    let mut put = |k: &str, v: Option<&str>| {
        if let Some(v) = v {
            out.insert(k.into(), v.into());
        }
    };
    put("provCode", e.prov_code.as_deref());
    put("kabCode", e.kab_code.as_deref());
    put("kecCode", e.kec_code.as_deref());
    if e.code.is_none() {
        put("coordinateSource", e.coordinate_source);
    } else {
        put("code", e.code.as_deref());
        put("coordinateSource", e.coordinate_source);
        put("boundaryStatus", e.boundary_status.as_deref());
    }
    Value::Object(out)
}

fn strip_not_place<'a>(tk: &[&'a str]) -> Vec<&'a str> {
    const METRO_W2: &[&str] = &[
        "bekasi",
        "jakarta",
        "tangerang",
        "depok",
        "selatan",
        "timur",
        "barat",
        "utara",
        "pusat",
    ];
    let (mut out, mut i) = (Vec::with_capacity(tk.len()), 0);
    while i < tk.len() {
        let (w0, w1, w2) = (
            tk[i],
            tk.get(i + 1).copied().unwrap_or(""),
            tk.get(i + 2).copied().unwrap_or(""),
        );
        let skip3 = (w0 == "polda" && w1 == "metro" && w2 == "jaya")
            || (w0 == "batu" && w1 == "bara" && w2 == "kalori")
            || (matches!(w0, "harga" | "pembangkit" | "tambang") && w1 == "batu" && w2 == "bara");
        let skip2 = (matches!(w0, "polres" | "polresta")
            && w1 == "metro"
            && METRO_W2.contains(&w2))
            || (w0 == "metro" && matches!(w1, "tv" | "jaya"))
            || (matches!(w0, "serang" | "serangan") && matches!(w1, "balik" | "udara" | "siber"));
        if skip3 {
            i += 3;
        } else if skip2 {
            i += 2;
        } else {
            out.push(w0);
            i += 1;
        }
    }
    out
}

fn starts_with(tk: &[&str], at: usize, words: &[&str]) -> bool {
    at + words.len() <= tk.len() && &tk[at..at + words.len()] == words
}

fn match_prefixed(
    tk: &[&str],
    word: &str,
    opt_pref: bool,
    req_pref: bool,
    follow: impl Fn(usize) -> bool,
) -> Option<(usize, usize)> {
    (0..tk.len()).find_map(|t| {
        if (opt_pref || req_pref)
            && matches!(tk[t], "di" | "ke" | "kecamatan" | "kec")
            && tk.get(t + 1) == Some(&word)
            && follow(t + 2)
        {
            Some((t, t + 2))
        } else if !req_pref && tk[t] == word && follow(t + 1) {
            Some((t, t + 1))
        } else {
            None
        }
    })
}

fn match_city_span(tk: &[&str], heads: &[&str], tail: &str) -> Option<(usize, usize)> {
    (0..tk.len()).find_map(|t| {
        if !heads.contains(&tk[t]) {
            return None;
        }
        (0..=4).rev().find_map(|k| {
            let end_idx = t + 1 + k;
            (end_idx < tk.len()
                && tk[end_idx] == tail
                && tk[t + 1..end_idx]
                    .iter()
                    .all(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase())))
            .then_some((t, end_idx + 1))
        })
    })
}

fn match_poi(matcher: &PoiMatcher, tk: &[&str]) -> Option<(usize, usize)> {
    match matcher {
        PoiMatcher::Volc(words, bare) => (0..tk.len()).find_map(|t| {
            let wl = words.len();
            if tk[t] == "awan" && tk.get(t + 1) == Some(&"panas") && starts_with(tk, t + 2, words) {
                Some((t, t + 2 + wl))
            } else if matches!(
                tk[t],
                "gunung" | "gunungapi" | "gn" | "g" | "erupsi" | "letusan" | "status" | "kawah"
            ) && starts_with(tk, t + 1, words)
            {
                Some((t, t + 1 + wl))
            } else if *bare && starts_with(tk, t, words) {
                Some((t, t + wl))
            } else {
                None
            }
        }),
        PoiMatcher::Phrases(phrases, cue) => (0..tk.len()).find_map(|t| {
            if *cue && (t == 0 || !SETS.cue_pre.contains(tk[t - 1])) {
                return None;
            }
            phrases
                .iter()
                .find_map(|p| starts_with(tk, t, p).then_some((t, t + p.len())))
        }),
        PoiMatcher::Istana => (0..tk.len()).find_map(|t| {
            let is_pre = "gedung,depan,sekitar,area,kawasan,menuju,geruduk,segel,kepung,di,ke"
                .split(',')
                .any(|w| w == tk[t]);
            if tk[t] == "istana"
                && matches!(tk.get(t + 1).copied(), Some("negara" | "merdeka" | "presiden"))
            {
                return Some((t, t + 2));
            }
            if ((matches!(tk[t], "demo" | "aksi" | "rapat") && tk.get(t + 1) == Some(&"di"))
                || (tk[t] == "panggil" && tk.get(t + 1) == Some(&"ke")))
                && tk.get(t + 2) == Some(&"istana")
            {
                return Some((t, t + 3));
            }
            if is_pre && tk.get(t + 1) == Some(&"istana") {
                return Some((t, t + 2));
            }
            if tk[t] == "istana" {
                let (n1, n2) = (
                    tk.get(t + 1).copied().unwrap_or(""),
                    tk.get(t + 2).copied().unwrap_or(""),
                );
                let excl = "maimun,pagaruyung,kadriah,siak,alwatzikhoebillah,bogor,cipanas,tampaksiring,garuda"
                    .split(',')
                    .any(|w| w == n1);
                if !excl && !(n1 == "anak" && n2 == "anak") {
                    return Some((t, t + 1));
                }
            }
            None
        }),
        PoiMatcher::Dpr => (0..tk.len()).find_map(|t| {
            let is_pre = "gedung,depan,kompleks,kawasan,menuju,geruduk,segel,kepung,di,ke"
                .split(',')
                .any(|w| w == tk[t]);
            let chk = |a: usize| {
                matches!(tk.get(a).copied(), Some("dpr" | "mpr" | "parlemen")).then_some((t, a + 1))
            };
            if matches!(tk[t], "demo" | "aksi")
                && tk.get(t + 1) == Some(&"di")
                && let Some(m) = chk(t + 2)
            {
                return Some(m);
            }
            if is_pre && let Some(m) = chk(t + 1) {
                return Some(m);
            }
            if tk[t] == "senayan" {
                Some((t, t + 1))
            } else if tk[t] == "dpr" && tk.get(t + 1) == Some(&"ri") {
                Some((t, t + 2))
            } else {
                None
            }
        }),
        PoiMatcher::MakasarLa => match_prefixed(tk, "makasar", true, false, |i| {
            matches!(tk.get(i).copied(), Some("jakarta" | "dki"))
        }),
        PoiMatcher::MakasarSpan => match_city_span(tk, &["jakarta", "dki"], "makasar"),
        PoiMatcher::GentengLa(req) => match_prefixed(tk, "genteng", false, *req, |i| {
            tk.get(i) == Some(&"surabaya")
        }),
        PoiMatcher::GentengSpan => match_city_span(tk, &["surabaya"], "genteng"),
        PoiMatcher::CandiLa(req) => match_prefixed(tk, "candi", false, *req, |i| {
            tk.get(i) == Some(&"semarang")
                || (matches!(tk.get(i).copied(), Some("kota" | "kabupaten"))
                    && tk.get(i + 1) == Some(&"semarang"))
        }),
        PoiMatcher::CandiSpan => match_city_span(tk, &["semarang"], "candi"),
    }
}

pub fn nodes() -> &'static [Node] {
    gazetteer().map(|g| g.nodes.as_slice()).unwrap_or(&[])
}

pub fn stats() -> GeoStats {
    gazetteer().map(|g| g.stats.clone()).unwrap_or_default()
}

pub fn locate_text(text: &str) -> Option<Value> {
    gazetteer().ok()?.locate(text)
}

pub fn query_map_labels_typed(
    s: f64,
    w: f64,
    n: f64,
    e: f64,
    z: Option<f64>,
    limit: Option<usize>,
) -> Vec<MapLabel> {
    let lim = limit.map_or(140.0, |v| v as f64);
    gazetteer()
        .map(|g| g.query_map_labels_raw(s, w, n, e, z.unwrap_or(5.0), lim))
        .unwrap_or_default()
}
