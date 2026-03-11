//! SCSS/SASS compiler with bounded cache.
//!
//! - Feature-gated by `scss` (grass)
//! - Deterministic cache key: (content, minify, file_path, load_paths)

#[cfg(feature = "scss")]
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
#[cfg(feature = "scss")]
use std::sync::{OnceLock, RwLock};
#[cfg(feature = "scss")]
use std::{fmt, sync::Arc};

#[cfg(feature = "scss")]
use super::hash::{ScopeHashBuilder, ScopeHashTarget, scope_hash_params_for};

/// Maximum number of cached SCSS compilations.
#[cfg(feature = "scss")]
const MAX_CACHE_SIZE: usize = 128;

pub struct ScssCompiler;

impl ScssCompiler {
    #[inline]
    pub fn is_file(path: &str) -> bool {
        let p = path.to_ascii_lowercase();
        p.ends_with(".scss") || p.ends_with(".sass")
    }

    #[cfg(feature = "scss")]
    pub fn compile(
        content: &str,
        file_path: Option<&Path>,
        minify: bool,
        extra_load_paths: &[PathBuf],
    ) -> Result<String, String> {
        let load_paths = Self::build_load_paths(file_path, extra_load_paths);

        let key = CompileKey::new(content, minify, file_path, &load_paths).hash();
        if let Ok(mut guard) = Self::cache().write()
            && let Some(hit) = guard.get(key)
        {
            return Ok(hit.to_string());
        }

        let opts = Self::grass_options(file_path, &load_paths);
        let compiled = grass::from_string(content.to_string(), &opts).map_err(|e| {
            if let Some(p) = file_path {
                format!("SCSS compilation error in '{}': {e}", p.display())
            } else {
                format!("SCSS compilation error: {e}")
            }
        })?;

        if let Ok(mut guard) = Self::cache().write() {
            guard.insert(key, Arc::<str>::from(compiled.as_str()));
        }

        Ok(compiled)
    }

    #[cfg(not(feature = "scss"))]
    pub fn compile(
        _content: &str,
        file_path: Option<&Path>,
        _minify: bool,
        _extra_load_paths: &[PathBuf],
    ) -> Result<String, String> {
        let where_ = file_path
            .map(|p| format!(" (requested by '{}')", p.display()))
            .unwrap_or_default();

        Err(format!(
            "SCSS support is not enabled{where_}. Enable it by adding the feature to your proc-macro crate dependency, e.g.: \
 dioxus_style_macros = {{ version = \"…\", features = [\"scss\"] }}",
        ))
    }

    #[cfg(feature = "scss")]
    fn build_load_paths(file_path: Option<&Path>, extra: &[PathBuf]) -> Vec<PathBuf> {
        let mut load_paths: Vec<PathBuf> = Vec::with_capacity(4);

        if let Some(fp) = file_path
            && let Some(dir) = fp.parent()
        {
            load_paths.push(dir.to_path_buf());
        }

        load_paths.extend(extra.iter().cloned());

        if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            load_paths.push(PathBuf::from(manifest));
        }

        load_paths
    }

    #[cfg(feature = "scss")]
    fn grass_options(file_path: Option<&Path>, load_paths: &[PathBuf]) -> grass::Options<'static> {
        use grass::{InputSyntax, Options, OutputStyle};

        let mut opts = Options::default()
            // LightningCSS is the default minifier/normalizer after SCSS preprocessing.
            .style(OutputStyle::Expanded)
            .quiet(false);

        if let Some(p) = file_path {
            let s = p.to_string_lossy().to_ascii_lowercase();
            if s.ends_with(".sass") {
                opts = opts.input_syntax(InputSyntax::Sass);
            } else if s.ends_with(".scss") {
                opts = opts.input_syntax(InputSyntax::Scss);
            }
        }

        for lp in load_paths {
            opts = opts.load_path(lp);
        }

        opts
    }

    #[inline]
    #[cfg(feature = "scss")]
    fn cache() -> &'static RwLock<ScssCache> {
        static SCSS_CACHE: OnceLock<RwLock<ScssCache>> = OnceLock::new();
        SCSS_CACHE.get_or_init(|| RwLock::new(ScssCache::default()))
    }
}

