//! Lossy WebP export via `libwebp-sys` (VP8 encoding).
//!
//! This module is only compiled when the `lossy-webp` feature is enabled.
//! The wasm32 target must not enable this feature — it gets the
//! "lossy WebP not supported in browser" error path instead.
//!
//! Quality range: 0..=100 (libwebp default 75).

use libwebp_sys::{WebPEncodeRGBA, WebPFree};

use crate::{ExportError, Plane};

/// WebP encoding quality. libwebp range 0..=100.
pub const MIN_QUALITY: f32 = 0.0;
pub const MAX_QUALITY: f32 = 100.0;
pub const DEFAULT_QUALITY: f32 = 75.0;

/// Encodes a plane as lossy WebP (VP8).
///
/// # Errors
/// [`ExportError::Encode`] when libwebp refuses the input.
pub fn encode(plane: &Plane<'_>, quality: f32) -> Result<Vec<u8>, ExportError> {
    let quality = quality.clamp(MIN_QUALITY, MAX_QUALITY);

    let width = plane.width.try_into().map_err(|_| ExportError::Encode {
        format: "webp",
        reason: format!("plane width {} overflows i32", plane.width),
    })?;
    let height = plane.height.try_into().map_err(|_| ExportError::Encode {
        format: "webp",
        reason: format!("plane height {} overflows i32", plane.height),
    })?;

    let rgba = plane.rgba;
    let stride = width * 4; // RGBA = 4 bytes per pixel

    let mut output_ptr: *mut u8 = std::ptr::null_mut();
    let encoded_size = unsafe {
        WebPEncodeRGBA(
            rgba.as_ptr(),
            width,
            height,
            stride,
            quality,
            &mut output_ptr,
        )
    };

    if encoded_size == 0 || output_ptr.is_null() {
        return Err(ExportError::Encode {
            format: "webp",
            reason: "libwebp: WebPEncodeRGBA produced no output".into(),
        });
    }

    // SAFETY: WebPEncodeRGBA allocates with malloc; output_ptr is valid and
    // encoded_size bytes are initialized. We own this allocation.
    let bytes = unsafe { std::slice::from_raw_parts(output_ptr, encoded_size).to_vec() };
    unsafe { WebPFree(output_ptr as *mut _) };

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn encode_lossy_produces_nonempty_webp() {
        let rgba = vec![255u8, 0, 0, 255]; // opaque red
        let plane = Plane::new(1, 1, &rgba).unwrap();
        let out = encode(&plane, 75.0).unwrap();
        assert!(!out.is_empty());
        // RIFF header + WebP signature.
        assert_eq!(&out[0..4], b"RIFF");
        assert_eq!(&out[8..12], b"WEBP");
    }

    #[test]
    fn encode_lossy_rejects_mismatched_plane() {
        // Plane says 2×2 (16 bytes) but only 4 bytes provided → Plane::new fails.
        let result = Plane::new(2, 2, &[0u8; 4]);
        assert!(result.is_err());
    }

    #[test]
    fn encode_lossy_clamps_quality() {
        let rgba = vec![255u8, 0, 0, 255];
        let plane = Plane::new(1, 1, &rgba).unwrap();
        // Out-of-range values should not error — clamp is applied.
        let out = encode(&plane, -10.0).unwrap();
        assert!(!out.is_empty());
        let out = encode(&plane, 150.0).unwrap();
        assert!(!out.is_empty());
    }
}
