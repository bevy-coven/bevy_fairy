//! Cut a rectangle out of a picture: [`crop_png`] for PNG bytes, and the one
//! bounds rule ([`crop_view`](crate::crop::crop_view)) that `bevy_fairy/screenshot`'s `rect` uses
//! too, so a crop made here and a crop made by the game are the same crop and are refused for the
//! same reasons.
//!
//! A rectangle that reaches outside the picture is **refused, never clamped**. A clamped crop is a
//! picture of a different place than the caller asked for, and nothing in the answer would say so.
//!
//! The rectangle is in the picture's own pixels. A caller working in logical pixels at a scale
//! factor converts first; [`Crop::picture`] carries the whole picture's size so it can check.

use std::io::Cursor;

use image::{GenericImageView, ImageFormat, SubImage};

/// A rectangle in picture pixels: `[x0, y0, x1, y1]`, origin at the top left, `x1` and `y1`
/// exclusive. `[8, 4, 40, 36]` is the 32 x 32 square whose top-left pixel is (8, 4).
///
/// Signed, so that a negative origin arrives as what it is and is refused by name instead of
/// failing to deserialise.
pub type CropRect = [i64; 4];

/// A crop of a PNG, as [`crop_png`] returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crop {
    /// The crop, encoded as an RGBA8 PNG.
    pub png: Vec<u8>,
    /// The crop's size: `[width, height]`, in pixels.
    pub size: [u32; 2],
    /// The whole picture's size: `[width, height]`, in pixels.
    pub picture: [u32; 2],
}

/// Cut `rect` out of a PNG and encode it as an RGBA8 PNG. Alpha is kept as it is: a transparent
/// pixel stays transparent.
///
/// Refuses, with a sentence naming the rectangle and the picture's size, a rectangle that is empty,
/// reversed, has a negative origin or reaches outside the picture (see [`crop_view`]); and bytes
/// that are not a PNG. Never clamps.
///
/// ```
/// # fn main() -> Result<(), String> {
/// # let mut source = Vec::new();
/// # image::RgbaImage::new(64, 48)
/// #     .write_to(&mut std::io::Cursor::new(&mut source), image::ImageFormat::Png)
/// #     .map_err(|e| e.to_string())?;
/// let crop = bevy_fairy::crop_png(&source, [8, 4, 40, 36])?;
/// assert_eq!(crop.size, [32, 32]);
/// assert_eq!(crop.picture, [64, 48]);
///
/// let refusal = bevy_fairy::crop_png(&source, [60, 40, 70, 50]).unwrap_err();
/// assert!(refusal.contains("[60, 40, 70, 50]") && refusal.contains("64 x 48"));
/// # Ok(())
/// # }
/// ```
pub fn crop_png(png: &[u8], rect: CropRect) -> Result<Crop, String> {
    let picture = image::load_from_memory_with_format(png, ImageFormat::Png)
        .map_err(|e| format!("cannot crop: the bytes are not a readable PNG ({e})"))?
        .to_rgba8();
    let cropped = crop_view(&picture, rect)?.to_image();
    let size = [cropped.width(), cropped.height()];
    let mut encoded = Cursor::new(Vec::new());
    cropped
        .write_to(&mut encoded, ImageFormat::Png)
        .map_err(|e| format!("could not encode the {} x {} crop as PNG: {e}", size[0], size[1]))?;
    Ok(Crop { png: encoded.into_inner(), size, picture: [picture.width(), picture.height()] })
}

/// The part of `picture` that `rect` names, or why it names none. This is the one bounds rule:
/// [`crop_png`] and `bevy_fairy/screenshot`'s `rect` both cut through it.
///
/// Refused, in this order: a negative origin (`x0 < 0` or `y0 < 0`), a reversed rectangle
/// (`x1 < x0` or `y1 < y0`), an empty one (`x0 == x1` or `y0 == y1`), and one whose far edge lies
/// past the picture (`x1 > width` or `y1 > height`). The rectangle that is exactly the whole
/// picture is allowed.
pub fn crop_view<I: GenericImageView>(picture: &I, rect: CropRect) -> Result<SubImage<&I>, String> {
    let (width, height) = picture.dimensions();
    let [x0, y0, x1, y1] = rect;
    let refuse = |why: &str| {
        Err(format!(
            "rect {rect:?} ([x0, y0, x1, y1], x1 and y1 exclusive) {why}; the picture is {width} x {height}, \
             so a rect inside it has 0 <= x0 < x1 <= {width} and 0 <= y0 < y1 <= {height}"
        ))
    };
    if x0 < 0 || y0 < 0 {
        return refuse("has a negative origin");
    }
    if x1 < x0 || y1 < y0 {
        return refuse("is reversed (x1 < x0 or y1 < y0)");
    }
    if x1 == x0 || y1 == y0 {
        return refuse("is empty (x0 == x1 or y0 == y1)");
    }
    if x1 > i64::from(width) || y1 > i64::from(height) {
        return refuse("reaches outside the picture, and a crop is never clamped");
    }
    // Every coordinate is now within 0..=width or 0..=height, both u32, so these cannot fail.
    let to_u32 = |v: i64| u32::try_from(v).map_err(|_| format!("rect {rect:?}: {v} is not a pixel coordinate"));
    let (x, y) = (to_u32(x0)?, to_u32(y0)?);
    Ok(picture.view(x, y, to_u32(x1)? - x, to_u32(y1)? - y))
}
