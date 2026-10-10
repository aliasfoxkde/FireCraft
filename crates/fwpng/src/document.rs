//! Document-level helpers over the private chunks: the `mkTS` node tree
//! (the actual document) and the `mkBS` legacy simplified copy.
//!
//! `mkBS` predates multi-page documents: it carries one simplified node
//! tree with no page structure. Fireworks kept writing it alongside
//! `mkTS` for backwards compatibility. Neither chunk is authoritative on
//! its own — the comparison helper exists so corpus analysis can measure
//! how far the legacy copy drifts from the real tree.

use crate::chunk::RawChunk;
use crate::error::FwPngError;
use crate::nodetree::{self, Node, NodeValue};
use std::io::Read;

/// Cap on the total compressed bytes one node-tree stream may occupy
/// before inflation (hostile-input rule).
pub const MAX_TREE_STREAM_COMPRESSED: usize = 64 * 1024 * 1024;

/// Why a node-tree chunk group could not be decoded.
enum TreeChunks<'a> {
    /// Chunk type absent from the file.
    Absent,
    /// Consecutive payloads of one logical stream (only the first
    /// carries any header semantics — for node trees there is none).
    Present(Vec<&'a [u8]>),
}

fn tree_payloads<'a>(
    chunks: &'a [RawChunk],
    ctype: &[u8; 4],
) -> Result<TreeChunks<'a>, FwPngError> {
    let mut group: Vec<&[u8]> = Vec::new();
    let mut total = 0usize;
    for chunk in chunks {
        if &chunk.ctype != ctype {
            if !group.is_empty() {
                break; // end of the consecutive run
            }
            continue;
        }
        if total + chunk.data.len() > MAX_TREE_STREAM_COMPRESSED {
            return Err(FwPngError::BadPayload {
                chunk: *ctype,
                offset: chunk.offset,
                reason: "node-tree stream exceeds the compressed-size cap",
            });
        }
        total += chunk.data.len();
        group.push(&chunk.data);
    }
    if group.is_empty() {
        Ok(TreeChunks::Absent)
    } else {
        Ok(TreeChunks::Present(group))
    }
}

/// Inflates a grouped node-tree payload and parses the node tree.
fn decode_group(group: &[&[u8]]) -> Result<Vec<Node>, FwPngError> {
    let mut joined = Vec::new();
    for piece in group {
        joined.extend_from_slice(piece);
    }
    let mut inflated = Vec::new();
    flate2::read::ZlibDecoder::new(&joined[..])
        .read_to_end(&mut inflated)
        .map_err(|_| FwPngError::Inflate {
            offset: 0,
            reason: "node-tree zlib stream failed to inflate",
        })?;
    nodetree::parse(&inflated)
}

/// Parses the `mkTS` tree (the document), or [`None`] when the file has
/// no `mkTS` chunk. A stream split across consecutive `mkTS` chunks is
/// reassembled.
///
/// # Errors
/// [`FwPngError::Inflate`] / [`FwPngError::BadNodeTree`] on a malformed
/// stream; [`FwPngError::BadPayload`] over the size cap.
pub fn parse_mkts(chunks: &[RawChunk]) -> Result<Option<Vec<Node>>, FwPngError> {
    match tree_payloads(chunks, b"mkTS")? {
        TreeChunks::Absent => Ok(None),
        TreeChunks::Present(group) => decode_group(&group).map(Some),
    }
}

/// Parses the `mkBS` legacy simplified tree, or [`None`] when absent
/// (modern minimal files may omit it).
///
/// # Errors
/// As [`parse_mkts`].
pub fn parse_mkbs(chunks: &[RawChunk]) -> Result<Option<Vec<Node>>, FwPngError> {
    match tree_payloads(chunks, b"mkBS")? {
        TreeChunks::Absent => Ok(None),
        TreeChunks::Present(group) => decode_group(&group).map(Some),
    }
}

/// How far the legacy `mkBS` copy tracks the current `mkTS` tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeComparison {
    /// Node count of the `mkTS` tree.
    pub current_nodes: usize,
    /// Node count of the `mkBS` tree.
    pub legacy_nodes: usize,
    /// `true` when both trees serialize to identical canonical bytes.
    pub identical: bool,
    /// Path of the first difference, e.g. `MKBv/LYLv/LAYv[1]`; `None`
    /// when identical.
    pub first_difference: Option<String>,
}

/// Compares a current tree against its legacy copy (for corpus
/// analysis: "does mkBS still agree with mkTS in this file?").
#[must_use]
pub fn compare_trees(current: &[Node], legacy: &[Node]) -> TreeComparison {
    let mut cmp = TreeComparison {
        current_nodes: count_nodes(current),
        legacy_nodes: count_nodes(legacy),
        identical: nodetree::serialize(current) == nodetree::serialize(legacy),
        first_difference: None,
    };
    if !cmp.identical {
        let mut path = String::new();
        first_diff(current, legacy, &mut path, &mut cmp);
    }
    cmp
}

fn count_nodes(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|n| 1 + n.as_children().map_or(0, count_nodes))
        .sum()
}

