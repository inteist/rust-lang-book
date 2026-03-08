use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[path = "mdbook_epub/svg.rs"]
mod svg;

use html_escape::decode_html_entities;
use regex::{Captures, Regex};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{IncludeBackground, styled_line_to_highlighted_html};
use syntect::parsing::SyntaxSet;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use toml::Value;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::svg::render_svg_to_png;

const USAGE: &str = "
Build an EPUB from mdBook HTML output.
Usage:
  mdbook_epub --book-dir <dir> --summary <summary> --book-toml <book_toml> --output <epub_path> [--validate]
  mdbook_epub --validate-only --output <epub_path>
  mdbook_epub (-h | --help)
Options:
  --book-dir <dir>          Path to built mdBook HTML output.
  --summary <summary>       Path to SUMMARY.md used to drive chapter order.
  --book-toml <book_toml>   Path to book.toml for title/authors metadata.
  --output <epub_path>      Target EPUB path.
  --validate                Validate generated EPUB after writing.
  --validate-only           Validate an existing EPUB and exit.
  -h --help                 Show this screen.
";

#[derive(Debug)]
struct Args {
    book_dir: Option<String>,
    summary: Option<String>,
    book_toml: Option<String>,
    output: Option<String>,
    validate: bool,
    validate_only: bool,
}

#[derive(Debug, Clone)]
struct SummaryEntry {
    title: String,
    md_path: String,
}

#[derive(Debug, Clone)]
struct Chapter {
    id: String,
    title: String,
    source_html: String,
    xhtml_path: String,
    content: String,
}

#[derive(Debug, Clone)]
struct BookMetadata {
    title: String,
    authors: Vec<String>,
}

#[derive(Debug, Clone)]
struct ManifestItem {
    id: String,
    href: String,
    media_type: String,
    properties: Option<String>,
}

fn main() -> Result<(), String> {
    let args = parse_args()?;

    let output = PathBuf::from(
        args.output
            .as_deref()
            .ok_or_else(|| "--output is required".to_string())?,
    );

    if args.validate_only {
        validate_epub(&output)?;
        println!("Validated EPUB: {}", output.display());
        return Ok(());
    }

    let book_dir = args
        .book_dir
        .as_deref()
        .ok_or_else(|| "--book-dir is required".to_string())?;
    let summary = args
        .summary
        .as_deref()
        .ok_or_else(|| "--summary is required".to_string())?;
    let book_toml = args
        .book_toml
        .as_deref()
        .ok_or_else(|| "--book-toml is required".to_string())?;

    let metadata = parse_book_metadata(Path::new(book_toml))?;
    let summary_entries = parse_summary(Path::new(summary))?;
    let chapters = build_chapters(Path::new(book_dir), &summary_entries)?;
    let (asset_items, asset_data) = collect_assets(Path::new(book_dir))?;

    if chapters.is_empty() {
        return Err(
            "No chapters found. Check --summary and --book-dir.".to_string()
        );
    }

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!("Could not create output dir {}: {e}", parent.display())
        })?;
    }

    write_epub(&output, &metadata, &chapters, &asset_items, &asset_data)?;

    if args.validate {
        validate_epub(&output)?;
    }

    println!(
        "Generated EPUB: {} ({} chapters, {} assets)",
        output.display(),
        chapters.len(),
        asset_items.len()
    );

    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        book_dir: None,
        summary: None,
        book_toml: None,
        output: None,
        validate: false,
        validate_only: false,
    };

    let argv: Vec<String> = env::args().skip(1).collect();
    if argv.is_empty() {
        return Err(USAGE.to_string());
    }

    let mut i = 0usize;
    while i < argv.len() {
        match argv[i].as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--book-dir" => {
                i += 1;
                let Some(value) = argv.get(i) else {
                    return Err("--book-dir requires a value".to_string());
                };
                args.book_dir = Some(value.clone());
            }
            "--summary" => {
                i += 1;
                let Some(value) = argv.get(i) else {
                    return Err("--summary requires a value".to_string());
                };
                args.summary = Some(value.clone());
            }
            "--book-toml" => {
                i += 1;
                let Some(value) = argv.get(i) else {
                    return Err("--book-toml requires a value".to_string());
                };
                args.book_toml = Some(value.clone());
            }
            "--output" => {
                i += 1;
                let Some(value) = argv.get(i) else {
                    return Err("--output requires a value".to_string());
                };
                args.output = Some(value.clone());
            }
            "--validate" => {
                args.validate = true;
            }
            "--validate-only" => {
                args.validate_only = true;
            }
            other => {
                return Err(format!("Unknown argument: {other}\n\n{USAGE}"));
            }
        }
        i += 1;
    }

    Ok(args)
}

