#![forbid(unsafe_code)]

use crate::port::tables::{CEREMONY_RE, re};
use regex::Regex;
use serde_json::{Value, json};
use std::sync::LazyLock;

pub struct FrameHit {
    pub topic: &'static str,
    pub category: &'static str,
    pub action: &'static str,
    pub strong: bool,
    pub event_type: &'static str,
    pub eyewitness_cue: bool,
    pub minor_waterlogging: bool,
}

pub struct EvalClause<'a> {
    pub raw: &'a str,
    pub norm_geo: &'a str,
    pub doc_raw: &'a str,
}

macro_rules! lazy_re {
    ($name:ident, $pat:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| re($pat));
    };
}

// banjir
lazy_re!(
    BANJIR_1,
    r"(?-u:\b)banjir\s+bandang(?-u:\b)|(?-u:\b)banjir\s+rob(?-u:\b)|(?-u:\b)banjir\s+lahar(?-u:\b)|(?-u:\b)banjir\s+lumpur(?-u:\b)"
);
lazy_re!(
    BANJIR_LONGSOR,
    r"(?-u:\b)tanah\s+longsor(?-u:\b)|(?-u:\b)longsor(?-u:\b)"
);
lazy_re!(
    BANJIR_LONGSOR_META,
    r"(?-u:\b)longsor\s+(?:suara|kursi|saham)(?-u:\b)"
);
lazy_re!(
    BANJIR_LONGSOR_ACT,
    r"(?-u:\b)(?:terjadi|melanda|menutup|menimbun|menerjang|rusak|ambruk|terputus|korban|evakuasi|warga|rumah|jalan|tebing|di\s+[a-z])(?-u:\b)"
);
lazy_re!(
    BANJIR_AIR_1,
    r"(?-u:\b)(?:air(?:nya)?(?:\s+(?:hujan|kali|sungai|selokan|got))?)\s+(?:(?:sudah|mulai|lagi|terus|makin)\s+){0,3}(?:naik\s+(?:masuk|ke)|masuk|naik|merendam|menggenang|mengalir\s+deras)(?-u:\b)[^.!?\n]{0,40}(?-u:\b)(?:rumah|kamar|teras|halaman|kosan|kontrakan|ruko|toko|gang|jalan|pemukiman|warga)(?-u:\b)"
);
lazy_re!(
    BANJIR_AIR_2,
    r"(?-u:\b)(?:air|banjir)\s+(?:sudah\s+|mulai\s+|masih\s+)?se(?:mata\s+kaki|betis|lutut|pinggang|dada|leher)(?-u:\b)"
);
lazy_re!(
    BANJIR_AIR_EXCL,
    r"(?-u:\b)(?:keran|kran|pipa|toren|galon|ember|ac\s+netes|bocor\s+atap)(?-u:\b)"
);
lazy_re!(
    BANJIR_GEN,
    r"(?-u:\b)(?:banjir|kebanjiran|terendam|merendam|menggenangi|air\s+meluap|sungai\s+meluap|kali\s+meluap|tanggul\s+jebol|air\s+bah)(?-u:\b)"
);
lazy_re!(
    BANJIR_STRONG_1,
    r"(?-u:\b)(?:merendam|terendam|menggenangi|tanggul\s+jebol|air\s+meluap|sungai\s+meluap|kali\s+meluap|air\s+bah|mengungsi|evakuasi|ketinggian\s+air|setinggi|se(?:mata\s+kaki|betis|lutut|pinggang|dada)|rumah\s+warga|pemukiman|jalan\s+raya|warga\s+terdampak|bpbd|bnpb|damkar|banjir\s+(?:parah|besar)|melanda|menerjang|mengepung|kebanjiran)(?-u:\b)|(?-u:\b)caah\s+gede(?-u:\b)"
);
lazy_re!(
    BANJIR_STRONG_2,
    r"(?-u:\b)(?:banjir\s+(?:parah|besar))(?-u:\b)"
);
lazy_re!(BANJIR_GENANGAN, r"(?-u:\b)genangan(?-u:\b)");

// kebakaran
lazy_re!(
    FIRE_EXCL_1,
    r"(?-u:\b)(?:film|bioskop|cfd|car\s+free\s+day|kolaborasi|strategi\s+film|edukasi|sosialisasi|simulasi|lomba)(?-u:\b)"
);
lazy_re!(
    FIRE_EXCL_2,
    r"(?-u:\b)(?:api\s+melalap|ludes|hangus|terbakar\s+hebat|kebakaran\s+(?:rumah|ruko|gudang|pabrik|pasar|gedung))(?-u:\b)"
);
lazy_re!(
    FIRE_MAIN,
    r"(?-u:\b)(?:kebakaran|terbakar|kobaran\s+api|si\s+jago\s+merah|dilalap\s+api|ludes\s+dilalap|asap\s+tebal|damkar)(?-u:\b)"
);
lazy_re!(
    FIRE_FOREST,
    r"(?-u:\b)(?:kebakaran\s+(?:hutan|lahan)|karhutla)(?-u:\b)"
);
lazy_re!(
    FIRE_ACT,
    r"(?-u:\b)(?:kebakaran|terbakar|kobaran\s+api|si\s+jago\s+merah|dilalap|ludes|hangus|padamkan|dipadamkan|memadamkan|asap\s+tebal)(?-u:\b)"
);
lazy_re!(
    FIRE_STRONG,
    r"(?-u:\b)(?:rumah|ruko|gedung|bangunan|pabrik|gudang|pasar|toko|kios|warung|sekolah|kampus|kantor|kosan|kontrakan|hotel|mall|bioskop|stasiun|mobil|motor|bus|truk|kapal|gardu|trafo|kabel|lahan|api|asap|ludes|hangus|padamkan|dipadamkan|evakuasi|korban|luka|tewas|meninggal|damkar|pemadam|di\s+[a-z])(?-u:\b)"
);

