use geo_sensors_rs::port::service::{GeoSensors, Ship, geo_centroid};
use geo_sensors_rs::port::{
    create_geo_sensors, csv_line, parse_cap_field, parse_csv, parse_magma, parse_polygon,
};
use serde_json::{Value, json};

#[test]
fn test_create_geo_sensors() {
    let res = create_geo_sensors(json!({})).unwrap();
    assert_eq!(
        res.get("flights").unwrap(),
        &json!({ "$function": "flights" })
    );
    assert_eq!(
        res.get("disaster").unwrap(),
        &json!({ "$function": "disaster" })
    );
    assert_eq!(res.get("shipMap").unwrap(), &json!({ "$map": [] }));
    assert!(create_geo_sensors(Value::Null).is_err());
}

#[test]
fn test_parse_magma() {
    let html = r#"
Level III (Siaga)
<table><tr><td> Merapi - Sleman, D.I. Yogyakarta <a href="/merapi">detail</a></td></tr>
<tr><td> Semeru - Lumajang, Jawa Timur <a href="/semeru">detail</a></td></tr></table>
Level II (Waspada)
<table><tr><td> Anak Krakatau - Lampung <a href="/krakatau">detail</a></td></tr></table>
Level I (Normal)
<table><tr><td> Sumbing - Magelang <a href="/sumbing">detail</a></td></tr></table>
"#;
    let res = parse_magma(json!(html)).unwrap();
    let arr = res.as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0]["name"], "Merapi");
    assert_eq!(arr[0]["level"], "LEVEL III");
    assert_eq!(arr[0]["status"], "Siaga");
    assert_eq!(arr[0]["lat"], -7.54);
    assert_eq!(arr[0]["lon"], 110.44);

    assert_eq!(arr[2]["name"], "Anak Krakatau");
    assert_eq!(arr[2]["level"], "LEVEL II");
    assert_eq!(arr[2]["status"], "Waspada");
    assert_eq!(arr[2]["lat"], -6.1);
    assert_eq!(arr[2]["lon"], 105.42);

    assert_eq!(arr[3]["name"], "Sumbing");
    assert_eq!(arr[3]["level"], "LEVEL I");
    assert_eq!(arr[3]["status"], "Normal");
    assert_eq!(arr[3]["lat"], -7.385);
    assert_eq!(arr[3]["lon"], 110.073);

    // Empty and no match
    assert_eq!(parse_magma(json!("")).unwrap(), json!([]));
    assert_eq!(parse_magma(json!("random text")).unwrap(), json!([]));

    // Errors
    assert!(parse_magma(Value::Null).is_err());
    assert!(parse_magma(json!(123)).is_err());
    assert!(parse_magma(json!(true)).is_err());
}

#[test]
fn test_csv_line() {
    assert_eq!(
        csv_line(json!("foo, bar , baz")).unwrap(),
        json!(["foo", "bar", "baz"])
    );
    assert_eq!(
        csv_line(json!("123, 456, 789, hello world")).unwrap(),
        json!(["123", "456", "789", "hello world"])
    );
    assert_eq!(csv_line(json!("single")).unwrap(), json!(["single"]));
    assert_eq!(csv_line(json!("")).unwrap(), json!([""]));

    assert!(csv_line(Value::Null).is_err());
    assert!(csv_line(json!(42)).is_err());
    assert!(csv_line(json!({})).is_err());
}

#[test]
fn test_parse_csv() {
    let csv = "latitude,longitude,acq_date,acq_time,satellite,instrument,confidence,frp,daynight\n\
               -6.2088,106.8456,2026-11-24,0345,Terra,MODIS,85,42.5,D\n\
               -7.2575,112.7521,2026-11-24,1420,Aqua,MODIS,90,55.1,N\n\
               invalid,100.0,2026-11-24,1200,Terra,MODIS,50,10.0,D\n";
    let res = parse_csv(json!(csv)).unwrap();
    let arr = res.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    assert_eq!(arr[0]["lat"], -6.2088);
    assert_eq!(arr[0]["lon"], 106.8456);
    assert_eq!(arr[0]["acqDate"], "2026-11-24");
    assert_eq!(arr[0]["acqTime"], "0345");
    assert_eq!(arr[0]["satellite"], "Terra");
    assert_eq!(arr[0]["instrument"], "MODIS");
    assert_eq!(arr[0]["confidence"], "85");
    assert_eq!(arr[0]["frp"], 42.5);
    assert_eq!(arr[0]["daynight"], "D");
    assert_eq!(arr[0]["time"], 1795491900000_i64);

    assert_eq!(arr[1]["lat"], -7.2575);
    assert_eq!(arr[1]["lon"], 112.7521);
    assert_eq!(arr[1]["daynight"], "N");
    assert_eq!(arr[1]["time"], 1795530000000_i64);

    // Missing header or short lines
    assert_eq!(
        parse_csv(json!("latitude,longitude\n1,2")).unwrap(),
        json!([])
    );
    assert_eq!(parse_csv(json!("")).unwrap(), json!([]));
    assert_eq!(parse_csv(json!("header only\n")).unwrap(), json!([]));

    assert!(parse_csv(Value::Null).is_err());
    assert!(parse_csv(json!(0)).is_err());
}