fn parse_book_metadata(path: &Path) -> Result<BookMetadata, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let value: Value = raw.parse::<Value>().map_err(|e| {
        format!("Failed to parse {} as TOML: {e}", path.display())
    })?;

    let title = value
        .get("book")
        .and_then(|v| v.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("The Rust Programming Language")
        .to_string();

    let authors = value
        .get("book")
        .and_then(|v| v.get("authors"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(BookMetadata { title, authors })
}

fn parse_summary(path: &Path) -> Result<Vec<SummaryEntry>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let mut entries = Vec::new();
    let link_re = Regex::new(r#"\[(?P<title>[^\]]+)\]\((?P<link>[^)]+)\)"#)
        .map_err(|e| format!("Bad summary regex: {e}"))?;

    for line in raw.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("- ") && !trimmed.starts_with('[') {
            continue;
        }

        if let Some(caps) = link_re.captures(trimmed) {
            let title = caps["title"].trim().to_string();
            let mut link = caps["link"].trim().to_string();
            if let Some((base, _)) = link.split_once('#') {
                link = base.to_string();
            }
            if let Some((base, _)) = link.split_once('?') {
                link = base.to_string();
            }
            if link.starts_with("http://")
                || link.starts_with("https://")
                || link.is_empty()
            {
                continue;
            }
            if !link.ends_with(".md") {
                continue;
            }

            entries.push(SummaryEntry {
                title,
                md_path: link,
            });
        }
    }

    Ok(entries)
}

fn build_chapters(
    book_dir: &Path,
    summary_entries: &[SummaryEntry],
) -> Result<Vec<Chapter>, String> {
    let mut chapters = Vec::new();
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme = preferred_highlight_theme()?;
    let chapter_path_map = build_chapter_path_map(summary_entries);

    for (idx, entry) in summary_entries.iter().enumerate() {
        let source_html = md_to_html(&entry.md_path);
        let source_path = book_dir.join(&source_html);

        if !source_path.exists() {
            continue;
        }

        let html = fs::read_to_string(&source_path).map_err(|e| {
            format!("Failed reading {}: {e}", source_path.display())
        })?;

        let title =
            parse_html_title(&html).unwrap_or_else(|| entry.title.clone());
        let main = extract_main_content(&html).ok_or_else(|| {
            format!(
                "Could not find <main> content in {}",
                source_path.display()
            )
        })?;

        let highlighted_main =
            highlight_code_blocks(&main, &syntax_set, &theme)?;
        let rewritten_main =
            rewrite_content_urls(&highlighted_main, &chapter_path_map);
        let chapter_name =
            format!("{:03}_{}.xhtml", idx + 1, slugify(&source_html));
        let xhtml_path = format!("chapters/{chapter_name}");

        chapters.push(Chapter {
            id: format!("chap{:03}", idx + 1),
            title,
            source_html,
            xhtml_path,
            content: rewritten_main,
        });
    }

    Ok(chapters)
}

fn collect_assets(
    book_dir: &Path,
) -> Result<(Vec<ManifestItem>, BTreeMap<String, Vec<u8>>), String> {
    let mut items = Vec::new();
    let mut data = BTreeMap::new();
    let mut seen = BTreeSet::new();

    for entry in WalkDir::new(book_dir).min_depth(1).into_iter() {
        let entry = entry.map_err(|e| {
            format!("Failed to walk {}: {e}", book_dir.display())
        })?;
        if entry.file_type().is_dir() {
            continue;
        }

        let rel = entry
            .path()
            .strip_prefix(book_dir)
            .map_err(|e| format!("Could not strip prefix: {e}"))?
            .to_string_lossy()
            .replace('\\', "/");

        let Some(ext) = entry.path().extension().and_then(|s| s.to_str())
        else {
            continue;
        };

        if ext.eq_ignore_ascii_case("html") {
            continue;
        }

        if rel == "mimetype" {
            continue;
        }

        let href = format!("book/{rel}");
        if seen.contains(&href) {
            continue;
        }
        seen.insert(href.clone());

        let media_type = media_type_for_path(&href).to_string();
        let id = format!("asset{}", seen.len());

        let bytes = fs::read(entry.path()).map_err(|e| {
            format!("Failed reading asset {}: {e}", entry.path().display())
        })?;

        items.push(ManifestItem {
            id,
            href: href.clone(),
            media_type,
            properties: None,
        });

        data.insert(href, bytes);

        if ext.eq_ignore_ascii_case("svg") {
            let png_href = format!("{}.png", rel.trim_end_matches(".svg"));
            let png_href = format!("book/{png_href}");

            if !seen.contains(&png_href) {
                let png_bytes = render_svg_to_png(
                    data.get(&format!("book/{rel}"))
                        .expect("svg bytes inserted into map"),
                )
                .map_err(|e| {
                    format!(
                        "Failed converting SVG asset {} to PNG: {e}",
                        entry.path().display()
                    )
                })?;

                seen.insert(png_href.clone());
                items.push(ManifestItem {
                    id: format!("asset{}", seen.len()),
                    href: png_href.clone(),
                    media_type: "image/png".to_string(),
                    properties: None,
                });
                data.insert(png_href, png_bytes);
            }
        }
    }

    items.sort_by(|a, b| a.href.cmp(&b.href));
    Ok((items, data))
}

fn write_epub(
    output: &Path,
    metadata: &BookMetadata,
    chapters: &[Chapter],
    assets: &[ManifestItem],
    asset_data: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let file = fs::File::create(output)
        .map_err(|e| format!("Failed to create {}: {e}", output.display()))?;
    let mut zip = ZipWriter::new(file);

    let stored = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", stored)
        .map_err(|e| format!("Failed to write mimetype: {e}"))?;
    zip.write_all(b"application/epub+zip")
        .map_err(|e| format!("Failed to write mimetype bytes: {e}"))?;

    let deflated = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated);

    zip.start_file("META-INF/container.xml", deflated)
        .map_err(|e| format!("Failed to write container.xml entry: {e}"))?;
    zip.write_all(container_xml().as_bytes())
        .map_err(|e| format!("Failed writing container.xml: {e}"))?;

    let css_path = "styles/epub.css";
    let css = default_epub_css();
    zip.start_file(format!("OEBPS/{css_path}"), deflated)
        .map_err(|e| format!("Failed to write css entry: {e}"))?;
    zip.write_all(css.as_bytes())
        .map_err(|e| format!("Failed writing css: {e}"))?;

    for chapter in chapters {
        let chapter_doc = chapter_xhtml(chapter, assets, css_path);
        zip.start_file(format!("OEBPS/{}", chapter.xhtml_path), deflated)
            .map_err(|e| {
                format!(
                    "Failed to write chapter entry {}: {e}",
                    chapter.xhtml_path
                )
            })?;
        zip.write_all(chapter_doc.as_bytes()).map_err(|e| {
            format!("Failed writing chapter {}: {e}", chapter.xhtml_path)
        })?;
    }

    for asset in assets {
        let Some(bytes) = asset_data.get(&asset.href) else {
            return Err(format!("Missing bytes for asset {}", asset.href));
        };

        zip.start_file(format!("OEBPS/{}", asset.href), deflated)
            .map_err(|e| {
                format!("Failed to write asset entry {}: {e}", asset.href)
            })?;
        zip.write_all(bytes)
            .map_err(|e| format!("Failed writing asset {}: {e}", asset.href))?;
    }

    let nav = nav_xhtml(chapters);
    zip.start_file("OEBPS/nav.xhtml", deflated)
        .map_err(|e| format!("Failed to write nav.xhtml entry: {e}"))?;
    zip.write_all(nav.as_bytes())
        .map_err(|e| format!("Failed writing nav.xhtml: {e}"))?;

    let ncx = toc_ncx(metadata, chapters);
    zip.start_file("OEBPS/toc.ncx", deflated)
        .map_err(|e| format!("Failed to write toc.ncx entry: {e}"))?;
    zip.write_all(ncx.as_bytes())
        .map_err(|e| format!("Failed writing toc.ncx: {e}"))?;

    let opf = content_opf(metadata, chapters, assets, css_path);
    zip.start_file("OEBPS/content.opf", deflated)
        .map_err(|e| format!("Failed to write content.opf entry: {e}"))?;
    zip.write_all(opf.as_bytes())
        .map_err(|e| format!("Failed writing content.opf: {e}"))?;

    zip.finish()
        .map_err(|e| format!("Failed to finalize EPUB zip: {e}"))?;

    Ok(())
}