// karhutla
lazy_re!(
    KARHUTLA_MAIN,
    r"(?-u:\b)(?:karhutla|kebakaran\s+(?:hutan|lahan)|kabut\s+asap|titik\s+panas|hotspot)(?-u:\b)"
);
lazy_re!(
    KARHUTLA_WIFI,
    r"(?i)(?-u:\b)(?:wifi|wi-fi|tethering|phone|fone|hp|ponsel|kuota|paket\s+data|download|gta|game|connect|password)(?-u:\b)"
);
lazy_re!(KARHUTLA_HOTSPOT, r"(?-u:\b)hotspot(?-u:\b)");
lazy_re!(
    KARHUTLA_HOTSPOT_CTX,
    r"(?-u:\b)(?:karhutla|hutan|lahan|gambut|kabut\s+asap|titik\s+panas|kebakaran|terpantau|satelit|bmkg|bnpb|bpbd|sipongi|riau|kalimantan|sumatera|jambi|sumsel)(?-u:\b)"
);
lazy_re!(
    KARHUTLA_STRONG,
    r"(?-u:\b)(?:meluas|terjadi|membakar|terpantau|hanguskan|padamkan|pemadaman|kabut\s+asap|lahan\s+gambut|sipongi|manggala\s+agni|bpbd|bnpb)(?-u:\b)"
);

// gempa
lazy_re!(TSUNAMI_RE, r"(?-u:\b)tsunami(?-u:\b)");
lazy_re!(
    TSUNAMI_EXCL,
    r"(?i)(?-u:\b)(?:surf(?:ing)?|ocean\s+tsunami|crowcaster)(?-u:\b)|(?-u:\b)R[1-9]\s+#[0-9]+(?-u:\b)|\$[0-9]+\.[0-9]+"
);
lazy_re!(
    TSUNAMI_STRONG,
    r"(?-u:\b)(?:peringatan(?:\s+dini)?|warning|waspada|siaga|awas|terjadi|menerjang|hantam|gelombang|air\s+laut\s+surut|bmkg|evakuasi)(?-u:\b)"
);
lazy_re!(
    ERUPSI_1,
    r"(?-u:\b)(?:erupsi|gunung\s+meletus|letusan|awan\s+panas|hujan\s+abu|guguran\s+lava|lahar\s+(?:dingin|hujan))(?-u:\b)"
);
lazy_re!(
    ERUPSI_2,
    r"(?-u:\b)(?:gunung|g\.)\s+[a-z]+(?-u:\b)[^.!?\n]{0,35}(?-u:\b)status[^.!?\n]{0,20}(?-u:\b)(?:awas|siaga|waspada)(?-u:\b)"
);
lazy_re!(
    ERUPSI_STRONG,
    r"(?-u:\b)(?:gunung|merapi|semeru|marapi|lewotobi|sinabung|bromo|kelud|krakatau|raung|dukono|ibu|karo|agam|flores|semburkan|melontarkan|terjadi|status[^.!?\n]{0,20}(?-u:\b)(?:awas|siaga|waspada)|pvmbg|vona|hujan\s+abu|awan\s+panas|evakuasi)(?-u:\b)"
);
lazy_re!(
    GEMPA_1,
    r"(?i)(?-u:\b)(?:gempa|guncangan|getaran|magnitudo|(?-u:\b)m\s*[0-9]+(?:[.,][0-9]+)?(?-u:\b)|(?-u:\b)mag\s*[:=]?\s*[0-9])"
);
lazy_re!(
    GEMPA_NOUN,
    r"(?-u:\b)(?:gempa(?:\s+bumi|\s+tektonik|\s+vulkanik|\s+susulan)?|magnitudo|(?-u:\b)mag\s*[:=]?\s*[0-9]|skala\s+richter|episentrum|pusat\s+gempa|bmkg)(?-u:\b)"
);
lazy_re!(
    GEMPA_STRONG,
    r"(?-u:\b)(?:mengguncang|diguncang|guncang|guncangan|getaran|terasa|dirasakan|merasakan|bergetar|bergoyang|panik|berhamburan|keluar\s+rumah|retak|roboh|ambruk|rusak|kedalaman|kedlmn|(?-u:\b)m\s*[0-9]+(?:[.,][0-9]+)?(?-u:\b)|(?-u:\b)mag(?:nitudo)?\s*[:=]?\s*[0-9]+(?:[.,][0-9]+)?(?-u:\b)|[0-9]+(?:[.,][0-9]+)?\s*(?:sr|skala\s+richter|mag|magnitudo)|episentrum|bmkg)(?-u:\b)"
);

// badai
lazy_re!(
    BADAI_MAIN,
    r"(?-u:\b)(?:puting\s+beliung|siklon(?:\s+tropis)?|bibit\s+siklon|badai(?:\s+petir)?|angin\s+kencang|angin\s+ribut|gelombang\s+pasang|hujan\s+ekstrem|cuaca\s+ekstrem)(?-u:\b)"
);
lazy_re!(BADAI_PUTING, r"(?-u:\b)puting\s+beliung(?-u:\b)");
lazy_re!(
    BADAI_PUTING_STRONG,
    r"(?-u:\b)(?:terjang|menerjang|hantam|menghantam|rusak|merusak|atap|pohon|tumbang|beterbangan|rumah|warga|porak|di\s+[a-z]|peringatan|waspada|bmkg)(?-u:\b)"
);
lazy_re!(BADAI_SIKLON, r"(?-u:\b)siklon(?-u:\b)");
lazy_re!(
    BADAI_SIKLON_STRONG,
    r"(?-u:\b)(?:peringatan|waspada|siaga|bmkg|bnpb|bpbd|terjadi|memicu|gelombang|badai|hujan|di\s+[a-z])(?-u:\b)"
);
lazy_re!(
    BADAI_STRONG,
    r"(?-u:\b)(?:terjang|menerjang|hantam|menghantam|rusak|merusak|roboh|tumbang|beterbangan|peringatan(?:\s+dini)?|keluarkan\s+peringatan|waspada|siaga|bmkg|bnpb|bpbd|kecepatan|knot|km/h|gelombang\s+pasang|hujan\s+ekstrem)(?-u:\b)"
);

