#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PACKAGE="${DX_PACKAGE:-preview_app}"
CSS_PATH="${CSSINJS_RELEASE_CSS_PATH:-$ROOT/assets/style.css}"
PUBLIC_DIR="${CSSINJS_RELEASE_PUBLIC_DIR:-$ROOT/target/dx/${PACKAGE}/release/web/public}"
RUN_BUILD=1

BUNDLE_FLAGS=()

usage() {
  cat <<USAGE
Usage: cssinjs/scripts/release_web_bundle.sh [options]

Options:
  --skip-build                  Skip 'dx build' and only emit/copy css bundle
  --package <name>              Dioxus package name (default: preview_app)
  --css <path>                  Canonical css output path (default: assets/style.css)
  --public-dir <path>           Release web public dir (default: target/dx/<pkg>/release/web/public)
  --skip-runtime-injector       Forwarded to cssinjs bundle CLI
  --skip-cssinjs                Forwarded to cssinjs bundle CLI
  --emit-cache-path-marker      Forwarded to cssinjs bundle CLI
  -h, --help                    Show this help

Deterministic output contract:
  canonical css:  <repo>/assets/style.css
  release mirror: <public-dir>/assets/style.css
USAGE
}

while (($#)); do
  case "$1" in
    --skip-build)
      RUN_BUILD=0
      shift
      ;;
    --package)
      [[ $# -ge 2 ]] || { echo "--package requires a value" >&2; exit 1; }
      PACKAGE="$2"
      shift 2
      ;;
    --css)
      [[ $# -ge 2 ]] || { echo "--css requires a value" >&2; exit 1; }
      CSS_PATH="$2"
      shift 2
      ;;
    --public-dir)
      [[ $# -ge 2 ]] || { echo "--public-dir requires a value" >&2; exit 1; }
      PUBLIC_DIR="$2"
      shift 2
      ;;
    --skip-runtime-injector|--skip-cssinjs|--emit-cache-path-marker)
      BUNDLE_FLAGS+=("$1")
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ "$RUN_BUILD" -eq 1 ]]; then
  echo "[cssinjs-release] dx build --platform web -p ${PACKAGE} --release"
  dx build --platform web -p "${PACKAGE}" --release
fi

echo "[cssinjs-release] cargo run -p cssinjs -- bundle"
cargo run -p cssinjs -- bundle \
  --css "$CSS_PATH" \
  "${BUNDLE_FLAGS[@]}"

echo "[cssinjs-release] mirror bundle into release public assets"
mkdir -p "$PUBLIC_DIR/assets"
cp -f "$CSS_PATH" "$PUBLIC_DIR/assets/style.css"

echo "[cssinjs-release] done"
echo "  canonical_css=$CSS_PATH"
echo "  release_css=$PUBLIC_DIR/assets/style.css"
