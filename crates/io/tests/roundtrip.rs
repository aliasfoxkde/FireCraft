//! Round-trip battery for the M8 task-1 raster exporters: every
//! encoder is proven by decoding its own output with the `image`
//! crate (an independent implementation) and comparing.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use io::bmp::{BmpDepth, BmpOptions};
use io::jpeg::{ChromaSampling, JpegOptions};
use io::tiff::{TiffCompression, TiffOptions};
use io::wbmp;
use io::{ExportError, Matte, Plane};

/// Opaque horizontal gradient with a diagonal alpha ramp: 37×23 (odd
/// width exercises BMP row padding; 851 pixels exercise WBMP row
/// packing).
fn gradient() -> Vec<u8> {
    let (width, height) = (37u32, 23u32);
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            rgba.push((x * 255 / (width - 1)) as u8);
            rgba.push((y * 255 / (height - 1)) as u8);
            rgba.push(255 - (x * 255 / (width - 1)) as u8);
            rgba.push((255 - (x + y) * 255 / (width + height - 2)) as u8);
        }
    }
    rgba
}

fn gradient_plane(rgba: &[u8]) -> Plane<'_> {
    Plane::new(37, 23, rgba).expect("gradient sizes match")
}

fn decode_rgba8(bytes: &[u8]) -> image::RgbaImage {
    image::load_from_memory(bytes)
        .expect("decoder accepts our stream")
        .to_rgba8()
}

#[test]
fn png_round_trips_exactly() {
    let rgba = gradient();
    let plane = gradient_plane(&rgba);
    let bytes = io::png::encode(&plane).expect("png encode");
    let decoded = decode_rgba8(&bytes);
    assert_eq!((decoded.width(), decoded.height()), (37, 23));
    assert_eq!(decoded.into_raw(), rgba);
}

#[test]
fn jpeg_baseline_444_stays_within_pinned_tolerance() {
    let rgba = gradient();
    let plane = gradient_plane(&rgba);
    let options = JpegOptions {
        quality: 95,
        chroma: ChromaSampling::S444,
        ..JpegOptions::default()
    };
    let bytes = io::jpeg::encode(&plane, &options).expect("jpeg encode");
    let decoded = decode_rgba8(&bytes);
    let expected = plane.flatten_rgb(options.matte);
    let (max_diff, sum_diff) = diff_stats(&expected, &decoded);
    assert!(max_diff <= 16, "max channel diff {max_diff}");
    assert!(
        sum_diff / u64::from(37u32 * 23) <= 4,
        "mean diff {}",
        sum_diff / u64::from(37u32 * 23)
    );
}

#[test]
fn jpeg_progressive_and_420_decode() {
    let rgba = gradient();
    let plane = gradient_plane(&rgba);
    let progressive = JpegOptions {
        progressive: true,
        ..JpegOptions::default()
    };
    let bytes = io::jpeg::encode(&plane, &progressive).expect("progressive encode");
    let decoded = decode_rgba8(&bytes);
    let expected = plane.flatten_rgb(progressive.matte);
    let (max_diff, _) = diff_stats(&expected, &decoded);
    assert!(max_diff <= 20, "progressive max diff {max_diff}");

    let subsampled = JpegOptions {
        quality: 80,
        chroma: ChromaSampling::S420,
        ..JpegOptions::default()
    };
    let bytes = io::jpeg::encode(&plane, &subsampled).expect("420 encode");
    let decoded = decode_rgba8(&bytes);
    let expected = plane.flatten_rgb(subsampled.matte);
    // Chroma is halved; the tolerance above the 4:4:4 window is for
    // subsampled color, not luma.
    let (max_diff, _) = diff_stats(&expected, &decoded);
    assert!(max_diff <= 24, "420 max diff {max_diff}");
}

#[test]
fn jpeg_flattens_alpha_over_the_matte() {
    let rgba = vec![255u8, 0, 0, 0]; // fully transparent red
    let plane = Plane::new(1, 1, &rgba).expect("1x1");
    let white = io::jpeg::encode(&plane, &JpegOptions::default()).expect("encode white matte");
    let decoded = decode_rgba8(&white).into_raw();
    assert!(
        decoded[0] > 240 && decoded[1] > 240 && decoded[2] > 240,
        "white matte got {decoded:?}"
    );

    let black = JpegOptions {
        matte: Matte::Black,
        ..JpegOptions::default()
    };
    let bytes = io::jpeg::encode(&plane, &black).expect("encode black matte");
    let decoded = decode_rgba8(&bytes).into_raw();
    assert!(
        decoded[0] < 16 && decoded[1] < 16 && decoded[2] < 16,
        "black matte got {decoded:?}"
    );
}

#[test]
fn jpeg_refuses_quality_zero_and_oversized_planes() {
    let rgba = gradient();
    let plane = gradient_plane(&rgba);
    let zero = JpegOptions {
        quality: 0,
        ..JpegOptions::default()
    };
    assert_eq!(
        io::jpeg::encode(&plane, &zero),
        Err(ExportError::Unsupported {
            format: "jpeg",
            reason: "quality must be 1..=100".to_owned(),
        })
    );
    // 70000 > the 65535 JPEG axis limit; zero height keeps the test
    // allocation-free.
    let wide = Plane::new(70000, 0, &[]).expect("empty plane is self-consistent");
    assert!(matches!(
        io::jpeg::encode(&wide, &JpegOptions::default()),
        Err(ExportError::TooLarge {
            format: "jpeg",
            limit: 65535,
            ..
        })
    ));
}

