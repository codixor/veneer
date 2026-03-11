use serde::Serialize;

use crate::{CssInJs, runtime_style_records};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PrecompileStyleRecord {
    pub source: String,
    pub style_id: String,
    pub path: Option<String>,
    pub scope: Option<String>,
    pub layer: Option<String>,
    pub tier: String,
    pub hash: Option<String>,
    pub cache_key: Option<String>,
    pub rewrite_signature: Option<String>,
    pub rewrite_enabled: Option<bool>,
    pub hashed: Option<bool>,
    pub css_var_key: Option<String>,
    pub algorithm: Option<String>,
    pub theme_scope: Option<String>,
    pub token_hash: Option<String>,
    pub nonce: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PrecompileStyleSnapshot {
    pub runtime_injector_count: usize,
    pub cssinjs_count: usize,
    pub total_count: usize,
    pub records: Vec<PrecompileStyleRecord>,
}

pub struct PrecompileStyleRecords;

impl PrecompileStyleRecords {
    #[inline]
    #[must_use]
    pub fn snapshot() -> PrecompileStyleSnapshot {
        let runtime = runtime_style_records();
        let cssinjs = CssInJs::records();

        let mut records = Vec::with_capacity(runtime.len() + cssinjs.len());

        for rec in &runtime {
            let source = if rec.tier == "scoped" {
                "scss_macro"
            } else {
                "runtime_injector"
            };
            records.push(PrecompileStyleRecord {
                source: source.to_string(),
                style_id: rec.style_id.clone(),
                path: None,
                scope: Some(rec.scope.clone()),
                layer: rec.layer.clone(),
                tier: rec.tier.clone(),
                hash: rec.hash.clone(),
                cache_key: Some(rec.cache_key.clone()),
                rewrite_signature: rec.rewrite_signature.clone(),
                rewrite_enabled: Some(rec.rewrite_enabled),
                hashed: None,
                css_var_key: None,
                algorithm: None,
                theme_scope: None,
                token_hash: None,
                nonce: None,
            });
        }

        for rec in &cssinjs {
            records.push(PrecompileStyleRecord {
                source: "cssinjs_runtime".to_string(),
                style_id: rec.style_id.clone(),
                path: rec.path.clone(),
                scope: rec.scope.clone(),
                layer: rec.layer.clone(),
                tier: rec.tier.clone(),
                hash: Some(rec.hash.clone()),
                cache_key: Some(rec.cache_key.clone()),
                rewrite_signature: Some(rec.rewrite_signature.clone()),
                rewrite_enabled: None,
                hashed: rec.hashed,
                css_var_key: rec.css_var_key.clone(),
                algorithm: rec.algorithm.clone(),
                theme_scope: rec.theme_scope.clone(),
                token_hash: rec.token_hash.clone(),
                nonce: rec.nonce.clone(),
            });
        }

        records.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then_with(|| a.style_id.cmp(&b.style_id))
                .then_with(|| a.tier.cmp(&b.tier))
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.scope.cmp(&b.scope))
                .then_with(|| a.layer.cmp(&b.layer))
                .then_with(|| a.cache_key.cmp(&b.cache_key))
        });

        PrecompileStyleSnapshot {
            runtime_injector_count: runtime.len(),
            cssinjs_count: cssinjs.len(),
            total_count: records.len(),
            records,
        }
    }
}