// kecelakaan
lazy_re!(LED_MAIN, r"(?-u:\b)(?:ledakan|meledak|dentuman)(?-u:\b)");
lazy_re!(
    LED_PHYS,
    r"(?-u:\b)(?:tabung\s+gas|elpiji|lpg|kompor|trafo|gardu|tangki|kilang|gudang|pabrik|ruko|rumah|gedung|bom|petasan|granat|baterai|mobil|motor|truk|bus|kapal|kaca\s+pecah|asap|api|korban|luka|tewas|meninggal|evakuasi|polisi|damkar|suara\s+ledakan|dengar\s+ledakan)(?-u:\b)"
);
lazy_re!(
    ACC_MAIN,
    r"(?-u:\b)(?:kecelakaan|tabrakan|tabrak\s+lari|ditabrak|menabrak|bertabrakan|bus\s+terguling|truk\s+terguling|kereta\s+anjlok|pesawat\s+jatuh|kapal\s+(?:tenggelam|karam))(?-u:\b)"
);
lazy_re!(
    ACC_MARITIME,
    r"(?-u:\b)(?:kapal|perahu|feri|ferry|speedboat|tongkang)(?-u:\b)[^.!?\n]{0,25}(?-u:\b)(?:tenggelam|karam|terbalik)(?-u:\b)"
);
lazy_re!(
    ACC_STRONG,
    r"(?-u:\b)(?:terjadi|tewas|meninggal|luka|korban|dievakuasi|terguling|anjlok|karam|tenggelam|beruntun|parah|tabrak\s+lari|tol|jalan|persimpangan|di\s+[a-z])(?-u:\b)"
);

// insiden
lazy_re!(
    INS_MAIN,
    r"(?-u:\b)(?:ambruk|runtuh|roboh|pohon\s+tumbang|tenggelam|hanyut|tergelincir|jembatan\s+putus|atap\s+jatuh|bangunan\s+roboh)(?-u:\b)"
);
lazy_re!(
    INS_PHYS,
    r"(?-u:\b)(?:jembatan|gedung|bangunan|atap|plafon|rumah|dinding|konstruksi|tembok|jalan|tebing|tower|tiang|kapal|perahu|sampan|tongkang|speedboat|dermaga|pohon|orang|bocah|anak|korban|warga|penumpang|kendaraan|mobil|motor|truk)(?-u:\b)"
);
lazy_re!(INS_TREE, r"(?-u:\b)pohon\s+tumbang(?-u:\b)");
lazy_re!(INS_SANK, r"(?-u:\b)(?:tenggelam|hanyut|karam)(?-u:\b)");
lazy_re!(
    INS_SEA,
    r"(?-u:\b)(?:kapal|laut|selat|ombak|nelayan|feri|perahu|sungai|perairan|dermaga)(?-u:\b)"
);

// demo
lazy_re!(
    DEMO_MAIN,
    r"(?-u:\b)(?:unjuk\s+rasa|aksi\s+massa|demonstrasi|demonstran|pengunjuk\s+rasa|(?:ber|men|di)?demo|protes|mogok|kerusuhan|bentrok|pemogokan|long\s+march|women'?s\s+march|hari\s+buruh|may\s+day|[a-z]+\s+memanggil|kampus\s+dikepung|logistik\s+dihalangi|kawan\s+diculik|(?:ga|gak|tidak|kurang)\s+kondusif|menyuarakan\s+keadilan|memperjuangkan\s+kebenaran)(?-u:\b)|(?-u:\b)(?:gelar|menggelar|melakukan)\s+aksi(?-u:\b)|(?-u:\b)aksi\s+(?:tolak|menolak|tuntut|menuntut|mahasiswa|buruh|damai|bela|massa|bem)(?-u:\b)|(?-u:\b)mahasiswa\s+(?:turun|bergerak|melawan|dikepung|diculik)(?-u:\b)|(?-u:\b)aksi\s+kamisan(?-u:\b)|(?-u:\b)kamisan(?-u:\b)|(?-u:\b)revolusi(?:\s+(?:rakyat|sosial|total))?(?-u:\b)"
);
lazy_re!(
    DEMO_EXCL,
    r"(?i)(?-u:\b)(?:demo\s+(?:lagu|musik|app|aplikasi|produk|game|masak|cooking|vokal|versi|website)|free\s+demo|live\s+demo|cooking\s+demo|happy\s+labor\s+day|revolusi\s+(?:skincare|kecantikan|wajah|kulit|glowing|jerawat|industri(?:\s+4\.0)?|mental|teknologi|digital|kebiasaan|gaya\s+hidup|tubuh|diet|perawatan|estetika)|bentrok\s+(?:sama\s+|dengan\s+)?(?:jadwal|agenda|kuliah|matkul|mata\s+kuliah|jam\s+kuliah|kelas|meeting|rapat|acara|praktikum|waktu|ujian|kalender|zoom)|tabrakan\s+(?:jadwal|agenda|kuliah|matkul))(?-u:\b)"
);
lazy_re!(
    DEMO_STRONG,
    r"(?-u:\b)(?:berlangsung|turun\s+ke\s+jalan|turun\s+ke\s+aspal|berkumpul|gelar|menggelar|melakukan\s+aksi|aksi\s+(?:tolak|menolak|tuntut|menuntut)|ricuh|bentrok|tuntut|menuntut|menolak|tolak|orasi|aksi|siapkan\s+pengamanan|menyiapkan\s+pengamanan|siagakan|disiagakan|kerahkan|dikerahkan|antisipasi|kawal\s+demo|amankan\s+demo|pengamanan|tiba\s+di\s+depan|gerbang|blokade|gas\s+air\s+mata|water\s+cannon|barikade|mahasiswa|buruh|ojol|dpr|dprd|istana|patung\s+kuda|polisi|aparat|kampus\s+dikepung|kawan\s+diculik|tidak\s+kondusif|ga\s+kondusif|memanggil|aksi\s+kamisan|kamisan|revolusi)(?-u:\b)"
);

