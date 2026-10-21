//! Tests of the `.wsz` skin format.

use std::io::Write;

use winrust_skin::{load_wsz_bytes, Rect};

/// Encodes an RGBA image as a BMP.
fn encode_bmp(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut image = image::RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let index = ((y * width + x) * 4) as usize;
            image.put_pixel(
                x,
                y,
                image::Rgba(rgba[index..index + 4].try_into().unwrap()),
            );
        }
    }

    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Bmp,
        )
        .unwrap();
    bytes
}

/// Builds a `.wsz` archive.
fn build_archive(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }
    buffer
}

#[test]
fn loads_a_skin_archive() {
    let pixels = vec![255u8; 4 * 4 * 4];
    let main = encode_bmp(4, 4, &pixels);
    let pledit =
        b"[Text]\nNormal=#00FF00\nCurrent=#FFFFFF\nSelected=#0000FF\nNormalBG=#101010\n".to_vec();
    let gen = b"0,255,0,255,0,0,255,0,0,0,0,0,255,255,255,0,255,255,0,255,0,255,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0".to_vec();

    let archive = build_archive(&[("main.bmp", main), ("pledit.txt", pledit), ("gen.txt", gen)]);

    let skin = load_wsz_bytes(&archive, "test skin").unwrap();

    assert_eq!(skin.name, "test skin");
    assert!(skin.has_image("main.bmp"));
    assert!(skin.has_image("MAIN.BMP"), "the lookup is case insensitive");

    let image = skin.image("main.bmp").unwrap();
    assert_eq!(image.width, 4);
    assert_eq!(image.height, 4);
    assert_eq!(image.pixel(0, 0), Some([255, 255, 255, 255]));

    assert_eq!(skin.playlist.foreground.r, 0);
    assert_eq!(skin.playlist.foreground.g, 255);
    assert_eq!(
        skin.playlist.current,
        winrust_skin::Color::rgb(255, 255, 255)
    );
    assert_eq!(skin.general[0], winrust_skin::Color::rgb(0, 255, 0));
    assert_eq!(skin.general[1], winrust_skin::Color::rgb(255, 0, 0));
}

#[test]
fn ignores_unknown_entries() {
    let archive = build_archive(&[
        ("readme.txt", b"nothing to see here".to_vec()),
        ("nested/folder/main.bmp", encode_bmp(2, 2, &[0u8; 16])),
    ]);

    let skin = load_wsz_bytes(&archive, "test").unwrap();
    assert!(skin.has_image("main.bmp"));
    assert_eq!(skin.image("main.bmp").unwrap().width, 2);
}

#[test]
fn rejects_a_broken_archive() {
    assert!(load_wsz_bytes(b"this is not a zip file", "broken").is_err());
}

#[test]
fn the_default_skin_is_complete() {
    let skin = winrust_skin::default_skin();
    for name in [
        "main.bmp",
        "titlebar.bmp",
        "cbuttons.bmp",
        "numbers.bmp",
        "posbar.bmp",
        "volume.bmp",
        "balance.bmp",
        "eqmain.bmp",
        "eq_ex.bmp",
        "pledit.bmp",
    ] {
        let image = skin
            .image(name)
            .unwrap_or_else(|| panic!("{name} is missing"));
        assert!(!image.is_empty(), "{name} is empty");
    }

    // The main window has to be the classic size.
    let main = skin.image("main.bmp").unwrap();
    assert_eq!(main.width, 275);
    assert_eq!(main.height, 116);
}

#[test]
fn crops_sprites_out_of_a_bitmap() {
    let mut pixels = vec![0u8; 4 * 4 * 4];
    pixels[0..4].copy_from_slice(&[1, 2, 3, 255]);
    let image = winrust_skin::Image::new(4, 4);
    let mut image = image;
    image.pixels = pixels;

    let sprite = image.crop(Rect::new(0, 0, 2, 2)).unwrap();
    assert_eq!(sprite.pixel(0, 0), Some([1, 2, 3, 255]));
    assert_eq!(sprite.pixel(1, 1), Some([0, 0, 0, 0]));
}
