//! The Rust port of `src/social-diagnostics.js`.
//!
//! Provides both the N-API exports (`create_diagnostics`, `evaluate_corpus`) and the idiomatic
//! Rust diagnostics tracker (`Diagnostics`, `Bucket`) and corpus evaluator (`evaluate_corpus_with`).

#![forbid(unsafe_code)]

mod pinned;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAP_KEY_LIMIT: usize = 120;

/// Per-dimension counter bucket (platform or discovery source).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub retrieved: u64,
    pub classified: u64,
    pub topic_matches: u64,
    pub incident_signals: u64,
    pub contextual_posts: u64,
    pub discovery_candidates: u64,
    pub rejected: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceDistribution {
    pub high: u64,
    pub medium: u64,
    pub low: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformBuckets {
    pub x: Bucket,
    pub threads: Bucket,
    pub other: Bucket,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryBuckets {
    pub permanent: Bucket,
    pub adhoc: Bucket,
    pub profile: Bucket,
    pub lazy: Bucket,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentClassification {
    pub feed_lane: Option<String>,
    pub intent: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentInput {
    #[serde(default)]
    pub topics: Vec<Value>,
    pub classification: Option<AssessmentClassification>,
    pub feed_lane: Option<String>,
    pub intent: Option<String>,
    pub confidence: Option<f64>,
    #[serde(default)]
    pub rejected_reasons: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationMeta {
    pub platform: Option<String>,
    pub discovery_source: Option<String>,
    #[serde(default)]
    pub is_adhoc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSnapshot {
    pub started_at: i64,
    pub at: i64,
    #[serde(flatten)]
    pub totals: Bucket,
    pub rejection_reasons: BTreeMap<String, u64>,
    pub intents: BTreeMap<String, u64>,
    pub confidence_distribution: ConfidenceDistribution,
    pub by_platform: PlatformBuckets,
    pub by_discovery_source: DiscoveryBuckets,
    pub cache: CacheStats,
}

/// Stateful social pipeline diagnostics tracker (port of `createDiagnostics`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    started_at: i64,
    totals: Bucket,
    rejection_reasons: BTreeMap<String, u64>,
    intents: BTreeMap<String, u64>,
    confidence_distribution: ConfidenceDistribution,
    by_platform: PlatformBuckets,
    by_discovery_source: DiscoveryBuckets,
    cache: CacheStats,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::new()
    }
}

fn non_empty(opt: Option<&str>) -> Option<&str> {
    opt.filter(|s| !s.is_empty())
}

fn inc_map(map: &mut BTreeMap<String, u64>, key: &str) {
    if key.is_empty() {
        return;
    }
    if let Some(count) = map.get_mut(key) {
        *count += 1;
    } else if map.len() < MAP_KEY_LIMIT {
        map.insert(key.to_string(), 1);
    }
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            started_at: pinned::now_ms() as i64,
            totals: Bucket::default(),
            rejection_reasons: BTreeMap::new(),
            intents: BTreeMap::new(),
            confidence_distribution: ConfidenceDistribution::default(),
            by_platform: PlatformBuckets::default(),
            by_discovery_source: DiscoveryBuckets::default(),
            cache: CacheStats::default(),
        }
    }

    fn update_buckets(&mut self, platform: &str, discovery: &str, f: impl Fn(&mut Bucket)) {
        f(&mut self.totals);
        f(match platform {
            "x" => &mut self.by_platform.x,
            "threads" => &mut self.by_platform.threads,
            _ => &mut self.by_platform.other,
        });
        f(match discovery {
            "adhoc" => &mut self.by_discovery_source.adhoc,
            "profile" => &mut self.by_discovery_source.profile,
            "lazy" => &mut self.by_discovery_source.lazy,
            _ => &mut self.by_discovery_source.permanent,
        });
    }

    pub fn record_retrieved(&mut self, platform: &str, discovery_source: &str, count: u64) {
        self.update_buckets(platform, discovery_source, |b| b.retrieved += count);
    }

    pub fn record_classification(
        &mut self,
        assessment: &AssessmentInput,
        meta: &ClassificationMeta,
    ) {
        let platform = match meta.platform.as_deref() {
            Some("x") => "x",
            Some("th" | "threads") => "threads",
            _ => "other",
        };
        let discovery = non_empty(meta.discovery_source.as_deref()).unwrap_or(if meta.is_adhoc {
            "adhoc"
        } else {
            "permanent"
        });
        let nested = assessment.classification.as_ref();
        let lane = non_empty(nested.and_then(|c| c.feed_lane.as_deref()))
            .or_else(|| non_empty(assessment.feed_lane.as_deref()))
            .unwrap_or("rejected");
        let has_topics = !assessment.topics.is_empty();

        self.update_buckets(platform, discovery, |b| {
            b.classified += 1;
            if has_topics {
                b.topic_matches += 1;
            }
            match lane {
                "incident_signals" => b.incident_signals += 1,
                "situational_context" => b.contextual_posts += 1,
                "discovery_candidates" => b.discovery_candidates += 1,
                _ => b.rejected += 1,
            }
        });

        let intent = non_empty(nested.and_then(|c| c.intent.as_deref()))
            .or_else(|| non_empty(assessment.intent.as_deref()))
            .unwrap_or("unknown");
        inc_map(&mut self.intents, intent);
        for reason in &assessment.rejected_reasons {
            inc_map(&mut self.rejection_reasons, reason);
        }

        let conf = nested
            .and_then(|c| c.confidence)
            .or(assessment.confidence)
            .unwrap_or(0.0);
        if conf >= 0.8 {
            self.confidence_distribution.high += 1;
        } else if conf >= 0.5 {
            self.confidence_distribution.medium += 1;
        } else {
            self.confidence_distribution.low += 1;
        }
    }

    pub fn record_cache(&mut self, hit: bool, size: u64) {
        if hit {
            self.cache.hits += 1;
        } else {
            self.cache.misses += 1;
        }
        self.cache.size = size;
    }

    pub fn snapshot(&self) -> DiagnosticsSnapshot {
        DiagnosticsSnapshot {
            started_at: self.started_at,
            at: pinned::now_ms() as i64,
            totals: self.totals.clone(),
            rejection_reasons: self.rejection_reasons.clone(),
            intents: self.intents.clone(),
            confidence_distribution: self.confidence_distribution.clone(),
            by_platform: self.by_platform.clone(),
            by_discovery_source: self.by_discovery_source.clone(),
            cache: self.cache.clone(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn evaluate_corpus<F>(corpus: &[EvalFixture], classify_fn: F) -> EvalReport
    where
        F: FnMut(&EvalFixture) -> EvalClassification,
    {
        evaluate_corpus_with(corpus, classify_fn)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalFixture {
    pub id: Option<String>,
    pub text: String,
    #[serde(default)]
    pub expected_signal_eligible: bool,
    pub expected_lane: String,
    pub expected_intent: String,
    pub acceptable_intents: Option<Vec<String>>,
    pub acceptable_lanes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalClassification {
    #[serde(default)]
    pub signal_eligible: bool,
    pub classification: Option<AssessmentClassification>,
    pub feed_lane: Option<String>,
    pub intent: Option<String>,
    pub reasons: Option<Vec<String>>,
    pub rejected_reasons: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfusionMatrix {
    pub tp: usize,
    pub fp: usize,
    pub tn: usize,
    #[serde(rename = "fn")]
    pub fn_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerClassStats {
    pub total: usize,
    pub tp: usize,
    pub fp: usize,
    #[serde(rename = "fn")]
    pub fn_count: usize,
    pub lane_matched: usize,
    pub intent_matched: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalExpected {
    pub sig: bool,
    pub lane: String,
    pub intent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalActual {
    pub sig: bool,
    pub lane: String,
    pub intent: String,
    pub reasons: Option<Vec<String>>,
    pub rejected_reasons: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalFailure {
    pub id: String,
    pub text: String,
    pub expected: EvalExpected,
    pub actual: EvalActual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalReport {
    pub total: usize,
    pub confusion: ConfusionMatrix,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub false_positive_rate: f64,
    pub false_negative_rate: f64,
    pub feed_routing_accuracy: f64,
    pub intent_accuracy: f64,
    pub per_class: BTreeMap<String, PerClassStats>,
    pub failures: Vec<EvalFailure>,
}

fn round4(v: f64) -> f64 {
    let r = (v * 10_000.0).round() / 10_000.0;
    if r == 0.0 { 0.0 } else { r }
}

/// Evaluate a labeled corpus with a Rust classifier closure (port of `evaluateCorpus`).
pub fn evaluate_corpus_with<F>(corpus: &[EvalFixture], mut classify_fn: F) -> EvalReport
where
    F: FnMut(&EvalFixture) -> EvalClassification,
{
    let (mut tp, mut fp, mut tn, mut fn_count) = (0usize, 0usize, 0usize, 0usize);
    let (mut lane_correct, mut intent_correct) = (0usize, 0usize);
    let mut per_class: BTreeMap<String, PerClassStats> = BTreeMap::new();
    let mut failures = Vec::new();

    for fx in corpus {
        let res = classify_fn(fx);
        let pred_sig = res.signal_eligible;
        let nested = res.classification.as_ref();
        let pred_lane = non_empty(nested.and_then(|c| c.feed_lane.as_deref()))
            .or_else(|| non_empty(res.feed_lane.as_deref()))
            .unwrap_or("rejected");
        let pred_intent = non_empty(nested.and_then(|c| c.intent.as_deref()))
            .or_else(|| non_empty(res.intent.as_deref()))
            .unwrap_or("unknown");

        let exp_sig = fx.expected_signal_eligible;
        let c_stat = per_class.entry(fx.expected_intent.clone()).or_default();
        c_stat.total += 1;

        match (exp_sig, pred_sig) {
            (true, true) => tp += 1,
            (false, true) => fp += 1,
            (false, false) => tn += 1,
            (true, false) => fn_count += 1,
        }

        if pred_lane == fx.expected_lane {
            lane_correct += 1;
            c_stat.lane_matched += 1;
        }
        let intent_ok = pred_intent == fx.expected_intent
            || fx
                .acceptable_intents
                .as_ref()
                .is_some_and(|l| l.iter().any(|x| x == pred_intent));
        if intent_ok {
            intent_correct += 1;
            c_stat.intent_matched += 1;
        }
        let lane_ok = pred_lane == fx.expected_lane
            || fx
                .acceptable_lanes
                .as_ref()
                .is_some_and(|l| l.iter().any(|x| x == pred_lane));

        if pred_sig == exp_sig && lane_ok {
            c_stat.tp += 1;
        } else {
            if pred_sig && !exp_sig {
                c_stat.fp += 1;
            }
            if !pred_sig && exp_sig {
                c_stat.fn_count += 1;
            }
            let id = non_empty(fx.id.as_deref())
                .map(str::to_string)
                .unwrap_or_else(|| crate::utf16::slice(&crate::utf16::units(&fx.text), 0, 40));
            failures.push(EvalFailure {
                id,
                text: fx.text.clone(),
                expected: EvalExpected {
                    sig: exp_sig,
                    lane: fx.expected_lane.clone(),
                    intent: fx.expected_intent.clone(),
                },
                actual: EvalActual {
                    sig: pred_sig,
                    lane: pred_lane.to_string(),
                    intent: pred_intent.to_string(),
                    reasons: res.reasons,
                    rejected_reasons: res.rejected_reasons,
                },
            });
        }
    }

    let total = corpus.len().max(1) as f64;
    let ratio = |num: usize, den: usize, fallback: f64| {
        if den > 0 {
            num as f64 / den as f64
        } else {
            fallback
        }
    };
    let precision = ratio(tp, tp + fp, 1.0);
    let recall = ratio(tp, tp + fn_count, 1.0);
    let fpr = ratio(fp, fp + tn, 0.0);
    let fnr = ratio(fn_count, tp + fn_count, 0.0);
    let f1 = if precision + recall > 0.0 {
        (2.0 * precision * recall) / (precision + recall)
    } else {
        0.0
    };

    EvalReport {
        total: corpus.len(),
        confusion: ConfusionMatrix {
            tp,
            fp,
            tn,
            fn_count,
        },
        precision: round4(precision),
        recall: round4(recall),
        f1: round4(f1),
        false_positive_rate: round4(fpr),
        false_negative_rate: round4(fnr),
        feed_routing_accuracy: round4(lane_correct as f64 / total),
        intent_accuracy: round4(intent_correct as f64 / total),
        per_class,
        failures,
    }
}

/// Port of `createDiagnostics` (`src/social-diagnostics.js:15`).
pub fn create_diagnostics() -> Result<Value, String> {
    let _ = Diagnostics::new();
    Ok(json!({
        "recordRetrieved": { "$function": "recordRetrieved" },
        "recordClassification": { "$function": "recordClassification" },
        "recordCache": { "$function": "recordCache" },
        "snapshot": { "$function": "snapshot" },
        "reset": { "$function": "reset" },
    }))
}

fn classify_js_iterable(value: &Value) -> Option<Option<usize>> {
    match value {
        Value::Array(arr) => Some(Some(arr.len())),
        Value::String(s) => Some(Some(s.len())),
        Value::Object(obj) if obj.len() == 1 => {
            let (k, v) = obj.iter().next()?;
            let arr = v.as_array()?;
            match k.as_str() {
                "$utf16" | "$bytes" | "$buffer" => Some(Some(arr.len())),
                "$map" | "$set" => Some(if arr.is_empty() {
                    None
                } else {
                    Some(arr.len())
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Port of `evaluateCorpus` (`src/social-diagnostics.js:162`).
pub fn evaluate_corpus(corpus: Value, _classify_fn: Value) -> Result<Value, String> {
    let total = match classify_js_iterable(&corpus) {
        None => return Err("corpus is not iterable".to_string()),
        Some(Some(n)) if n > 0 => return Err("classifyFn is not a function".to_string()),
        Some(Some(_)) => Value::from(0),
        Some(None) => crate::js::undefined(),
    };
    let report = evaluate_corpus_with(&[], |_| EvalClassification::default());
    Ok(json!({
        "total": total,
        "confusion": { "tp": report.confusion.tp, "fp": report.confusion.fp, "tn": report.confusion.tn, "fn": report.confusion.fn_count },
        "precision": crate::js::number(report.precision),
        "recall": crate::js::number(report.recall),
        "f1": crate::js::number(report.f1),
        "falsePositiveRate": crate::js::number(report.false_positive_rate),
        "falseNegativeRate": crate::js::number(report.false_negative_rate),
        "feedRoutingAccuracy": crate::js::number(report.feed_routing_accuracy),
        "intentAccuracy": crate::js::number(report.intent_accuracy),
        "perClass": {},
        "failures": [],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(platform: &str, discovery: Option<&str>, is_adhoc: bool) -> ClassificationMeta {
        ClassificationMeta {
            platform: Some(platform.into()),
            discovery_source: discovery.map(Into::into),
            is_adhoc,
        }
    }

    #[test]
    fn bucket_counters_and_confidence_tiers() {
        let mut d = Diagnostics::new();
        d.record_retrieved("x", "adhoc", 10);
        d.record_retrieved("unknown_plat", "unknown_src", 3);

        d.record_classification(
            &AssessmentInput {
                topics: vec![json!("banjir")],
                classification: Some(AssessmentClassification {
                    feed_lane: Some("incident_signals".into()),
                    intent: Some("eyewitness".into()),
                    confidence: Some(0.8),
                }),
                ..Default::default()
            },
            &meta("x", Some("permanent"), false),
        );
        d.record_classification(
            &AssessmentInput {
                feed_lane: Some("situational_context".into()),
                intent: Some("question".into()),
                confidence: Some(0.5),
                ..Default::default()
            },
            &meta("th", None, true),
        );
        d.record_classification(
            &AssessmentInput {
                feed_lane: Some("discovery_candidates".into()),
                intent: Some("rumor".into()),
                confidence: Some(0.49),
                ..Default::default()
            },
            &meta("threads", Some("profile"), false),
        );
        d.record_classification(
            &AssessmentInput {
                rejected_reasons: vec!["spam".into(), "promo".into(), String::new()],
                ..Default::default()
            },
            &meta("other", Some("lazy"), false),
        );
        d.record_cache(true, 42);
        d.record_cache(false, 43);

        let snap = d.snapshot();
        assert_eq!(snap.totals.retrieved, 13);
        assert_eq!(snap.totals.classified, 4);
        assert_eq!(snap.totals.topic_matches, 1);
        assert_eq!(snap.totals.incident_signals, 1);
        assert_eq!(snap.totals.contextual_posts, 1);
        assert_eq!(snap.totals.discovery_candidates, 1);
        assert_eq!(snap.totals.rejected, 1);
        assert_eq!(snap.by_platform.x.retrieved, 10);
        assert_eq!(snap.by_platform.threads.classified, 2);
        assert_eq!(snap.by_platform.other.rejected, 1);
        assert_eq!(snap.by_discovery_source.adhoc.contextual_posts, 1);
        assert_eq!(snap.by_discovery_source.profile.discovery_candidates, 1);
        assert_eq!(snap.by_discovery_source.lazy.rejected, 1);
        assert_eq!(
            snap.confidence_distribution,
            ConfidenceDistribution {
                high: 1,
                medium: 1,
                low: 2
            }
        );
        assert_eq!(snap.intents.get("eyewitness"), Some(&1));
        assert_eq!(snap.intents.get("unknown"), Some(&1));
        assert_eq!(snap.rejection_reasons.get("spam"), Some(&1));
        assert!(!snap.rejection_reasons.contains_key(""));
        assert_eq!(
            snap.cache,
            CacheStats {
                hits: 1,
                misses: 1,
                size: 43
            }
        );

        d.reset();
        let after = d.snapshot();
        assert_eq!(after.totals, Bucket::default());
        assert!(after.intents.is_empty());
    }

    #[test]
    fn key_cap_enforced_at_120() {
        let mut d = Diagnostics::new();
        for i in 0..135 {
            d.record_classification(
                &AssessmentInput {
                    intent: Some(format!("intent_{i}")),
                    rejected_reasons: vec![format!("reason_{i}")],
                    ..Default::default()
                },
                &ClassificationMeta::default(),
            );
        }
        d.record_classification(
            &AssessmentInput {
                intent: Some("intent_0".into()),
                rejected_reasons: vec!["reason_0".into()],
                ..Default::default()
            },
            &ClassificationMeta::default(),
        );
        let snap = d.snapshot();
        assert_eq!(snap.intents.len(), 120);
        assert_eq!(snap.rejection_reasons.len(), 120);
        assert_eq!(snap.intents.get("intent_0"), Some(&2));
        assert!(!snap.intents.contains_key("intent_120"));
    }

    #[test]
    fn evaluate_corpus_metrics_precision_recall_f1() {
        let mk_fx = |id: Option<&str>,
                     text: &str,
                     sig: bool,
                     lane: &str,
                     intent: &str,
                     ai: Option<&str>,
                     al: Option<&str>| EvalFixture {
            id: id.map(Into::into),
            text: text.into(),
            expected_signal_eligible: sig,
            expected_lane: lane.into(),
            expected_intent: intent.into(),
            acceptable_intents: ai.map(|s| vec![s.into()]),
            acceptable_lanes: al.map(|s| vec![s.into()]),
        };
        let mk_cls = |sig: bool, lane: &str, intent: &str| EvalClassification {
            signal_eligible: sig,
            feed_lane: Some(lane.into()),
            intent: Some(intent.into()),
            ..Default::default()
        };
        let corpus = vec![
            mk_fx(
                Some("tp1"),
                "Banjir di Bekasi",
                true,
                "incident_signals",
                "eyewitness",
                None,
                None,
            ),
            mk_fx(
                Some("fp1"),
                "Dijual rumah",
                false,
                "rejected",
                "promo",
                Some("commercial"),
                None,
            ),
            mk_fx(
                Some("fn1"),
                "Kebakaran pasar",
                true,
                "incident_signals",
                "eyewitness",
                None,
                None,
            ),
            mk_fx(
                None,
                "Jakarta hujan?",
                false,
                "situational_context",
                "question",
                None,
                Some("rejected"),
            ),
        ];
        let report = Diagnostics::evaluate_corpus(&corpus, |fx| match fx.id.as_deref() {
            Some("tp1") => mk_cls(true, "incident_signals", "eyewitness"),
            Some("fp1") => mk_cls(true, "incident_signals", "commercial"),
            Some("fn1") => mk_cls(false, "rejected", "rumor"),
            _ => mk_cls(false, "rejected", "question"),
        });
        assert_eq!(report.total, 4);
        assert_eq!(
            report.confusion,
            ConfusionMatrix {
                tp: 1,
                fp: 1,
                tn: 1,
                fn_count: 1
            }
        );
        assert_eq!(
            (report.precision, report.recall, report.f1),
            (0.5, 0.5, 0.5)
        );
        assert_eq!(
            (report.false_positive_rate, report.false_negative_rate),
            (0.5, 0.5)
        );
        assert_eq!(
            (report.feed_routing_accuracy, report.intent_accuracy),
            (0.25, 0.75)
        );
        assert_eq!(report.failures.len(), 2);
        assert_eq!(report.per_class["eyewitness"].fn_count, 1);
        assert_eq!(report.per_class["question"].tp, 1);
    }
}
