//! Async `NewsService` for DJP & OSINT news ingestion, categorization, geocoding, and clustering.

#![allow(
    dead_code,
    unused_imports,
    clippy::duplicate_mod,
    clippy::collapsible_if,
    clippy::manual_map,
    clippy::too_many_arguments
)]

#[path = "../../geocode-rs/src/service.rs"]
pub mod geocode_service;

#[path = "../../assessment-rs/src/service.rs"]
pub mod assessment_service;

#[path = "../../pajak-cluster-rs/src/service.rs"]
pub mod pajak_cluster_service;

#[path = "evmerge_service.rs"]
pub mod evmerge_service;

use crate::port::{
    break_norm, clean_pub, is_banned_host_url_internal, is_banned_news_item, js_to_string, pinned,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use tokio::sync::RwLock;
use url::Url;

pub const DEFAULT_GEO_WINDOW_MS: i64 = 48 * 3600 * 1000;
pub const INCIDENT_RETAIN_MS: i64 = 48 * 3600 * 1000;
pub const NEWS_FEED_CAP: usize = 200;

pub const NEWS_FEEDS: &[(&str, &str)] = &[
    ("ANTARA", "https://www.antaranews.com/rss/terkini.xml"),
    ("VOI", "https://voi.id/rss"),
    ("SINDO", "https://www.sindonews.com/rss"),
    ("CNN INDONESIA", "https://www.cnnindonesia.com/rss"),
    ("DETIK", "https://news.detik.com/rss"),
    ("TEMPO", "https://rss.tempo.co/"),
    ("LIPUTAN6", "https://feed.liputan6.com/rss/news"),
    ("REPUBLIKA", "https://www.republika.co.id/rss"),
    (
        "BBC INDONESIA",
        "https://feeds.bbci.co.uk/indonesia/rss.xml",
    ),
    ("DW INDONESIA", "https://rss.dw.com/rdf/rss-ind-all"),
    (
        "CNA INDONESIA",
        "https://www.cna.id/api/v1/rss-outbound-feed?_format=xml",
    ),
    ("TRIBUN", "https://www.tribunnews.com/rss"),
    ("CNBC ID", "https://www.cnbcindonesia.com/news/rss"),
    ("JPNN", "https://www.jpnn.com/rss"),
    ("DETIK JABAR", "https://www.detik.com/jabar/rss"),
    ("DETIK JATENG", "https://www.detik.com/jateng/rss"),
    ("DETIK JATIM", "https://www.detik.com/jatim/rss"),
    ("DETIK JOGJA", "https://www.detik.com/jogja/rss"),
    ("DETIK SUMUT", "https://www.detik.com/sumut/rss"),
    ("DETIK SULSEL", "https://www.detik.com/sulsel/rss"),
    ("DETIK BALI", "https://www.detik.com/bali/rss"),
    (
        "ANTARA JABAR",
        "https://jabar.antaranews.com/rss/terkini.xml",
    ),
    (
        "ANTARA JATENG",
        "https://jateng.antaranews.com/rss/terkini.xml",
    ),
    (
        "ANTARA JATIM",
        "https://jatim.antaranews.com/rss/terkini.xml",
    ),
    ("ANTARA BALI", "https://bali.antaranews.com/rss/terkini.xml"),
    (
        "ANTARA SUMUT",
        "https://sumut.antaranews.com/rss/terkini.xml",
    ),
    (
        "ANTARA MAKASSAR",
        "https://makassar.antaranews.com/rss/terkini.xml",
    ),
    (
        "ANTARA PAPUA",
        "https://papua.antaranews.com/rss/terkini.xml",
    ),
    (
        "ANTARA KALTIM",
        "https://kaltim.antaranews.com/rss/terkini.xml",
    ),
    ("ANTARA ACEH", "https://aceh.antaranews.com/rss/terkini.xml"),
    ("TRIBUN JABAR", "https://jabar.tribunnews.com/rss"),
    ("TRIBUN JATENG", "https://jateng.tribunnews.com/rss"),
    ("TRIBUN JATIM", "https://jatim.tribunnews.com/rss"),
    ("TRIBUN JOGJA", "https://jogja.tribunnews.com/rss"),
    ("TRIBUN MEDAN", "https://medan.tribunnews.com/rss"),
    ("TRIBUN MAKASSAR", "https://makassar.tribunnews.com/rss"),
    ("TRIBUN BALI", "https://bali.tribunnews.com/rss"),
    ("TRIBUN PAPUA", "https://papua.tribunnews.com/rss"),
    ("RRI", "https://rri.co.id/rss"),
    ("DETIK FINANCE", "https://finance.detik.com/rss"),
    (
        "ANTARA EKONOMI",
        "https://www.antaranews.com/rss/ekonomi.xml",
    ),
    (
        "ANTARA FINANSIAL",
        "https://www.antaranews.com/rss/ekonomi-finansial.xml",
    ),
    ("CNN EKONOMI", "https://www.cnnindonesia.com/ekonomi/rss"),
    ("CNBC MARKET", "https://www.cnbcindonesia.com/market/rss"),
    ("KONTAN NASIONAL", "https://nasional.kontan.co.id/rss"),
    ("KONTAN KEUANGAN", "https://keuangan.kontan.co.id/rss"),
    ("TEMPO BISNIS", "https://rss.tempo.co/bisnis"),
    ("SINDO EKBIS", "https://ekbis.sindonews.com/rss"),
    ("IDX CHANNEL", "https://www.idxchannel.com/rss"),
    (
        "GNEWS · PAJAK",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=pajak%20OR%20DJP%20OR%20%22Direktorat%20Jenderal%20Pajak%22%20OR%20%22lapor%20SPT%22%20OR%20%22faktur%20pajak%22%20OR%20%22penerimaan%20pajak%22%20OR%20%22restitusi%20pajak%22%20OR%20%22pemeriksaan%20pajak%22%20OR%20%22pidana%20pajak%22%20OR%20%22Kanwil%20DJP%22%20when%3A2d",
    ),
    (
        "GNEWS · CORETAX",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=Coretax%20OR%20%22Core%20Tax%22%20OR%20%22e-Faktur%22%20OR%20%22e-Bupot%22%20OR%20%22NPWP%2016%20digit%22%20OR%20%22gangguan%20Coretax%22%20OR%20%22error%20Coretax%22%20OR%20%22Kring%20Pajak%22%20when%3A2d",
    ),
    (
        "GNEWS · DDTC ORTAX",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=site%3Anews.ddtc.co.id%20OR%20site%3Aortax.org%20OR%20site%3Apajak.go.id%20when%3A2d",
    ),
    (
        "GNEWS · KEMENKEU PAJAK",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=site%3Akemenkeu.go.id%20OR%20site%3Ahukumonline.com%20pajak%20when%3A2d",
    ),
    (
        "GNEWS · PIDANA PAJAK",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=%22pidana%20pajak%22%20OR%20%22pengadilan%20pajak%22%20OR%20%22faktur%20fiktif%22%20OR%20%22sita%20aset%20pajak%22%20OR%20%22PPATK%20pajak%22%20OR%20%22gijzeling%22%20OR%20%22penipuan%20pajak%20DJP%22%20when%3A2d",
    ),
    (
        "GNEWS · FISKAL PAJAK",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=%22tax%20ratio%22%20OR%20%22UU%20HPP%22%20OR%20%22PMK%20pajak%22%20OR%20%22Komisi%20XI%20pajak%22%20OR%20%22insentif%20pajak%22%20OR%20%22Badan%20Penerimaan%20Negara%22%20OR%20%22tax%20amnesty%22%20when%3A2d",
    ),
    (
        "GNEWS · DEMO",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=demo%20OR%20%22unjuk%20rasa%22%20OR%20%22aksi%20massa%22%20when%3A2d",
    ),
    (
        "GNEWS · KEBAKARAN",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=kebakaran%20OR%20karhutla%20OR%20%22kebakaran%20hutan%22%20when%3A2d",
    ),
    (
        "GNEWS · BANJIR",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=banjir%20OR%20longsor%20OR%20%22tanah%20longsor%22%20OR%20%22banjir%20bandang%22%20OR%20%22cuaca%20ekstrem%22%20when%3A2d",
    ),
    (
        "GNEWS · BENCANA",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=gempa%20OR%20tsunami%20OR%20erupsi%20OR%20letusan%20OR%20%22gunung%20meletus%22%20OR%20%22puting%20beliung%22%20OR%20%22siklon%20tropis%22%20OR%20%22angin%20kencang%22%20OR%20badai%20OR%20%22hujan%20abu%22%20OR%20%22awan%20panas%22%20when%3A2d",
    ),
    (
        "GNEWS · INSIDEN",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=kecelakaan%20OR%20ledakan%20OR%20ambruk%20when%3A2d",
    ),
    (
        "GNEWS · KRIMINAL",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=penembakan%20OR%20pembunuhan%20OR%20penangkapan%20OR%20terorisme%20when%3A2d",
    ),
    (
        "GNEWS · KESEHATAN",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=wabah%20OR%20DBD%20OR%20%22demam%20berdarah%22%20OR%20campak%20OR%20rabies%20OR%20kolera%20OR%20mpox%20OR%20polio%20OR%20TBC%20OR%20KLB%20OR%20keracunan%20OR%20%22miras%20oplosan%22%20OR%20stunting%20when%3A2d",
    ),
    (
        "GNEWS · KERACUNAN",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=keracunan%20OR%20%22keracunan%20massal%22%20OR%20%22keracunan%20makanan%22%20OR%20%22keracunan%20MBG%22%20OR%20%22makanan%20basi%22%20OR%20%22miras%20oplosan%22%20OR%20intoksikasi%20when%3A2d",
    ),
    (
        "GNEWS · EKONOMI",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=phk%20OR%20rupiah%20OR%20bbm%20OR%20%22listrik%20padam%22%20when%3A2d",
    ),
    (
        "GNEWS · POLITIK",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=pemilu%20OR%20pilkada%20OR%20pilpres%20OR%20kpu%20OR%20perang%20OR%20konflik%20OR%20%22gencatan%20senjata%22%20OR%20ultimatum%20OR%20kudeta%20OR%20invasi%20OR%20%22partai%20politik%22%20OR%20DPR%20when%3A2d",
    ),
    (
        "GNEWS · SIBER",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=ransomware%20OR%20%22serangan%20siber%22%20OR%20phishing%20OR%20%22bocor%20data%22%20OR%20BSSN%20OR%20%22akun%20dibajak%22%20OR%20%22dompet%20kripto%22%20OR%20exploit%20OR%20%22zero%20day%22%20OR%20deepfake%20OR%20%22data%20breach%22%20OR%20%22peretasan%22%20when%3A2d",
    ),
    (
        "GNEWS · LINGKUNGAN",
        "https://news.google.com/rss/search?hl=id&gl=ID&ceid=ID:id&q=karhutla%20OR%20deforestasi%20OR%20%22sungai%20tercemar%22%20OR%20%22sampah%20plastik%22%20OR%20longsor%20OR%20%22tambang%20ilegal%22%20when%3A2d",
    ),
];

static SKIP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:pascabencana|mitigasi|edukasi|sosialisasi|pelatihan|olahraga|timnas|sepak bola|badminton|film|sinema|trailer|kpop|drakor|anime|amerika serikat|filipina|thailand|malaysia|korea|jepang|tiongkok|china|inggris|prancis|jerman|australia|india|pakistan|iran|israel|palestina|ukraina|rusia|gaza)").expect("valid skip regex")
});

