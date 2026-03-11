#[cfg(not(target_arch = "wasm32"))]
use std::{
    fs, io,
    path::Path,
    sync::{Mutex, OnceLock},
};

#[cfg(target_arch = "wasm32")]
fn main() {
    eprintln!("cssinjs bundle CLI is unavailable for wasm32 targets");
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(err) = run_cli() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_cli() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Err("missing command".to_string());
    };

    match command.as_str() {
        "bundle" => run_bundle(args.collect()),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_bundle(args: Vec<String>) -> Result<(), String> {
    let mut css_path = String::from("assets/style.css");
    let mut include_runtime_injector = true;
    let mut include_cssinjs = true;
    let mut emit_cache_path_marker = false;
    let mut once = false;
    let mut cache_id = String::from("cli");

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--css" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err("--css requires a path".to_string());
                };
                css_path = value.clone();
            }
            "--skip-runtime-injector" => include_runtime_injector = false,
            "--skip-cssinjs" => include_cssinjs = false,
            "--emit-cache-path-marker" => emit_cache_path_marker = true,
            "--once" => once = true,
            "--cache-id" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err("--cache-id requires a value".to_string());
                };
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err("--cache-id cannot be empty".to_string());
                }
                cache_id = trimmed.to_string();
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            unknown => {
                return Err(format!("unknown bundle flag: {unknown}"));
            }
        }
        i += 1;
    }

    if !once && cache_id != "cli" {
        return Err("--cache-id requires --once".to_string());
    }

    let options = cssinjs::BundleBuildOptions {
        include_runtime_injector,
        include_cssinjs,
        emit_cache_path_marker,
    };

    let summary = if once {
        let mut cache = once_extract_cache()
            .lock()
            .map_err(|_| "failed to acquire bundle once cache lock".to_string())?;
        let output = cssinjs::build_bundle_once_with_cache(&mut cache, cache_id.as_str(), options);
        write_bundle_output_css(css_path.as_str(), output)
            .map_err(|err| format!("failed to write bundle (once): {err}"))?
    } else {
        cssinjs::write_bundle_files(css_path.as_str(), options)
            .map_err(|err| format!("failed to write bundle: {err}"))?
    };

    println!("cssinjs bundle generated");
    if once {
        println!("  mode: once");
        println!("  cache_id: {cache_id}");
    } else {
        println!("  mode: full");
    }
    println!("  css: {}", summary.css_path.display());
    println!("  bytes: {}", summary.css_len);
    println!("  records: {}", summary.total_record_count);
    println!("  css_xxh3_64: {}", summary.css_xxh3_64);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn once_extract_cache() -> &'static Mutex<cssinjs::BundleExtractCache> {
    static CACHE: OnceLock<Mutex<cssinjs::BundleExtractCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(cssinjs::BundleExtractCache::default()))
}

#[cfg(not(target_arch = "wasm32"))]
fn write_bundle_output_css(
    css_path: impl AsRef<Path>,
    output: cssinjs::BundleOutput,
) -> io::Result<cssinjs::BundleWriteSummary> {
    let css_path = css_path.as_ref();

    if let Some(parent) = css_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(css_path, output.css.as_bytes())?;

    Ok(cssinjs::BundleWriteSummary {
        css_path: css_path.to_path_buf(),
        css_len: output.metadata.total_css_len,
        total_record_count: output.metadata.total_record_count,
        css_xxh3_64: output.metadata.css_xxh3_64,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn print_usage() {
    eprintln!("cssinjs <command>");
    eprintln!("commands:");
    eprintln!(
        "  bundle [--css <path>] [--skip-runtime-injector] [--skip-cssinjs] [--emit-cache-path-marker] [--once] [--cache-id <id>]"
    );
}
