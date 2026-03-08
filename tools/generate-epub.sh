#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HTML_DIR="${ROOT_DIR}/tmp/epub-html"
DIST_DIR="${ROOT_DIR}/dist"
EPUB_PATH="${DIST_DIR}/the-rust-programming-language.epub"

mkdir -p "${HTML_DIR}" "${DIST_DIR}"

echo "[1/4] Building HTML with mdbook"
(
    cd "${ROOT_DIR}"
    mdbook build -d "${HTML_DIR}"
)

echo "[2/4] Building EPUB generator binary"
(
    cd "${ROOT_DIR}"
    cargo build -p rust-book-tools --bin mdbook_epub
)

echo "[3/4] Generating EPUB"
(
    cd "${ROOT_DIR}"
    cargo run -p rust-book-tools --bin mdbook_epub -- \
        --book-dir "${HTML_DIR}" \
        --summary "${ROOT_DIR}/src/SUMMARY.md" \
        --book-toml "${ROOT_DIR}/book.toml" \
        --output "${EPUB_PATH}" \
        --validate \
        "$@"
)

echo "[4/4] Verifying ZIP integrity"
unzip -t "${EPUB_PATH}" >/dev/null

echo "EPUB generated successfully: ${EPUB_PATH}"