// penembakan, pencurian, penangkapan, kriminal
lazy_re!(
    SHOT_MAIN,
    r"(?-u:\b)(?:penembakan|ditembak|tertembak|baku\s+tembak|terkena\s+tembakan|berondong\s+tembakan)(?-u:\b)"
);
lazy_re!(
    SHOT_STRONG,
    r"(?-u:\b)(?:polisi|polres|polda|polsek|tni|aparat|senjata|senpi|peluru|pelaku|korban|tewas|meninggal|luka|kritis|darah|otk|kkb|begal|rampok|timah\s+panas|tkp|baku\s+tembak|penembakan)(?-u:\b)"
);
lazy_re!(
    THEFT_MAIN,
    r"(?-u:\b)(?:pencurian|curanmor|begal|perampokan|jambret|dirampok|kemalingan|maling|pencuri|dicuri|mencuri)(?-u:\b)"
);
lazy_re!(
    THEFT_STRONG,
    r"(?-u:\b)(?:motor|mobil|hp|ponsel|uang|duit|emas|perhiasan|laptop|sepeda|toko|rumah|warung|brankas|korban|pelaku|tertangkap|ditangkap|diamankan|dihakimi|terekam|cctv|polres|polsek|polda|tkp|beraksi|dirampok|kemalingan|curanmor|dibegal)(?-u:\b)"
);
lazy_re!(
    ARREST_MAIN,
    r"(?-u:\b)(?:penangkapan|ditangkap|diamankan|ott|operasi\s+tangkap\s+tangan|razia|penahanan|tersangka)(?-u:\b)"
);
lazy_re!(
    ARREST_CTX,
    r"(?-u:\b)(?:polisi|polres|polda|polsek|kpk|kejaksaan|bnn|densus|aparat|pelaku|tersangka|buron|maling|begal|rampok|pembunuh|bandar|narkoba|koruptor|suap|penipu|preman|senpi)(?-u:\b)"
);
lazy_re!(
    ARREST_STRONG,
    r"(?-u:\b)(?:ditangkap|diamankan|penangkapan|ott|ditahan)(?-u:\b)"
);
lazy_re!(
    CRIME_MAIN,
    r"(?-u:\b)(?:pembunuhan|dibunuh|pengeroyokan|dikeroyok|perampokan|dirampok|penculikan|diculik|perkosaan|rudapaksa|terorisme|bom\s+bunuh\s+diri|narkoba|kdrt)(?-u:\b)"
);
lazy_re!(CRIME_HOMICIDE, r"(?-u:\b)(?:pembunuhan|dibunuh)(?-u:\b)");
lazy_re!(CRIME_NARC, r"(?-u:\b)narkoba(?-u:\b)");
lazy_re!(
    CRIME_STRONG,
    r"(?-u:\b)(?:terjadi|tewas|meninggal|luka|korban|pelaku|tersangka|ditangkap|diamankan|polisi|polres|polda|ditemukan|warga|di\s+[a-z])(?-u:\b)"
);

// kesehatan
lazy_re!(
    HEALTH_GAME_1,
    r"(?i)(?-u:\b)(?:game|gaming|crashed|tome|grabs?|pick\s*ups?|crawled|dcing|killer|survivor|hooked|gen|pallets?|steam|lobby|fps)(?-u:\b)"
);
lazy_re!(
    HEALTH_GAME_2,
    r"(?i)(?-u:\b)(?:dead\s+by\s+daylight|main\s+dbd|play\s+dbd|jogar\s+dbd)(?-u:\b)"
);
lazy_re!(HEALTH_DBD, r"(?i)(?-u:\b)dbd(?-u:\b)");
lazy_re!(
    HEALTH_GAME_3,
    r"(?i)(?-u:\b)(?:game|gaming|crashed|tome|grabs?|pick\s*ups?|crawled|dcing|perks?|killer|survivor|hooked|gen|pallets?|lobby|steam|fps)(?-u:\b)"
);
lazy_re!(
    HEALTH_MAIN,
    r"(?-u:\b)(?:keracunan|intoksikasi|wabah|klb|kejadian\s+luar\s+biasa|dbd|demam\s+berdarah|campak|difteri|kolera|rabies|polio|mpox|leptospirosis|tifus|malaria|serangan\s+jantung|stroke|gagal\s+ginjal|miras\s+oplosan|oplosan\s+maut|muntah|mual|diare)(?-u:\b)"
);
lazy_re!(
    HEALTH_STRONG_1,
    r"(?-u:\b)(?:kasus|pasien|terjangkit|keracunan\s+(?:massal|makanan|mbg|pangan|nasi|katering)|korban|meninggal|tewas|dirawat|dilarikan\s+ke|puskesmas|rumah\s+sakit|rsud|rsup|klb|wabah|intoksikasi|oplosan)(?-u:\b)"
);
lazy_re!(HEALTH_MUAL, r"(?-u:\b)(?:mual|muntah)(?-u:\b)");
lazy_re!(
    HEALTH_MUAL_CTX,
    r"(?-u:\b)(?:massal|keracunan|makanan|mbg|santri|siswa|murid|warga|rumah\s+sakit|puskesmas|dilarikan)(?-u:\b)"
);

// listrik padam
lazy_re!(
    POWER_MAIN,
    r"(?-u:\b)(?:listrik\s+padam|padam\s+listrik|pemadaman\s+listrik|mati\s+listrik|mati\s+lampu|byarpet|blackout)(?-u:\b)"
);
lazy_re!(POWER_BLACKOUT, r"(?-u:\b)blackout(?-u:\b)");
lazy_re!(
    POWER_BLACKOUT_CTX,
    r"(?-u:\b)(?:listrik|pln|gardu|trafo|area|wilayah|padam|mati\s+lampu)(?-u:\b)"
);
lazy_re!(
    POWER_STRONG,
    r"(?-u:\b)(?:pemadaman|padam|mati\s+listrik|mati\s+lampu|listrik\s+padam|pln|gardu|trafo|area|wilayah|kecamatan|kabupaten|kota|sebagian|jam|sejak)(?-u:\b)"
);

