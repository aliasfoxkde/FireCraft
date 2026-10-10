//! The `mkTS` node tree: an ASCII FourCC-tagged tree of integers, floats,
//! bools, UTF-16 strings, and child containers.
//!
//! Grammar (big-endian throughout):
//!
//! ```text
//! node    := fourcc '{' payload '}'
//! fourcc  := 4 printable ASCII; the 4th char is the type letter:
//!            'i' hex integer, 'f' decimal float, 'b' bool, 's' string,
//!            'v' container (payload = child nodes)
//! int     := hex ASCII (e.g. "1A", "-2")
//! float   := decimal ASCII (e.g. "1.0", "-0.5")
//! bool    := "0" | "1"
//! string  := u16 char-count, then count x u16 UTF-16 code units
//! ```
//!
//! Example: `MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}}}`.
//!
//! Serialization emits hex integers in lowercase and floats via Rust's
//! shortest round-trip representation (integral floats gain a `.0`).
//! Byte-identity with files written by Fireworks itself is pinned in M2
//! against the reference corpus.

use crate::error::FwPngError;

/// Maximum container nesting accepted while parsing (real trees are < 20).
pub const MAX_DEPTH: usize = 64;
/// Maximum node count accepted while parsing (real trees have < 10^5).
pub const MAX_NODES: usize = 1_000_000;
/// Maximum bytes in a single scalar literal (int/float/bool text).
pub const MAX_SCALAR_LEN: usize = 4096;

/// The typed payload of a node.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeValue {
    /// Hex-encoded 64-bit signed integer (`'i'` type letter).
    Int(i64),
    /// Decimal float (`'f'`).
    Float(f64),
    /// `0`/`1` boolean (`'b'`).
    Bool(bool),
    /// UTF-16 string (`'s'`).
    Str(String),
    /// Container with child nodes (`'v'`).
    Children(Vec<Node>),
}

/// One node: a four-character code plus its typed payload.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The FourCC as it appears in the stream (type letter included).
    pub fourcc: [u8; 4],
    /// The typed payload.
    pub value: NodeValue,
}

impl Node {
    /// The ThreeCC (FourCC minus its type letter) as a string.
    #[must_use]
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.fourcc[..3]).unwrap_or_default()
    }

    /// The node payload as an integer, if it is one.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match &self.value {
            NodeValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// The node payload as a float, if it is one.
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match &self.value {
            NodeValue::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// The node payload as a bool, if it is one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            NodeValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// The node payload as a string, if it is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            NodeValue::Str(v) => Some(v),
            _ => None,
        }
    }

    /// The node payload as children, if it is a container.
    #[must_use]
    pub fn as_children(&self) -> Option<&[Node]> {
        match &self.value {
            NodeValue::Children(v) => Some(v),
            _ => None,
        }
    }
}

/// Parses a complete node-tree stream into its root nodes.
///
/// Trailing ASCII whitespace is tolerated. Any other trailing bytes are an
/// error — a truncated or mis-framed tree must not parse partially.
///
/// # Errors
/// [`FwPngError::BadNodeTree`] for structural violations,
/// [`FwPngError::BadUtf16`] for invalid string data, and
/// [`FwPngError::LimitExceeded`] when depth or node caps are exceeded.
pub fn parse(input: &[u8]) -> Result<Vec<Node>, FwPngError> {
    let mut p = Parser {
        input,
        pos: 0,
        nodes: 0,
    };
    let nodes = p.parse_nodes(0)?;
    while matches!(p.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        p.pos += 1;
    }
    if p.pos != input.len() {
        return Err(FwPngError::BadNodeTree {
            offset: p.pos,
            reason: "trailing bytes after root nodes".into(),
        });
    }
    Ok(nodes)
}

