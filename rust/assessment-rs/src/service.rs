#![forbid(unsafe_code)]

use crate::port::analysis;
use crate::port::tables::{
    CEREMONY_RE, GEO_AUTHOR_CURRENT, GEO_AUTHOR_HOMETOWN, GEO_EVENT_LOCATION, GEO_HISTORICAL,
    GEO_MENTIONED_IN_PASSING, GEO_READER_LOCATION, LOCAL_MACRO_RE, LOCAL_ORG_RE,
    metaphor_blocks_topic_str, re,
};
use crate::port::{current_year_utc, is_historical_str};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::sync::LazyLock;

pub const VERSION: &str = "2026-11-23.1";

#[derive(Debug, Clone)]
pub struct TopicSpec {
    pub id: String,
    pub pos: Regex,
    pub neg: Option<Regex>,
    pub ctx: Option<Regex>,
    pub strong: Option<Regex>,
    pub adhoc: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AssessOptions {
    pub year: Option<f64>,
    pub listing: bool,
    pub spam: Option<Regex>,
    pub noise: Option<Regex>,
}

pub fn category_of(topic: &str) -> Option<&'static str> {
    Some(match topic {
        "coretax" | "pajak" => "pajak",
        "demo" => "demo",
        "kebakaran" | "karhutla" => "fire",
        "banjir" => "flood",
        "gempa" | "badai" => "disaster",
        "kecelakaan" | "insiden" | "transport" | "pendidikan" => "insiden",
        "penembakan" | "pencurian" | "penangkapan" | "kriminal" | "hukum" => "kriminal",
        "phk" | "rupiah" | "ihsg" | "listrik padam" | "bbm" | "pangan" | "bansos" => "ekonomi",
        "wabah" | "kesehatan" | "mbg" => "kesehatan",
        "pemilu" | "politik" | "konflik" => "politik",
        "ransomware" | "serangan siber" | "akun dibajak" => "siber",
        _ => return None,
    })
}

pub fn is_feed_only(topic: &str) -> bool {
    matches!(
        topic,
        "mbg" | "bbm" | "pangan" | "bansos" | "hukum" | "transport" | "pendidikan"
    )
}

macro_rules! lazy_re {
    ($name:ident, $pat:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| re($pat));
    };
}

