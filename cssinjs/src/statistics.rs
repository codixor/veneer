use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Clone, Debug, PartialEq)]
pub enum StatisticValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

impl From<String> for StatisticValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for StatisticValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<i64> for StatisticValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<i32> for StatisticValue {
    fn from(value: i32) -> Self {
        Self::Integer(i64::from(value))
    }
}

impl From<usize> for StatisticValue {
    fn from(value: usize) -> Self {
        Self::Integer(value as i64)
    }
}

impl From<f64> for StatisticValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<f32> for StatisticValue {
    fn from(value: f32) -> Self {
        Self::Float(f64::from(value))
    }
}

impl From<bool> for StatisticValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatisticEntry {
    pub global: Vec<String>,
    pub component: BTreeMap<String, StatisticValue>,
}

fn statistic_store() -> &'static RwLock<BTreeMap<String, StatisticEntry>> {
    static STORE: OnceLock<RwLock<BTreeMap<String, StatisticEntry>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(BTreeMap::new()))
}

fn statistic_build_store() -> &'static RwLock<BTreeMap<String, StatisticEntry>> {
    static STORE: OnceLock<RwLock<BTreeMap<String, StatisticEntry>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(BTreeMap::new()))
}

#[must_use]
pub fn statistic() -> BTreeMap<String, StatisticEntry> {
    statistic_store()
        .read()
        .expect("cssinjs statistic snapshot lock poisoned")
        .clone()
}

#[must_use]
pub fn statistic_build() -> BTreeMap<String, StatisticEntry> {
    statistic_build_store()
        .read()
        .expect("cssinjs statistic build snapshot lock poisoned")
        .clone()
}

#[derive(Clone, Debug)]
pub struct TrackedTokenMap<T> {
    values: Arc<BTreeMap<String, T>>,
    keys: Arc<RwLock<BTreeSet<String>>>,
}

impl<T> TrackedTokenMap<T> {
    fn new(values: Arc<BTreeMap<String, T>>, keys: Arc<RwLock<BTreeSet<String>>>) -> Self {
        Self { values, keys }
    }

    #[must_use]
    pub fn raw(&self) -> &BTreeMap<String, T> {
        self.values.as_ref()
    }

    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.record(key);
        self.values.contains_key(key)
    }

    fn record(&self, key: &str) {
        self.keys
            .write()
            .expect("cssinjs statistic key tracker lock poisoned")
            .insert(key.to_string());
    }
}

impl<T> TrackedTokenMap<T>
where
    T: Clone,
{
    #[must_use]
    pub fn get(&self, key: &str) -> Option<T> {
        self.record(key);
        self.values.get(key).cloned()
    }

    #[must_use]
    pub fn snapshot(&self) -> BTreeMap<String, T> {
        self.values.as_ref().clone()
    }
}

#[derive(Clone, Debug)]
pub struct StatisticToken<T> {
    pub token: TrackedTokenMap<T>,
    keys: Arc<RwLock<BTreeSet<String>>>,
}

impl<T> StatisticToken<T> {
    #[must_use]
    pub fn keys(&self) -> BTreeSet<String> {
        self.keys
            .read()
            .expect("cssinjs statistic keys lock poisoned")
            .clone()
    }

    pub fn flush<K, V, I>(&self, component_name: &str, component_token: I)
    where
        K: Into<String>,
        V: Into<StatisticValue>,
        I: IntoIterator<Item = (K, V)>,
    {
        let global = self.keys().into_iter().collect::<Vec<_>>();
        let component_patch = component_token
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect::<BTreeMap<_, _>>();

        for store in [statistic_store(), statistic_build_store()] {
            let mut guard = store
                .write()
                .expect("cssinjs statistic flush lock poisoned");
            let entry = guard.entry(component_name.to_string()).or_default();
            entry.global = global.clone();
            entry.component.extend(component_patch.clone());
        }
    }
}

#[must_use]
pub fn statistic_token<T>(token: &BTreeMap<String, T>) -> StatisticToken<T>
where
    T: Clone,
{
    let values = Arc::new(token.clone());
    let keys = Arc::new(RwLock::new(BTreeSet::new()));

    StatisticToken {
        token: TrackedTokenMap::new(values, Arc::clone(&keys)),
        keys,
    }
}

pub fn debug_reset_statistics_for_tests() {
    statistic_store()
        .write()
        .expect("cssinjs statistic reset lock poisoned")
        .clear();
    statistic_build_store()
        .write()
        .expect("cssinjs statistic build reset lock poisoned")
        .clear();
}