/// Serializes root nodes back to the wire format.
#[must_use]
pub fn serialize(nodes: &[Node]) -> Vec<u8> {
    let mut out = Vec::new();
    serialize_nodes(nodes, &mut out);
    out
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    nodes: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    fn err_here(&self, reason: &str) -> FwPngError {
        FwPngError::BadNodeTree {
            offset: self.pos,
            reason: reason.into(),
        }
    }

    fn parse_nodes(&mut self, depth: usize) -> Result<Vec<Node>, FwPngError> {
        let mut out = Vec::new();
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                self.pos += 1;
            }
            match self.peek() {
                None | Some(b'}') => return Ok(out),
                _ => out.push(self.parse_node(depth)?),
            }
        }
    }

    fn parse_node(&mut self, depth: usize) -> Result<Node, FwPngError> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(FwPngError::LimitExceeded {
                limit: "node count",
            });
        }
        if depth >= MAX_DEPTH {
            return Err(FwPngError::LimitExceeded {
                limit: "container nesting depth",
            });
        }
        let start = self.pos;
        let fourcc = self.parse_fourcc()?;
        self.expect_open(&fourcc, start)?;
        match fourcc[3] {
            b'i' | b'I' => {
                let text = self.read_scalar(&fourcc, start)?;
                let parsed = std::str::from_utf8(&text).ok().and_then(|s| {
                    // Signed form ("-1a") or 64-bit two's complement
                    // ("ffffffffffffffe6") both decode; writers in the
                    // wild may use either.
                    i64::from_str_radix(s, 16)
                        .or_else(|_| u64::from_str_radix(s, 16).map(|v| v as i64))
                        .ok()
                });
                let v = parsed.ok_or_else(|| self.err_at(start, "invalid hex integer literal"))?;
                Ok(Node {
                    fourcc,
                    value: NodeValue::Int(v),
                })
            }
            b'f' | b'F' => {
                let text = self.read_scalar(&fourcc, start)?;
                let parsed = std::str::from_utf8(&text)
                    .ok()
                    .and_then(|s| s.parse::<f64>().ok());
                let v = parsed.ok_or_else(|| self.err_at(start, "invalid float literal"))?;
                Ok(Node {
                    fourcc,
                    value: NodeValue::Float(v),
                })
            }
            b'b' | b'B' => {
                let text = self.read_scalar(&fourcc, start)?;
                let v = match text.as_slice() {
                    b"0" => false,
                    b"1" => true,
                    _ => return Err(self.err_at(start, "bool literal must be 0 or 1")),
                };
                Ok(Node {
                    fourcc,
                    value: NodeValue::Bool(v),
                })
            }
            b's' | b'S' => {
                let v = self.parse_string(start)?;
                Ok(Node {
                    fourcc,
                    value: NodeValue::Str(v),
                })
            }
            b'v' | b'V' => {
                let children = self.parse_nodes(depth + 1)?;
                if self.bump() != Some(b'}') {
                    return Err(self.err_at(start, "unterminated container"));
                }
                Ok(Node {
                    fourcc,
                    value: NodeValue::Children(children),
                })
            }
            other => Err(FwPngError::BadNodeTree {
                offset: start,
                reason: format!("unknown node type letter {:?}", other as char),
            }),
        }
    }

    fn parse_fourcc(&mut self) -> Result<[u8; 4], FwPngError> {
        let bytes = self
            .input
            .get(self.pos..self.pos + 4)
            .ok_or(FwPngError::UnexpectedEof {
                offset: self.pos,
                needed: 4,
            })?;
        if !bytes.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
            return Err(self.err_here("FourCC must be printable ASCII"));
        }
        let mut out = [0; 4];
        out.copy_from_slice(bytes);
        self.pos += 4;
        Ok(out)
    }

    fn expect_open(&mut self, fourcc: &[u8; 4], start: usize) -> Result<(), FwPngError> {
        if self.bump() == Some(b'{') {
            Ok(())
        } else {
            Err(FwPngError::BadNodeTree {
                offset: start,
                reason: format!(
                    "node {:?} is missing its opening brace",
                    String::from_utf8_lossy(fourcc)
                ),
            })
        }
    }

    /// Reads a brace-free scalar up to the closing brace, consuming it.
    /// Returns an owned copy — scalar literals are tiny (capped), so this
    /// sidesteps borrowing `self` across error construction.
    fn read_scalar(&mut self, fourcc: &[u8; 4], start: usize) -> Result<Vec<u8>, FwPngError> {
        let end = self.input[self.pos..]
            .iter()
            .position(|&b| b == b'}')
            .ok_or_else(|| FwPngError::BadNodeTree {
                offset: start,
                reason: format!(
                    "node {:?} is missing its closing brace",
                    String::from_utf8_lossy(fourcc)
                ),
            })?;
        if end > MAX_SCALAR_LEN {
            return Err(FwPngError::BadNodeTree {
                offset: start,
                reason: format!("scalar literal exceeds {MAX_SCALAR_LEN} bytes"),
            });
        }
        let text = self.input[self.pos..self.pos + end].to_vec();
        self.pos += end + 1;
        Ok(text)
    }

    fn parse_string(&mut self, start: usize) -> Result<String, FwPngError> {
        let count_bytes =
            self.input
                .get(self.pos..self.pos + 2)
                .ok_or(FwPngError::UnexpectedEof {
                    offset: self.pos,
                    needed: 2,
                })?;
        let count = u16::from_be_bytes([count_bytes[0], count_bytes[1]]);
        self.pos += 2;
        let byte_len = count as usize * 2;
        let data =
            self.input
                .get(self.pos..self.pos + byte_len)
                .ok_or(FwPngError::UnexpectedEof {
                    offset: self.pos,
                    needed: byte_len,
                })?;
        self.pos += byte_len;
        if self.bump() != Some(b'}') {
            return Err(self.err_at(start, "string node is missing its closing brace"));
        }
        let units: Vec<u16> = data
            .chunks_exact(2)
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect();
        String::from_utf16(&units).map_err(|_| FwPngError::BadUtf16 { offset: start })
    }

    fn err_at(&self, offset: usize, reason: &str) -> FwPngError {
        FwPngError::BadNodeTree {
            offset,
            reason: reason.into(),
        }
    }
}

