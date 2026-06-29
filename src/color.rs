use anyhow::Result;
use material_colors::color::Argb;
use material_colors::image::{FilterType, ImageReader};
use std::path::Path;

/// Dominant vibrant color via Material You (Celebi quantization + Score
/// ranking), which favors chromatic colors over muddy greys.
pub fn extract_source_color(img_path: &Path) -> Result<Argb> {
    let mut image = ImageReader::open(img_path)?;
    // Quantize over a 128px thumbnail — accurate enough for one accent and
    // near-instant on 4K. Triangle resampling: filter quality is irrelevant
    // when we only need the dominant color, and it's faster than Lanczos.
    image.resize(128, 128, FilterType::Triangle);
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