pub struct NewsService {
    client: reqwest::Client,
    news_extra: Vec<(String, String)>,
    override_feeds: Option<Vec<(String, String)>>,
    cache: RwLock<HashMap<String, (i64, Value)>>,
    history: RwLock<HashMap<String, Vec<Value>>>,
    incident_hist: RwLock<HashMap<String, Value>>,
}

impl Default for NewsService {
    fn default() -> Self {
        Self::new()
    }
}

impl NewsService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self {
            client,
            news_extra: Vec::new(),
            override_feeds: None,
            cache: RwLock::new(HashMap::new()),
            history: RwLock::new(HashMap::new()),
            incident_hist: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_feeds(extra: Vec<(String, String)>) -> Self {
        let mut s = Self::new();
        s.news_extra = extra;
        s
    }

    pub fn with_custom_feeds(feeds: Vec<(String, String)>) -> Self {
        let mut s = Self::new();
        s.override_feeds = Some(feeds);
        s
    }

    pub fn all_feeds(&self) -> Vec<(String, String)> {
        if let Some(ref custom) = self.override_feeds {
            return custom.clone();
        }
        let mut res: Vec<(String, String)> = NEWS_FEEDS
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        res.extend(self.news_extra.clone());
        res
    }

    pub fn decode_xml_text(s: &str) -> String {
        let mut text = s.to_string();
        if let Some(start) = text.find("<![CDATA[") {
            if let Some(end) = text[start..].find("]]>") {
                text = format!(
                    "{}{}{}",
                    &text[..start],
                    &text[start + 9..start + end],
                    &text[start + end + 3..]
                );
            }
        }
        text = text
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&nbsp;", " ")
            .replace("&amp;", "&");

        static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").expect("tag re"));
        TAG_RE.replace_all(&text, "").trim().to_string()
    }