lazy_re!(
    CYBER_TARGET,
    r"(?i)(?-u:\b)(server|sistem|jaringan|komputer|perangkat|database|situs|website|layanan|akun|data|berkas|file|rumah sakit|hospital|network|device|files|computers?)(?-u:\b)"
);
lazy_re!(
    CYBER_ACTION,
    r"(?i)(?-u:\b)(diserang|diretas|dibobol|terinfeksi|terenkripsi|dienkripsi|dicuri|bocor|lumpuh|terganggu|dikunci|dijual|menyerang|menginfeksi|serangan|attack(?:ed)?|infect(?:ed)?|encrypt(?:ed)?|breach(?:ed)?|leak(?:ed)?|locked|stolen|hack(?:er|ed|ing|s)?|dihack[A-Za-z0-9_]*|dijebol|dieksploitasi|phish[A-Za-z0-9_]*|scam[A-Za-z0-9_]*|deepfake|spyware|backdoor|defac[A-Za-z0-9_]*|cryptojack[A-Za-z0-9_]*|skimming|carding|exfiltrat[A-Za-z0-9_]*|vishing|smishing|sextortion)(?-u:\b)"
);
lazy_re!(
    DENIED,
    r"(?i)(?-u:\b)(tidak|tak|nggak|gak|ga|kagak|kaga|bukan|belum|no|not|never)\s+(?:ada\s+|terjadi\s+|mengalami\s+|a\s+|an\s+)?(?:gempa|tsunami|erupsi|kebakaran|banjir|longsor|kecelakaan|penembakan|serangan|kebocoran|peretasan|wabah|keracunan|phk|pemadaman|ransomware|malware|attack|breach)|(?-u:\b)(?:gempa|tsunami|erupsi|kebakaran|banjir|kebocoran data|peretasan|serangan siber)(?-u:\b)[^.!?]{0,30}(?-u:\b)(?:tidak terjadi|tidak benar|hoaks|hoax|palsu|dibantah)|(?-u:\b)(?:hoaks|hoax|video lama|kabar palsu|dibantah|denies|false claim)(?-u:\b)"
);
lazy_re!(
    DENIED_ANY,
    r"(?i)(?-u:\b)(?:hoaks?|kabar palsu|false claim|misinformation|disinformasi|berita palsu|isu hoax)(?-u:\b)"
);
lazy_re!(
    EDUCATION,
    r"(?i)(?-u:\b)(apa itu|pengertian|tutorial|cara (?:mencegah|mengatasi|menghapus|menghindari)|tips|edukasi|simulasi|latihan|mitigasi|kesiapsiagaan|tas bencana|go[- ]?bag|potensi|risiko|rawan|prediksi|diprediksi|andaikan|seandainya|bagaimana jika|what is|how to|exercise|simulation)(?-u:\b)"
);
lazy_re!(
    AID,
    r"(?i)(?-u:\b)(salurkan|menyalurkan|penyaluran|donasi|rekonstruksi|rehabilitasi|kirim bantuan|pusdalops|rakor|seminar|webinar)(?-u:\b)"
);
lazy_re!(
    QUESTION,
    r"(?i)\?|(?-u:\b)(apakah|apa benar|benarkah|beneran|katanya|rumornya|rumour|rumor|is it true|ada yang dengar|ada yang tahu|nggak sih|tidak sih)(?-u:\b)"
);
lazy_re!(
    WARNING,
    r"(?i)(?-u:\b)(peringatan\s+(?:dini|cuaca|tsunami|banjir|longsor|erupsi|gelombang|angin|bmkg)|(?:keluarkan|mengeluarkan|rilis|merilis)\s+peringatan|warning|early warning|status (?:awas|siaga)|waspada tsunami|hujan ekstrem)(?-u:\b)"
);
lazy_re!(
    EVENT_ACTION,
    r"(?i)(?-u:\b)(ambruk|runtuh|roboh|tenggelam|hanyut|pohon tumbang|tergelincir|dikeroyok|dibunuh|dirampok|begal|terjadi|melanda|menerjang|menghantam|merendam|terendam|menggenangi|jebol|meluap|mengguncang|diguncang|guncang|bergetar|terasa|dirasakan|erupsi|meletus|terbakar|dilalap|hangus|karam|terguling|tabrak|tabrakan|bertabrakan|menabrak|ditabrak|anjlok|jatuh|ditembak|tertembak|kemalingan|ditangkap|diamankan|mengalami|terdampak|dilaporkan|laporkan|meninggal|tewas|tewaskan|luka|dievakuasi|terjebak|meledak|ledakan|padam|putus|reported|hits?|struck|porak[- ]poranda|berhamburan|beterbangan|sapu|menyapu|porakporandakan|memporakporandakan|hempas|menghempas|rusak|merusak|masuk rumah|menggenang)(?-u:\b)"
);
lazy_re!(
    METAPHOR,
    r"(?i)(?:post[- ]?)?(?:blue|blauwe)[- ]?tsunami|ocean\s+tsunami|surfing\s+(?:a\s+)?tsunami|crowcaster\.com|(?-u:\b)R[1-9]\s+#[0-9]+(?-u:\b)[^.!?\n]{0,60}\$[0-9]+\.[0-9]+|tsunami\s+(?:biru|politik|protes|suara|pemilu|dukungan|saham|cuan|vonis)|(?-u:\b)(?:opposition|elections?|political|politik|pemilu)(?-u:\b)[^.!?]{0,200}(?-u:\b)tsunami(?-u:\b)|(?-u:\b)(?:humans?|people|manusia|jiwa|otak|hati)(?-u:\b)[^.!?]{0,80}(?-u:\b)(?:malware|virus)(?-u:\b)|(?-u:\b)wabah\s+(?:kebencian|cinta|rindu|judi|judol|korupsi|kemiskinan|hoaks|kebohongan|intoleransi|k-?pop|drakor|tiktok)|(?-u:\b)blackout\s+(?:party|music|festival|concert)|(?-u:\b)dpr\s+(?:ian|live|cream|artic|rem|regime)(?-u:\b)|(?-u:\b)dream\s+perfect\s+regime(?-u:\b)|(?-u:\b)(?:banjir|kebanjiran)\s+(?:pc|photocards?|merch|lightstick)(?-u:\b)|satu\s+album\s+banjir|(?-u:\b)bem(?-u:\b)[^.!?\n]{0,80}(?-u:\b)(?:kode|css|html|modifier|element|frontend|styling|component)(?-u:\b)|(?-u:\b)bebas\s+(?:laka\s+(?:ataupun|atau|dan)\s+)?banjir|(?:kyk|kaya|kayak|seperti|ibarat)\s+(?:nunggu|menunggu\s+)?(?:orang\s+hamil|pengumuman\s+snbt|snbt|sbmptn)|pembangkit\s+listrik\s+tenaga\s+kangen|(?-u:\b)(?:nadia\s+omara|ku\s+racuni|bujuran\s+keracunan)(?-u:\b)|(?:suruh|disuruh)\s+jadi\s+anjing\s+rabies|(?-u:\b)jam\s+aja\s+di\s*korupsi(?-u:\b)|(?-u:\b)kampung\s+durian\s+runtuh(?-u:\b)|(?-u:\b)teori\s+perang\s+sun\s+tzu(?-u:\b)|(?-u:\b)(?:bebas|anti|aman|tanpa)\s+(?:banjir|macet|kebakaran|gempa|longsor|badai)(?-u:\b)|(?-u:\b)badai\s+(?:pasti\s+berlalu|asmara|cinta|kehidupan|hidup|rumah\s+tangga|ekonomi|cuan|rezeki|politik|pemilu|ujian|cobaan|cedera|kritik|protes|kecaman|hujatan|penolakan|isu|gosip|kontroversi|salju)(?-u:\b)|(?-u:\b)(?:cantik|rambut|kece|gaya|tampilan)\s+badai(?-u:\b)|(?-u:\b)(?:melewati|hadapi|menghadapi)\s+badai\s+(?:hidup|kehidupan)(?-u:\b)|(?-u:\b)(?:kebakaran\s+jenggot|kebakaran\s+(?:harga|promo|diskon)|terbakar\s+(?:api\s+)?(?:amarah|emosi|cemburu|asmara|rindu)|tersulut\s+api\s+(?:amarah|emosi)|membakar\s+semangat|panggung\s+terbakar|membakar\s+panggung|sayap(?:-sayap)?\s+(?:pupus\s+)?terbakar)(?-u:\b)|(?-u:\b)(?:gempa\s+(?:politik|lokal)|baby\s+gempa|boboiboy(?:\s+gempa)?|baju\s+gempa|kaos\s+gempa|goyang\s+gempa|joget\s+gempa|senam\s+gempa|guncangan\s+(?:politik|kabinet|ekonomi|jiwa|batin)|mengguncang\s+(?:jagat(?:\s+maya)?|dunia(?:\s+maya)?|publik|parlemen))(?-u:\b)|(?-u:\b)(?:banjir\s+(?:order|orderan|cuan|promo|diskon|hadiah|bonus|ucapan|pujian|penghargaan|bintang|rating|air\s*mata|gol(?-u:\b)))(?-u:\b)|(?-u:\b)(?:longsor\s+(?:suara|kurs|saham|suara\s+partai))(?-u:\b)|(?-u:\b)(?:ihsg|rupiah|saham|kurs)\s+(?:ambruk|anjlok)(?-u:\b)|(?-u:\b)(?:ditembak\s+duluan|ditembak\s+(?:sama\s+)?(?:gebetan|doi|crush|cewek|cowok|temen|teman|orang)|nembak\s+(?:gebetan|doi|crush|cewek|cowok)|ditembak\s+(?:hati|cinta|asmara|batin|panah|tatapan|mati\s+rasa)|mati\s+rasa\s+hatiku|(?:bikin\s+)?sim\s+nembak|nembak\s+(?:sim|nilai|wifi|sinyal|harga|jawaban|soal|gawang)|tembakan\s+ke\s+gawang|(?:ditembak|nembak|tertembak)[^.!?]{0,35}(?:pacaran|pacar|gebetan|doi|jadian|crush|confess|baper|pdkt|kencan|dating|tinder|bumble|dinner|makan malem|makan malam|nyatain|nyatakan)|(?:pacaran|pacar|gebetan|doi|jadian|crush|pdkt|kencan|dating)[^.!?]{0,35}(?:ditembak|nembak|tertembak))(?-u:\b)|(?-u:\b)(?:pencuri|maling|rampok|begal)[^.!?]{0,20}(?:hati|cinta|perhatian|senyum)(?-u:\b)|(?-u:\b)(?:curi\s+poin|mencuri\s+poin|mencuri\s+start|curi\s+start|begal\s+(?:rekening|saldo|pulsa|data|politik|partai|konstitusi)|pembegalan\s+(?:partai|konstitusi))(?-u:\b)|(?-u:\b)(?:dibunuh|membunuh)[^.!?]{0,20}(?:rasa|rindu|sepi|waktu|kebosanan|karakter|mimpi|harapan|cinta)(?-u:\b)|(?-u:\b)pembunuhan\s+karakter|membunuh\s+karakter(?-u:\b)|(?-u:\b)(?:ngedrama|drama|pura[- ]?pura|alasan|bohong|settingan)\s+(?:kecelakaan|tabrakan)(?-u:\b)|(?-u:\b)(?:kecelakaan|tabrakan)\s+(?:boongan|bohong[- ]bohongan|palsu|drama|settingan)(?-u:\b)|(?-u:\b)(?:tabrakan\s+(?:logika|ideologi|pendapat|argumen|prinsip|pemikiran|jadwal|acara|event|konser|libur|ujian|meeting|praktik|kepentingan)|(?:menabrak|tabrak)\s+(?:aturan|konstitusi|hukum|uu|norma|etika|logika))(?-u:\b)|(?-u:\b)(?:tergelincir\s+(?:lidah|dari\s+puncak|klasemen|ke\s+(?:lembah|jurang)\s+(?:dosa|maksiat)))(?-u:\b)|(?-u:\b)(?:durian\s+runtuh|jembatan\s+cinta)(?-u:\b)|(?-u:\b)(?:hanyut\s+dalam\s+(?:alunan|suasana|musik|emosi|perasaan|khayalan|lagu)|tenggelam\s+(?:dalam\s+(?:lamunan|duka|kesedihan|kenangan|nostalgia|dosa)|di\s+dasar\s+klasemen))(?-u:\b)|(?-u:\b)(?:racun\s+(?:tiktok|shopee|ig|belanja|skincare|ootd|cinta|asmara|dunia)|teracuni|keracunan\s+(?:cinta|asmara)|meracuni\s+(?:pikiran|otak|jiwa)|gila\s+anjing\s+(?:lu|lo|banget|deh))(?-u:\b)|(?-u:\b)(?:sid\s+bukan\s+pahlawan|pelacur\s+jalanan\s+bermahkotakan|bermahkotakan\s+duri|tutup\s+matamu\s+kawan\s+besarkan\s+hati|emyu(?:\s+baby)?|munyuk|bobotoh\s+persib)(?-u:\b)|(?-u:\b)[A-Za-z]+(?:\s+[A-Za-z]+){1,2}\s+of\s+(?:Doctor\s+X|Tigers\s+sniff)(?-u:\b)|(?-u:\b)(?:mantan\s+rp|anak\s+rp|pacar\s+rl|main\s+rp)(?-u:\b)|(?-u:\b)(?:bentrok|tabrakan)\s+(?:sama\s+|dengan\s+)?(?:jadwal|agenda|kuliah|matkul|mata\s+kuliah|jam\s+kuliah|kelas|meeting|rapat|acara|praktikum|waktu|ujian|kalender|zoom)(?-u:\b)|(?-u:\b)revolusi\s+(?:skincare|kecantikan|wajah|kulit|glowing|jerawat|industri(?:\s+4\.0)?|mental|teknologi|digital|kebiasaan|gaya\s+hidup|tubuh|diet|perawatan|estetika)(?-u:\b)|(?-u:\b)(?:banjir|gempa|tsunami|erupsi|kebakaran|longsor|tanah\s+longsor|ambruk|runtuh|kecelakaan|ledakan|penembakan|wabah|keracunan)(?-u:\b)[^.!?\n]{0,60}(?-u:\b)(?:wkwk+|wkkwk+|wkwkwk+|hahaha+|hehehe+|xixi+|ngakak|kocak|bjir\s+kocak|anjir\s+kocak|giveaway|saldo\s+dana|shopee\s+haul)(?-u:\b)"
);