/// Depth-first walk recording the first structural difference. Names,
/// value kinds, and value payloads all count; index divergence is
/// reported at the container where the shapes diverge.
fn first_diff(a: &[Node], b: &[Node], path: &mut String, cmp: &mut TreeComparison) {
    if cmp.first_difference.is_some() {
        return;
    }
    for (i, (na, nb)) in a.iter().zip(b.iter()).enumerate() {
        let name = String::from_utf8_lossy(&na.fourcc).into_owned();
        let anchor = format!("{path}/{name}[{i}]");
        if na.fourcc != nb.fourcc {
            cmp.first_difference = Some(anchor);
            return;
        }
        match (&na.value, &nb.value) {
            // Containers are compared by descending; whole-value equality
            // here would report the outermost container instead of the
            // scalar that actually drifted.
            (NodeValue::Children(ca), NodeValue::Children(cb)) => {
                let mut nested = anchor;
                first_diff(ca, cb, &mut nested, cmp);
                if cmp.first_difference.is_some() {
                    return;
                }
            }
            (va, vb) if va != vb => {
                cmp.first_difference = Some(anchor);
                return;
            }
            _ => {}
        }
    }
    if a.len() != b.len() {
        cmp.first_difference = Some(format!("{path} (different child counts)"));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::chunk::{PNG_SIGNATURE, parse_chunks, write_chunk};
    use std::io::Write as _;

    fn zlib(raw: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(raw).expect("in-memory write");
        enc.finish().expect("in-memory finish")
    }

    fn file_with_trees(mkts: &[u8], mkbs: Option<&[u8]>) -> Vec<RawChunk> {
        let mut file = Vec::from(PNG_SIGNATURE);
        write_chunk(&mut file, b"mkTS", &zlib(mkts));
        if let Some(legacy) = mkbs {
            write_chunk(&mut file, b"mkBS", &zlib(legacy));
        }
        write_chunk(&mut file, b"IEND", &[]);
        parse_chunks(&file).expect("parses")
    }

    #[test]
    fn parses_mkts_and_mkbs() {
        let tree = b"MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}}}";
        let chunks = file_with_trees(tree, Some(tree));
        let current = parse_mkts(&chunks).expect("mkTS").expect("present");
        let legacy = parse_mkbs(&chunks).expect("mkBS").expect("present");
        assert_eq!(current.len(), 1);
        assert_eq!(legacy.len(), 1);
    }

    #[test]
    fn absent_mkbs_is_none() {
        let chunks = file_with_trees(b"MKBv{FRCi{1}}", None);
        assert!(parse_mkbs(&chunks).expect("ok").is_none());
        assert!(parse_mkts(&chunks).expect("ok").is_some());
    }

    #[test]
    fn split_mkts_stream_reassembles() {
        let tree = b"MKBv{LYLv{LAYv{BKGb{0}}FRCi{1}}}";
        let mut compressed = zlib(tree);
        let mut file = Vec::from(PNG_SIGNATURE);
        let split = compressed.split_off(compressed.len() / 2);
        write_chunk(&mut file, b"mkTS", &compressed);
        write_chunk(&mut file, b"mkTS", &split);
        write_chunk(&mut file, b"IEND", &[]);
        let chunks = parse_chunks(&file).expect("parses");
        let current = parse_mkts(&chunks).expect("mkTS").expect("present");
        assert_eq!(current.len(), 1, "split stream must reassemble to one root");
    }

    #[test]
    fn identical_trees_report_identical() {
        let tree = b"MKBv{FRCi{1}}";
        let chunks = file_with_trees(tree, Some(tree));
        let current = parse_mkts(&chunks).expect("mkTS").expect("present");
        let legacy = parse_mkbs(&chunks).expect("mkBS").expect("present");
        let cmp = compare_trees(&current, &legacy);
        assert!(cmp.identical);
        assert!(cmp.first_difference.is_none());
        assert_eq!(cmp.current_nodes, 2, "MKBv root plus its FRCi child");
    }

    #[test]
    fn differing_trees_report_first_difference_with_path() {
        let current_tree = b"MKBv{FRCi{1}XLCi{0}}";
        let legacy_tree = b"MKBv{FRCi{2}XLCi{0}}";
        let chunks = file_with_trees(current_tree, Some(legacy_tree));
        let current = parse_mkts(&chunks).expect("mkTS").expect("present");
        let legacy = parse_mkbs(&chunks).expect("mkBS").expect("present");
        let cmp = compare_trees(&current, &legacy);
        assert!(!cmp.identical);
        assert_eq!(
            cmp.first_difference.as_deref(),
            Some("/MKBv[0]/FRCi[0]"),
            "value mismatch inside the root container"
        );
    }

    #[test]
    fn shape_divergence_is_reported_at_the_container() {
        let current_tree = b"MKBv{FRCi{1}XLCi{0}}";
        let legacy_tree = b"MKBv{FRCi{1}}";
        let chunks = file_with_trees(current_tree, Some(legacy_tree));
        let current = parse_mkts(&chunks).expect("mkTS").expect("present");
        let legacy = parse_mkbs(&chunks).expect("mkBS").expect("present");
        let cmp = compare_trees(&current, &legacy);
        assert!(!cmp.identical);
        assert_eq!(cmp.current_nodes, 3, "MKBv + FRCi + XLCi");
        assert_eq!(cmp.legacy_nodes, 2, "MKBv + FRCi");
        assert!(cmp.first_difference.is_some());
    }
}