fn validate_epub(path: &Path) -> Result<(), String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
    let mut archive = ZipArchive::new(file).map_err(|e| {
        format!("{} is not a valid zip archive: {e}", path.display())
    })?;

    for required in [
        "mimetype",
        "META-INF/container.xml",
        "OEBPS/content.opf",
        "OEBPS/toc.ncx",
        "OEBPS/nav.xhtml",
        "OEBPS/styles/epub.css",
    ] {
        archive
            .by_name(required)
            .map_err(|_| format!("Missing required EPUB entry: {required}"))?;
    }

    let mut chapter_count = 0usize;
    let mut image_count = 0usize;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed reading zip entry {i}: {e}"))?;
        let name = entry.name();

        if name.starts_with("OEBPS/chapters/") && name.ends_with(".xhtml") {
            chapter_count += 1;
        }

        if name.starts_with("OEBPS/book/") {
            let mt = media_type_for_path(name);
            if mt.starts_with("image/") {
                image_count += 1;
            }
        }
    }

    if chapter_count == 0 {
        return Err(
            "EPUB has no chapter XHTML files in OEBPS/chapters".to_string()
        );
    }

    if image_count == 0 {
        return Err("EPUB has no bundled images under OEBPS/book".to_string());
    }

    let mut mimetype = archive
        .by_name("mimetype")
        .map_err(|e| format!("Failed to read mimetype entry: {e}"))?;
    let mut buffer = String::new();
    io::Read::read_to_string(&mut mimetype, &mut buffer)
        .map_err(|e| format!("Failed reading mimetype contents: {e}"))?;
    if buffer.trim() != "application/epub+zip" {
        return Err(
            "mimetype entry does not contain application/epub+zip".to_string()
        );
    }

    Ok(())
}