#[cfg(feature = "scss")]
#[derive(Clone)]
struct CacheEntry {
    css: Arc<str>,
    generation: u64,
}

#[cfg(feature = "scss")]
impl fmt::Debug for CacheEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheEntry")
            .field("len", &self.css.len())
            .field("generation", &self.generation)
            .finish()
    }
}

#[cfg(feature = "scss")]
#[derive(Debug)]
struct ScssCache {
    map: HashMap<u64, CacheEntry>,
    order: VecDeque<(u64, u64)>,
    generation: u64,
}

#[cfg(feature = "scss")]
impl Default for ScssCache {
    fn default() -> Self {
        Self {
            map: HashMap::with_capacity(64),
            order: VecDeque::with_capacity(64),
            generation: 1,
        }
    }
}

#[cfg(feature = "scss")]
impl ScssCache {
    #[inline]
    fn bump_generation(&mut self) -> u64 {
        let g = self.generation;
        self.generation = self.generation.wrapping_add(1).max(1);
        g
    }

    fn get(&mut self, key: u64) -> Option<Arc<str>> {
        let generation = self.bump_generation();
        let entry = self.map.get_mut(&key)?;
        entry.generation = generation;
        self.order.push_back((key, generation));
        Some(entry.css.clone())
    }

    fn insert(&mut self, key: u64, value: Arc<str>) {
        let generation = self.bump_generation();

        match self.map.get_mut(&key) {
            Some(existing) => {
                existing.css = value;
                existing.generation = generation;
            }
            None => {
                self.map.insert(
                    key,
                    CacheEntry {
                        css: value,
                        generation,
                    },
                );
            }
        }

        self.order.push_back((key, generation));
        self.evict_if_needed();
    }

    fn evict_if_needed(&mut self) {
        while self.map.len() > MAX_CACHE_SIZE {
            let Some((k, g)) = self.order.pop_front() else {
                break;
            };
            let stale = self.map.get(&k).is_none_or(|entry| entry.generation != g);
            if stale {
                continue;
            }
            self.map.remove(&k);
        }

        if self.order.len() > MAX_CACHE_SIZE.saturating_mul(8) {
            self.compact_queue();
        }
    }

    fn compact_queue(&mut self) {
        let mut next = VecDeque::with_capacity(self.map.len().saturating_mul(2));
        for (&k, entry) in self.map.iter() {
            next.push_back((k, entry.generation));
        }
        self.order = next;
    }
}

#[cfg(feature = "scss")]
struct CompileKey<'a> {
    content: &'a str,
    minify: bool,
    file_path: Option<&'a Path>,
    load_paths: &'a [PathBuf],
}

#[cfg(feature = "scss")]
impl<'a> CompileKey<'a> {
    #[inline]
    fn new(
        content: &'a str,
        minify: bool,
        file_path: Option<&'a Path>,
        load_paths: &'a [PathBuf],
    ) -> Self {
        Self {
            content,
            minify,
            file_path,
            load_paths,
        }
    }

    #[inline]
    fn hash(&self) -> u64 {
        let cfg = scope_hash_params_for(ScopeHashTarget::ScssCache);
        let mut b = ScopeHashBuilder::new(cfg.as_cfg());

        b.update_bytes(if self.minify { b"m1" } else { b"m0" });
        b.update_bytes(b"|content:");
        b.update_str(self.content);

        if let Some(p) = self.file_path {
            b.update_bytes(b"|file:");
            b.update_str(p.to_string_lossy().as_ref());
        }

        for lp in self.load_paths {
            b.update_bytes(b"|load:");
            b.update_str(lp.to_string_lossy().as_ref());
        }

        b.finish_u64()
    }
}