#[test]
fn bmp_24bit_round_trips_opaque_pixels() {
    let rgba = gradient();
    let plane = gradient_plane(&rgba);
    let opaque: Vec<u8> = plane
        .rgba
        .chunks_exact(4)
        .flat_map(|px| [px[0], px[1], px[2], 255])
        .collect();
    let opaque_plane = Plane::new(37, 23, &opaque).expect("same shape");
    let options = BmpOptions::default();
    let bytes = io::bmp::encode(&opaque_plane, &options).expect("bmp encode");
    let decoded = decode_rgba8(&bytes);
    assert_eq!(
        decoded.into_raw(),
        opaque,
        "24-bit BMP must be lossless on opaque input"
    );
}

#[test]
fn bmp_32bit_round_trips_alpha_exactly() {
    let rgba = gradient();
    let plane = gradient_plane(&rgba);
    let options = BmpOptions {
        depth: BmpDepth::Bpp32,
        ..BmpOptions::default()
    };
    let bytes = io::bmp::encode(&plane, &options).expect("bmp encode");
    let decoded = decode_rgba8(&bytes);
    assert_eq!(
        decoded.into_raw(),
        rgba,
        "V4 BITFIELDS BMP must preserve alpha"
    );
}

#[test]
fn tiff_round_trips_in_both_compressions() {
    let rgba = gradient();
    let plane = gradient_plane(&rgba);
    for compression in [TiffCompression::None, TiffCompression::Packbits] {
        let options = TiffOptions { compression };
        let bytes = io::tiff::encode(&plane, &options).expect("tiff encode");
        let decoded = decode_rgba8(&bytes);
        assert_eq!(
            decoded.into_raw(),
            rgba,
            "tiff {compression:?} must be lossless"
        );
    }
}

#[test]
fn wbmp_round_trips_through_the_threshold_policy() {
    let rgba = gradient();
    let plane = gradient_plane(&rgba);
    let bytes = wbmp::encode(&plane, Matte::White).expect("wbmp encode");
    let (width, height, rgba) = wbmp::decode(&bytes).expect("wbmp decode");
    assert_eq!((width, height), (37, 23));
    for (y, row) in rgba.chunks_exact(37 * 4).enumerate() {
        for (x, px) in row.chunks_exact(4).enumerate() {
            let expected = if plane.luma_at(x as u32, y as u32, Matte::White) >= 128 {
                255
            } else {
                0
            };
            assert_eq!(px, &[expected, expected, expected, 255], "at {x},{y}");
        }
    }
}

#[test]
fn wbmp_header_uses_multi_byte_dimensions() {
    // Width 300 needs two 7-bit groups; height 1 keeps it tiny.
    let rgba = vec![128u8; 300 * 4];
    let plane = Plane::new(300, 1, &rgba).expect("1-row plane");
    let bytes = wbmp::encode(&plane, Matte::White).expect("encode");
    assert_eq!(
        &bytes[..4],
        &[0x00, 0x00, 0xAC, 0x02],
        "300 = 0b100101100 → 0xAC 0x02"
    );
    let decoded = wbmp::decode(&bytes).expect("decode");
    assert_eq!((decoded.0, decoded.1), (300, 1));
}

#[test]
fn wbmp_decode_refuses_hostile_headers() {
    assert_eq!(
        wbmp::decode(&[0x00]),
        Err(io::ImportError::Truncated {
            format: "wbmp",
            needed: "type + fixed header octets",
        })
    );
    assert!(matches!(
        wbmp::decode(&[0x01, 0x00, 0x01, 0x01, 0xFF]),
        Err(io::ImportError::Unsupported { format: "wbmp", .. })
    ));
    assert!(matches!(
        wbmp::decode(&[0x00, 0x20, 0x01, 0x01, 0xFF]),
        Err(io::ImportError::Unsupported { format: "wbmp", .. })
    ));
    // 2^40 × 2^40 pixels must be refused without allocating.
    assert!(matches!(
        wbmp::decode(&[
            0x00, 0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x20, 0x80, 0x80, 0x80, 0x80, 0x80, 0x20
        ]),
        Err(io::ImportError::Unsupported { format: "wbmp", .. })
    ));
    // Truncated pixel array.
    assert!(matches!(
        wbmp::decode(&[0x00, 0x00, 0x08, 0x08, 0xFF]),
        Err(io::ImportError::Truncated { format: "wbmp", .. })
    ));
}

#[test]
fn plane_construction_validates_the_byte_length() {
    assert!(matches!(
        Plane::new(2, 2, &[0u8; 15]),
        Err(ExportError::PlaneSize {
            width: 2,
            height: 2,
            len: 15
        })
    ));
    assert!(Plane::new(2, 2, &[0u8; 16]).is_ok());
}

fn diff_stats(expected_rgb: &[u8], decoded: &image::RgbaImage) -> (u32, u64) {
    let raw = decoded.as_raw();
    let mut max = 0u32;
    let mut sum = 0u64;
    for (px, out) in expected_rgb.chunks_exact(3).zip(raw.chunks_exact(4)) {
        for (channel, decoded) in px.iter().zip(out[..3].iter()) {
            let diff = i32::from(*channel) - i32::from(*decoded);
            let diff = diff.unsigned_abs();
            max = max.max(diff);
            sum += u64::from(diff);
        }
    }
    (max, sum)
}