fn container_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
        <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#
}

fn content_opf(
    metadata: &BookMetadata,
    chapters: &[Chapter],
    assets: &[ManifestItem],
    css_path: &str,
) -> String {
    let mut manifest = String::new();
    manifest.push_str("    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n");
    manifest.push_str("    <item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>\n");
    manifest.push_str(&format!(
        "    <item id=\"epub_css\" href=\"{}\" media-type=\"text/css\"/>\n",
        xml_escape(css_path)
    ));

    for chapter in chapters {
        manifest.push_str(&format!(
            "    <item id=\"{}\" href=\"{}\" media-type=\"application/xhtml+xml\"/>\n",
            xml_escape(&chapter.id),
            xml_escape(&chapter.xhtml_path)
        ));
    }

    for item in assets {
        let properties = item
            .properties
            .as_ref()
            .map(|p| format!(" properties=\"{}\"", xml_escape(p)))
            .unwrap_or_default();

        manifest.push_str(&format!(
            "    <item id=\"{}\" href=\"{}\" media-type=\"{}\"{} />\n",
            xml_escape(&item.id),
            xml_escape(&item.href),
            xml_escape(&item.media_type),
            properties
        ));
    }

    let mut spine = String::new();
    for chapter in chapters {
        spine.push_str(&format!(
            "    <itemref idref=\"{}\"/>\n",
            xml_escape(&chapter.id)
        ));
    }

    let mut creators = String::new();
    for author in &metadata.authors {
        creators.push_str(&format!(
            "    <dc:creator>{}</dc:creator>\n",
            xml_escape(author)
        ));
    }

    let identifier = format!("rust-book-{}", slugify(&metadata.title));
    let modified = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\" xml:lang=\"en\">\n\
    <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
    <dc:identifier id=\"bookid\">{identifier}</dc:identifier>\n\
    <dc:title>{title}</dc:title>\n\
{creators}    <dc:language>en</dc:language>\n\
        <meta property=\"dcterms:modified\">{modified}</meta>\n\
  </metadata>\n\
  <manifest>\n\
{manifest}  </manifest>\n\
  <spine toc=\"ncx\">\n\
{spine}  </spine>\n\
</package>\n",
        identifier = xml_escape(&identifier),
        title = xml_escape(&metadata.title),
        creators = creators,
        modified = xml_escape(&modified),
        manifest = manifest,
        spine = spine,
    )
}

