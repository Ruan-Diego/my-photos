//! One original → one JPEG: decoded by content, scaled to fit, turned upright,
//! transparency flattened on white.

use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::metadata::Orientation;
use image::{DynamicImage, ExtendedColorType, ImageError, ImageReader, Limits, RgbImage};

use super::{ThumbError, ThumbSize, JPEG_QUALITY};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub jpeg: Vec<u8>,
    /// As displayed, after the EXIF orientation.
    pub width: u32,
    pub height: u32,
}

/// A decoder that panics on a malformed file counts as a corrupt file (F36).
pub fn render(
    original: &Path,
    size: ThumbSize,
    orientation: Option<u16>,
) -> Result<Rendered, ThumbError> {
    crate::isolate::contained(|| decode_and_render(original, size, orientation)).unwrap_or(Err(ThumbError::Corrupt))
}

fn decode_and_render(
    original: &Path,
    size: ThumbSize,
    orientation: Option<u16>,
) -> Result<Rendered, ThumbError> {
    let mut reader = ImageReader::open(original)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|_| ThumbError::Unreadable)?;
    reader.limits(decode_limits());
    if reader.format().is_none() {
        return Err(ThumbError::Unsupported);
    }
    let image = reader.decode().map_err(|error| match error {
        ImageError::Unsupported(_) => ThumbError::Unsupported,
        _ => ThumbError::Corrupt,
    })?;

    let edge = size.max_edge();
    let mut image = if image.width().max(image.height()) > edge {
        image.thumbnail(edge, edge)
    } else {
        image
    };
    if let Some(orientation) = orientation
        .and_then(|value| u8::try_from(value).ok())
        .and_then(Orientation::from_exif)
    {
        image.apply_orientation(orientation);
    }

    let rgb = flatten_on_white(image);
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY)
        .encode(rgb.as_raw(), rgb.width(), rgb.height(), ExtendedColorType::Rgb8)
        .map_err(|_| ThumbError::Corrupt)?;
    Ok(Rendered {
        jpeg,
        width: rgb.width(),
        height: rgb.height(),
    })
}

/// F36: a decompression bomb (a tiny file whose header claims a gigantic image) stops at
/// the header instead of allocating. 40 000 px a side and 1 GiB cover any real photo or
/// panorama; the `image` default alone is 512 MB per allocation, with no size check.
fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_EDGE);
    limits.max_image_height = Some(MAX_EDGE);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    limits
}

const MAX_EDGE: u32 = 40_000;
const MAX_DECODE_BYTES: u64 = 1 << 30;

fn flatten_on_white(image: DynamicImage) -> RgbImage {
    if !image.color().has_alpha() {
        return image.into_rgb8();
    }
    let rgba = image.into_rgba8();
    let mut rgb = RgbImage::new(rgba.width(), rgba.height());
    for (target, source) in rgb.pixels_mut().zip(rgba.pixels()) {
        let alpha = u16::from(source[3]);
        for channel in 0..3 {
            let value = u16::from(source[channel]) * alpha + 255 * (255 - alpha);
            target[channel] = ((value + 127) / 255) as u8;
        }
    }
    rgb
}