// serangan siber
lazy_re!(
    CYBER_ACT,
    r"(?-u:\b)(?:ransomware|malware|diserang|diretas|dibobol|dibajak|terenkripsi|dienkripsi|dicuri|bocor|kebocoran\s+data|data\s+breach|ddos|defacement|deface|phishing|dieksploitasi|lumpuh|takedown|sensor\s+konten|blokir\s+konten|takedown\s+konten)(?-u:\b)"
);
lazy_re!(
    CYBER_TARGET,
    r"(?-u:\b)(?:server|sistem|jaringan|database|situs|website|layanan|akun|data|pdn|bssn|bank|rumah\s+sakit|pemerintah|kementerian|aplikasi|komdigi|kominfo|konten(?:\s+kritis)?)(?-u:\b)"
);
lazy_re!(
    CYBER_RANSOM,
    r"(?-u:\b)(?:ransomware|lockbit|blackcat)(?-u:\b)"
);
lazy_re!(CYBER_DDOS, r"(?-u:\b)ddos(?-u:\b)");
lazy_re!(CYBER_BREACH, r"(?-u:\b)(?:bocor|kebocoran|breach)(?-u:\b)");

// konflik, politik, phk, ihsg
lazy_re!(
    KONFLIK_MAIN,
    r"(?-u:\b)(?:baku\s+tembak|kontak\s+senjata|serangan\s+udara|serangan\s+rudal|serangan\s+bom|invasi|kudeta|gencatan\s+senjata|darurat\s+militer|pertempuran\s+(?:sengit|bersenjata)|operasi\s+militer|penyanderaan|kkb\s+papua|separatis\s+menyerang)(?-u:\b)"
);
lazy_re!(
    POL_1,
    r"(?-u:\b)(?:resmi\s+dilantik|dilantik|mengundurkan\s+diri|resignation|dipecat|diberhentikan|reshuffle\s+kabinet|divonis|dimakzulkan|impeachment)(?-u:\b)"
);
lazy_re!(
    POL_2,
    r"(?-u:\b)anggaran\s+(?:komdigi|kominfo|kementerian|menteri)[^.\n]{0,30}(?-u:\b)triliun(?-u:\b)|(?-u:\b)takedown\s+konten(?:\s+kritis)?(?-u:\b)|(?-u:\b)takedown(?-u:\b)[^.\n]{0,30}(?-u:\b)(?:komdigi|kominfo)(?-u:\b)|(?-u:\b)(?:komdigi|kominfo)(?-u:\b)[^.\n]{0,30}(?-u:\b)takedown(?-u:\b)|(?-u:\b)resignation\s+of\s+(?:indonesia['’]?s\s+)?(?:president|vice[- ]?president|minister)(?-u:\b)|(?-u:\b)indonesian\s+politics(?-u:\b)"
);
lazy_re!(
    POL_OPINION,
    r"(?-u:\b)(?:usul|wacana|menurut|komentar|opini|mengkhianati|katanya|andai|jika)(?-u:\b)"
);
lazy_re!(
    POL_OVERRIDE,
    r"(?-u:\b)anggaran|komdigi|takedown|resignation(?-u:\b)"
);
lazy_re!(
    PHK_MAIN,
    r"(?-u:\b)(?:phk|pemutusan\s+hubungan\s+kerja|dirumahkan|pabrik\s+tutup)(?-u:\b)"
);
lazy_re!(
    PHK_STRONG,
    r"(?-u:\b)(?:melakukan|lakukan|terkena|mengalami|memutus|merumahkan|pabrik\s+tutup|tutup\s+dan\s+lakukan\s+phk|(?:puluhan|ratusan|ribuan|[0-9]+)\s+(?:buruh|karyawan|pekerja))(?-u:\b)"
);
lazy_re!(
    IHSG_MAIN,
    r"(?-u:\b)(?:trading\s+halt|bursa\s+efek\s+indonesia|(?-u:\b)ihsg(?-u:\b))(?-u:\b)"
);
lazy_re!(
    IHSG_STRONG,
    r"(?-u:\b)(?:trading\s+halt|suspensi\s+perdagangan|dihentikan\s+sementara)(?-u:\b)"
);

pub const FRAME_TOPICS: &[&str] = &[
    "banjir",
    "kebakaran",
    "karhutla",
    "gempa",
    "badai",
    "kecelakaan",
    "insiden",
    "demo",
    "penembakan",
    "pencurian",
    "penangkapan",
    "kriminal",
    "kesehatan",
    "listrik padam",
    "serangan siber",
    "konflik",
    "politik",
    "phk",
    "ihsg",
];

fn hit(
    topic: &'static str,
    category: &'static str,
    action: &'static str,
    strong: bool,
    event_type: &'static str,
) -> Option<FrameHit> {
    Some(FrameHit {
        topic,
        category,
        action,
        strong,
        event_type,
        eyewitness_cue: false,
        minor_waterlogging: false,
    })
}

