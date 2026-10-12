//! `EXPv` → codec options: the resolution order that turns a
//! document's recorded export settings (M8 task 8's model mapping)
//! into the encoders' option structs. Explicit user flags win over
//! recorded settings, which win over codec defaults.

use crate::gif::Loop;
use crate::{Matte, gif, jpeg, png};
use doc::ExportOptions;

/// The export settings an export consults: the page's `EXPv` when it
/// has one, else the document-level defaults, else none.
#[must_use]
pub fn for_page(document: &doc::Document, page_index: usize) -> Option<&ExportOptions> {
    let page = document.pages.get(page_index);
    page.and_then(|page| page.export.as_ref())
        .or(document.export.as_ref())
}

/// Animated-GIF behaviors from `AACb`/`AADb`/`FLPi`/`PLDi`. Every
/// mapped field defaults the way the codec always has (full frames,
/// infinite loop, no dither) when the document records nothing.
#[must_use]
pub fn gif_options(export: Option<&ExportOptions>) -> gif::GifOptions {
    let Some(export) = export else {
        return gif::GifOptions::default();
    };
    let repeat = match export.frame_loop {
        None | Some(0) => Loop::Infinite,
        Some(-1 | 4294967295) => Loop::Never,
        Some(value) => Loop::Times(u16::try_from(value).unwrap_or(u16::MAX)),
    };
    gif::GifOptions {
        crop: export.anim_crop.unwrap_or(false),
        diff: export.anim_diff.unwrap_or(false),
        repeat,
        dither: export.dither.unwrap_or(0).clamp(0, 100) as u8,
    }
}

/// The JPEG settings a caller stated explicitly (`None` quality = not
/// stated); recorded `EXPv` settings fill only the unstated gaps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatedJpeg {
    /// Quality stated on the command line / dialog.
    pub quality: Option<u8>,
    /// Progressive scans stated up front.
    pub progressive: bool,
    /// Matte stated up front.
    pub matte: Matte,
}

/// JPEG options from `JPQi`/`JPPb`/`FCLi`: stated flags win, then the
/// document's recorded settings, then the codec defaults.
#[must_use]
pub fn jpeg_options(export: Option<&ExportOptions>, stated: StatedJpeg) -> jpeg::JpegOptions {
    let quality = stated
        .quality
        .or_else(|| {
            export
                .and_then(|export| export.jpeg_quality)
                .map(|value| value.clamp(1, 100) as u8)
        })
        .unwrap_or(90);
    let progressive = stated.progressive
        || export
            .and_then(|export| export.jpeg_progressive)
            .unwrap_or(false);
    let matte = match export.and_then(|export| export.matte_raw) {
        Some(raw) if stated.matte == Matte::White => matte_from_abgr(raw),
        _ => stated.matte,
    };
    jpeg::JpegOptions {
        quality,
        progressive,
        matte,
        ..jpeg::JpegOptions::default()
    }
}

/// PNG options from `GFIb`.
#[must_use]
pub fn png_options(export: Option<&ExportOptions>) -> png::PngOptions {
    png::PngOptions {
        interlaced: export.and_then(|export| export.interlaced).unwrap_or(false),
    }
}

/// The `EXPv` `FCLi` matte raw is ABGR (`a`|`b`|`g`|`r` from the top
/// byte, per the dictionary — note this inverts `TMCi`'s ARGB): an
/// opaque entry becomes its RGB triplet.
#[must_use]
pub fn matte_from_abgr(raw: i64) -> Matte {
    let word = u32::try_from(raw).unwrap_or(0xFFFF_FFFF);
    Matte::Rgb(word as u8, (word >> 8) as u8, (word >> 16) as u8)
}
