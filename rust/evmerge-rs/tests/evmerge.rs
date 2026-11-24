use evmerge_rs::js::undefined;
use evmerge_rs::port::{break_norm, ev_hash, ev_rare, hazard_of, merge_events, rep_score};
use serde_json::{Value, json};

const NOW: i64 = 1_800_000_000_000;

fn pin(title: &str, name: &str, kind: &str, ts: i64) -> Value {
    json!({ "title": title, "name": name, "kind": kind, "ts": ts })
}

#[test]
fn single_shared_token_does_not_merge() {
    let out = merge_events(
        json!([
            pin(
                "Kebakaran rumah sariwati melati di Bandung",
                "Bandung",
                "fire",
                NOW - 3_600_000
            ),
            pin(
                "Kebakaran ruko sariwati jebol di Bandung",
                "Bandung",
                "fire",
                NOW - 3_600_000
            ),
        ]),
        json!([]),
        json!({}),
    )
    .unwrap();
    assert_eq!(out.as_array().unwrap().len(), 2);
}

#[test]
fn two_shared_rare_tokens_merge_into_one_pin() {
    let out = merge_events(
        json!([
            pin(
                "Kebakaran rumah sariwati melati di Bandung",
                "Bandung",
                "fire",
                NOW - 3_600_000
            ),
            pin(
                "Kebakaran rumah sariwati melati jebol di Bandung",
                "Bandung",
                "fire",
                NOW - 3_600_000
            ),
        ]),
        json!([]),
        json!({}),
    )
    .unwrap();
    let arr = out.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["n"], json!(2));
}

#[test]
fn transitive_chain_does_not_merge_a_and_c() {
    let out = merge_events(
        json!([
            pin(
                "Kebakaran rumah senja anggrek di Bandung",
                "Bandung",
                "fire",
                NOW - 3_600_000
            ),
            pin(
                "Kebakaran rumah senja anggrek melati di Bandung",
                "Bandung",
                "fire",
                NOW - 3_600_000
            ),
            pin(
                "Kebakaran rumah melati cempaka di Bandung",
                "Bandung",
                "fire",
                NOW - 3_600_000
            ),
        ]),
        json!([]),
        json!({}),
    )
    .unwrap();
    let arr = out.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert!(!arr.iter().any(|p| p.get("n") == Some(&json!(3))));
}

#[test]
fn empty_rare_u_incidents_do_not_collide_on_eid() {
    let out = merge_events(
        json!([
            pin(
                "Kebakaran Rumah di Jakarta Selatan",
                "Jakarta Selatan",
                "fire",
                NOW - 1_800_000
            ),
            pin(
                "Kebakaran Pasar di Jakarta Selatan",
                "Jakarta Selatan",
                "fire",
                NOW - 900_000
            ),
        ]),
        json!([]),
        json!({}),
    )
    .unwrap();
    let arr = out.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_ne!(arr[0]["eid"], arr[1]["eid"]);
}

#[test]
fn generic_words_do_not_boost_rare_overlap() {
    let out = merge_events(
        json!([
            pin(
                "Kebakaran rumah sariwati mangga malam kemarin di Jakarta Selatan",
                "Jakarta Selatan",
                "fire",
                NOW - 3_600_000
            ),
            pin(
                "Kebakaran ruko sariwati kenanga siang barusan di Jakarta Selatan",
                "Jakarta Selatan",
                "fire",
                NOW - 3_600_000
            ),
        ]),
        json!([]),
        json!({}),
    )
    .unwrap();
    assert_eq!(out.as_array().unwrap().len(), 2);
}