lazy_re!(
    EN_GOV_RE,
    r"(?-u:\b)(?:[Ii]ndonesia['’]?s\s+)?(?:[Vv]ice[- ]?[Pp]resident|[Pp]resident|[Mm]inister|[Gg]overnment|[Pp]arliament)(?-u:\b)"
);
lazy_re!(
    TARGET_ENTITY_RE,
    r"(?-u:\b)(?:[Ss]erver|[Jj]aringan|[Ss]istem|[Ss]itus|[Ww]ebsite|[Ll]ayanan|[Aa]kun|[Pp]abrik|[Pp]erusahaan|[Rr]umah\s+[Ss]akit|[Rr][Ss][Uu]?[Dd]?|[Pp]uskesmas|[Kk]linik|[Ss][Pp][Pp][Gg]|[Bb]adan\s+[Gg]izi\s+[Nn]asional|[Kk]omdigi|[Kk]ominfo|[Kk]anwil\s+[Dd][Jj][Pp]|[Kk][Pp][Pp]\s+(?:[Pp]ratama|[Mm]adya|[Kk]husus))\s+((?:[A-Z][A-Za-z0-9_.-]*)(?:\s+[A-Z][A-Za-z0-9_.-]*){0,3})(?-u:\b)"
);
lazy_re!(
    DOC_HIST_RE,
    r"(?i)(?-u:\b)(?:baca(?:in)?|belajar|buku|artikel|kisah|cerita)\s+sejarah(?-u:\b)"
);
lazy_re!(
    TRADING_HALT_RE,
    r"(?i)(?-u:\b)(?:trading\s+halt|bursa\s+efek)(?-u:\b)"
);
lazy_re!(PROSE_STRIP_RE, r"https?://\S+|[@#]\S+");
lazy_re!(
    ID_PROSE_RE,
    r"(?i)(?-u:\b)(?:yang|yg|di|ke|dari|ini|itu|untuk|utk|dengan|dgn|atau|pada|akan|sudah|udah|udh|belum|blm|tidak|tak|bukan|ga|gak|nggak|enggak|engga|kagak|kaga|jangan|jgn|warga|korban|mahasiswa|polisi|aparat|pemerintah|kawan|orang|org|rumah|rmh|jalan|jln|kota|daerah|wilayah|tempat|kantor|sekolah|kampus|pasar|mall|stasiun|bandara|tol|sungai|kali|tanggul|hujan|ujan|angin|api|asap|listrik|air|lampu|mati\s+lampu|mati\s+listrik|listrik\s+padam|byarpet|banjir|kebanjiran|longsor|kebakaran|terbakar|damkar|karhutla|gempa|erupsi|puting\s+beliung|kecelakaan|tabrakan|penembakan|ditembak|pencurian|curanmor|begal|jambret|dirampok|maling|penangkapan|ditangkap|diamankan|tersangka|korupsi|suap|ott|unjuk\s+rasa|demonstrasi|aksi\s+massa|aksi\s+kamisan|kamisan|revolusi|bentrok|ricuh|mogok|keracunan|wabah|demam\s+berdarah|puskesmas|sembako|minyak\s+goreng|pertalite|pertamax|spbu|bansos|pemilu|pilkada|pilpres|caleg|menteri|presiden|gubernur|bupati|prabowo|gibran|jokowi|ihsg|rupiah|diretas|dibobol|peretasan|kebocoran\s+data|bocor|hari|pagi|siang|sore|malam|malem|jam|tahun|bulan|minggu|tadi|barusan|kemarin|kmrn|besok|sekarang|skrg|ada|adakah|apakah|apa|siapa|kapan|dimana|dmn|kemana|gimana|gmn|bagaimana|kenapa|knp|mengapa|berapa|karena|krn|karna|kalau|kalo|klo|jika|bila|tapi|tp|tetapi|terus|trs|jadi|jd|masih|msh|lagi|lg|baru|abis|habis|mau|pengen|pingin|bisa|bs|sama|sm|buat|bikin|pakai|pake|cuma|cm|cuman|hanya|banyak|semua|para|tiap|setiap|saat|waktu|pas|terjadi|melanda|merendam|terendam|mengungsi|evakuasi|tewas|meninggal|luka|rusak|ambruk|roboh|runtuh|jebol|meluap|padam|naik|turun|parah|banget|bgt|sekali|bener|benar|beneran|katanya|kabarnya|kayak|kyk|seperti|kaya|malah|padahal|memang|emang|emg|pasti|mungkin|kayaknya|agak|lumayan|cukup|kurang|lebih|paling|makin|tambah|sih|dong|deh|nih|tuh|kok|lah|kah|mah|pun|kek|yak|yaa|yah|yuk|mari|ayo|tolong|plis|jir|anjir|njir|bjir|anjay|astaga|waduh|buset|parno|hectic|jujurly|literally\s+(?:di|udah|lagi|bikin|parah|gue|gw|aku|ada|ga|gak)|gue|gw|gua|aku|saya|lo|lu|kamu|kalian|kami|kita|mereka|dia|beliau|pak|bu|bang|bg|mas|mbak|mba|kak|om|tante|nder|min|mimin|bro|sis|guys\s+(?:ada|di|yg|ini|itu|kok|udah|lagi)|wkwk[A-Za-z0-9_]*|haha[A-Za-z0-9_]*|hehe[A-Za-z0-9_]*|cok|cuk|rek|lur|gan|bray|wes|wis|ora|nek|neng|ning|ndek|iki|iku|koe|koen|ngopo|gawe|ketok|enek|ana|teu|geus|ayeuna|atuh|euy|punten|sampurasun|wargi|caah|lindhu|udan|banyu|cai|indonesia['’]?s?\s+(?:vice[- ]?president|president|government|politics|parliament|cabinet)|indonesian\s+politics)(?-u:\b)"
);
lazy_re!(
    FOREIGN_NON_ID_RE,
    r"(?i)[¿¡ñÑçÇßãõÃÕåøæÅØÆłńśźżŁŃŚŹŻčřšžČŘŠŽğışĞİŞ]|(?-u:\b)(?:habra|habrá|dado|povo|assusta|infidelite|infidélité|nouveau|nouvelle|is\s+het|gaat\s+het|hebben\s+we|het\s+kabinet|uit\s+het|staat\s+op\s+vallen|gevallen|klappen|gestapt|vvd|cda|d66|pvv|fvd|nsc|tweede\s+kamer|eerste\s+kamer|den\s+haag|nederland|nederlands|verkiezingen|tussenformeren|belasting|gestemd|stemming|conclusie|hoezo|democratie|formatie|regerende\s+partijen|coalitieleden|verkiezingsuitslag|debacle-kabinet|links\s+kabinet|varian\s+baharu|baharu|dikesan|sejauh\s+mana\s+bahayanya)(?-u:\b)"
);
lazy_re!(
    SIT_GROUND_EXTRA_RE,
    r"(?i)(?-u:\b)(?:rupiah|ihsg|dpr(?:\s+ri)?|mpr|wapres|prabowo|gibran|danantara|pertamina|ojk|bpom|(?-u:\b)kai(?-u:\b)|(?-u:\b)mbg(?-u:\b)|mati\s+lampu|mati\s+listrik|listrik\s+padam|padam\s+listrik|pemadaman\s+listrik)(?-u:\b)"
);

