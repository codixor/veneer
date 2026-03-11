use serde::Serialize;
use std::collections::{HashMap, HashSet};
use xxhash_rust::xxh3::Xxh3;

use crate::backend as style_backend;

#[cfg(not(target_arch = "wasm32"))]
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleBuildOptions {
    pub include_runtime_injector: bool,
    pub include_cssinjs: bool,
    pub emit_cache_path_marker: bool,
}

impl Default for BundleBuildOptions {
    fn default() -> Self {
        Self {
            include_runtime_injector: true,
            include_cssinjs: true,
            emit_cache_path_marker: false,
        }
    }
}

impl From<BundleBuildOptions> for style_backend::FullCssBundleOptions {
    fn from(value: BundleBuildOptions) -> Self {
        Self {
            include_runtime_injector: value.include_runtime_injector,
            include_cssinjs: value.include_cssinjs,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BundleMetadata {
    pub schema_version: u8,
    pub include_runtime_injector: bool,
    pub include_cssinjs: bool,
    pub emit_cache_path_marker: bool,
    pub runtime_css_len: usize,
    pub cssinjs_css_len: usize,
    pub cache_path_css_len: usize,
    pub total_css_len: usize,
    pub css_xxh3_64: String,
    pub runtime_injector_count: usize,
    pub cssinjs_record_count: usize,
    pub total_record_count: usize,
    pub records: Vec<style_backend::PrecompileStyleRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleOutput {
    pub css: String,
    pub metadata: BundleMetadata,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BundleExtractState {
    extracted_fragments: HashSet<String>,
}

impl BundleExtractState {
    #[must_use]
    pub fn extracted_count(&self) -> usize {
        self.extracted_fragments.len()
    }

    pub fn clear(&mut self) {
        self.extracted_fragments.clear();
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BundleExtractCache {
    entities: HashMap<String, BundleExtractState>,
}

impl BundleExtractCache {
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }

    #[must_use]
    pub fn remove(&mut self, cache_id: &str) -> bool {
        self.entities.remove(cache_id.trim()).is_some()
    }

    pub fn state_mut(&mut self, cache_id: &str) -> &mut BundleExtractState {
        self.entities
            .entry(cache_id.trim().to_string())
            .or_default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleWriteSummary {
    pub css_path: PathBuf,
    pub css_len: usize,
    pub total_record_count: usize,
    pub css_xxh3_64: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleWriteSummary {
    pub css_len: usize,
    pub total_record_count: usize,
    pub css_xxh3_64: String,
}

fn normalize_css_text(input: &str) -> String {
    let trimmed = input.trim_end_matches('\n');
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len() + 1);
    out.push_str(trimmed);
    out.push('\n');
    out
}

fn css_hash_hex(css: &str) -> String {
    let mut hasher = Xxh3::new();
    hasher.update(css.as_bytes());
    let digest = hasher.digest();
    format!("{digest:016x}")
}

fn append_css_chunk(target: &mut String, chunk: &str) {
    let chunk = chunk.trim_end_matches('\n');
    if chunk.is_empty() {
        return;
    }
    target.push_str(chunk);
    target.push('\n');
}

fn style_record_key(record: &style_backend::PrecompileStyleRecord) -> String {
    if let Some(cache_key) = record
        .cache_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return cache_key.to_string();
    }
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        record.source,
        record.style_id,
        record.tier,
        record.path.as_deref().unwrap_or(""),
        record.scope.as_deref().unwrap_or(""),
        record.layer.as_deref().unwrap_or(""),
        record.rewrite_signature.as_deref().unwrap_or(""),
    )
}

fn style_record_fragment_key(
    record: &style_backend::PrecompileStyleRecord,
    key: &str,
    css: Option<&str>,
) -> String {
    let mut hasher = Xxh3::new();
    hasher.update(record.source.as_bytes());
    hasher.update(b"|");
    hasher.update(key.as_bytes());
    hasher.update(b"|");
    hasher.update(record.style_id.as_bytes());
    hasher.update(b"|");
    hasher.update(record.tier.as_bytes());
    hasher.update(b"|");
    hasher.update(record.path.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(record.scope.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(record.layer.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(record.rewrite_signature.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    if let Some(css) = css {
        hasher.update(css.as_bytes());
    }
    format!("{:016x}", hasher.digest())
}

fn include_record(
    record: &style_backend::PrecompileStyleRecord,
    options: &BundleBuildOptions,
) -> bool {
    match record.source.as_str() {
        "cssinjs_runtime" => options.include_cssinjs,
        _ => options.include_runtime_injector,
    }
}

fn split_record_counts(records: &[style_backend::PrecompileStyleRecord]) -> (usize, usize) {
    let mut runtime_count = 0usize;
    let mut cssinjs_count = 0usize;
    for record in records {
        if record.source == "cssinjs_runtime" {
            cssinjs_count += 1;
        } else {
            runtime_count += 1;
        }
    }
    (runtime_count, cssinjs_count)
}

fn cache_path_marker_css(records: &[style_backend::PrecompileStyleRecord]) -> Option<String> {
    let mut path_pairs = records
        .iter()
        .filter(|record| record.source == "cssinjs_runtime")
        .filter_map(|record| {
            let path = record
                .path
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())?;
            let style_id = record.style_id.trim();
            (!style_id.is_empty()).then(|| (path.to_string(), style_id.to_string()))
        })
        .collect::<Vec<_>>();

    if path_pairs.is_empty() {
        return None;
    }

    path_pairs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    path_pairs.dedup();

    let content = path_pairs
        .iter()
        .map(|(path, style_id)| format!("{path}:{style_id}"))
        .collect::<Vec<_>>()
        .join(";");

    let escaped = content
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\a ");

    Some(format!(
        ".data-ant-cssinjs-cache-path{{content:\"{escaped}\";}}\n"
    ))
}

#[must_use]
pub fn build_bundle(options: BundleBuildOptions) -> BundleOutput {
    let full_snapshot = style_backend::FullCssBundle::snapshot(options.clone().into());
    let records_snapshot = style_backend::PrecompileStyleRecords::snapshot();

    let records = records_snapshot
        .records
        .into_iter()
        .filter(|record| include_record(record, &options))
        .collect::<Vec<_>>();

    let mut css = normalize_css_text(full_snapshot.css.as_str());
    let marker_css = if options.emit_cache_path_marker {
        cache_path_marker_css(records.as_slice())
    } else {
        None
    };
    let cache_path_css_len = marker_css.as_ref().map_or(0, String::len);
    if let Some(marker_css) = marker_css {
        append_css_chunk(&mut css, marker_css.as_str());
    }

    let (runtime_injector_count, cssinjs_record_count) = split_record_counts(records.as_slice());
    let metadata = BundleMetadata {
        schema_version: 1,
        include_runtime_injector: options.include_runtime_injector,
        include_cssinjs: options.include_cssinjs,
        emit_cache_path_marker: options.emit_cache_path_marker,
        runtime_css_len: full_snapshot.runtime_css_len,
        cssinjs_css_len: full_snapshot.cssinjs_css_len,
        cache_path_css_len,
        total_css_len: css.len(),
        css_xxh3_64: css_hash_hex(css.as_str()),
        runtime_injector_count,
        cssinjs_record_count,
        total_record_count: records.len(),
        records,
    };

    BundleOutput { css, metadata }
}

#[must_use]
pub fn build_bundle_once(
    state: &mut BundleExtractState,
    options: BundleBuildOptions,
) -> BundleOutput {
    let records_snapshot = style_backend::PrecompileStyleRecords::snapshot();
    let runtime_entries = if options.include_runtime_injector {
        style_backend::runtime_style_css_entries()
    } else {
        Vec::new()
    };
    let cssinjs_entries = if options.include_cssinjs {
        style_backend::CssInJs::css_entries()
    } else {
        Vec::new()
    };

    let runtime_css_by_key = runtime_entries
        .iter()
        .map(|entry| (entry.cache_key.as_str(), entry.css.as_ref()))
        .collect::<HashMap<_, _>>();
    let cssinjs_css_by_key = cssinjs_entries
        .iter()
        .map(|entry| (entry.cache_key.as_str(), entry.rendered_css.as_ref()))
        .collect::<HashMap<_, _>>();

    let mut fresh_runtime_keys = HashSet::<String>::new();
    let mut fresh_cssinjs_keys = HashSet::<String>::new();
    let mut fresh_records = Vec::new();

    for record in records_snapshot.records {
        if !include_record(&record, &options) {
            continue;
        }

        let key = style_record_key(&record);
        let css = if record.source == "cssinjs_runtime" {
            cssinjs_css_by_key.get(key.as_str()).copied()
        } else {
            runtime_css_by_key.get(key.as_str()).copied()
        };
        let fragment_key = style_record_fragment_key(&record, key.as_str(), css);

        if state.extracted_fragments.insert(fragment_key) {
            if record.source == "cssinjs_runtime" {
                let _ = fresh_cssinjs_keys.insert(key);
            } else {
                let _ = fresh_runtime_keys.insert(key);
            }
            fresh_records.push(record);
        }
    }

    let mut runtime_css = String::new();
    if options.include_runtime_injector {
        for entry in runtime_entries {
            if fresh_runtime_keys.contains(entry.cache_key.as_str()) {
                append_css_chunk(&mut runtime_css, entry.css.as_ref());
            }
        }
    }

    let mut cssinjs_css = String::new();
    if options.include_cssinjs {
        for entry in cssinjs_entries {
            if fresh_cssinjs_keys.contains(entry.cache_key.as_str()) {
                append_css_chunk(&mut cssinjs_css, entry.rendered_css.as_ref());
            }
        }
    }

    let mut css = String::with_capacity(runtime_css.len() + cssinjs_css.len() + 128);
    append_css_chunk(&mut css, runtime_css.as_str());
    append_css_chunk(&mut css, cssinjs_css.as_str());

    let marker_css = if options.emit_cache_path_marker {
        cache_path_marker_css(fresh_records.as_slice())
    } else {
        None
    };
    let cache_path_css_len = marker_css.as_ref().map_or(0, String::len);
    if let Some(marker_css) = marker_css {
        append_css_chunk(&mut css, marker_css.as_str());
    }

    let (runtime_injector_count, cssinjs_record_count) =
        split_record_counts(fresh_records.as_slice());
    let metadata = BundleMetadata {
        schema_version: 1,
        include_runtime_injector: options.include_runtime_injector,
        include_cssinjs: options.include_cssinjs,
        emit_cache_path_marker: options.emit_cache_path_marker,
        runtime_css_len: runtime_css.len(),
        cssinjs_css_len: cssinjs_css.len(),
        cache_path_css_len,
        total_css_len: css.len(),
        css_xxh3_64: css_hash_hex(css.as_str()),
        runtime_injector_count,
        cssinjs_record_count,
        total_record_count: fresh_records.len(),
        records: fresh_records,
    };

    BundleOutput { css, metadata }
}

#[must_use]
pub fn build_bundle_once_with_cache(
    cache: &mut BundleExtractCache,
    cache_id: &str,
    options: BundleBuildOptions,
) -> BundleOutput {
    let state = cache.state_mut(cache_id);
    build_bundle_once(state, options)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_bundle_files(
    css_path: impl AsRef<Path>,
    options: BundleBuildOptions,
) -> io::Result<BundleWriteSummary> {
    let css_path = css_path.as_ref();

    if let Some(parent) = css_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let output = build_bundle(options);
    fs::write(css_path, output.css.as_bytes())?;

    Ok(BundleWriteSummary {
        css_path: css_path.to_path_buf(),
        css_len: output.metadata.total_css_len,
        total_record_count: output.metadata.total_record_count,
        css_xxh3_64: output.metadata.css_xxh3_64,
    })
}

#[cfg(target_arch = "wasm32")]
pub fn write_bundle_files(
    _css_path: impl AsRef<std::path::Path>,
    _options: BundleBuildOptions,
) -> Result<BundleWriteSummary, String> {
    Err("cssinjs bundle file emission is unavailable on wasm32".to_string())
}