pub fn detect_frame(topic: &'static str, c: &EvalClause<'_>) -> Option<FrameHit> {
    let t = c.norm_geo;
    let raw_or_t = if c.raw.is_empty() { t } else { c.raw };
    match topic {
        "banjir" => {
            if BANJIR_1.is_match(t) {
                return hit("banjir", "flood", "flooded", true, "flood");
            }
            if BANJIR_LONGSOR.is_match(t) && !BANJIR_LONGSOR_META.is_match(t) {
                let strong = BANJIR_LONGSOR_ACT.is_match(t);
                return hit("banjir", "flood", "landslide", strong, "landslide");
            }
            if BANJIR_AIR_1.is_match(t) || BANJIR_AIR_2.is_match(t) {
                if BANJIR_AIR_EXCL.is_match(t) {
                    return None;
                }
                return Some(FrameHit {
                    topic: "banjir",
                    category: "flood",
                    action: "inundated",
                    strong: true,
                    event_type: "flood",
                    eyewitness_cue: true,
                    minor_waterlogging: false,
                });
            }
            if BANJIR_GEN.is_match(t) {
                let strong = BANJIR_STRONG_1.is_match(raw_or_t) || BANJIR_STRONG_2.is_match(t);
                return hit("banjir", "flood", "inundated", strong, "flood");
            }
            if BANJIR_GENANGAN.is_match(t) {
                return Some(FrameHit {
                    topic: "banjir",
                    category: "flood",
                    action: "waterlogging",
                    strong: false,
                    event_type: "waterlogging",
                    eyewitness_cue: false,
                    minor_waterlogging: true,
                });
            }
            None
        }
        "kebakaran" => {
            if FIRE_EXCL_1.is_match(t) && !FIRE_EXCL_2.is_match(t) {
                return None;
            }
            if !FIRE_MAIN.is_match(t) || FIRE_FOREST.is_match(t) {
                return None;
            }
            let strong = FIRE_ACT.is_match(t) && FIRE_STRONG.is_match(t);
            hit("kebakaran", "fire", "burning", strong, "fire")
        }
        "karhutla" => {
            if !KARHUTLA_MAIN.is_match(t) || KARHUTLA_WIFI.is_match(t) {
                return None;
            }
            if KARHUTLA_HOTSPOT.is_match(t) && !KARHUTLA_HOTSPOT_CTX.is_match(t) {
                return None;
            }
            hit(
                "karhutla",
                "fire",
                "wildfire_burning",
                KARHUTLA_STRONG.is_match(t),
                "wildfire",
            )
        }
        "gempa" => {
            if TSUNAMI_RE.is_match(t) {
                if TSUNAMI_EXCL.is_match(t) {
                    return None;
                }
                return hit(
                    "gempa",
                    "disaster",
                    "tsunami_alert",
                    TSUNAMI_STRONG.is_match(t),
                    "tsunami",
                );
            }
            if ERUPSI_1.is_match(t) || ERUPSI_2.is_match(t) {
                return hit(
                    "gempa",
                    "disaster",
                    "erupted",
                    ERUPSI_STRONG.is_match(t),
                    "eruption",
                );
            }
            if GEMPA_1.is_match(t) {
                if !GEMPA_NOUN.is_match(t) {
                    return None;
                }
                return hit(
                    "gempa",
                    "disaster",
                    "shook",
                    GEMPA_STRONG.is_match(t),
                    "earthquake",
                );
            }
            None
        }
        "badai" => {
            if !BADAI_MAIN.is_match(t) {
                return None;
            }
            if BADAI_PUTING.is_match(t) {
                return hit(
                    "badai",
                    "disaster",
                    "struck",
                    BADAI_PUTING_STRONG.is_match(t),
                    "tornado",
                );
            }
            if BADAI_SIKLON.is_match(t) {
                return hit(
                    "badai",
                    "disaster",
                    "cyclone",
                    BADAI_SIKLON_STRONG.is_match(t),
                    "cyclone",
                );
            }
            hit(
                "badai",
                "disaster",
                "severe_weather",
                BADAI_STRONG.is_match(t),
                "storm",
            )
        }
        "kecelakaan" => {
            if LED_MAIN.is_match(t) {
                if !LED_PHYS.is_match(t) {
                    return None;
                }
                return hit("kecelakaan", "insiden", "exploded", true, "explosion");
            }
            if ACC_MAIN.is_match(t) {
                let is_maritime = ACC_MARITIME.is_match(t);
                let strong = ACC_STRONG.is_match(t);
                return hit(
                    "kecelakaan",
                    "insiden",
                    "crashed",
                    strong,
                    if is_maritime {
                        "maritime_accident"
                    } else {
                        "accident"
                    },
                );
            }
            None
        }
        "insiden" => {
            if !INS_MAIN.is_match(t) || !INS_PHYS.is_match(t) {
                return None;
            }
            if INS_TREE.is_match(t) {
                return hit("insiden", "insiden", "fallen", true, "tree_fall");
            }
            if INS_SANK.is_match(t) {
                let sea = INS_SEA.is_match(t);
                return hit(
                    "insiden",
                    "insiden",
                    "sank",
                    true,
                    if sea { "maritime_accident" } else { "accident" },
                );
            }
            hit("insiden", "insiden", "collapsed", true, "collapse")
        }
        "demo" => {
            if !DEMO_MAIN.is_match(t) || DEMO_EXCL.is_match(t) {
                return None;
            }
            hit(
                "demo",
                "demo",
                "protesting",
                DEMO_STRONG.is_match(t),
                "demo",
            )
        }
        "penembakan" => {
            if !SHOT_MAIN.is_match(t) || !SHOT_STRONG.is_match(t) {
                return None;
            }
            hit("penembakan", "kriminal", "shot", true, "violent_crime")
        }
        "pencurian" => {
            if !THEFT_MAIN.is_match(t) {
                return None;
            }
            hit(
                "pencurian",
                "kriminal",
                "stolen",
                THEFT_STRONG.is_match(t),
                "violent_crime",
            )
        }
        "penangkapan" => {
            if !ARREST_MAIN.is_match(t) || !ARREST_CTX.is_match(t) {
                return None;
            }
            hit(
                "penangkapan",
                "kriminal",
                "arrested",
                ARREST_STRONG.is_match(t),
                "violent_crime",
            )
        }
        "kriminal" => {
            if !CRIME_MAIN.is_match(t) {
                return None;
            }
            let is_homicide = CRIME_HOMICIDE.is_match(t);
            let is_narc = CRIME_NARC.is_match(t);
            let action = if is_homicide { "homicide" } else { "crime" };
            let ev_type = if is_narc {
                "narcotics"
            } else if is_homicide {
                "homicide"
            } else {
                "violent_crime"
            };
            hit(
                "kriminal",
                "kriminal",
                action,
                CRIME_STRONG.is_match(t),
                ev_type,
            )
        }
        "kesehatan" => {
            if HEALTH_GAME_1.is_match(raw_or_t) || HEALTH_GAME_2.is_match(raw_or_t) {
                return None;
            }
            if HEALTH_DBD.is_match(raw_or_t) && HEALTH_GAME_3.is_match(c.doc_raw) {
                return None;
            }
            if !HEALTH_MAIN.is_match(t) {
                return None;
            }
            let strong = HEALTH_STRONG_1.is_match(t)
                || (HEALTH_MUAL.is_match(t) && HEALTH_MUAL_CTX.is_match(t));
            hit(
                "kesehatan",
                "kesehatan",
                "health_case",
                strong,
                "health_incident",
            )
        }
        "listrik padam" => {
            if !POWER_MAIN.is_match(t) {
                return None;
            }
            if POWER_BLACKOUT.is_match(t) && !POWER_BLACKOUT_CTX.is_match(t) {
                return None;
            }
            hit(
                "listrik padam",
                "ekonomi",
                "power_outage",
                POWER_STRONG.is_match(t),
                "listrik padam",
            )
        }
        "serangan siber" => {
            if !CYBER_ACT.is_match(t) {
                return None;
            }
            let strong = CYBER_TARGET.is_match(t);
            let ev_type = if CYBER_RANSOM.is_match(t) {
                "ransomware"
            } else if CYBER_DDOS.is_match(t) {
                "ddos"
            } else if CYBER_BREACH.is_match(t) {
                "data_breach"
            } else {
                "intrusion"
            };
            hit("serangan siber", "siber", "cyber_attack", strong, ev_type)
        }
        "konflik" => {
            if CEREMONY_RE.is_match(t) || !KONFLIK_MAIN.is_match(t) {
                return None;
            }
            hit("konflik", "politik", "armed_conflict", true, "konflik")
        }
        "politik" => {
            if !(POL_1.is_match(t) || POL_2.is_match(t)) {
                return None;
            }
            if POL_OPINION.is_match(t) && !POL_OVERRIDE.is_match(t) {
                return None;
            }
            hit("politik", "politik", "institutional_event", true, "politik")
        }
        "phk" => {
            if !PHK_MAIN.is_match(t) {
                return None;
            }
            hit("phk", "ekonomi", "layoffs", PHK_STRONG.is_match(t), "phk")
        }
        "ihsg" => {
            if !IHSG_MAIN.is_match(t) || !IHSG_STRONG.is_match(t) {
                return None;
            }
            hit("ihsg", "ekonomi", "market_halt", true, "ihsg")
        }
        _ => None,
    }
}