// Per-topic isReport regexes
lazy_re!(
    R_CORETAX_1,
    r"(?i)coretax|core\s*tax|e-?faktur|e-?bupot|faktur\s+pajak|npwp|spt|djp"
);
lazy_re!(
    R_CORETAX_2,
    r"(?i)error|eror|down|gangguan|gagal|lemot|lambat|timeout|crash|bug|lumpuh|kendala|trouble|login|otp|akses|bisa|tidak\s+bisa|gabisa|maintenance|pemeliharaan|update|perbaikan|rilis|implementasi|penerapan|migrasi|aktivasi|terbitkan|upload|unduh|antre|keluh|phishing|penipuan|palsu|resmi|imbau|tegas|capai|tumbuh|realisasi"
);
lazy_re!(
    R_PAJAK_1,
    r"(?i)pajak|djp|ditjen\s+pajak|direktorat\s+jenderal\s+pajak|kpp|kanwil\s+djp|kring\s+pajak|spt|npwp|ppn|pph|faktur\s+pajak|fiskus"
);
lazy_re!(
    R_PAJAK_2,
    r"(?i)penerimaan|realisasi|target|raup|capai|tembus|tumbuh|setoran|restitusi|pemeriksaan|penagihan|tunggakan|sita|dirampas|perampasan|gijzeling|penyanderaan|pidana|penggelapan|pengemplang|faktur\s+fiktif|korupsi|suap|(?-u:\b)ott(?-u:\b)|ditangkap|tersangka|penipuan|phishing|apk\s+palsu|modus|imbau|tegas|bantah|klarifikasi|tarif|kenaikan|naik|turun|insentif|diskon|bebas|pembebasan|aturan|pmk|uu|peraturan|kebijakan|lapor|pelaporan|batas\s+waktu|denda|sanksi|demo|protes|tolak|gangguan|error|eror|down|antre|keluhan|sosialisasi|sinergi"
);
lazy_re!(
    R_KESEHATAN_1,
    r"(?i)kasus|pasien|terjangkit|keracunan|wabah|(?-u:\b)klb(?-u:\b)|kejadian luar biasa|meninggal|tewas|korban|dirawat|terinfeksi|dilarikan|intoksikasi"
);
lazy_re!(R_KESEHATAN_MUAL, r"(?i)(?:mual|muntah)");
lazy_re!(
    R_KESEHATAN_MUAL_CTX,
    r"(?i)(?:massal|keracunan|makanan|mbg|dilarikan|rumah\s+sakit|(?-u:\b)rs(?-u:\b)|puskesmas|korban|banyak\s+orang|warga)"
);
lazy_re!(
    R_INSIDEN_OBJ,
    r"(?i)(?:jembatan|gedung|bangunan|atap|plafon|rumah|dinding|konstruksi|tembok|jalan|tebing|tower|tiang|kapal|perahu|sampan|tongkang|speedboat|dermaga|pohon|orang|bocah|anak|korban|warga|penumpang)(?-u:\b)"
);
lazy_re!(
    R_INSIDEN_NEG,
    r"(?i)(?:hati|jiwa|perasaan|senyuman|cinta|asmara)(?-u:\b)"
);
lazy_re!(
    R_CYBER_TAKEDOWN,
    r"(?i)(?-u:\b)(?:takedown|sensor|blokir)(?-u:\b)"
);
lazy_re!(
    R_SEISMIC_ACT,
    r"(?i)(?:guncangan|getaran|terasa|dirasakan|kerasa|bergoyang|bergetar|panik|berhamburan|retak|roboh|rusak|kedalaman|kedlmn|(?-u:\b)m\s*[0-9]+(?:[.,][0-9]+)?(?-u:\b)|(?-u:\b)mag(?:nitudo)?\s*[:=]?\s*[0-9]+(?:[.,][0-9]+)?(?-u:\b)|[0-9]+(?:\.[0-9]+)?\s*(?:sr|skala\s+richter|(?-u:\b)m(?-u:\b))|episentrum|pusat\s+gempa|bmkg|pvmbg|vona)"
);
lazy_re!(
    R_SEISMIC_CUE,
    r"(?i)(?-u:\b)(?:gempa|guncangan)(?:\s+bumi)?\s+(?:terasa|dirasakan|berpusat)?\s*di\s+[A-Za-z]"
);
lazy_re!(
    R_QUAKE_VOLC,
    r"(?i)(?:gempa|magnitudo|(?-u:\b)mag\s*[:=]?\s*[0-9]|erupsi|letusan|meletus|awan\s+panas|hujan\s+abu|lahar)"
);
lazy_re!(
    R_BADAI_1,
    r"(?i)(?-u:\b)(?:kecepatan|knot|(?-u:\b)km/h(?-u:\b)|bmkg|bnpb|bpbd|waspada|siaga)(?-u:\b)"
);
lazy_re!(
    R_BADAI_2,
    r"(?i)badai|siklon|puting beliung|angin kencang|angin ribut|badai petir|gelombang pasang|bibit siklon|hujan ekstrem|cuaca ekstrem"
);
lazy_re!(
    R_FIRE_NEG,
    r"(?i)(?-u:\b)(?:film|bioskop|cfd|car\s+free\s+day|kolaborasi|strategi\s+film)(?-u:\b)"
);
lazy_re!(
    R_FIRE_1,
    r"(?i)(?-u:\b)(?:kebakaran|terbakar|kobaran\s+api|si\s+jago\s+merah|dilalap|ludes|hangus|asap\s+tebal|padamkan)(?-u:\b)"
);
lazy_re!(
    R_FIRE_2,
    r"(?i)(?-u:\b)(?:kebakaran|terbakar)(?:\s+(?:rumah|ruko|gedung|pabrik|pasar|hutan|lahan|gudang|kios))?\s+(?:di|melanda|hanguskan)"
);
lazy_re!(
    R_KECELAKAAN,
    r"(?i)(?-u:\b)(?:longsor|kecelakaan|penembakan|pencurian)(?:\s+(?:rumah|gedung|pabrik|pasar|hutan|lahan))?\s+(?:di|melanda|merendam)"
);
lazy_re!(R_GENANGAN, r"(?i)(?-u:\b)genangan(?-u:\b)");
lazy_re!(
    R_BANJIR_SERIOUS,
    r"(?i)(?-u:\b)(?:banjir\s+bandang|banjir\s+rob|tanggul\s+jebol|air\s+meluap|merendam|terendam|evakuasi|mengungsi)(?-u:\b)"
);
lazy_re!(
    R_BANJIR_2,
    r"(?i)(?-u:\b)(?:banjir|longsor)(?:\s+(?:rumah|gedung|pabrik|pasar|warga|pemukiman))?\s+(?:di|melanda|merendam)"
);
lazy_re!(
    R_KARHUTLA_1,
    r"(?i)(?:titik\s+panas|hotspot|kebakaran\s+(?:hutan|lahan)|karhutla)\s+(?:meluas|terjadi|membakar|terpantau|hanguskan|di)"
);
lazy_re!(R_KARHUTLA_NEG, r"(?i)tolak|tuntutan|isu|demo|unjuk\s+rasa");
lazy_re!(
    R_PENANGKAPAN_1,
    r"(?i)ditangkap|diamankan|penangkapan|(?-u:\b)ott(?-u:\b)"
);
lazy_re!(
    R_PENANGKAPAN_2,
    r"(?i)(?-u:\b)(?:polisi|polres|polda|polsek|kpk|bnn|densus|kejaksaan|aparat|pelaku|tersangka|buron|maling|begal|rampok|koruptor|suap|penipu|narkoba)(?-u:\b)"
);
lazy_re!(
    R_DEMO_1,
    r"(?i)unjuk rasa|aksi massa|demonstrasi|demonstran|pengunjuk rasa|(?-u:\b)demo(?-u:\b)|protes|mogok|kerusuhan|bentrok|pemogokan|women'?s\s+march|international\s+women'?s\s+day|hari\s+buruh|buruh\s+sedunia|kawal demo|gelar aksi|rencana aksi|aksi\s+kamisan|kamisan|revolusi"
);
lazy_re!(
    R_DEMO_2,
    r"(?i)berlangsung|turun ke jalan|berkumpul|(?:meng)?gelar|ricuh|bentrok|tuntut|menuntut|menolak|tolak|(?-u:\b)orasi(?-u:\b)|(?-u:\b)march(?-u:\b)|(?-u:\b)aksi(?-u:\b)|siapkan pengamanan|menyiapkan pengamanan|(?:di)?siagakan|rencana (?:aksi|demo|unjuk rasa)|(?:meng)?kawal (?:demo|aksi|massa)|(?:meng)?amankan (?:demo|aksi)|pengamanan (?:demo|aksi)|(?:di)?kerahkan|(?:di)?terjunkan|antisipasi|aksi\s+kamisan|kamisan|revolusi"
);
lazy_re!(
    R_WABAH,
    r"(?i)kasus|pasien|terjangkit|keracunan|wabah (?:dbd|campak|kolera|difteri)|kejadian luar biasa|intoksikasi"
);
lazy_re!(
    R_LISTRIK,
    r"(?i)listrik padam|mati listrik|pemadaman listrik|padam listrik|mati lampu|pln[^.!?]{0,40}blackout"
);
lazy_re!(
    R_PHK,
    r"(?i)melakukan|lakukan|terkena|mengalami|memutus|dirumahkan|pabrik tutup|tutup[^.!?]{0,25}phk|phk[^.!?]{0,30}(?:[0-9]+|ratusan|ribuan)"
);
lazy_re!(
    R_IHSG,
    r"(?i)trading halt|dihentikan|anjlok|merosot|turun\s+[0-9]"
);
lazy_re!(R_RUPIAH, r"(?i)melemah|menguat|anjlok");
lazy_re!(
    R_PEMILU,
    r"(?i)ricuh|bentrok|ditunda|dibatalkan|pemungutan suara ulang"
);
lazy_re!(
    R_POLITIK_1,
    r"(?i)(?-u:\b)(?:resmi\s+dilantik|dilantik|dipecat|mengundurkan\s+diri|resignation|divonis|reshuffle|impeachment|dimakzulkan)(?-u:\b)"
);
lazy_re!(
    R_POLITIK_2,
    r"(?i)(?-u:\b)(?:komdigi|kominfo)(?-u:\b)[^.\n]{0,40}(?-u:\b)(?:takedown|anggaran|triliun)(?-u:\b)"
);
lazy_re!(
    R_POLITIK_NEG,
    r"(?i)(?-u:\b)(?:usul|wacana|menurut|angkat\s+bicara|komentar|opini)(?-u:\b)"
);
lazy_re!(
    R_POLITIK_ALLOW,
    r"(?i)(?-u:\b)(?:komdigi|anggaran|resignation)(?-u:\b)"
);
lazy_re!(
    R_KONFLIK_NEG1,
    r"(?i)(?-u:\b)(?:upacara|tabur\s+bunga|hari\s+pahlawan|dirgahayu|ziarah|hut\s+tni|museum\s+militer)(?-u:\b)"
);
lazy_re!(
    R_KONFLIK_NEG2,
    r"(?i)perang\s+(?:antar\s+)?(?:fandom|fans?|warganet|netizen|cuitan|bocil)"
);
lazy_re!(
    R_KONFLIK_NEG3,
    r"(?i)drone\s+(?:fpv|rakitan|mainan|kamera|balap)"
);
lazy_re!(
    R_KONFLIK_POS,
    r"(?i)(?-u:\b)(?:baku\s+tembak|kontak\s+senjata|serangan\s+udara|serangan\s+rudal|invasi|kudeta|gencatan\s+senjata|darurat\s+militer|pertempuran|penyanderaan|kkb\s+papua|separatis)(?-u:\b)"
);

// Event type regexes
lazy_re!(
    ET_CORETAX_ERR,
    r"(?i)error|eror|down|gangguan|gagal|lemot|lambat|timeout|crash|bug|lumpuh|kendala|trouble"
);
lazy_re!(
    ET_PAJAK_ENF,
    r"(?i)pidana|korupsi|suap|ott|penggelapan|faktur\s+fiktif|penipuan|phishing|sita|gijzeling|penyanderaan"
);
lazy_re!(ET_MARITIME, r"(?i)tenggelam|karam");
lazy_re!(ET_COLLAPSE, r"(?i)ambruk|runtuh");
lazy_re!(ET_TREE, r"(?i)pohon tumbang");
lazy_re!(ET_NARC, r"(?i)narkoba");
lazy_re!(ET_HOMICIDE, r"(?i)pembunuhan|dibunuh");
lazy_re!(ET_TSUNAMI, r"(?i)tsunami");
lazy_re!(ET_ERUPTION, r"(?i)erupsi|letusan|meletus|awan panas|lahar");
lazy_re!(ET_TORNADO, r"(?i)puting beliung|tornado");
lazy_re!(ET_CYCLONE, r"(?i)siklon");
lazy_re!(ET_LONGSOR, r"(?i)longsor");
lazy_re!(ET_RANSOM, r"(?i)ransomware");
lazy_re!(ET_DDOS, r"(?i)ddos");
lazy_re!(ET_BREACH, r"(?i)bocor|kebocoran|breach");

struct OrderedSet {
    items: Vec<String>,
    seen: HashSet<String>,
}