#[test]
fn permutation_invariant_eid_and_representative() {
    let a = json!({
        "title": "Kebakaran gudang tekstil cipondoh merambat ke tiga ruko",
        "name": "Tangerang",
        "kind": "fire",
        "ts": NOW - 2_000_000,
        "src": "ANTARA"
    });
    let b = json!({
        "title": "Gudang tekstil cipondoh terbakar hebat malam ini",
        "name": "Tangerang",
        "kind": "fire",
        "ts": NOW - 1_000_000,
        "src": "DETIK"
    });
    let r1 = merge_events(json!([a.clone(), b.clone()]), json!([]), json!({})).unwrap();
    let r2 = merge_events(json!([b, a]), json!([]), json!({})).unwrap();
    assert_eq!(r1.as_array().unwrap().len(), 1);
    assert_eq!(r2.as_array().unwrap().len(), 1);
    assert_eq!(r1[0]["eid"], r2[0]["eid"]);
    assert_eq!(r1[0]["title"], r2[0]["title"]);
}

#[test]
fn operational_focus_metadata_never_alters_identity() {
    let base_a = json!({
        "title": "Ledakan kilang pertamina cilamaya karawang terbakar",
        "name": "Karawang",
        "kind": "fire",
        "ts": NOW - 3_600_000,
        "operationalFocus": "evakuasi",
        "recommendedAction": "dispatch_damkar"
    });
    let base_b = json!({
        "title": "Kebakaran kilang pertamina cilamaya karawang malam ini",
        "name": "Karawang",
        "kind": "fire",
        "ts": NOW - 3_600_000,
        "operationalFocus": "lalu_lintas",
        "recommendedAction": "rekayasa_jalan"
    });
    let with_focus = merge_events(
        json!([base_a.clone(), base_b.clone()]),
        json!([]),
        json!({}),
    )
    .unwrap();
    let mut a2 = base_a;
    let mut b2 = base_b;
    a2["operationalFocus"] = undefined();
    b2["operationalFocus"] = undefined();
    let without_focus = merge_events(json!([a2, b2]), json!([]), json!({})).unwrap();
    assert_eq!(with_focus.as_array().unwrap().len(), 1);
    assert_eq!(with_focus[0]["eid"], without_focus[0]["eid"]);
}

#[test]
fn prev_pins_preserves_eid_across_refreshes() {
    let first = merge_events(
        json!([pin(
            "Gempa magnitudo 5.4 guncang Cianjur selatan",
            "Cianjur",
            "disaster",
            NOW - 3_600_000
        )]),
        json!([]),
        json!({}),
    )
    .unwrap();
    let second = merge_events(
        json!([pin(
            "Update gempa magnitudo 5.4 Cianjur selatan warga mengungsi",
            "Cianjur",
            "disaster",
            NOW - 600_000
        )]),
        first.clone(),
        json!({}),
    )
    .unwrap();
    assert_eq!(second.as_array().unwrap().len(), 1);
    assert_eq!(second[0]["eid"], first[0]["eid"]);
}

#[test]
fn retain_prev_ms_and_helpers() {
    let carried = merge_events(
        json!([]),
        json!([
            { "eid": "e_keep", "title": "Ledakan pipa gas Bekasi", "name": "Bekasi", "kind": "fire", "ts": NOW - 1_000_000 },
            { "eid": "e_old", "title": "Old incident", "name": "Bekasi", "kind": "fire", "ts": NOW - 50_000_000 },
            { "eid": "e_node", "title": "Node", "name": "Bekasi", "kind": "node", "ts": NOW - 1_000_000 }
        ]),
        json!({ "now": NOW, "retainPrevMs": 14_400_000 }),
    )
    .unwrap();
    let arr = carried.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["eid"], json!("e_keep"));
    assert_eq!(arr[0]["carried"], json!(true));
    assert_eq!(
        break_norm(json!("Banjir Jakarta - Kompas.com")).unwrap(),
        json!("banjir jakarta")
    );
    assert_eq!(hazard_of(json!("Erupsi Merapi")).unwrap(), json!("erupsi"));
    assert_eq!(
        rep_score(json!({ "title": "Kebakaran gudang" })).unwrap(),
        json!(2)
    );
    assert_eq!(ev_hash(json!("x")).unwrap(), json!("3t3x"));
    assert_eq!(
        ev_rare("Kebakaran rumah sariwati melati di Bandung", "bandung"),
        vec!["sariwati", "melati"]
    );
}