// extractImpact
lazy_re!(
    IMP_FAT_1,
    r"(?i)(?-u:\b)([0-9]+)\s+(?:orang\s+|warga\s+|korban\s+|jiwa\s+|penumpang\s+|siswa\s+)?(?:tewas|meninggal(?:\s+dunia)?|wafat|gugur)(?-u:\b)"
);
lazy_re!(
    IMP_FAT_2,
    r"(?i)(?-u:\b)(?:tewas|meninggal(?:\s+dunia)?)\s+(?:sebanyak\s+)?([0-9]+)\s+(?:orang|warga|jiwa)(?-u:\b)"
);
lazy_re!(
    IMP_FAT_0,
    r"(?i)(?-u:\b)(?:tidak|belum|nihil|nol)\s+ada\s+(?:laporan\s+)?korban\s+(?:jiwa|meninggal|tewas)(?-u:\b)"
);
lazy_re!(
    IMP_FAT_W2,
    r"(?i)(?-u:\b)(?:dua|2)\s+(?:orang\s+|warga\s+)?(?:tewas|meninggal)(?-u:\b)"
);
lazy_re!(
    IMP_FAT_W1,
    r"(?i)(?-u:\b)(?:satu|seorang)\s+(?:orang\s+|warga\s+|pria|wanita|siswa|anak|korban)?\s*(?:tewas|meninggal)(?-u:\b)"
);
lazy_re!(
    IMP_FAT_W3,
    r"(?i)(?-u:\b)(?:tiga)\s+(?:orang\s+|warga\s+)?(?:tewas|meninggal)(?-u:\b)"
);
lazy_re!(
    IMP_INJ,
    r"(?i)(?-u:\b)([0-9]+)\s+(?:orang\s+|warga\s+|korban\s+|petugas\s+)?(?:luka(?:-luka|\s+berat|\s+ringan)?|terluka|cedera)(?-u:\b)"
);
lazy_re!(
    IMP_DISP,
    r"(?i)(?-u:\b)([0-9]+|puluhan|ratusan|ribuan)\s+(?:orang\s+|warga\s+|kk\s+|keluarga\s+)?(?:mengungsi|dievakuasi|terdampak)(?-u:\b)"
);
lazy_re!(
    IMP_PROP,
    r"(?i)(?-u:\b)([0-9]+|puluhan|ratusan|ribuan|sejumlah|banyak)\s+(?:rumah|ruko|kios|warung|bangunan|gedung|atap\s+rumah|mobil|kendaraan|motor|kapal)\s+(?:terendam|rusak|hangus|ludes|terbakar|hancur|ambruk|tertimbun|tenggelam)(?-u:\b)"
);

pub fn extract_impact_str(s: &str) -> Value {
    let mut fatalities = Value::Null;
    if let Some(c) = IMP_FAT_1.captures(s).or_else(|| IMP_FAT_2.captures(s)) {
        if let Some(m) = c.get(1)
            && let Ok(n) = m.as_str().parse::<f64>()
        {
            fatalities = crate::js::number(n);
        }
    } else if IMP_FAT_0.is_match(s) {
        fatalities = json!(0);
    } else if IMP_FAT_W2.is_match(s) {
        fatalities = json!(2);
    } else if IMP_FAT_W1.is_match(s) {
        fatalities = json!(1);
    } else if IMP_FAT_W3.is_match(s) {
        fatalities = json!(3);
    }

    let injuries = IMP_INJ
        .captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok())
        .map(crate::js::number)
        .unwrap_or(Value::Null);

    let displacement = IMP_DISP
        .captures(s)
        .and_then(|c| c.get(1))
        .map(|m| {
            let txt = m.as_str();
            if txt.bytes().all(|b| b.is_ascii_digit()) {
                txt.parse::<f64>()
                    .map(crate::js::number)
                    .unwrap_or(Value::Null)
            } else {
                Value::String(txt.to_lowercase())
            }
        })
        .unwrap_or(Value::Null);

    let property_damage = IMP_PROP
        .find(s)
        .map(|m| Value::String(m.as_str().to_lowercase()))
        .unwrap_or(Value::Null);

    json!({
        "fatalities": fatalities,
        "injuries": injuries,
        "displacement": displacement,
        "propertyDamage": property_damage,
    })
}