impl OrderedSet {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            seen: HashSet::new(),
        }
    }
    fn insert(&mut self, s: impl Into<String>) {
        let s = s.into();
        if self.seen.insert(s.clone()) {
            self.items.push(s);
        }
    }
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub fn concrete_target(span: &str, category: Option<&str>) -> Option<String> {
    if !matches!(
        category,
        Some("pajak" | "siber" | "ekonomi" | "politik" | "kesehatan")
    ) {
        return None;
    }
    if let Some(m) = LOCAL_ORG_RE.find(span) {
        return Some(m.as_str().to_uppercase());
    }
    if let Some(m) = EN_GOV_RE.find(span) {
        return Some(m.as_str().to_uppercase());
    }
    TARGET_ENTITY_RE
        .captures(span)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

pub fn event_type<'a>(topic: &'a str, span: &str) -> &'a str {
    match topic {
        "coretax" => {
            if ET_CORETAX_ERR.is_match(span) {
                "coretax_outage"
            } else {
                "tax_system"
            }
        }
        "pajak" => {
            if ET_PAJAK_ENF.is_match(span) {
                "tax_enforcement"
            } else {
                "tax_policy"
            }
        }
        "insiden" => {
            if ET_MARITIME.is_match(span) {
                "maritime_accident"
            } else if ET_COLLAPSE.is_match(span) {
                "collapse"
            } else if ET_TREE.is_match(span) {
                "tree_fall"
            } else {
                "accident"
            }
        }
        "kriminal" => {
            if ET_NARC.is_match(span) {
                "narcotics"
            } else if ET_HOMICIDE.is_match(span) {
                "homicide"
            } else {
                "violent_crime"
            }
        }
        "kesehatan" | "wabah" => "health_incident",
        "gempa" => {
            if ET_TSUNAMI.is_match(span) {
                "tsunami"
            } else if ET_ERUPTION.is_match(span) {
                "eruption"
            } else {
                "earthquake"
            }
        }
        "badai" => {
            if ET_TORNADO.is_match(span) {
                "tornado"
            } else if ET_CYCLONE.is_match(span) {
                "cyclone"
            } else {
                "storm"
            }
        }
        "banjir" => {
            if ET_LONGSOR.is_match(span) {
                "landslide"
            } else {
                "flood"
            }
        }
        "ransomware" => {
            if ET_RANSOM.is_match(span) {
                "ransomware"
            } else {
                "malware"
            }
        }
        "serangan siber" => {
            if ET_RANSOM.is_match(span) {
                "ransomware"
            } else if ET_DDOS.is_match(span) {
                "ddos"
            } else if ET_BREACH.is_match(span) {
                "data_breach"
            } else {
                "intrusion"
            }
        }
        _ => topic,
    }
}