fn nav_xhtml(chapters: &[Chapter]) -> String {
    let mut body = String::new();
    body.push_str("<ol>\n");
    for chapter in chapters {
        body.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>\n",
            xml_escape(&chapter.xhtml_path),
            xml_escape(&chapter.title)
        ));
    }

    body.push_str("</ol>\n");

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE html>\n\
<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"en\">\n\
<head><title>Table of Contents</title></head>\n\
<body>\n\
<nav epub:type=\"toc\" id=\"toc\">\n\
<h1>Table of Contents</h1>\n\
{}\
</nav>\n\
</body>\n\
</html>\n",
        body
    )
}

fn toc_ncx(metadata: &BookMetadata, chapters: &[Chapter]) -> String {
    let mut nav_points = String::new();
    for (idx, chapter) in chapters.iter().enumerate() {
        nav_points.push_str(&format!(
            "  <navPoint id=\"{}\" playOrder=\"{}\">\n\
    <navLabel><text>{}</text></navLabel>\n\
    <content src=\"{}\"/>\n\
  </navPoint>\n",
            xml_escape(&chapter.id),
            idx + 1,
            xml_escape(&chapter.title),
            xml_escape(&chapter.xhtml_path)
        ));
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n\
  <head>\n\
    <meta name=\"dtb:uid\" content=\"{}\"/>\n\
  </head>\n\
  <docTitle><text>{}</text></docTitle>\n\
  <navMap>\n\
{}  </navMap>\n\
</ncx>\n",
        xml_escape(&format!("rust-book-{}", slugify(&metadata.title))),
        xml_escape(&metadata.title),
        nav_points
    )
}

fn chapter_xhtml(
    chapter: &Chapter,
    _assets: &[ManifestItem],
    css_path: &str,
) -> String {
    let mut css_links = String::new();
    css_links.push_str(&format!(
        "<link rel=\"stylesheet\" type=\"text/css\" href=\"../{}\"/>\n",
        xml_escape(css_path)
    ));

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE html>\n\
<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"en\">\n\
<head>\n\
<meta charset=\"utf-8\"/>\n\
<title>{}</title>\n\
{}\
</head>\n\
<body>\n\
<article class=\"chapter\" data-source=\"{}\">\n\
{}\
</article>\n\
</body>\n\
</html>\n",
        xml_escape(&chapter.title),
        css_links,
        xml_escape(&chapter.source_html),
        chapter.content
    )
}

fn default_epub_css() -> &'static str {
    r#"body {
    margin: 0;
    padding: 1rem;
    font-family: Georgia, "Times New Roman", serif;
    line-height: 1.55;
    column-count: 1 !important;
    column-width: auto !important;
}

article.chapter {
    margin: 0;
    max-width: none;
    width: auto;
    padding: 0;
    column-count: 1 !important;
    column-width: auto !important;
}

pre,
code {
  font-family: ui-monospace, Menlo, Monaco, Consolas, monospace;
}

pre {
    white-space: pre;
    overflow-x: auto;
  padding: 0.75rem;
    border: 1px solid #d7d7d7;
  border-radius: 0.25rem;
    background: #f8f8f8;
}

pre code {
    display: block;
}

.header {
    text-decoration: none;
}

.boring {
    opacity: 0.65;
}

.table-wrapper {
    overflow-x: auto;
}

img,
svg {
  max-width: 100%;
  height: auto;
    break-inside: avoid;
}
"#
}

fn preferred_highlight_theme() -> Result<Theme, String> {
    let themes = ThemeSet::load_defaults();
    for name in ["InspiredGitHub", "base16-ocean.light", "Solarized (light)"] {
        if let Some(theme) = themes.themes.get(name) {
            return Ok(theme.clone());
        }
    }

    themes
        .themes
        .values()
        .next()
        .cloned()
        .ok_or_else(|| "No syntax highlighting themes available".to_string())
}