    pub fn extract_tag(block: &str, tag_name: &str) -> String {
        let pattern = format!(r"(?i)<{tag_name}[^>]*>([\s\S]*?)</{tag_name}>");
        if let Ok(re) = Regex::new(&pattern) {
            if let Some(cap) = re.captures(block) {
                if let Some(m) = cap.get(1) {
                    return Self::decode_xml_text(m.as_str());
                }
            }
        }
        String::new()
    }

    pub fn extract_tag_raw(block: &str, tag_name: &str) -> String {
        let pattern = format!(r"(?i)<{tag_name}[^>]*>([\s\S]*?)</{tag_name}>");
        if let Ok(re) = Regex::new(&pattern) {
            if let Some(cap) = re.captures(block) {
                if let Some(m) = cap.get(1) {
                    return m.as_str().to_string();
                }
            }
        }
        String::new()
    }

    pub fn parse_feed(&self, xml: &str, feed_url: &str) -> Vec<Value> {
        let mut out = Vec::new();
        static ITEM_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)<item[\s>]([\s\S]*?)</item>").expect("item re"));
        static ENTRY_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)<entry[\s>]([\s\S]*?)</entry>").expect("entry re"));
        static LINK_HREF_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"(?i)<link[^>]*href="([^"]+)""#).expect("link re"));
        static MEDIA_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"(?i)<(?:enclosure|media:content|media:thumbnail)[^>]*url="([^"]+)""#)
                .expect("media url")
        });
        static IMG_SRC_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"(?i)<img[^>]*src="([^"]+)""#).expect("img src"));

        for cap in ITEM_RE.captures_iter(xml) {
            let b = &cap[1];
            let title = Self::extract_tag(b, "title");
            let mut raw_link = Self::extract_tag(b, "link");
            if raw_link.is_empty() {
                raw_link = Self::extract_tag(b, "guid");
            }
            let url = if let Ok(base) = Url::parse(feed_url) {
                base.join(&raw_link)
                    .map_or(raw_link.clone(), |u| u.to_string())
            } else {
                raw_link.clone()
            };
            let mut pub_name = Self::extract_tag(b, "source");
            if let Ok(Value::String(cp)) = clean_pub(Value::String(pub_name.clone())) {
                pub_name = cp;
            }
            let is_banned = is_banned_news_item(json!({
                "title": title,
                "url": url,
                "pub": pub_name,
            }))
            .unwrap_or(Value::Bool(false))
                == Value::Bool(true);

            if !title.is_empty() && !url.is_empty() && !is_banned {
                let d_str = Self::extract_tag(b, "pubDate");
                let ts = chrono::DateTime::parse_from_rfc2822(&d_str)
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339(&d_str)
                            .map(|dt| dt.timestamp_millis())
                            .unwrap_or(0)
                    });
                let desc = Self::extract_tag(b, "description");
                let mut rss_imgs = Vec::new();
                if let Some(c) = MEDIA_URL_RE.captures(b) {
                    if let Some(m) = c.get(1) {
                        rss_imgs.push(m.as_str().to_string());
                    }
                }
                if rss_imgs.is_empty() {
                    if let Some(c) = IMG_SRC_RE.captures(b) {
                        if let Some(m) = c.get(1) {
                            rss_imgs.push(m.as_str().to_string());
                        }
                    }
                }
                let rss_img = rss_imgs.first().cloned().unwrap_or_default();
                out.push(json!({
                    "title": title,
                    "url": url,
                    "ts": ts,
                    "img": "",
                    "rssImg": rss_img,
                    "rssImgs": rss_imgs,
                    "pub": pub_name,
                    "desc": desc,
                }));
            }
        }

        for cap in ENTRY_RE.captures_iter(xml) {
            let b = &cap[1];
            let title = Self::extract_tag(b, "title");
            let raw_link = LINK_HREF_RE
                .captures(b)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_else(|| Self::extract_tag(b, "link"));
            let url = if let Ok(base) = Url::parse(feed_url) {
                base.join(&raw_link)
                    .map_or(raw_link.clone(), |u| u.to_string())
            } else {
                raw_link.clone()
            };
            let is_banned = is_banned_news_item(json!({ "title": title, "url": url }))
                .unwrap_or(Value::Bool(false))
                == Value::Bool(true);

            if !title.is_empty() && !url.is_empty() && !is_banned {
                let d_str = Self::extract_tag(b, "updated");
                let ts = chrono::DateTime::parse_from_rfc3339(&d_str)
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or(0);
                let desc = Self::extract_tag(b, "summary");
                out.push(json!({
                    "title": title,
                    "url": url,
                    "ts": ts,
                    "img": "",
                    "rssImg": "",
                    "rssImgs": [],
                    "pub": "",
                    "desc": desc,
                }));
            }
        }
        out
    }

    pub async fn fetch_feed(&self, id: &str, url: &str) -> Value {
        match self.client.get(url).send().await {
            Ok(resp) => {
                let ok = resp.status().is_success();
                if ok {
                    let xml = resp.text().await.unwrap_or_default();
                    let items = self.parse_feed(&xml, url);
                    let mut tagged = Vec::with_capacity(items.len());
                    for mut it in items {
                        if let Some(obj) = it.as_object_mut() {
                            obj.insert("src".to_string(), Value::String(id.to_string()));
                        }
                        tagged.push(it);
                    }
                    json!({ "id": id, "ok": true, "stale": false, "items": tagged })
                } else {
                    json!({ "id": id, "ok": false, "err": format!("HTTP {}", resp.status()), "items": [] })
                }
            }
            Err(e) => {
                json!({ "id": id, "ok": false, "err": e.to_string(), "items": [] })
            }
        }
    }

    pub fn merge_news_hist(&self, feed_id: &str, items: &[Value], _now: i64) {
        let mut hist = self.history.blocking_write();
        let list = hist.entry(feed_id.to_string()).or_default();
        let mut existing_urls: HashSet<String> = list
            .iter()
            .filter_map(|x| {
                x.get("url")
                    .and_then(|u| u.as_str())
                    .map(ToString::to_string)
            })
            .collect();
        for it in items {
            if let Some(u) = it.get("url").and_then(|u| u.as_str()) {
                if !existing_urls.contains(u) {
                    existing_urls.insert(u.to_string());
                    list.push(it.clone());
                }
            }
        }
        if list.len() > NEWS_FEED_CAP {
            list.sort_by_key(|b| b.get("ts").and_then(|t| t.as_i64()).unwrap_or(0));
            list.reverse();
            list.truncate(NEWS_FEED_CAP);
        }
    }

    pub async fn news(&self) -> Result<Value, String> {
        let feeds = self.all_feeds();
        let mut sources = Vec::with_capacity(feeds.len());
        for (id, url) in feeds {
            let res = self.fetch_feed(&id, &url).await;
            sources.push(res);
        }
        let now = pinned::now_ms() as i64;
        Ok(json!({
            "ok": true,
            "sources": sources,
            "at": now,
        }))
    }

    pub fn assess_category(&self, title: &str, desc: &str) -> (String, f64, bool) {
        let text = format!("{title} {desc}");
        let assess_res = assessment_service::assess_item(
            &text,
            &[],
            &assessment_service::AssessOptions::default(),
        );
        if let Some(topics) = assess_res.get("topics").and_then(|v| v.as_array()) {
            if let Some(first_topic) = topics.first().and_then(|v| v.as_str()) {
                if let Some(cat) = assessment_service::category_of(first_topic) {
                    return match cat {
                        "pajak" => ("pajak".to_string(), 0.78, true),
                        "demo" => ("demo".to_string(), 0.82, true),
                        "fire" => ("fire".to_string(), 0.96, true),
                        "flood" => ("flood".to_string(), 0.86, true),
                        "disaster" => ("disaster".to_string(), 1.0, true),
                        "insiden" => ("insiden".to_string(), 0.62, true),
                        "kriminal" => ("kriminal".to_string(), 0.78, true),
                        "kesehatan" => ("kesehatan".to_string(), 0.65, true),
                        "siber" => ("siber".to_string(), 0.55, false),
                        "ekonomi" => ("ekonomi".to_string(), 0.50, false),
                        "politik" => ("politik".to_string(), 0.55, false),
                        _ => ("insiden".to_string(), 0.50, true),
                    };
                }
            }
        }
        let lower = text.to_lowercase();
        if lower.contains("pajak") || lower.contains("djp") || lower.contains("coretax") {
            ("pajak".to_string(), 0.78, true)
        } else if lower.contains("kebakaran") || lower.contains("karhutla") {
            ("fire".to_string(), 0.96, true)
        } else if lower.contains("banjir") || lower.contains("longsor") {
            ("flood".to_string(), 0.86, true)
        } else if lower.contains("gempa") || lower.contains("tsunami") || lower.contains("erupsi") {
            ("disaster".to_string(), 1.0, true)
        } else if lower.contains("demo") || lower.contains("unjuk rasa") {
            ("demo".to_string(), 0.82, true)
        } else if lower.contains("kecelakaan") || lower.contains("tabrakan") {
            ("insiden".to_string(), 0.62, true)
        } else if lower.contains("penembakan")
            || lower.contains("pembunuhan")
            || lower.contains("kriminal")
        {
            ("kriminal".to_string(), 0.78, true)
        } else if lower.contains("keracunan") || lower.contains("wabah") || lower.contains("dbd") {
            ("kesehatan".to_string(), 0.65, true)
        } else {
            ("insiden".to_string(), 0.50, true)
        }
    }

    pub async fn breaking(&self) -> Result<Value, String> {
        let news_val = self.news().await?;
        let now = pinned::now_ms() as i64;
        let mut seen = HashSet::new();
        let mut items = Vec::new();

        if let Some(srcs) = news_val.get("sources").and_then(|v| v.as_array()) {
            for s in srcs {
                if let Some(raw_items) = s.get("items").and_then(|v| v.as_array()) {
                    for it in raw_items {
                        let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let desc = it.get("desc").and_then(|v| v.as_str()).unwrap_or("");
                        if SKIP_RE.is_match(title) || SKIP_RE.is_match(desc) {
                            continue;
                        }
                        let norm = break_norm(Value::String(title.to_string()))
                            .map(|v| v.as_str().unwrap_or("").to_string())
                            .unwrap_or_default();
                        if norm.is_empty() || seen.contains(&norm) {
                            continue;
                        }
                        seen.insert(norm);

                        let (cat, sev, geo) = self.assess_category(title, desc);
                        let ts = it.get("ts").and_then(|v| v.as_i64()).unwrap_or(now);
                        let age_hours = ((now - ts).max(0) as f64) / 3_600_000.0;
                        let recency = (-age_hours / 20.0).exp().max(0.16);
                        let weight = sev * recency;

                        let mut item_obj = it.clone();
                        if let Some(obj) = item_obj.as_object_mut() {
                            obj.insert("cat".to_string(), Value::String(cat));
                            obj.insert("weight".to_string(), json!(weight));
                            obj.insert("geo".to_string(), Value::Bool(geo));
                        }
                        items.push(item_obj);
                    }
                }
            }
        }

        items.sort_by(|a, b| {
            let wa = a.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let wb = b.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
            wb.partial_cmp(&wa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = items.len();
        Ok(json!({
            "ok": true,
            "items": items,
            "total": total,
            "at": now,
        }))
    }

    pub async fn incidents(&self) -> Result<Value, String> {
        let news_val = self.news().await?;
        let now = pinned::now_ms() as i64;
        let mut out = Vec::new();

        if let Some(srcs) = news_val.get("sources").and_then(|v| v.as_array()) {
            for s in srcs {
                if let Some(raw_items) = s.get("items").and_then(|v| v.as_array()) {
                    for it in raw_items {
                        let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let desc = it.get("desc").and_then(|v| v.as_str()).unwrap_or("");
                        let (cat, sev, geo) = self.assess_category(title, desc);
                        if !geo {
                            continue;
                        }
                        if let Some(loc) = geocode_service::locate_text(title) {
                            let ts = it.get("ts").and_then(|v| v.as_i64()).unwrap_or(now);
                            let age_ms = (now - ts).max(0) as f64;
                            let recency = (-age_ms / (20.0 * 3_600_000.0)).exp().max(0.16);
                            let weight = (sev * recency * 1000.0).round() / 1000.0;

                            let mut pin = loc;
                            if let Some(obj) = pin.as_object_mut() {
                                obj.insert("kind".to_string(), Value::String(cat));
                                obj.insert("title".to_string(), Value::String(title.to_string()));
                                obj.insert("ts".to_string(), json!(ts));
                                obj.insert("weight".to_string(), json!(weight));
                                if let Some(u) = it.get("url") {
                                    obj.insert("url".to_string(), u.clone());
                                }
                                if let Some(src) = it.get("src") {
                                    obj.insert("src".to_string(), src.clone());
                                }
                            }
                            out.push(pin);
                        }
                    }
                }
            }
        }

        let prev_pins = {
            let hist = self.incident_hist.read().await;
            Value::Array(hist.values().cloned().collect())
        };

        let merged_val = evmerge_service::merge_events(
            Value::Array(out),
            prev_pins,
            json!({ "now": now, "retainPrevMs": INCIDENT_RETAIN_MS }),
        )?;

        let merged_items = merged_val.as_array().cloned().unwrap_or_default();
        {
            let mut hist = self.incident_hist.write().await;
            for p in &merged_items {
                if let Some(eid) = p.get("eid").and_then(|v| v.as_str()) {
                    hist.insert(eid.to_string(), p.clone());
                }
            }
        }

        let total = merged_items.len();
        Ok(json!({
            "ok": true,
            "items": merged_items,
            "total": total,
            "at": now,
        }))
    }

    pub async fn situation(&self) -> Result<Value, String> {
        let (news_res, inc_res) = tokio::join!(self.news(), self.incidents());
        let news_val = news_res?;
        let inc_val = inc_res?;
        let now = pinned::now_ms() as i64;

        let mut cat_stats: HashMap<String, usize> = HashMap::new();
        for cat in &[
            "pajak",
            "demo",
            "fire",
            "flood",
            "disaster",
            "insiden",
            "siber",
            "kriminal",
            "kesehatan",
            "ekonomi",
            "politik",
        ] {
            cat_stats.insert(cat.to_string(), 0);
        }

        let mut regions: HashMap<String, usize> = HashMap::new();
        let mut sit_items = Vec::new();

        if let Some(srcs) = news_val.get("sources").and_then(|v| v.as_array()) {
            for s in srcs {
                if let Some(raw_items) = s.get("items").and_then(|v| v.as_array()) {
                    for it in raw_items {
                        let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let desc = it.get("desc").and_then(|v| v.as_str()).unwrap_or("");
                        let (cat, _, _) = self.assess_category(title, desc);
                        *cat_stats.entry(cat).or_insert(0) += 1;
                        sit_items.push(it.clone());
                    }
                }
            }
        }

        let inc_items = inc_val.get("items").and_then(|v| v.as_array());
        let inc_count = inc_val.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        if let Some(arr) = inc_items {
            for p in arr {
                if let Some(prov) = p.get("prov").and_then(|v| v.as_str()) {
                    *regions.entry(prov.to_string()).or_insert(0) += 1;
                }
            }
        }

        let mut region_rows: Vec<Value> = regions
            .into_iter()
            .map(|(name, n)| json!({ "name": name, "n": n }))
            .collect();
        region_rows
            .sort_by_key(|r| std::cmp::Reverse(r.get("n").and_then(|v| v.as_u64()).unwrap_or(0)));

        Ok(json!({
            "ok": true,
            "stats": cat_stats,
            "daily": {
                "total24h": inc_count,
            },
            "regions": region_rows,
            "items": sit_items,
            "located": inc_count,
            "at": now,
        }))
    }

    pub async fn pajak_clusters(
        &self,
        win: Option<&str>,
        theme: Option<&str>,
        scope: Option<&str>,
        social_posts: Option<&[Value]>,
    ) -> Result<Value, String> {
        let news_val = self.news().await?;
        let now = pinned::now_ms() as i64;
        let mut news_items = Vec::new();

        if let Some(srcs) = news_val.get("sources").and_then(|v| v.as_array()) {
            for s in srcs {
                if let Some(items) = s.get("items").and_then(|v| v.as_array()) {
                    for it in items {
                        let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let desc = it.get("desc").and_then(|v| v.as_str()).unwrap_or("");
                        let (cat, _, _) = self.assess_category(title, desc);
                        if cat == "pajak" || pajak_cluster_service::is_tax_text_str(title) {
                            news_items.push(it.clone());
                        }
                    }
                }
            }
        }

        let posts: &[Value] = social_posts.unwrap_or(&[]);
        let payload = pajak_cluster_service::build_pajak_clusters_value(
            &news_items,
            posts,
            now as f64,
            win.unwrap_or("24h"),
            theme.unwrap_or(""),
            scope.unwrap_or(""),
        );
        Ok(payload)
    }
}
