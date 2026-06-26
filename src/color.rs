use anyhow::Result;
use material_colors::color::Argb;
use material_colors::image::{FilterType, ImageReader};
use std::path::Path;

/// Picks the dominant *vibrant* color via Material You's own pipeline:
/// Celebi quantization to 128 buckets, then Score ranking (which favors
/// chromatic, well-represented colors over muddy greys). This is what makes
/// the palette pop — naive pixel-averaging just blends everything to grey.
pub fn extract_source_color(img_path: &Path) -> Result<Argb> {
    let mut image = ImageReader::open(img_path)?;
    // Downscale first: quantization over a 128x128 thumbnail is plenty
    // accurate and keeps extraction near-instant on 4K wallpapers.
    image.resize(128, 128, FilterType::Lanczos3);
    Ok(ImageReader::extract_color(&image))
}

pub fn darken(color: Argb, factor: f32) -> String {
    fmt_hex(
        (color.red as f32 * factor) as u8,
        (color.green as f32 * factor) as u8,
        (color.blue as f32 * factor) as u8,
    )
}

pub fn mute(color: Argb, factor: f32) -> String {
    fmt_hex(
        (color.red as f32 * factor + 128.0 * (1.0 - factor)) as u8,
        (color.green as f32 * factor + 128.0 * (1.0 - factor)) as u8,
        (color.blue as f32 * factor + 128.0 * (1.0 - factor)) as u8,
    )
}

fn fmt_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}
