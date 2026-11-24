use news_service_rs::port::service::{NEWS_FEEDS, NewsService};
use news_service_rs::port::{
    break_norm, clean_pub, is_banned_host_url, is_banned_news_item, wib_day_start_ms,
};
use serde_json::{Value, json};

#[test]
fn test_clean_pub() {
    assert_eq!(
        clean_pub(json!("  Kompas.com  ")).unwrap(),
        json!("Kompas.com")
    );
    assert_eq!(
        clean_pub(json!("https://www.detik.com/news/123")).unwrap(),
        json!("detik.com")
    );
    assert_eq!(
        clean_pub(json!("www.antaranews.com")).unwrap(),
        json!("antaranews.com")
    );
    assert_eq!(clean_pub(json!("")).unwrap(), json!(""));
    assert_eq!(clean_pub(Value::Null).unwrap(), json!(""));
    assert_eq!(clean_pub(json!(0)).unwrap(), json!(""));
}

#[test]
fn test_is_banned_host_url() {
    assert_eq!(
        is_banned_host_url(json!("https://infopublik.id/kategori/nusantara")).unwrap(),
        json!(true)
    );
    assert_eq!(
        is_banned_host_url(json!("https://koran.tempo.co/read/123")).unwrap(),
        json!(true)
    );
    assert_eq!(
        is_banned_host_url(json!("https://www.instagram.com/p/123")).unwrap(),
        json!(true)
    );
    assert_eq!(
        is_banned_host_url(json!("https://twitter.com/user/status/1")).unwrap(),
        json!(true)
    );
    assert_eq!(
        is_banned_host_url(json!("https://www.antaranews.com/berita/1")).unwrap(),
        json!(false)
    );
    assert_eq!(is_banned_host_url(json!("")).unwrap(), json!(false));
    assert_eq!(is_banned_host_url(Value::Null).unwrap(), json!(false));
}

#[test]
fn test_is_banned_news_item() {
    assert_eq!(
        is_banned_news_item(json!({ "pub": "instagram", "url": "https://example.com" })).unwrap(),
        json!(true)
    );
    assert_eq!(
        is_banned_news_item(
            json!({ "title": "Berita Hoax infopublik", "url": "https://example.com" })
        )
        .unwrap(),
        json!(true)
    );
    assert_eq!(
        is_banned_news_item(json!({
            "title": "Penerimaan Pajak Tumbuh",
            "url": "https://antaranews.com/1",
            "pub": "Antara"
        }))
        .unwrap(),
        json!(false)
    );
    assert_eq!(is_banned_news_item(Value::Null).unwrap(), json!(false));
}

#[test]
fn test_break_norm() {
    assert_eq!(
        break_norm(json!("Banjir Bandang Terjang Semarang - Detik News")).unwrap(),
        json!("banjir bandang terjang semarang")
    );
    assert_eq!(
        break_norm(json!("Coretax System Siap Meluncur! - CNBC Indonesia")).unwrap(),
        json!("coretax system siap meluncur")
    );
    assert_eq!(
        break_norm(json!("  Halo   Dunia   123  ")).unwrap(),
        json!("halo dunia 123")
    );
    assert_eq!(break_norm(json!(0)).unwrap(), json!("0"));
}

#[test]
fn test_wib_day_start_ms() {
    assert_eq!(
        wib_day_start_ms(json!(1700000000000_i64)).unwrap(),
        json!(1699981200000_i64)
    );
    assert_eq!(
        wib_day_start_ms(json!(1732433000000_i64)).unwrap(),
        json!(1732381200000_i64)
    );
    assert_eq!(wib_day_start_ms(Value::Null).unwrap(), json!(-25200000_i64));
    assert_eq!(wib_day_start_ms(json!([])).unwrap(), json!(-25200000_i64));
    assert!(
        wib_day_start_ms(json!({ "$bigint": "1" })).is_err(),
        "BigInt must fail"
    );
}

#[test]
fn test_news_feeds_roster() {
    assert!(NEWS_FEEDS.len() >= 67, "At least 67 news feeds required");
    let antara = NEWS_FEEDS.iter().find(|(id, _)| *id == "ANTARA");
    assert!(antara.is_some());
    let gnews_pajak = NEWS_FEEDS.iter().find(|(id, _)| *id == "GNEWS · PAJAK");
    assert!(gnews_pajak.is_some());
}

#[test]
fn test_parse_rss_and_atom() {
    let svc = NewsService::new();
    let sample_rss = r#"<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0">
      <channel>
        <title>Sample News</title>
        <item>
          <title><![CDATA[Banjir Bandang di Semarang]]></title>
          <link>https://news.example.com/banjir-semarang</link>
          <pubDate>Mon, 24 Nov 2026 07:00:00 +0000</pubDate>
          <description>Air merendam pemukiman warga</description>
          <source url="https://example.com">Example News</source>
          <enclosure url="https://example.com/img.jpg" type="image/jpeg" />
        </item>
        <item>
          <title>Artikel dari Instagram</title>
          <link>https://www.instagram.com/p/123</link>
          <pubDate>Mon, 24 Nov 2026 07:00:00 +0000</pubDate>
        </item>
      </channel>
    </rss>"#;

    let items = svc.parse_feed(sample_rss, "https://news.example.com/rss");
    assert_eq!(items.len(), 1, "Banned item should be excluded");
    assert_eq!(items[0]["title"], "Banjir Bandang di Semarang");
    assert_eq!(items[0]["rssImg"], "https://example.com/img.jpg");
    assert_eq!(items[0]["pub"], "Example News");

    let sample_atom = r#"<?xml version="1.0" encoding="utf-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
      <title>Atom Feed</title>
      <entry>
        <title>Gempa Bumi M 5.2 Guncang Malang</title>
        <link href="https://news.example.com/gempa-malang" />
        <updated>2026-11-24T07:00:00Z</updated>
        <summary>Gempa bumi terasa di Jawa Timur</summary>
      </entry>
    </feed>"#;

    let atom_items = svc.parse_feed(sample_atom, "https://news.example.com/atom");
    assert_eq!(atom_items.len(), 1);
    assert_eq!(atom_items[0]["title"], "Gempa Bumi M 5.2 Guncang Malang");
}

#[tokio::test]
async fn test_situation_and_incidents_methods() {
    let svc = NewsService::with_custom_feeds(vec![]);
    let sit_res = svc.situation().await;
    assert!(sit_res.is_ok());
    let sit = sit_res.unwrap();
    assert_eq!(sit["ok"], true);
    assert!(sit["stats"].is_object());
    assert!(sit["regions"].is_array());

    let inc_res = svc.incidents().await;
    assert!(inc_res.is_ok());
    let inc = inc_res.unwrap();
    assert_eq!(inc["ok"], true);
    assert!(inc["items"].is_array());
}

#[tokio::test]
async fn test_pajak_clusters_method() {
    let svc = NewsService::with_custom_feeds(vec![]);
    let cluster_res = svc.pajak_clusters(Some("24h"), None, None, None).await;
    assert!(cluster_res.is_ok());
    let cluster = cluster_res.unwrap();
    assert!(cluster.get("clusters").is_some());
}
