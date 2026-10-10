//! The error type for all `fwpng` operations.

use std::fmt;

/// Everything that can go wrong while parsing a Fireworks PNG.
///
/// Offsets are byte positions: absolute file offsets for chunk-level
/// errors, chunk-payload offsets for private-chunk and node-tree errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FwPngError {
    /// The stream does not start with the PNG signature.
    BadSignature,
    /// The stream ended before a complete element was read.
    UnexpectedEof {
        /// Byte offset where reading stopped.
        offset: usize,
        /// Bytes the element needed.
        needed: usize,
    },
    /// A chunk's CRC-32 does not cover its bytes.
    BadCrc {
        /// The chunk's four-byte type.
        chunk: [u8; 4],
        /// CRC recorded in the file.
        expected: u32,
        /// CRC computed over the actual bytes.
        got: u32,
        /// File offset of the chunk's length field.
        offset: usize,
    },
    /// A chunk header or type is malformed.
    BadChunk {
        /// Chunk type when known, else zeros.
        chunk: [u8; 4],
        /// File offset of the chunk's length field.
        offset: usize,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A zlib stream failed to inflate.
    Inflate {
        /// Payload-relative offset of the stream start.
        offset: usize,
        /// What went wrong.
        reason: &'static str,
    },
    /// A private-chunk payload is malformed.
    BadPayload {
        /// The chunk's four-byte type.
        chunk: [u8; 4],
        /// Payload-relative offset where the problem was found.
        offset: usize,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// The node tree is structurally invalid.
    BadNodeTree {
        /// Stream offset where the problem was found.
        offset: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// A string node held invalid UTF-16.
    BadUtf16 {
        /// Stream offset of the string data.
        offset: usize,
    },
    /// A structural safety limit (depth, node count) was exceeded —
    /// defensive cap against hostile input.
    LimitExceeded {
        /// Which limit.
        limit: &'static str,
    },
    /// The standard (non-private) PNG image stream failed to decode.
    PngDecode {
        /// What the decoder rejected, as reported by the `png` crate.
        reason: String,
    },
    /// A decoded image layout has no RGBA8 mapping (decoder
    /// normalization should have prevented this).
    UnsupportedImage {
        /// The layout that has no mapping.
        layout: String,
    },
}

impl fmt::Display for FwPngError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadSignature => write!(f, "not a PNG stream (bad signature)"),
            Self::UnexpectedEof { offset, needed } => {
                write!(
                    f,
                    "unexpected end of stream at {offset} (needed {needed} more bytes)"
                )
            }
            Self::BadCrc {
                chunk,
                expected,
                got,
                offset,
            } => {
                write!(
                    f,
                    "CRC mismatch in chunk {:?} at offset {offset}: file says {expected:#010x}, computed {got:#010x}",
                    String::from_utf8_lossy(chunk)
                )
            }
            Self::BadChunk {
                chunk,
                offset,
                reason,
            } => {
                write!(
                    f,
                    "malformed chunk {:?} at offset {offset}: {reason}",
                    String::from_utf8_lossy(chunk)
                )
            }
            Self::Inflate { offset, reason } => {
                write!(f, "zlib inflate failed at {offset}: {reason}")
            }
            Self::BadPayload {
                chunk,
                offset,
                reason,
            } => {
                write!(
                    f,
                    "malformed {:?} payload at {offset}: {reason}",
                    String::from_utf8_lossy(chunk)
                )
            }
            Self::BadNodeTree { offset, reason } => {
                write!(f, "malformed node tree at {offset}: {reason}")
            }
            Self::BadUtf16 { offset } => {
                write!(f, "invalid UTF-16 in string node at {offset}")
            }
            Self::LimitExceeded { limit } => {
                write!(f, "structural limit exceeded: {limit}")
            }
            Self::PngDecode { reason } => {
                write!(f, "standard PNG stream failed to decode: {reason}")
            }
            Self::UnsupportedImage { layout } => {
                write!(f, "no RGBA8 mapping for image layout: {layout}")
            }
        }
    }
}

impl std::error::Error for FwPngError {}