fn serialize_nodes(nodes: &[Node], out: &mut Vec<u8>) {
    for node in nodes {
        out.extend_from_slice(&node.fourcc);
        out.push(b'{');
        match &node.value {
            // Canonical integer form: 64-bit two's-complement lowercase hex
            // (printf %llx semantics — the likely writer form; M2 pins this
            // against the corpus). The parser additionally accepts a signed
            // "-1a" spelling.
            NodeValue::Int(v) => out.extend_from_slice(format!("{:x}", *v as u64).as_bytes()),
            NodeValue::Float(v) => {
                let text = if v.is_finite() && *v == v.trunc() && v.abs() < 1e15 {
                    format!("{v:.1}")
                } else {
                    format!("{v}")
                };
                out.extend_from_slice(text.as_bytes());
            }
            NodeValue::Bool(b) => out.push(if *b { b'1' } else { b'0' }),
            NodeValue::Str(s) => {
                let units: Vec<u16> = s.encode_utf16().collect();
                out.extend_from_slice(&(units.len() as u16).to_be_bytes());
                for unit in units {
                    out.extend_from_slice(&unit.to_be_bytes());
                }
            }
            NodeValue::Children(children) => serialize_nodes(children, out),
        }
        out.push(b'}');
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn canonical_example_round_trips() {
        let src = b"MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}}}";
        let tree = parse(src).expect("parses");
        assert_eq!(serialize(&tree), src.to_vec());
    }

    #[test]
    fn all_scalar_types_round_trip() {
        let tree = vec![
            Node {
                fourcc: *b"NUMi",
                value: NodeValue::Int(-255),
            },
            Node {
                fourcc: *b"RATf",
                value: NodeValue::Float(0.5),
            },
            Node {
                fourcc: *b"VISb",
                value: NodeValue::Bool(true),
            },
            Node {
                fourcc: *b"NM4s",
                value: NodeValue::Str("Sun\u{1F525}fire".into()),
            },
            Node {
                fourcc: *b"GRDv",
                value: NodeValue::Children(vec![Node {
                    fourcc: *b"EMPi",
                    value: NodeValue::Int(0),
                }]),
            },
        ];
        let bytes = serialize(&tree);
        let back = parse(&bytes).expect("re-parses");
        assert_eq!(back, tree);
    }

    #[test]
    fn negative_hex_integers_round_trip() {
        let tree = vec![Node {
            fourcc: *b"NEGi",
            value: NodeValue::Int(-26),
        }];
        let bytes = serialize(&tree);
        assert_eq!(&bytes, b"NEGi{ffffffffffffffe6}");
        assert_eq!(parse(&bytes).expect("parses"), tree);
        // Signed spelling from foreign writers decodes identically.
        assert_eq!(parse(b"NEGi{-1a}").expect("signed form parses"), tree);
    }

    #[test]
    fn truncation_never_parses_partially() {
        assert!(parse(b"MKBv{FRCi{1}").is_err());
        assert!(parse(b"MKBv{FRCi1}").is_err(), "missing open brace");
        assert!(parse(b"MKBv{FRZz{1}}").is_err(), "unknown type letter");
        assert!(parse(b"MKBv{BKGb{2}}").is_err(), "bool out of domain");
        assert!(parse(b"MKBv{}}").is_err(), "stray closing brace");
        assert!(
            parse(b"MKBv{NMs{\0\x05\x00h").is_err(),
            "string overruns EOF"
        );
    }

    #[test]
    fn lone_surrogate_string_is_rejected() {
        // NM4s with count 1 and a high surrogate D800, no low surrogate.
        let bad = b"NM4s{\0\x01\xD8\x00}";
        assert!(matches!(parse(bad), Err(FwPngError::BadUtf16 { .. })));
    }

    #[test]
    fn depth_limit_is_enforced() {
        let mut src = Vec::new();
        for _ in 0..(MAX_DEPTH + 5) {
            src.extend_from_slice(b"AB1v{");
        }
        src.extend(std::iter::repeat_n(b'}', MAX_DEPTH + 5));
        assert!(matches!(
            parse(&src),
            Err(FwPngError::LimitExceeded {
                limit: "container nesting depth"
            })
        ));
    }

    #[test]
    fn trailing_garbage_is_an_error() {
        assert!(parse(b"MKBv{FRCi{1}}JUNK").is_err());
    }

    #[test]
    fn trailing_whitespace_is_tolerated() {
        let tree = parse(b"MKBv{FRCi{1}}\n").expect("parses");
        assert_eq!(tree.len(), 1);
    }
}
