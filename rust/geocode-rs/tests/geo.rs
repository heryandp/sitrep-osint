use geocode_rs::port::service::{locate_text, nodes, stats};

type Gold = (
    &'static str,
    Option<&'static str>,
    Option<&'static [&'static str]>,
);
const GOLD: &[Gold] = &[
    (
        "BPBD: Semua Kecamatan di Kota Malang Rawan Terdampak Bencana",
        Some("Malang"),
        None,
    ),
    (
        "Atap Aula Kelurahan Limusnunggal Sukabumi Ambruk",
        Some("Kelurahan Limusnunggal, Kecamatan Cibeureum, Sukabumi, Jawa Barat"),
        None,
    ),
    (
        "Pemkab Barito Selatan Perpanjang Status Tanggap Darurat Karhutla",
        Some("Barito Selatan"),
        None,
    ),
    (
        "Marapi Erupsi Sabtu pagi, sebaran abu vulkanik terpantau di Agam",
        Some("Gunung Marapi"),
        None,
    ),
    (
        "Kebakaran Pasar di Kota Bandung Hanguskan 20 Kios",
        Some("Bandung"),
        None,
    ),
    (
        "Banjir Rendam Kabupaten Bandung, Ratusan Warga Mengungsi",
        Some("Kab. Bandung"),
        None,
    ),
    (
        "Aktivitas Erupsi Gunung Ile Lewotolok di NTT Masih Tinggi",
        Some("Gunung Lewotolok"),
        None,
    ),
    (
        "Demo Buruh di Kantor Gubernur Jawa Barat Tuntut UMP Naik",
        Some("Jawa Barat"),
        None,
    ),
    (
        "Gempa M5,2 Guncang Cilacap, Warga Panik",
        Some("Cilacap"),
        None,
    ),
    ("Kecelakaan Bus di Tol Cipali Tewaskan 5 Orang", None, None),
    ("Metro TV Gelar Diskusi Soal Ekonomi", None, None),
    ("Harga Batu Bara Naik di Pasar Global", None, None),
    ("Serangan Balik Pemain Persija Bikin Gol", None, None),
    (
        "Jenazah korban kecelakaan helikopter dipulangkan ke Jawa Tengah",
        None,
        None,
    ),
    (
        "Korban dilarikan ke RSUD Bandung usai kecelakaan di Cilegon",
        Some("Cilegon"),
        None,
    ),
    (
        "Gerak Cepat Pemerintah Tangani Banjir di Aceh dan Sumatra Utara",
        Some("Aceh"),
        Some(&["Aceh", "Sumatera Utara"]),
    ),
    (
        "Longsor di Aceh, Jawa Tengah dan Jawa Timur Telan Korban",
        Some("Aceh"),
        Some(&["Aceh", "Jawa Tengah", "Jawa Timur"]),
    ),
    (
        "Pria Asal Bogor Ditangkap di Cilegon karena Curanmor",
        Some("Cilegon"),
        None,
    ),
    ("Gempa di NTB", Some("Nusa Tenggara Barat"), None),
    ("Banjir di DIY", Some("DI Yogyakarta"), None),
    ("Gempa di DKI", Some("Jakarta"), None),
    ("gibran dimakzulkan di istana", Some("Istana Merdeka"), None),
    (
        "Massa Demonstran Segel Istana Wapres Gibran",
        Some("Istana Merdeka"),
        None,
    ),
    (
        "Demo di DPR Tuntut Batalkan Revisi UU",
        Some("DPR / MPR Senayan"),
        None,
    ),
    (
        "Aksi Buruh di Depan Gedung MK",
        Some("Mahkamah Konstitusi"),
        None,
    ),
    (
        "Kebakaran di Istana Maimun Medan",
        Some("Istana Maimun"),
        None,
    ),
    (
        "Demonstrasi di depan Istana Bogor",
        Some("Istana Bogor"),
        None,
    ),
];
const BIG_GOLD: &[(&str, &str, &str, f64, f64)] = &[
    (
        "Kebakaran di Kelurahan Gambir, Jakarta Pusat",
        "31.71.01.1001",
        "kelurahan",
        -6.176377,
        106.826707,
    ),
    (
        "Banjir di Desa Keude Bakongan, Kecamatan Bakongan, Kabupaten Aceh Selatan",
        "11.01.01.2001",
        "desa",
        2.93109,
        97.48459,
    ),
    (
        "Banjir di Desa Limusnunggal, Kecamatan Cileungsi, Kabupaten Bogor",
        "32.01.07.2011",
        "desa",
        -6.375136,
        106.96945,
    ),
    (
        "Banjir di Desa Limusnunggal, Kecamatan Bantargadung, Kabupaten Sukabumi",
        "32.02.04.2004",
        "desa",
        -6.956538,
        106.640443,
    ),
    (
        "Atap Aula Kelurahan Limusnunggal Sukabumi Ambruk",
        "32.72.07.1004",
        "kelurahan",
        -6.947022,
        106.941743,
    ),
    (
        "Banjir di Desa Sukamaju, Kecamatan Bantan, Kabupaten Bengkalis",
        "14.03.02.2015",
        "desa",
        1.479932,
        102.400612,
    ),
];

#[test]
fn gold() {
    for (t, exp, multi) in GOLD {
        let r = locate_text(t);
        let got = r.as_ref().and_then(|v| v["name"].as_str());
        assert_eq!(got, *exp, "{t}");
        if let Some(m) = multi {
            let all: Vec<&str> = r
                .as_ref()
                .and_then(|v| v["all"].as_array())
                .map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect())
                .unwrap_or_default();
            assert_eq!(all, m.to_vec(), "{t}");
        }
    }
}

#[test]
fn big_gold() {
    for (t, code, prec, la, lo) in BIG_GOLD {
        let r = locate_text(t).unwrap_or_default();
        assert_eq!(r["code"].as_str(), Some(*code), "{t}");
        assert_eq!(r["prec"].as_str(), Some(*prec), "{t}");
        assert!((r["la"].as_f64().unwrap_or(0.0) - la).abs() < 1e-9, "{t}");
        assert!((r["lo"].as_f64().unwrap_or(0.0) - lo).abs() < 1e-9, "{t}");
        assert_eq!(
            r["coordinateSource"].as_str(),
            Some("BIG administrative polygon centroid")
        );
    }
    assert!(locate_text("Banjir di Desa Limusnunggal").is_none());
    assert!(locate_text("Gempa di Jepang").is_none());
    assert!(locate_text("Gempa di London").is_none());
}

#[test]
fn nodes_and_stats() {
    assert_eq!(nodes().len(), 514);
    let s = stats();
    assert_eq!((s.villages, s.desa, s.kelurahan), (83529, 75034, 8495));
}
