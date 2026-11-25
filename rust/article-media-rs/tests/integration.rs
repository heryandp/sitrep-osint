#[test]
fn test_case_4() {
    let html = r#"<!DOCTYPE html><html><head>
    <title>Breaking News - Polisi Kantongi Calon Tersangka Pembunuhan Perempuan di Lombok Tengah - bsinews.id</title>
    <meta property="og:title" content="Breaking News - Polisi Kantongi Calon Tersangka Pembunuhan Perempuan di Lombok Tengah - bsinews.id">
    <meta property="og:site_name" content="Bsinews.id">
    <meta property="og:image" content="https://cdn.bsinews.id/media/images/2026/11/3VWhGUgKQg.jpeg?location=1&width=1200&height=674&quality=80&watermark=1">
  </head><body>
    <article>
      <h1>Breaking News - Polisi Kantongi Calon Tersangka Pembunuhan Perempuan di Lombok Tengah - bsinews.id</h1>
      <figure>
        <a class="spotlight" href="https://cdn.bsinews.id/media/images/2026/11/3VWhGUgKQg.jpeg?location=1&width=&height=&quality=90&fit=1" data-description="Kasat Reskrim Polresta Lombok Tengah. (Foto: Edi Suryansyah/detikBali)">
          <img src="https://cdn.bsinews.id/media/images/2026/11/3VWhGUgKQg.jpeg?location=1&width=&height=&quality=90&fit=1" alt="Foto Olah TKP" width="100%" decoding="async">
        </a>
        <figcaption>Kasat Reskrim Polresta Lombok Tengah. (Foto: Edi Suryansyah/detikBali)</figcaption>
      </figure>
      <p>Penyelidikan kematian perempuan berinisial M (57) yang ditemukan terbakar di Desa Selebung, Kecamatan Batukliang, Lombok Tengah, mulai mengerucut pada satu calon tersangka.</p>
      <p>Menurut kepolisian, penyidik telah mengumpulkan sejumlah barang bukti dan meminta keterangan saksi di lokasi kejadian perkara.</p>
    </article>
  </body></html>"#;

    let res = article_media_rs::port::extract_html_image_candidates(
        serde_json::Value::String(html.to_string()),
        serde_json::Value::String(
            "https://bsinews.id/polisi-kantongi-calon-tersangka-lombok".to_string(),
        ),
    )
    .unwrap();
    assert_eq!(res["bodyCands"][0]["source"], "body_lead");
    assert_eq!(res["bodyCands"][0]["pos"], 1);
}
