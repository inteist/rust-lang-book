use resvg::{tiny_skia, usvg};
use std::sync::Arc;

/// Rasterize an SVG into PNG bytes using system fonts for text shaping.
pub fn render_svg_to_png(svg_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut options = usvg::Options::default();
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    options.fontdb = Arc::new(fontdb);

    let tree = usvg::Tree::from_data(svg_bytes, &options)
        .map_err(|e| format!("Invalid SVG: {e}"))?;
    let size = tree.size().to_int_size();

    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| {
            format!(
                "Could not create pixmap for {}x{}",
                size.width(),
                size.height()
            )
        })?;

    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap_mut);

    pixmap
        .encode_png()
        .map_err(|e| format!("Failed encoding PNG: {e}"))
}
