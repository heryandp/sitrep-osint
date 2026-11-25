//! Async/stateful `SocialHistory` ring buffer for thread-safe retention and snapshotting.

use crate::port::{
    AGE, CLASSIFIER_VERSION, DROP_STATS, is_truthy, key_of, tag_mut, tag_sig_of, ts_of_num,
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

pub const DEFAULT_HISTORY_CAPACITY: usize = 5000;

#[derive(Debug, Clone)]
pub struct SocialHistory {
    inner: Arc<RwLock<HistoryInner>>,
}

#[derive(Debug)]
struct HistoryInner {
    capacity: usize,
    max_age_ms: f64,
    order: VecDeque<String>,
    posts: HashMap<String, Value>,
}

impl Default for SocialHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl SocialHistory {
    pub fn new() -> Self {
        Self::with_retention(DEFAULT_HISTORY_CAPACITY, AGE)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_retention(capacity, AGE)
    }

    pub fn with_retention(capacity: usize, max_age_ms: f64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HistoryInner {
                capacity: capacity.max(1),
                max_age_ms,
                order: VecDeque::with_capacity(capacity.min(1000)),
                posts: HashMap::with_capacity(capacity.min(1000)),
            })),
        }
    }

    pub async fn len(&self) -> usize {
        let guard = self.inner.read().await;
        guard.posts.len()
    }

    pub async fn is_empty(&self) -> bool {
        let guard = self.inner.read().await;
        guard.posts.is_empty()
    }

    pub async fn clear(&self) {
        let mut guard = self.inner.write().await;
        guard.order.clear();
        guard.posts.clear();
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        let guard = self.inner.read().await;
        guard.posts.get(key).cloned()
    }

    pub async fn has(&self, key: &str) -> bool {
        let guard = self.inner.read().await;
        guard.posts.contains_key(key)
    }

    pub async fn remove(&self, key: &str) -> Option<Value> {
        let mut guard = self.inner.write().await;
        guard.posts.remove(key)
    }

    pub async fn insert(&self, post: Value) -> Result<String, String> {
        let key_val = key_of(post.clone())?;
        let key = key_val.as_str().unwrap_or("").to_string();
        if key.is_empty() {
            return Err("Cannot determine key for post".into());
        }

        let mut guard = self.inner.write().await;
        if !guard.posts.contains_key(&key) {
            guard.order.push_back(key.clone());
        }
        guard.posts.insert(key.clone(), post);

        while guard.posts.len() > guard.capacity {
            if let Some(oldest_key) = guard.order.pop_front() {
                guard.posts.remove(&oldest_key);
            } else {
                break;
            }
        }

        Ok(key)
    }

    pub async fn add(
        &self,
        mut post: Value,
        extra: Option<&Value>,
        now: Option<f64>,
        meta: Option<&Value>,
    ) -> Result<bool, String> {
        let ok = tag_mut(&mut post, extra, now, meta)?;
        if !ok {
            return Ok(false);
        }
        self.insert(post).await?;
        Ok(true)
    }

    pub async fn prune(&self, now: f64) -> usize {
        let mut guard = self.inner.write().await;
        let max_age = guard.max_age_ms;
        let mut to_remove = Vec::new();

        for (k, p) in &guard.posts {
            let ts = ts_of_num(p);
            if !ts.is_finite() || ts <= 0.0 || ts > now || (now - ts) > max_age {
                to_remove.push(k.clone());
            }
        }

        let removed = to_remove.len();
        for k in &to_remove {
            guard.posts.remove(k);
        }
        guard.order.retain(|k| !to_remove.contains(k));
        removed
    }

    pub async fn retag(
        &self,
        extra: &[Value],
        now: Option<f64>,
        keep: impl Fn(&Value) -> bool,
    ) -> Result<usize, String> {
        let extra_val = Value::Array(extra.to_vec());
        let cur_sig = tag_sig_of(&extra_val);
        let now_val = now.unwrap_or_else(crate::port::now_ms);

        let active_adhoc: HashSet<String> = extra
            .iter()
            .filter(|t| t.get("adhoc").is_some_and(is_truthy))
            .filter_map(|t| t.get("id").and_then(Value::as_str))
            .map(|s| s.to_lowercase())
            .collect();

        let mut guard = self.inner.write().await;
        let max_age = guard.max_age_ms;
        let mut to_remove = Vec::new();

        let keys: Vec<String> = guard.posts.keys().cloned().collect();
        for key in keys {
            let Some(p) = guard.posts.get_mut(&key) else {
                continue;
            };

            let ts = ts_of_num(p);
            if !ts.is_finite() || ts <= 0.0 || ts > now_val || (now_val - ts) > max_age {
                to_remove.push(key);
                continue;
            }

            let saved_adhoc: Vec<String> = p
                .get("adhocPhrases")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .filter(|s| {
                            s.len() <= 60
                                && !crate::port::analysis::is_generic_location_phrase_str(s)
                        })
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();

            let has_stale_adhoc = saved_adhoc
                .iter()
                .any(|s| !active_adhoc.contains(&s.to_lowercase()));
            let p_sig = p.get("_tagSig").and_then(Value::as_str).unwrap_or("");
            let assessment = p.get("assessment");
            let has_version = assessment.and_then(|a| a.get("version")).is_some();

            if !has_stale_adhoc && p_sig == cur_sig && has_version {
                let report_type = assessment
                    .and_then(|a| a.get("reportType"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let is_invalid_report =
                    matches!(report_type, "denied" | "historical" | "educational");
                let feed_eligible = assessment
                    .and_then(|a| a.get("feedEligible"))
                    .is_some_and(is_truthy);
                let topics_valid = p
                    .get("topics")
                    .and_then(Value::as_array)
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);

                if is_invalid_report || !feed_eligible || !topics_valid || !keep(p) {
                    to_remove.push(key);
                }
                continue;
            }

            let tagged = tag_mut(p, Some(&extra_val), Some(now_val), None)?;
            if !tagged || !keep(p) {
                to_remove.push(key);
            }
        }

        let purged = to_remove.len();
        for k in &to_remove {
            guard.posts.remove(k);
        }
        guard.order.retain(|k| !to_remove.contains(k));
        Ok(purged)
    }

    pub async fn snapshot(&self, now: Option<f64>) -> Value {
        let guard = self.inner.read().await;
        let now_val = now.unwrap_or_else(crate::port::now_ms);

        let mut posts: Vec<Value> = guard.posts.values().cloned().collect();
        posts.sort_by(|a, b| {
            let tb = ts_of_num(b);
            let ta = ts_of_num(a);
            tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
        });

        json!({
            "ok": true,
            "snapshot": true,
            "classifierVersion": CLASSIFIER_VERSION,
            "snapshotAt": now_val,
            "posts": posts,
            "at": now_val,
            "coverage": "sampled",
        })
    }

    pub async fn posts(&self) -> Vec<Value> {
        let guard = self.inner.read().await;
        let mut posts: Vec<Value> = guard.posts.values().cloned().collect();
        posts.sort_by(|a, b| {
            let tb = ts_of_num(b);
            let ta = ts_of_num(a);
            tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
        });
        posts
    }

    pub fn drop_stats(&self) -> Value {
        DROP_STATS.snapshot()
    }
}