#[test]
fn test_parse_cap_field() {
    let xml = r#"<alert xmlns="urn:oasis:names:tc:emergency:cap:1.2">
  <identifier>BMKG-20261124-001</identifier>
  <headline><![CDATA[Peringatan Dini Cuaca Ekstrem Jawa Barat]]></headline>
  <description>Hujan deras disertai kilat/petir di wilayah Bogor dan Sukabumi</description>
  <area>
    <polygon>-6.5,106.8 -6.6,106.9 -6.7,106.7 -6.5,106.8</polygon>
  </area>
</alert>"#;

    assert_eq!(
        parse_cap_field(json!(xml), json!("headline")).unwrap(),
        json!("Peringatan Dini Cuaca Ekstrem Jawa Barat")
    );
    assert_eq!(
        parse_cap_field(json!(xml), json!("description")).unwrap(),
        json!("Hujan deras disertai kilat/petir di wilayah Bogor dan Sukabumi")
    );
    assert_eq!(
        parse_cap_field(json!(xml), json!("nonexistent")).unwrap(),
        json!("")
    );

    assert_eq!(parse_cap_field(json!("x"), json!("x")).unwrap(), json!(""));
    assert_eq!(parse_cap_field(json!("x"), json!({})).unwrap(), json!(""));

    assert!(parse_cap_field(Value::Null, json!("tag")).is_err());
    assert!(parse_cap_field(json!(123), json!("tag")).is_err());
}

#[test]
fn test_parse_polygon() {
    assert_eq!(
        parse_polygon(json!("-6.5,106.8 -6.6,106.9 -6.7,106.7")).unwrap(),
        json!([[-6.5, 106.8], [-6.6, 106.9], [-6.7, 106.7]])
    );
    assert_eq!(
        parse_polygon(json!("-6.5 106.8 -6.6 106.9")).unwrap(),
        json!([[-6.5, 106.8], [-6.6, 106.9]])
    );
    assert_eq!(parse_polygon(json!("")).unwrap(), json!([]));
    assert_eq!(parse_polygon(json!("invalid")).unwrap(), json!([]));

    assert!(parse_polygon(Value::Null).is_err());
    assert!(parse_polygon(json!(true)).is_err());
}

#[test]
fn test_geo_centroid() {
    let geom = json!({
        "type": "Polygon",
        "coordinates": [[[106.86, -6.24], [106.88, -6.24], [106.88, -6.22], [106.86, -6.22], [106.86, -6.24]]]
    });
    let (lat, lon) = geo_centroid(&geom);
    assert!((lat - (-6.23)).abs() < 1e-6);
    assert!((lon - 106.87).abs() < 1e-6);

    let empty = json!({});
    let (lat_nan, lon_nan) = geo_centroid(&empty);
    assert!(lat_nan.is_nan());
    assert!(lon_nan.is_nan());
}

#[tokio::test]
async fn test_geo_sensors_service() {
    let sensors = GeoSensors::new();
    assert!(!sensors.is_ais_live().await);
    sensors.stop_ais().await;
    assert!(!sensors.is_ais_live().await);

    // Add a ship to ship_map
    {
        let mut map = sensors.ship_map.write().await;
        map.insert(
            "123456789".to_string(),
            Ship {
                mmsi: json!("123456789"),
                name: "MV TEST".to_string(),
                dest: "JAKARTA".to_string(),
                ship_type: json!(70),
                lat: -6.1,
                lon: 106.8,
                sog: Some(12.5),
                hdg: Some(180),
                at: geo_sensors_rs::port::service::now_ms(),
            },
        );
    }

    let ships = sensors.ships().await;
    assert_eq!(ships.len(), 1);
    assert_eq!(ships[0].name, "MV TEST");
    assert_eq!(ships[0].sog, Some(12.5));
}
