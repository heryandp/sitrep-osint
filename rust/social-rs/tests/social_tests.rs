use serde_json::json;
use social_rs::port::{
    astroturf_sig, explain_post, soc_adhoc, soc_assess, soc_balance, soc_foreign_lang,
    soc_hash_junk, soc_hot, soc_keep_th, soc_keep_x, soc_keyword_mine, soc_non_latin, soc_queries,
    soc_topics, soc_vi,
};

#[test]
fn test_soc_non_latin() {
    assert_eq!(
        soc_non_latin(json!("kebakaran kios")).unwrap(),
        json!(false)
    );
    assert_eq!(
        soc_non_latin(json!("これは日本語のテストです")).unwrap(),
        json!(true)
    );
    assert_eq!(
        soc_non_latin(json!("한국어 테스트입니다")).unwrap(),
        json!(true)
    );
    assert_eq!(
        soc_non_latin(json!("นี่คือการทดสอบภาษาไทย")).unwrap(),
        json!(true)
    );
}

#[test]
fn test_soc_vi() {
    assert_eq!(soc_vi(json!("kebakaran kios")).unwrap(), json!(false));
    assert_eq!(
        soc_vi(json!("đây là một bài kiểm tra tiếng việt với nhiều dấu")).unwrap(),
        json!(true)
    );
}

#[test]
fn test_soc_hash_junk() {
    assert_eq!(
        soc_hash_junk(json!("kebakaran di pasar")).unwrap(),
        json!(false)
    );
    assert_eq!(
        soc_hash_junk(json!("#a #b #c #d #e #f #g #h #i #j #k #l")).unwrap(),
        json!(true)
    );
    assert_eq!(
        soc_hash_junk(json!("$BTC $ETH $SOL crypto trading cuan token")).unwrap(),
        json!(true)
    );
}

#[test]
fn test_astroturf_sig() {
    assert_eq!(
        astroturf_sig(json!("Halo @user https://example.com #tag TEST!")).unwrap(),
        json!("halo test")
    );
    assert_eq!(
        astroturf_sig(json!({"a":"b"})).unwrap(),
        json!("object object")
    );
    assert_eq!(astroturf_sig(json!(null)).unwrap(), json!(""));
}

#[test]
fn test_soc_foreign_lang() {
    assert_eq!(
        soc_foreign_lang(json!("kebakaran melanda pasar")).unwrap(),
        json!(false)
    );
    assert_eq!(
        soc_foreign_lang(json!("het kabinet staat op vallen")).unwrap(),
        json!(true)
    );
    assert_eq!(
        soc_foreign_lang(json!("estoy muy feliz por que todos tienen que hacer")).unwrap(),
        json!(true)
    );
}

#[test]
fn test_soc_adhoc() {
    assert!(soc_adhoc(json!({})).is_err());
    let ad = soc_adhoc(json!(["banjir bandang", "kebakaran hebat"])).unwrap();
    let arr = ad.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], "banjir bandang");
    assert_eq!(arr[0]["adhoc"], true);
}

#[test]
fn test_soc_keep_x_and_th() {
    assert!(soc_keep_x(json!(null)).is_err());
    assert!(soc_keep_th(json!(null)).is_err());

    let post = json!({
        "text": "kebakaran melanda kios",
        "likes": 50,
        "topics": ["kebakaran"],
    });
    assert_eq!(soc_keep_x(post.clone()).unwrap(), json!(true));
    assert_eq!(soc_keep_th(post).unwrap(), json!(true));
}

#[test]
fn test_soc_balance() {
    assert_eq!(
        soc_balance(json!([]), json!(10), json!("user"), None, None, None).unwrap(),
        json!([])
    );
    assert_eq!(
        soc_balance(
            json!([{"user": "a"}]),
            json!(0),
            json!("user"),
            None,
            None,
            None
        )
        .unwrap(),
        json!([])
    );
    assert!(
        soc_balance(
            json!([{"user": "a"}]),
            json!(5),
            json!({}),
            None,
            None,
            None
        )
        .is_err()
    );
}

#[test]
fn test_soc_queries() {
    let qx = soc_queries(json!("x"), json!([])).unwrap();
    assert!(qx.as_array().unwrap().len() >= 30);
    let qth = soc_queries(json!("th"), json!([])).unwrap();
    assert!(qth.as_array().unwrap().len() >= 30);
}

#[test]
fn test_soc_hot() {
    assert!(soc_hot(json!(0), None).is_err());
    let res = soc_hot(json!([]), Some(json!(1000000))).unwrap();
    assert_eq!(res["stats"], json!({}));
    assert_eq!(res["hot"], json!([]));
}

#[test]
fn test_soc_keyword_mine() {
    assert!(soc_keyword_mine(json!([]), json!("banjir"), json!(null)).is_err());
    let res = soc_keyword_mine(json!([]), json!("banjir"), json!({})).unwrap();
    assert_eq!(res, json!([]));
}

#[test]
fn test_soc_assess_and_explain() {
    let text =
        json!("Kebakaran hebat melanda ruko dan pasar di Tambora Jakarta Barat, damkar dikerahkan");
    let assessed = soc_assess(text.clone(), json!([]), json!({})).unwrap();
    assert!(
        assessed["topics"]
            .as_array()
            .unwrap()
            .contains(&json!("kebakaran"))
    );
    assert_eq!(assessed["scope"], "Indonesia");

    let topics = soc_topics(text.clone(), json!([])).unwrap();
    assert!(topics.as_array().unwrap().contains(&json!("kebakaran")));

    let explained = explain_post(text, json!([]), json!({})).unwrap();
    assert_eq!(explained["version"], assessed["version"]);
    assert_eq!(explained["topics"], assessed["topics"]);
    assert_eq!(explained["intent"], assessed["intent"]);
}
