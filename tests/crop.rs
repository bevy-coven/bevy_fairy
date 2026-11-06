//! `crop_png` against a picture this test paints itself, so every expected pixel comes from the
//! pattern below and never from the function under test. Issue #15's acceptance items 1 to 5.

use std::io::Cursor;

use bevy_fairy::{crop_png, CropRect};
use image::{ImageFormat, Rgba, RgbaImage};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;

/// A pixel that differs from every other pixel of the 64 x 48 picture, so a crop that is off by one
/// in any direction compares unequal.
fn pattern(x: u32, y: u32) -> Rgba<u8> {
    Rgba([x as u8 * 4, y as u8 * 5, (x ^ y) as u8, 255])
}

fn picture() -> RgbaImage {
    RgbaImage::from_fn(WIDTH, HEIGHT, pattern)
}

fn png_of(image: &RgbaImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    image.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).expect("the test picture encodes");
    bytes
}

fn decode(png: &[u8]) -> RgbaImage {
    image::load_from_memory_with_format(png, ImageFormat::Png).expect("the crop is a PNG").to_rgba8()
}

#[test]
fn a_crop_is_the_source_pixels_at_the_rectangle() {
    let crop = crop_png(&png_of(&picture()), [8, 4, 40, 36]).expect("an inside rectangle crops");
    assert_eq!(crop.size, [32, 32]);
    assert_eq!(crop.picture, [WIDTH, HEIGHT]);
    let pixels = decode(&crop.png);
    assert_eq!(pixels.dimensions(), (32, 32), "the PNG is the size the answer says");
    for (x, y, pixel) in pixels.enumerate_pixels() {
        assert_eq!(*pixel, pattern(x + 8, y + 4), "crop pixel ({x}, {y})");
    }
}

#[test]
fn an_empty_reversed_negative_or_outside_rectangle_is_refused_naming_it_and_the_picture() {
    let source = png_of(&picture());
    let refused: [(CropRect, &str); 5] = [
        ([0, 0, 0, 10], "empty"),
        ([10, 0, 5, 10], "reversed"),
        ([-1, 0, 10, 10], "negative"),
        ([0, 0, 65, 10], "outside"),
        ([60, 40, 70, 50], "outside"),
    ];
    for (rect, why) in refused {
        let refusal = crop_png(&source, rect).expect_err("refused, never clamped");
        let named = format!("[{}, {}, {}, {}]", rect[0], rect[1], rect[2], rect[3]);
        assert!(refusal.contains(&named), "names the rectangle {named}: {refusal}");
        assert!(refusal.contains("64 x 48"), "names the picture's size: {refusal}");
        assert!(refusal.contains(why), "says it is {why}: {refusal}");
    }
}

#[test]
fn the_whole_picture_rectangle_returns_the_same_pixels() {
    let source = picture();
    let crop = crop_png(&png_of(&source), [0, 0, WIDTH as i64, HEIGHT as i64]).expect("the whole picture crops");
    assert_eq!(crop.size, [WIDTH, HEIGHT]);
    assert_eq!(decode(&crop.png), source);
}

#[test]
fn a_half_transparent_pixel_keeps_its_alpha() {
    let mut source = picture();
    let half = Rgba([200, 100, 50, 128]);
    source.put_pixel(10, 10, half);
    let crop = crop_png(&png_of(&source), [10, 10, 12, 12]).expect("crops");
    let pixels = decode(&crop.png);
    assert_eq!(*pixels.get_pixel(0, 0), half, "alpha 128 survives, not flattened to 255");
    assert_eq!(*pixels.get_pixel(1, 1), pattern(11, 11));
}

#[test]
fn bytes_that_are_not_a_png_are_refused() {
    for bytes in [&b""[..], b"GIF89a not a png at all", &[0x89, b'P', b'N', b'G']] {
        let refusal = crop_png(bytes, [0, 0, 1, 1]).expect_err("not a PNG");
        assert!(refusal.contains("not a readable PNG"), "{refusal}");
    }
}