fn flag(obj: &Value, key: &str) -> bool {
    obj.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn is_report(
    topic: &str,
    span: &str,
    has_location: bool,
    has_target: bool,
    nlp_clause: Option<&Value>,
) -> bool {
    let identity = has_location || has_target;
    if let Some(cr) = nlp_clause {
        if let Some(f) = cr.get("flags")
            && (flag(f, "isDenial")
                || flag(f, "isHist")
                || flag(f, "isEdu")
                || flag(f, "isQuest")
                || flag(f, "isHearsay")
                || flag(f, "isCond")
                || flag(f, "isReader")
                || flag(f, "isSatire")
                || flag(f, "isCommentaryMeta")
                || flag(f, "isCommercial")
                || flag(f, "isAd")
                || flag(f, "isVent")
                || flag(f, "isSpam"))
        {
            return false;
        }
        let has_nlp_strong = cr
            .get("events")
            .and_then(Value::as_array)
            .is_some_and(|evs| {
                evs.iter().any(|e| {
                    e.get("topic").and_then(Value::as_str) == Some(topic)
                        && flag(e, "strong")
                        && !flag(e, "minorWaterlogging")
                })
            });
        if has_nlp_strong && identity {
            return true;
        }
    }

    match topic {
        "coretax" => identity && R_CORETAX_1.is_match(span) && R_CORETAX_2.is_match(span),
        "pajak" => identity && R_PAJAK_1.is_match(span) && R_PAJAK_2.is_match(span),
        "kriminal" => identity && EVENT_ACTION.is_match(span),
        "kesehatan" => {
            identity
                && (R_KESEHATAN_1.is_match(span)
                    || (R_KESEHATAN_MUAL.is_match(span) && R_KESEHATAN_MUAL_CTX.is_match(span)))
        }
        "insiden" => {
            identity
                && EVENT_ACTION.is_match(span)
                && R_INSIDEN_OBJ.is_match(span)
                && !R_INSIDEN_NEG.is_match(span)
        }
        "ransomware" => CYBER_TARGET.is_match(span) && CYBER_ACTION.is_match(span) && identity,
        "serangan siber" | "akun dibajak" => {
            (CYBER_ACTION.is_match(span) || R_CYBER_TAKEDOWN.is_match(span)) && identity
        }
        "gempa" => {
            if !identity {
                return false;
            }
            if ET_TSUNAMI.is_match(span) {
                return EVENT_ACTION.is_match(span) || WARNING.is_match(span);
            }
            let has_act = EVENT_ACTION.is_match(span) || R_SEISMIC_ACT.is_match(span);
            let has_cue = R_SEISMIC_CUE.is_match(span);
            (has_act || has_cue) && R_QUAKE_VOLC.is_match(span)
        }
        "badai" => {
            identity
                && (EVENT_ACTION.is_match(span)
                    || WARNING.is_match(span)
                    || R_BADAI_1.is_match(span))
                && R_BADAI_2.is_match(span)
        }
        "kebakaran" => {
            identity
                && !R_FIRE_NEG.is_match(span)
                && R_FIRE_1.is_match(span)
                && (EVENT_ACTION.is_match(span) || R_FIRE_2.is_match(span))
        }
        "kecelakaan" | "penembakan" | "pencurian" => {
            identity && (EVENT_ACTION.is_match(span) || R_KECELAKAAN.is_match(span))
        }
        "banjir" => {
            if !identity {
                return false;
            }
            if R_GENANGAN.is_match(span) && !R_BANJIR_SERIOUS.is_match(span) {
                return false;
            }
            EVENT_ACTION.is_match(span) || R_BANJIR_2.is_match(span)
        }
        "karhutla" => identity && R_KARHUTLA_1.is_match(span) && !R_KARHUTLA_NEG.is_match(span),
        "penangkapan" => {
            identity
                && R_PENANGKAPAN_1.is_match(span)
                && !CEREMONY_RE.is_match(span)
                && R_PENANGKAPAN_2.is_match(span)
        }
        "demo" => identity && R_DEMO_1.is_match(span) && R_DEMO_2.is_match(span),
        "wabah" => identity && R_WABAH.is_match(span),
        "listrik padam" => identity && R_LISTRIK.is_match(span),
        "phk" => identity && R_PHK.is_match(span),
        "ihsg" => R_IHSG.is_match(span) && (has_target || LOCAL_ORG_RE.is_match(span)),
        "rupiah" => {
            R_RUPIAH.is_match(span)
                && span.bytes().any(|b| b.is_ascii_digit())
                && (has_target || LOCAL_ORG_RE.is_match(span))
        }
        "pemilu" => identity && R_PEMILU.is_match(span),
        "politik" => {
            identity
                && (R_POLITIK_1.is_match(span) || R_POLITIK_2.is_match(span))
                && (!R_POLITIK_NEG.is_match(span) || R_POLITIK_ALLOW.is_match(span))
        }
        "konflik" => {
            identity
                && !R_KONFLIK_NEG1.is_match(span)
                && !R_KONFLIK_NEG2.is_match(span)
                && !R_KONFLIK_NEG3.is_match(span)
                && R_KONFLIK_POS.is_match(span)
        }
        _ => false,
    }
}

fn copy_loc_obj(loc: &Value) -> Value {
    let mut m = Map::new();
    for k in ["name", "prov", "prec", "la", "lo"] {
        if let Some(v) = loc.get(k) {
            m.insert(k.to_string(), v.clone());
        }
    }
    for k in ["provCode", "kabCode", "kecCode", "code"] {
        if let Some(v) = loc.get(k)
            && !v.is_null()
            && v.as_str().is_none_or(|s| !s.is_empty())
        {
            m.insert(k.to_string(), v.clone());
        }
    }
    Value::Object(m)
}

/// Full Rust port of `assess(text, registry, options)` from `src/assessment.js`.
pub fn assess_item(text: &str, registry: &[TopicSpec], options: &AssessOptions) -> Value {
    let raw = analysis::decode_basic_entities_str(text);
    let year = options.year.unwrap_or_else(current_year_utc);
    let nlp = analysis::analyze_post_str(&raw, year);

    let mut findings: Vec<Value> = Vec::new();
    let mut topics = OrderedSet::new();
    let mut reasons = OrderedSet::new();
    let mut rejected_reasons = OrderedSet::new();

    let intent = nlp
        .get("intent")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let nlp_flags = nlp.get("flags").unwrap_or(&Value::Null);
    let has_actionable = flag(nlp_flags, "hasActionableEvent");
    let any_official_warning = flag(nlp_flags, "anyOfficialWarning");

    let spam_flag = options.spam.as_ref().is_some_and(|r| r.is_match(&raw)) || intent == "spam";
    let listing_flag = options.listing || intent == "commercial_listing";
    let ad_flag = intent == "advertisement";
    let vent_flag = intent == "personal_vent";
    let pure_opinion_flag = intent == "opinion" && !has_actionable && !any_official_warning;
    let religious_flag = flag(nlp_flags, "anyReligious") && !has_actionable;
    let hard_non_incident = spam_flag
        || ad_flag
        || vent_flag
        || pure_opinion_flag
        || religious_flag
        || (listing_flag && !has_actionable);
    if religious_flag {
        reasons.insert("religious_discourse");
        rejected_reasons.insert("no_concrete_event");
    }

    let doc_hist = DOC_HIST_RE.is_match(&raw);
    let any_foreign_clause = flag(nlp_flags, "anyForeign");
    let nlp_clauses = nlp
        .get("clauses")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    if !hard_non_incident {
        for tp in registry {
            if tp.adhoc && analysis::is_generic_location_phrase_str(&tp.id) {
                continue;
            }
            if listing_flag && is_feed_only(&tp.id) {
                continue;
            }
            for (idx, cr) in nlp_clauses.iter().enumerate() {
                let clause_obj = cr.get("clause").unwrap_or(&Value::Null);
                let span = clause_obj.get("raw").and_then(Value::as_str).unwrap_or("");
                let prose_span = clause_obj
                    .get("rawNoTags")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(span);
                let norm_geo = clause_obj
                    .get("normGeo")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let prose_text = prose_span.trim();
                let norm_prose_text = clause_obj
                    .get("normProse")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let cr_events = cr
                    .get("events")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let has_prose_event = !cr_events.is_empty();

                let matched_prose = (!prose_text.is_empty() && tp.pos.is_match(prose_text))
                    || (!tp.adhoc
                        && !norm_prose_text.is_empty()
                        && tp.pos.is_match(norm_prose_text));
                let allow_tag_fallback = !has_prose_event
                    || cr_events
                        .iter()
                        .any(|e| e.get("topic").and_then(Value::as_str) == Some(tp.id.as_str()));
                let matched_tags = !matched_prose && allow_tag_fallback && tp.pos.is_match(span);
                if !(matched_prose || matched_tags) {
                    continue;
                }

                let cr_flags = cr.get("flags").unwrap_or(&Value::Null);
                let is_trading_halt_fin =
                    matches!(tp.id.as_str(), "ihsg" | "rupiah") && TRADING_HALT_RE.is_match(span);
                let metaphors: Vec<String> = cr_flags
                    .get("metaphors")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let target_txt = if prose_text.is_empty() {
                    span
                } else {
                    prose_text
                };
                let is_metaphor = metaphor_blocks_topic_str(&metaphors, &tp.id)
                    || (!is_trading_halt_fin && METAPHOR.is_match(target_txt))
                    || options
                        .noise
                        .as_ref()
                        .is_some_and(|r| r.is_match(target_txt))
                    || tp
                        .neg
                        .as_ref()
                        .is_some_and(|r| r.is_match(target_txt) || r.is_match(norm_prose_text));
                if is_metaphor {
                    reasons.insert("metaphor_or_nonincident_usage");
                    rejected_reasons.insert("metaphorical_language");
                    continue;
                }

                let has_nlp_frame = cr_events
                    .iter()
                    .any(|e| e.get("topic").and_then(Value::as_str) == Some(tp.id.as_str()));
                if let Some(ctx_re) = &tp.ctx {
                    let strong_hit = tp
                        .strong
                        .as_ref()
                        .is_some_and(|r| r.is_match(span) || r.is_match(norm_geo));
                    if !strong_hit
                        && !ctx_re.is_match(span)
                        && !ctx_re.is_match(norm_geo)
                        && !has_nlp_frame
                    {
                        reasons.insert("insufficient_topic_context");
                        continue;
                    }
                }

                topics.insert(&tp.id);
                let category = category_of(&tp.id);
                let feed_only = is_feed_only(&tp.id);
                let is_adhoc = tp.adhoc;

                let target = if is_adhoc {
                    None
                } else {
                    concrete_target(span, category)
                };
                let cr_geo = cr.get("geo").unwrap_or(&Value::Null);
                let mut scope = if is_adhoc {
                    "unknown".to_string()
                } else {
                    cr_geo
                        .get("scope")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string()
                };
                let mut location: Option<Value> = if is_adhoc {
                    None
                } else {
                    cr_geo
                        .get("location")
                        .filter(|v| !v.is_null())
                        .map(copy_loc_obj)
                };

                if !is_adhoc && location.is_none() && scope == "unknown" && !any_foreign_clause {
                    let adj_indices: [Option<usize>; 2] = [idx.checked_sub(1), idx.checked_add(1)];
                    for adj in adj_indices.into_iter().flatten() {
                        let Some(nb) = nlp_clauses.get(adj) else {
                            continue;
                        };
                        let nb_geo = nb.get("geo").unwrap_or(&Value::Null);
                        let nb_flags = nb.get("flags").unwrap_or(&Value::Null);
                        let primary_role = nb_geo
                            .get("primaryRole")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        let bad_role = matches!(
                            primary_role,
                            GEO_READER_LOCATION
                                | GEO_AUTHOR_CURRENT
                                | GEO_AUTHOR_HOMETOWN
                                | GEO_HISTORICAL
                        );
                        let nb_loc = nb_geo.get("location").filter(|v| !v.is_null());
                        if nb_geo.get("scope").and_then(Value::as_str) == Some("Indonesia")
                            && let Some(nloc) = nb_loc
                            && !bad_role
                            && !flag(nb_flags, "isDenial")
                            && !flag(nb_flags, "isHist")
                            && !flag(nb_flags, "isReader")
                            && !flag(nb_flags, "isHearsay")
                            && !flag(nb_flags, "isCommercial")
                            && !flag(nb_flags, "isAd")
                            && !flag(nb_flags, "isVent")
                            && !flag(nb_flags, "isOpinion")
                            && !flag(nb_flags, "isSpam")
                        {
                            scope = "Indonesia".to_string();
                            location = Some(nloc.clone());
                            break;
                        }
                    }
                }

                let neg_tok: HashSet<&str> = cr_flags
                    .get("negatedTokens")
                    .and_then(Value::as_array)
                    .map(|arr| arr.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                let is_topic_negated = neg_tok.contains(tp.id.as_str())
                    || (tp.id == "banjir" && neg_tok.contains("flood"))
                    || (tp.id == "gempa" && neg_tok.contains("earthquake"));

                let mut report_type = if flag(cr_flags, "isDenial")
                    || DENIED_ANY.is_match(span)
                    || DENIED.is_match(span)
                    || is_topic_negated
                {
                    "denied"
                } else if doc_hist
                    || flag(cr_flags, "isHist")
                    || is_historical_str(span, span, year)
                {
                    "historical"
                } else if !flag(cr_flags, "isCeremony")
                    && (flag(cr_flags, "isWarning") || WARNING.is_match(span))
                {
                    "warning"
                } else if flag(cr_flags, "isEdu")
                    || EDUCATION.is_match(span)
                    || flag(cr_flags, "isCond")
                {
                    "educational"
                } else if AID.is_match(span) {
                    "response"
                } else if flag(cr_flags, "isQuest")
                    || flag(cr_flags, "isHearsay")
                    || QUESTION.is_match(span)
                    || flag(cr_flags, "isSatire")
                {
                    "unverified_question"
                } else {
                    "commentary"
                };

                let is_minor_wl =
                    tp.id == "banjir" && cr_events.iter().any(|e| flag(e, "minorWaterlogging"));
                if is_minor_wl {
                    report_type = "commentary";
                    if is_topic_negated {
                        reasons.insert("negated_incident");
                    }
                    reasons.insert("minor_waterlogging_condition");
                }

                let report = !is_adhoc
                    && !flag(cr_flags, "isCeremony")
                    && is_report(
                        &tp.id,
                        prose_span,
                        location.is_some(),
                        target.is_some(),
                        Some(cr),
                    );
                if report_type == "commentary"
                    && report
                    && !flag(cr_flags, "isReader")
                    && !flag(cr_flags, "isSatire")
                {
                    report_type = "reported";
                }

                let ev_type = if has_nlp_frame {
                    cr_events
                        .iter()
                        .find(|e| e.get("topic").and_then(Value::as_str) == Some(tp.id.as_str()))
                        .and_then(|e| e.get("type").and_then(Value::as_str))
                        .unwrap_or_else(|| event_type(&tp.id, span))
                        .to_string()
                } else {
                    event_type(&tp.id, span).to_string()
                };

                let eligible = !spam_flag
                    && !feed_only
                    && !is_adhoc
                    && !flag(cr_flags, "isCeremony")
                    && !flag(cr_flags, "isReader")
                    && !flag(cr_flags, "isSatire")
                    && !flag(cr_flags, "isQuest")
                    && !flag(cr_flags, "isHearsay")
                    && !flag(cr_flags, "isCommentaryMeta")
                    && intent != "rumor"
                    && scope == "Indonesia"
                    && (location.is_some() || target.is_some())
                    && (report
                        || (report_type == "warning"
                            && (location.is_some() || target.is_some())
                            && !is_topic_negated))
                    && matches!(report_type, "reported" | "warning");

                if scope != "Indonesia" {
                    reasons.insert(if scope == "foreign" {
                        "foreign_scope"
                    } else {
                        "scope_not_established"
                    });
                }
                if !report && report_type != "warning" {
                    reasons.insert("no_concrete_event");
                }
                if !eligible {
                    reasons.insert(report_type);
                } else {
                    reasons.insert(if report_type == "warning" {
                        "official_warning_cue"
                    } else {
                        "incident_action_detected"
                    });
                    if location.is_some() {
                        reasons.insert("event_location_resolved");
                    }
                    if flag(cr_flags, "isEyewitness") {
                        reasons.insert("eyewitness_indicator");
                    }
                }

                let mut f_obj = Map::new();
                f_obj.insert("topic".into(), Value::String(tp.id.clone()));
                f_obj.insert(
                    "category".into(),
                    category.map_or(Value::Null, |c| Value::String(c.into())),
                );
                f_obj.insert("type".into(), Value::String(ev_type));
                f_obj.insert("location".into(), location.unwrap_or(Value::Null));
                f_obj.insert("target".into(), target.map_or(Value::Null, Value::String));
                f_obj.insert("scope".into(), Value::String(scope));
                f_obj.insert("reportType".into(), Value::String(report_type.into()));
                f_obj.insert("signalEligible".into(), Value::Bool(eligible));
                if feed_only {
                    f_obj.insert("feedOnly".into(), Value::Bool(true));
                }
                if is_adhoc {
                    f_obj.insert("adhoc".into(), Value::Bool(true));
                }
                f_obj.insert("evidence".into(), Value::String(span.into()));
                f_obj.insert(
                    "severity".into(),
                    Value::String(
                        if report_type == "warning" {
                            "warning"
                        } else {
                            "unknown"
                        }
                        .into(),
                    ),
                );
                findings.push(Value::Object(f_obj));
            }
        }
    }

    let extracted_events = nlp
        .get("extractedEvents")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    if !hard_non_incident && !extracted_events.is_empty() {
        for ev in extracted_events {
            if flag(ev, "minorWaterlogging") {
                continue;
            }
            let ev_topic = ev.get("topic").and_then(Value::as_str).unwrap_or("");
            if !registry.iter().any(|t| t.id == ev_topic && !t.adhoc) {
                continue;
            }
            if findings.iter().any(|f| {
                f.get("topic").and_then(Value::as_str) == Some(ev_topic)
                    && flag(f, "signalEligible")
            }) {
                continue;
            }
            let ev_scope = ev.get("scope").and_then(Value::as_str).unwrap_or("");
            let ev_loc = ev.get("location").filter(|v| !v.is_null());
            let loc = if ev_scope == "Indonesia"
                && let Some(l) = ev_loc
            {
                Some(json!({
                    "name": l.get("name").cloned().unwrap_or(Value::Null),
                    "prov": l.get("prov").cloned().unwrap_or(Value::Null),
                    "prec": l.get("prec").cloned().unwrap_or(Value::Null),
                }))
            } else {
                None
            };
            let ev_raw = ev
                .get("evidence")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(Value::as_str)
                .unwrap_or(&raw);
            let ev_cat = ev.get("category").and_then(Value::as_str);
            let target = concrete_target(ev_raw, ev_cat)
                .or_else(|| LOCAL_ORG_RE.find(ev_raw).map(|m| m.as_str().to_uppercase()));
            let is_warning = ev.get("action").and_then(Value::as_str) == Some("warning_issued")
                || intent == "official_warning";
            let loc_role_ok = ev_loc
                .is_none_or(|l| l.get("role").and_then(Value::as_str) == Some(GEO_EVENT_LOCATION));
            let has_geo_or_target =
                (loc.is_some() && loc_role_ok) || (target.is_some() && ev_scope == "Indonesia");
            let cf = ev.get("clauseFlags").unwrap_or(&Value::Null);
            let is_reportable = flag(ev, "strong")
                && ev_scope == "Indonesia"
                && has_geo_or_target
                && intent != "rumor"
                && !flag(nlp_flags, "anyHearsay")
                && !flag(cf, "isCommentaryMeta")
                && !flag(cf, "isCeremony")
                && !flag(cf, "isDenial")
                && !flag(cf, "isHist")
                && !flag(cf, "isQuest")
                && !flag(cf, "isHearsay")
                && !flag(cf, "isReader")
                && !flag(cf, "isSatire");

            if is_reportable {
                topics.insert(ev_topic);
                let rt = if is_warning { "warning" } else { "reported" };
                reasons.insert(if is_warning {
                    "official_warning_cue"
                } else {
                    "incident_action_detected"
                });
                if ev.get("sourceType").and_then(Value::as_str) == Some("eyewitness") {
                    reasons.insert("eyewitness_indicator");
                }
                if loc.is_some() {
                    reasons.insert("event_location_resolved");
                }
                findings.push(json!({
                    "topic": ev_topic,
                    "category": ev_cat,
                    "type": ev.get("eventType").cloned().unwrap_or(Value::Null),
                    "location": loc.unwrap_or(Value::Null),
                    "target": target,
                    "scope": "Indonesia",
                    "reportType": rt,
                    "signalEligible": true,
                    "evidence": ev_raw,
                    "severity": if is_warning { "warning" } else { "unknown" },
                }));
            }
        }
    }

    if spam_flag {
        reasons.insert("spam");
        rejected_reasons.insert("spam_or_engagement_bait");
    }
    if listing_flag && !has_actionable {
        rejected_reasons.insert("commercial_listing");
    }
    if ad_flag && !has_actionable {
        rejected_reasons.insert("advertisement");
    }
    if vent_flag && !has_actionable {
        rejected_reasons.insert("personal_vent");
    }
    if pure_opinion_flag {
        rejected_reasons.insert("opinion_or_commentary");
        rejected_reasons.insert("no_concrete_event");
    }
    if intent == "historical" && !has_actionable {
        rejected_reasons.insert("historical_reference");
    }
    if intent == "educational" && !has_actionable {
        rejected_reasons.insert("educational_content");
    }
    if intent == "denial_or_debunk" {
        let has_neg = nlp
            .get("negatedEvents")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty());
        rejected_reasons.insert(if has_neg {
            "negated_incident"
        } else {
            "denial_or_debunk"
        });
    }
    if any_foreign_clause
        && !findings
            .iter()
            .any(|f| f.get("scope").and_then(Value::as_str) == Some("Indonesia"))
    {
        rejected_reasons.insert("foreign_scope");
    }
    if flag(nlp_flags, "anyMetaphor") && !has_actionable {
        rejected_reasons.insert("metaphorical_language");
    }
    let geo_role = nlp
        .get("geographicContext")
        .and_then(|g| g.get("role"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if matches!(
        geo_role,
        GEO_AUTHOR_CURRENT | GEO_READER_LOCATION | GEO_MENTIONED_IN_PASSING
    ) && !has_actionable
    {
        rejected_reasons.insert("location_not_event_location");
    }

    let events: Vec<Value> = findings
        .iter()
        .filter(|f| flag(f, "signalEligible"))
        .cloned()
        .collect();
    let eligible = !events.is_empty();

    let non_adhoc_findings: Vec<&Value> = findings.iter().filter(|f| !flag(f, "adhoc")).collect();
    let scope = if eligible
        || non_adhoc_findings
            .iter()
            .any(|f| f.get("scope").and_then(Value::as_str) == Some("Indonesia"))
        || geo_role == GEO_EVENT_LOCATION
    {
        "Indonesia"
    } else if any_foreign_clause {
        "foreign"
    } else {
        "unknown"
    };

    let report_type = if eligible {
        if events
            .iter()
            .all(|f| f.get("reportType").and_then(Value::as_str) == Some("warning"))
        {
            "warning"
        } else {
            "reported"
        }
    } else {
        non_adhoc_findings
            .first()
            .copied()
            .or_else(|| findings.first())
            .and_then(|f| f.get("reportType").and_then(Value::as_str))
            .unwrap_or(match intent {
                "denial_or_debunk" => "denied",
                "historical" => "historical",
                "educational" => "educational",
                "question" => "unverified_question",
                _ => "irrelevant",
            })
    };

    let has_permanent_topic = topics
        .items
        .iter()
        .any(|tid| !registry.iter().any(|r| r.id == *tid && r.adhoc));
    let has_situational_intent = matches!(
        intent,
        "question"
            | "rumor"
            | "discussion"
            | "personal_experience"
            | "satire"
            | "incident_report"
            | "eyewitness_report"
            | "opinion"
    ) || flag(nlp_flags, "anyMinorWaterlogging");

    let has_crisis_context = nlp_clauses.iter().any(|c| {
        let cf = c.get("flags").unwrap_or(&Value::Null);
        flag(cf, "hasHazardCue") || flag(cf, "hasHazardQuestion")
    }) || non_adhoc_findings.iter().any(|f| {
        !flag(f, "adhoc")
            && f.get("scope").and_then(Value::as_str) == Some("Indonesia")
            && !matches!(
                f.get("reportType").and_then(Value::as_str),
                Some("denied" | "historical" | "educational")
            )
    });

    let prose_clean = PROSE_STRIP_RE.replace_all(&raw, " ");
    let is_foreign_lang = any_foreign_clause
        || nlp
            .get("layerTrace")
            .and_then(|t| t.get("isForeignLanguage"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let has_id_grounding = !is_foreign_lang
        && (scope == "Indonesia"
            || ID_PROSE_RE.is_match(&prose_clean)
            || LOCAL_ORG_RE.is_match(&prose_clean)
            || LOCAL_MACRO_RE.is_match(&prose_clean));
    let is_unanchored_foreign = is_foreign_lang
        || (!has_id_grounding
            && (scope == "foreign"
                || FOREIGN_NON_ID_RE.is_match(&prose_clean)
                || !prose_clean.trim().is_empty()));

    let has_sit_grounding = scope == "Indonesia"
        || extracted_events.iter().any(|e| flag(e, "strong"))
        || any_official_warning
        || flag(nlp_flags, "anyOfficialUpdate")
        || findings.iter().any(|f| {
            f.get("scope").and_then(Value::as_str) == Some("Indonesia")
                || f.get("target").is_some_and(|t| !t.is_null())
        })
        || LOCAL_ORG_RE.is_match(&raw)
        || LOCAL_MACRO_RE.is_match(&raw)
        || SIT_GROUND_EXTRA_RE.is_match(&raw);

    let confidence = nlp.get("confidence").and_then(Value::as_f64).unwrap_or(0.0);

    let feed_lane = if is_unanchored_foreign {
        rejected_reasons.insert("foreign_scope");
        "rejected"
    } else if eligible {
        "incident_signals"
    } else if !hard_non_incident
        && !is_foreign_lang
        && !topics.is_empty()
        && !matches!(report_type, "denied" | "historical" | "educational")
        && !matches!(
            intent,
            "historical" | "educational" | "denial_or_debunk" | "unrelated" | "unknown"
        )
        && scope != "foreign"
        && has_id_grounding
        && has_sit_grounding
        && (has_permanent_topic || has_crisis_context)
        && (has_situational_intent
            || non_adhoc_findings.iter().any(|f| {
                flag(f, "feedOnly") && f.get("scope").and_then(Value::as_str) == Some("Indonesia")
            }))
    {
        "situational_context"
    } else if !hard_non_incident
        && !matches!(
            intent,
            "historical" | "educational" | "denial_or_debunk" | "unrelated"
        )
        && !matches!(report_type, "denied" | "historical" | "educational")
        && scope != "foreign"
        && (!topics.is_empty() || flag(nlp_flags, "anySatire"))
        && confidence < 0.62
    {
        rejected_reasons.insert("low_confidence");
        "discovery_candidates"
    } else {
        if rejected_reasons.is_empty() {
            if intent == "question" {
                rejected_reasons.insert("question_without_event_evidence");
            } else if topics.is_empty() {
                rejected_reasons.insert("unrelated_topic");
            } else {
                rejected_reasons.insert("no_concrete_event");
            }
        }
        "rejected"
    };

    let final_topics: Vec<String> = if feed_lane == "rejected" && hard_non_incident {
        Vec::new()
    } else {
        topics.items
    };
    let feed_eligible = matches!(feed_lane, "incident_signals" | "situational_context");

    let final_intent = if eligible
        && !matches!(
            intent,
            "incident_report" | "eyewitness_report" | "official_warning" | "official_update"
        ) {
        if report_type == "warning" {
            "official_warning"
        } else if flag(nlp_flags, "anyEyewitness") {
            "eyewitness_report"
        } else {
            "incident_report"
        }
    } else {
        intent
    };

    let norm_doc = nlp.get("normDoc").unwrap_or(&Value::Null);
    let out_topics: Vec<String> = if spam_flag {
        Vec::new()
    } else {
        final_topics.clone()
    };
    let out_rejected: Vec<String> = if feed_eligible {
        Vec::new()
    } else {
        rejected_reasons.items
    };
    let severity = if eligible
        && events
            .iter()
            .any(|f| f.get("severity").and_then(Value::as_str) == Some("warning"))
    {
        "warning"
    } else {
        "unknown"
    };

    let stripped_extracted: Vec<Value> = extracted_events
        .iter()
        .map(|ev| {
            let mut m = ev.as_object().cloned().unwrap_or_default();
            m.remove("clauseFlags");
            Value::Object(m)
        })
        .collect();

    let mut top3: Vec<(String, f64)> = nlp
        .get("intentScores")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| {
                    let n = v.as_f64()?;
                    (n > 0.0).then(|| (k.clone(), n))
                })
                .collect()
        })
        .unwrap_or_default();
    top3.sort_by(|a, b| b.1.total_cmp(&a.1));
    top3.truncate(3);
    let top3_val: Vec<Value> = top3
        .into_iter()
        .map(|(k, v)| json!([k, crate::js::number(v)]))
        .collect();

    json!({
        "version": VERSION,
        "topics": out_topics,
        "scope": scope,
        "reportType": report_type,
        "feedEligible": feed_eligible,
        "signalEligible": eligible,
        "events": events,
        "findings": findings,
        "reasons": reasons.items,
        "rejectedReasons": out_rejected,
        "severity": severity,
        "normalizedText": norm_doc.get("normalized").cloned().unwrap_or(Value::String(String::new())),
        "contentHash": norm_doc.get("contentHash").cloned().unwrap_or(Value::String(String::new())),
        "intent": final_intent,
        "feedLane": feed_lane,
        "confidence": nlp.get("confidence").cloned().unwrap_or(json!(0)),
        "classification": {
            "intent": final_intent,
            "topicMatch": !final_topics.is_empty(),
            "incidentEvidence": eligible || has_actionable,
            "feedLane": feed_lane,
            "confidence": nlp.get("confidence").cloned().unwrap_or(json!(0)),
        },
        "geographicContext": nlp.get("geographicContext").cloned().unwrap_or(Value::Null),
        "event": nlp.get("event").cloned().unwrap_or(Value::Null),
        "extractedEvents": stripped_extracted,
        "layerTrace": {
            "normalization": {
                "clauses": nlp_clauses.len(),
                "hashtagsSegmented": norm_doc.get("hashtagsSegmented").cloned().unwrap_or(json!([])),
                "reduplication": norm_doc.get("reduplication").cloned().unwrap_or(json!([])),
            },
            "intentScoresTop3": top3_val,
            "geoRole": geo_role,
            "negatedEvents": nlp.get("negatedEvents").cloned().unwrap_or(json!([])),
            "metaphors": nlp.get("metaphorHits").cloned().unwrap_or(json!([])),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_registry() -> Vec<TopicSpec> {
        vec![
            TopicSpec {
                id: "coretax".into(),
                pos: re(r"(?i)\bcore\s*tax\b|\bcoretax\b"),
                neg: None,
                ctx: None,
                strong: None,
                adhoc: false,
            },
            TopicSpec {
                id: "pajak".into(),
                pos: re(r"(?i)\b(?:pajak|djp|kpp)\b"),
                neg: None,
                ctx: None,
                strong: None,
                adhoc: false,
            },
            TopicSpec {
                id: "banjir".into(),
                pos: re(r"(?i)\b(?:banjir|longsor)\b"),
                neg: None,
                ctx: None,
                strong: None,
                adhoc: false,
            },
            TopicSpec {
                id: "gempa".into(),
                pos: re(r"(?i)\b(?:gempa|tsunami|erupsi)\b"),
                neg: None,
                ctx: None,
                strong: None,
                adhoc: false,
            },
            TopicSpec {
                id: "kebakaran".into(),
                pos: re(r"(?i)\b(?:kebakaran|terbakar)\b"),
                neg: None,
                ctx: None,
                strong: None,
                adhoc: false,
            },
        ]
    }

    #[test]
    fn assess_tax_coretax_outage() {
        let reg = sample_registry();
        let opts = AssessOptions {
            year: Some(2026.0),
            ..Default::default()
        };
        let res = assess_item(
            "Coretax DJP error dan down tidak bisa login hari ini di Jakarta",
            &reg,
            &opts,
        );
        assert_eq!(res["version"], VERSION);
        assert_eq!(res["scope"], "Indonesia");
        assert_eq!(res["signalEligible"], true);
        assert_eq!(res["feedLane"], "incident_signals");
        assert_eq!(res["events"][0]["type"], "coretax_outage");
    }

    #[test]
    fn assess_disaster_flood_eyewitness() {
        let reg = sample_registry();
        let opts = AssessOptions {
            year: Some(2026.0),
            ..Default::default()
        };
        let res = assess_item(
            "Barusan saya lihat sendiri banjir merendam ratusan rumah warga di Bekasi hari ini",
            &reg,
            &opts,
        );
        assert_eq!(res["scope"], "Indonesia");
        assert_eq!(res["signalEligible"], true);
        assert_eq!(res["feedLane"], "incident_signals");
        assert_eq!(res["events"][0]["topic"], "banjir");
    }

    #[test]
    fn assess_foreign_post_rejected() {
        let reg = sample_registry();
        let opts = AssessOptions {
            year: Some(2026.0),
            ..Default::default()
        };
        let res = assess_item(
            "Major earthquake hits Tokyo Japan today causing tsunami warning",
            &reg,
            &opts,
        );
        assert_eq!(res["signalEligible"], false);
        assert_eq!(res["feedLane"], "rejected");
        assert_eq!(res["scope"], "foreign");
    }

    #[test]
    fn assess_historical_post_not_signal() {
        let reg = sample_registry();
        let opts = AssessOptions {
            year: Some(2026.0),
            ..Default::default()
        };
        let res = assess_item(
            "Mengenang gempa besar yang mengguncang Aceh pada tahun 2004",
            &reg,
            &opts,
        );
        assert_eq!(res["signalEligible"], false);
        assert_eq!(res["reportType"], "historical");
    }

    #[test]
    fn assess_metaphor_rejected() {
        let reg = sample_registry();
        let opts = AssessOptions {
            year: Some(2026.0),
            ..Default::default()
        };
        let res = assess_item(
            "Banjir orderan dan banjir cuan di toko kami Jakarta",
            &reg,
            &opts,
        );
        assert_eq!(res["signalEligible"], false);
    }

    #[test]
    fn assess_warning_bmkg_eligible() {
        let reg = sample_registry();
        let opts = AssessOptions {
            year: Some(2026.0),
            ..Default::default()
        };
        let res = assess_item(
            "BMKG mengeluarkan peringatan dini tsunami di Padang Sumatera Barat",
            &reg,
            &opts,
        );
        assert_eq!(res["signalEligible"], true);
        assert_eq!(res["reportType"], "warning");
        assert_eq!(res["severity"], "warning");
    }
}