// extractAffectedEntity
lazy_re!(
    ENT_RES,
    r"(?i)(?-u:\b)(?:rumah\s+warga|permukiman|pemukiman|perumahan|rumah\s+saya|kosan|kontrakan|rumah|warga)(?-u:\b)"
);
lazy_re!(
    ENT_COM,
    r"(?i)(?-u:\b)(?:ruko|toko|kios|warung|pasar|mall|pusat\s+perbelanjaan|hotel)(?-u:\b)"
);
lazy_re!(
    ENT_IND,
    r"(?i)(?-u:\b)(?:pabrik|gudang|kilang|industri|kawasan\s+industri)(?-u:\b)"
);
lazy_re!(
    ENT_SCH,
    r"(?i)(?-u:\b)(?:sekolah|sdn|smp|sma|smk|madrasah|pesantren|kampus|universitas|siswa|murid|mahasiswa|santri)(?-u:\b)"
);
lazy_re!(
    ENT_HOS,
    r"(?i)(?-u:\b)(?:rumah\s+sakit|rsud|rsup|puskesmas|klinik|pasien)(?-u:\b)"
);
lazy_re!(
    ENT_ROAD,
    r"(?i)(?-u:\b)(?:jembatan|jalan\s+tol|jalan\s+raya|jalan|tol|ruas\s+jalan|kendaraan|mobil|truk|bus|motor|kereta|krl|mrt|lrt)(?-u:\b)"
);
lazy_re!(
    ENT_VES,
    r"(?i)(?-u:\b)(?:kapal|perahu|feri|ferry|tongkang|speedboat|dermaga|pelabuhan)(?-u:\b)"
);
lazy_re!(
    ENT_PWR,
    r"(?i)(?-u:\b)(?:listrik|gardu|trafo|pln|jaringan\s+listrik)(?-u:\b)"
);
lazy_re!(
    ENT_DIG,
    r"(?i)(?-u:\b)(?:server|database|situs|website|pdn|peretasan|diretas|diserang|ransomware|kebocoran\s+data)(?-u:\b)"
);
lazy_re!(
    ENT_NAT,
    r"(?i)(?-u:\b)(?:hutan|lahan|gambut|gunung|tebing)(?-u:\b)"
);

pub fn extract_affected_entity_str(s: &str) -> Option<&'static str> {
    if ENT_RES.is_match(s) {
        return Some("residential_area");
    }
    if ENT_COM.is_match(s) {
        return Some("commercial_building");
    }
    if ENT_IND.is_match(s) {
        return Some("industrial_facility");
    }
    if ENT_SCH.is_match(s) {
        return Some("school");
    }
    if ENT_HOS.is_match(s) {
        return Some("hospital");
    }
    if ENT_ROAD.is_match(s) {
        return Some("road_infrastructure");
    }
    if ENT_VES.is_match(s) {
        return Some("vessel");
    }
    if ENT_PWR.is_match(s) {
        return Some("power_grid");
    }
    if ENT_DIG.is_match(s) {
        return Some("digital_system");
    }
    if ENT_NAT.is_match(s) {
        return Some("natural_environment");
    }
    None
}

// extractEventTime
lazy_re!(
    TIME_HIST,
    r"(?i)(?-u:\b)(?:tahun|pada|sejak|era)\s+((?:19|20)[0-9]{2})(?-u:\b)|(?-u:\b)(?:dulu|jaman\s+dulu|tahun\s+lalu|bulan\s+lalu|pekan\s+lalu|kemarin)(?-u:\b)"
);
lazy_re!(
    TIME_WARN_1,
    r"(?i)(?-u:\b)(?:peringatan\s+(?:dini|cuaca|tsunami|banjir)|berpotensi|potensi|waspada|siaga|awas|hingga\s+(?:sore|malam|besok))(?-u:\b)"
);
lazy_re!(
    TIME_WARN_2,
    r"(?i)(?-u:\b)hingga\s+(?:sore|malam|besok|pukul\s+[0-9]{1,2}[:.][0-9]{2}\s*wib)(?-u:\b)"
);
lazy_re!(
    TIME_EXACT,
    r"(?i)(?-u:\b)(?:pukul\s+[0-9]{1,2}[:.][0-9]{2}(?:[:.][0-9]{2})?\s*(?:wib|wita|wit)?|[0-9]{1,2}[:.][0-9]{2}(?:[:.][0-9]{2})?\s*(?:wib|wita|wit)|barusan|baru\s+saja|saat\s+ini|hari\s+ini|pagi\s+ini|siang\s+ini|sore\s+ini|malam\s+ini|tadi\s+(?:pagi|siang|sore|malam)|semalam)(?-u:\b)"
);
lazy_re!(
    TIME_CURR,
    r"(?i)(?-u:\b)(?:barusan|baru\s+saja|saat\s+ini)(?-u:\b)"
);

pub fn extract_event_time_str(s: &str, is_historical: bool) -> Value {
    if is_historical {
        let val = TIME_HIST
            .find(s)
            .map(|m| Value::String(m.as_str().to_lowercase()))
            .unwrap_or(Value::Null);
        return json!({ "value": val, "type": "historical" });
    }
    if TIME_WARN_1.is_match(s) {
        let val = TIME_WARN_2
            .find(s)
            .map(|m| Value::String(m.as_str().to_lowercase()))
            .unwrap_or(Value::Null);
        return json!({ "value": val, "type": "warning_forecast" });
    }
    if let Some(m) = TIME_EXACT.find(s) {
        let matched = m.as_str();
        let ty = if TIME_CURR.is_match(matched) {
            "current"
        } else {
            "recent"
        };
        return json!({ "value": matched.to_lowercase(), "type": ty });
    }
    json!({ "value": null, "type": "unknown" })
}