fn highlight_code_blocks(
    html_fragment: &str,
    syntax_set: &SyntaxSet,
    theme: &Theme,
) -> Result<String, String> {
    let code_re = Regex::new(
                r#"(?s)<pre(?P<pre_attrs>[^>]*)>\s*<code(?P<code_attrs>[^>]*)>(?P<code>.*?)</code>\s*</pre>"#,
        )
        .map_err(|e| format!("Bad code block regex: {e}"))?;

    let mut out = String::with_capacity(html_fragment.len() + 4096);
    let mut last_end = 0usize;

    for caps in code_re.captures_iter(html_fragment) {
        let Some(m) = caps.get(0) else {
            continue;
        };

        out.push_str(&html_fragment[last_end..m.start()]);

        let code_attrs =
            caps.name("code_attrs").map(|m| m.as_str()).unwrap_or("");
        let raw_code = caps.name("code").map(|m| m.as_str()).unwrap_or("");
        let lang = language_from_code_attrs(code_attrs).unwrap_or("txt");
        let highlighted = highlight_source(raw_code, lang, syntax_set, theme)?;

        out.push_str(&highlighted);
        last_end = m.end();
    }

    out.push_str(&html_fragment[last_end..]);
    Ok(out)
}

fn language_from_code_attrs(code_attrs: &str) -> Option<&str> {
    let class_re = Regex::new(r#"class\s*=\s*\"([^\"]+)\""#).ok()?;
    let classes = class_re.captures(code_attrs)?.get(1)?.as_str();

    for class_name in classes.split_whitespace() {
        if let Some(lang) = class_name.strip_prefix("language-") {
            if !lang.is_empty() {
                return Some(lang);
            }
        }
    }

    None
}

fn highlight_source(
    raw_code_html: &str,
    language: &str,
    syntax_set: &SyntaxSet,
    theme: &Theme,
) -> Result<String, String> {
    let html_tag_re = Regex::new(r#"(?s)<[^>]+>"#)
        .map_err(|e| format!("Bad inline markup regex: {e}"))?;
    let unwrapped = html_tag_re.replace_all(raw_code_html, "");
    let decoded = decode_html_entities(&unwrapped).to_string();
    let syntax = syntax_set
        .find_syntax_by_token(language)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut out = String::from("<pre class=\"playground\"><code>");
    for line in decoded.split_inclusive('\n') {
        let regions = highlighter
            .highlight_line(line, syntax_set)
            .map_err(|e| format!("Syntax highlighting failed: {e}"))?;
        let html =
            styled_line_to_highlighted_html(&regions, IncludeBackground::No)
                .map_err(|e| {
                    format!("Highlight HTML conversion failed: {e}")
                })?;
        out.push_str(&html);
    }
    out.push_str("</code></pre>");

    Ok(out)
}

fn md_to_html(md_path: &str) -> String {
    format!("{}.html", md_path.trim_end_matches(".md"))
}

fn build_chapter_path_map(
    summary_entries: &[SummaryEntry],
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (idx, entry) in summary_entries.iter().enumerate() {
        let source_html = md_to_html(&entry.md_path);
        let chapter_name =
            format!("{:03}_{}.xhtml", idx + 1, slugify(&source_html));
        map.insert(source_html, format!("{chapter_name}"));
    }
    map
}

fn parse_html_title(html: &str) -> Option<String> {
    let title_re = Regex::new(r#"(?s)<title>(?P<title>.*?)</title>"#).ok()?;
    let captured = title_re.captures(html)?;
    let full = captured.name("title")?.as_str().trim();
    Some(full.split(" - ").next().unwrap_or(full).trim().to_string())
}

fn extract_main_content(html: &str) -> Option<String> {
    let main_re = Regex::new(r#"(?s)<main>\s*(?P<main>.*?)\s*</main>"#).ok()?;
    main_re
        .captures(html)
        .and_then(|caps| caps.name("main").map(|m| m.as_str().to_string()))
}

fn rewrite_content_urls(
    html_fragment: &str,
    chapter_path_map: &BTreeMap<String, String>,
) -> String {
    let href_re =
        Regex::new(r#"href=\"(?P<url>[^\"]+)\""#).expect("valid href regex");
    let src_re =
        Regex::new(r#"src=\"(?P<url>[^\"]+)\""#).expect("valid src regex");

    let href_rewritten = href_re
        .replace_all(html_fragment, |caps: &Captures<'_>| {
            let url = caps.name("url").expect("url capture").as_str();
            let rewritten = rewrite_href_url(url, chapter_path_map);
            format!("href=\"{rewritten}\"")
        })
        .to_string();

    src_re
        .replace_all(&href_rewritten, |caps: &Captures<'_>| {
            let url = caps.name("url").expect("url capture").as_str();
            let rewritten = rewrite_src_url(url);
            format!("src=\"{rewritten}\"")
        })
        .to_string()
}

fn rewrite_href_url(
    href: &str,
    chapter_path_map: &BTreeMap<String, String>,
) -> String {
    if is_external_or_anchor_url(href) {
        return href.to_string();
    }

    let (path, suffix) = split_url_path_and_suffix(href);
    let normalized = normalize_local_path(path);

    if normalized.ends_with(".html") {
        if let Some(mapped) = chapter_path_map.get(&normalized) {
            return format!("{mapped}{suffix}");
        }
        return format!(
            "{}.xhtml{suffix}",
            normalized.trim_end_matches(".html")
        );
    }

    href.to_string()
}

fn rewrite_src_url(src: &str) -> String {
    if is_external_or_anchor_url(src) {
        return src.to_string();
    }

    let (path, suffix) = split_url_path_and_suffix(src);
    let normalized = normalize_local_path(path);

    let mut rewritten_path = if normalized.ends_with(".svg") {
        format!("{}.png", normalized.trim_end_matches(".svg"))
    } else {
        normalized
    };

    rewritten_path = format!("../book/{rewritten_path}");
    format!("{rewritten_path}{suffix}")
}

fn is_external_or_anchor_url(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with("#")
        || url.starts_with("javascript:")
        || url.starts_with("data:")
}

fn split_url_path_and_suffix(url: &str) -> (&str, String) {
    if let Some((base, frag)) = url.split_once('#') {
        return (base, format!("#{frag}"));
    }
    if let Some((base, query)) = url.split_once('?') {
        return (base, format!("?{query}"));
    }
    (url, String::new())
}

fn normalize_local_path(path: &str) -> String {
    path.trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }

    while out.contains("__") {
        out = out.replace("__", "_");
    }

    out.trim_matches('_').to_string()
}

fn media_type_for_path(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "xhtml" => "application/xhtml+xml",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",
        "json" => "application/json",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{
        build_chapter_path_map, extract_main_content, highlight_code_blocks,
        language_from_code_attrs, media_type_for_path, parse_summary,
        preferred_highlight_theme, render_svg_to_png, rewrite_content_urls,
        rewrite_href_url, rewrite_src_url,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};
    use syntect::parsing::SyntaxSet;

    #[test]
    fn summary_parser_collects_expected_entries() {
        let summary = "# Book\n\n[Front](title-page.md)\n- [Chapter](ch01.md)\n  - [Section](ch01-01.md)\n";
        let path = temp_path("summary_parser");
        fs::write(&path, summary).expect("write summary fixture");

        let entries = parse_summary(&path).expect("parse summary");

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].title, "Front");
        assert_eq!(entries[1].title, "Chapter");
        assert_eq!(entries[2].title, "Section");

        fs::remove_file(path).expect("cleanup summary fixture");
    }

    #[test]
    fn rewrites_html_links_but_keeps_externals() {
        let mut map = BTreeMap::new();
        map.insert(
            "ch01-00-intro.html".to_string(),
            "001_ch01_00_intro_html.xhtml".to_string(),
        );

        assert_eq!(
            rewrite_href_url("ch01-00-intro.html", &map),
            "001_ch01_00_intro_html.xhtml"
        );
        assert_eq!(
            rewrite_href_url("ch01-00-intro.html#part", &map),
            "001_ch01_00_intro_html.xhtml#part"
        );
        assert_eq!(
            rewrite_href_url("https://doc.rust-lang.org", &map),
            "https://doc.rust-lang.org"
        );
        assert_eq!(rewrite_href_url("#local", &map), "#local");
    }

    #[test]
    fn rewrites_local_image_sources_to_book_assets() {
        assert_eq!(
            rewrite_src_url("img/trpl04-02.svg"),
            "../book/img/trpl04-02.png"
        );
        assert_eq!(
            rewrite_src_url("img/trpl21-01.png"),
            "../book/img/trpl21-01.png"
        );
        assert_eq!(
            rewrite_src_url("https://example.com/image.png"),
            "https://example.com/image.png"
        );
    }

    #[test]
    fn chapter_path_map_matches_generated_filenames() {
        let entries = vec![
            super::SummaryEntry {
                title: "A".to_string(),
                md_path: "ch01-01-installation.md".to_string(),
            },
            super::SummaryEntry {
                title: "B".to_string(),
                md_path: "ch04-01-what-is-ownership.md".to_string(),
            },
        ];

        let map = build_chapter_path_map(&entries);
        assert_eq!(
            map.get("ch01-01-installation.html").map(String::as_str),
            Some("001_ch01_01_installation_html.xhtml")
        );
        assert_eq!(
            map.get("ch04-01-what-is-ownership.html")
                .map(String::as_str),
            Some("002_ch04_01_what_is_ownership_html.xhtml")
        );
    }

    #[test]
    fn extracts_main_content() {
        let html = "<html><body><main><h1>Hello</h1></main></body></html>";
        let main = extract_main_content(html).expect("extract main");
        assert_eq!(main, "<h1>Hello</h1>");
    }

    #[test]
    fn media_types_cover_common_assets() {
        assert_eq!(media_type_for_path("book/img/foo.png"), "image/png");
        assert_eq!(media_type_for_path("book/css/site.css"), "text/css");
        assert_eq!(
            media_type_for_path("chapters/one.xhtml"),
            "application/xhtml+xml"
        );
        assert_eq!(
            media_type_for_path("book/unknown.bin"),
            "application/octet-stream"
        );
    }

    #[test]
    fn language_is_detected_from_code_classes() {
        assert_eq!(
            language_from_code_attrs(" class=\"language-rust edition2024\" "),
            Some("rust")
        );
        assert_eq!(language_from_code_attrs(" class=\"ignore\" "), None);
    }

    #[test]
    fn code_blocks_are_highlighted_to_html() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme = preferred_highlight_theme().expect("theme available");
        let input = "<pre><code class=\"language-rust\">fn main() {\\n    let x = 1;\\n}\\n</code></pre>";

        let out = highlight_code_blocks(input, &syntax_set, &theme)
            .expect("highlighting succeeds");

        assert!(out.contains("<pre class=\"playground\"><code>"));
        assert!(out.contains("fn"));
        assert!(!out.contains("class=\"language-rust\""));
    }

    #[test]
    fn svg_assets_can_be_rendered_to_png() {
        let svg = br#"<svg xmlns='http://www.w3.org/2000/svg' width='8' height='8'><rect width='8' height='8' fill='red'/></svg>"#;
        let png = render_svg_to_png(svg).expect("svg renders to png");

        assert!(png.len() > 16);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn text_only_svg_renders_visible_pixels() {
        let svg = br#"<svg xmlns='http://www.w3.org/2000/svg' width='220' height='90'><text x='8' y='64' font-size='64' fill='black'>s1</text></svg>"#;
        let png = render_svg_to_png(svg).expect("text svg renders to png");

        assert!(png_has_visible_pixels(&png));
    }

    #[test]
    fn content_url_rewriter_updates_links_and_sources() {
        let mut map = BTreeMap::new();
        map.insert(
            "ch04-01-what-is-ownership.html".to_string(),
            "016_ch04_01_what_is_ownership_html.xhtml".to_string(),
        );

        let input = r#"<p><a href="ch04-01-what-is-ownership.html#the-string-type">x</a><img src="img/trpl04-02.svg"/></p>"#;
        let output = rewrite_content_urls(input, &map);

        assert!(
            output.contains(
                "href=\"016_ch04_01_what_is_ownership_html.xhtml#the-string-type\""
            )
        );
        assert!(output.contains("src=\"../book/img/trpl04-02.png\""));
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}_{nanos}.md"))
    }

    fn png_has_visible_pixels(png_bytes: &[u8]) -> bool {
        let decoder = png::Decoder::new(Cursor::new(png_bytes));
        let mut reader = decoder.read_info().expect("decode png header");
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf).expect("decode png pixels");
        let data = &buf[..frame.buffer_size()];

        match frame.color_type {
            png::ColorType::Rgba => data.chunks_exact(4).any(|px| px[3] != 0),
            png::ColorType::Rgb => data.chunks_exact(3).any(|px| px != [0, 0, 0]),
            png::ColorType::GrayscaleAlpha => {
                data.chunks_exact(2).any(|px| px[1] != 0)
            }
            png::ColorType::Grayscale => data.iter().any(|v| *v != 0),
            png::ColorType::Indexed => data.iter().any(|v| *v != 0),
        }
    }
}
