//! Seeded noise. Deterministic by construction: the generator is
//! explicit-seed `SplitMix64` and the stream is consumed in fixed
//! row-major order, so a given `(input, seed, amount, monochrome)`
//! replays bit-identically — journal replay depends on it. The seed is
//! part of every filter payload that carries noise (M7 ticket rule).

use crate::{Surface, clamp255};
use color::Rgba8;

/// `SplitMix64`: 64-bit state, one multiply-xor cascade per draw.
/// Not cryptographic — it is fast, stateless per draw, and its output
/// is pinned by these tests, which is all replay needs.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advances the state and returns the next 64 pseudorandom bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Adds uniform noise to the straight RGB channels; alpha is untouched.
///
/// - `amount` 0..=100 clamps; **`0` returns an exact copy** (no
///   unpremultiply round trip, so identity is byte-exact even on
///   translucent pixels).
/// - `monochrome` draws one delta per pixel (grain, luminance noise)
///   instead of three (color speccle).
///
/// Deltas are `±amount·255/100` worst-case, drawn modulo the span of
/// the seeded stream — deterministic, and the premultiplied invariant
/// (`rgb ≤ alpha`) is preserved because the op runs through the
/// point-op path ([`Surface::map_straight`], decision 0004).
#[must_use]
pub fn add_noise(surface: &Surface, seed: u64, amount: u32, monochrome: bool) -> Surface {
    let amount = amount.min(100);
    if amount == 0 {
        return surface.clone();
    }
    let span = u64::from(amount) * 255 / 100; // worst-case delta magnitude
    let mut rng = SplitMix64::new(seed);
    surface.map_straight(|px| {
        // One draw folded to a signed delta in [-span, span] (modulo
        // bias at this span is far below one u8 level and fixed for a
        // given seed — determinism is the contract, not distribution).
        let draw = |rng: &mut SplitMix64| -> i64 {
            (rng.next_u64() % (2 * span + 1)) as i64 - span as i64
        };
        if monochrome {
            let d = draw(&mut rng);
            Rgba8::new(
                clamp255(i64::from(px.r) + d),
                clamp255(i64::from(px.g) + d),
                clamp255(i64::from(px.b) + d),
                px.a,
            )
        } else {
            Rgba8::new(
                clamp255(i64::from(px.r) + draw(&mut rng)),
                clamp255(i64::from(px.g) + draw(&mut rng)),
                clamp255(i64::from(px.b) + draw(&mut rng)),
                px.a,
            )
        }
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 12×12 opaque color wash.
    fn wash() -> Surface {
        let mut s = Surface::new(12, 12).unwrap();
        for y in 0..12u32 {
            for x in 0..12u32 {
                s.set_straight(
                    x,
                    y,
                    Rgba8::new((x * 21) as u8, (y * 21) as u8, ((x + y) * 10) as u8, 255),
                );
            }
        }
        s
    }

    #[test]
    fn splitmix64_stream_is_pinned() {
        // The first draws for seed 0 are stable constants of the
        // algorithm; if these move, every noise payload replays
        // differently — the pin is the point.
        let mut g = SplitMix64::new(0);
        assert_eq!(g.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(g.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        // Seed sensitivity: adjacent seeds diverge immediately.
        let mut h = SplitMix64::new(1);
        assert_ne!(g.next_u64(), h.next_u64());
        // Full-period sanity: the state advances (no self-lock).
        let mut k = SplitMix64::new(u64::MAX);
        let mut distinct = std::collections::HashSet::new();
        for _ in 0..64 {
            distinct.insert(k.next_u64());
        }
        assert_eq!(distinct.len(), 64);
    }

    #[test]
    fn noise_amount_zero_is_a_byte_exact_copy() {
        let s = wash();
        assert_eq!(add_noise(&s, 42, 0, false), s);
    }

    #[test]
    fn noise_is_deterministic_and_seed_sensitive() {
        let s = wash();
        let a = add_noise(&s, 1234, 20, false);
        let b = add_noise(&s, 1234, 20, false);
        assert_eq!(a, b, "same seed must replay identically");

        let c = add_noise(&s, 1235, 20, false);
        assert_ne!(a, c, "different seeds must diverge");

        // Fixed-input check: at amount 100 the mean-square delta is
        // bounded by (255)²/3 in theory; a loose but meaningful bound is
        // that noise actually moved most pixels and stayed within span.
        let noisy = add_noise(&s, 7, 100, false);
        let mut moved = 0u32;
        for y in 0..12u32 {
            for x in 0..12u32 {
                let o = s.get(x, y).unwrap();
                let n = noisy.get(x, y).unwrap();
                if o[..3] != n[..3] {
                    moved += 1;
                }
                for ch in 0..3 {
                    let d = i64::from(n[ch]) - i64::from(o[ch]);
                    assert!(d.abs() <= 255, "delta {d} exceeds amount-100 span");
                }
            }
        }
        assert!(moved > 100, "amount-100 noise moved only {moved} pixels");
    }

    #[test]
    fn monochrome_noise_keeps_gray_gray() {
        // A flat gray field under monochrome noise: every pixel keeps
        // r == g == b (one shared delta), which color noise breaks.
        let flat = Surface::from_straight(16, 16, &[128u8; 16 * 16 * 4]).unwrap();
        let grain = add_noise(&flat, 99, 50, true);
        for y in 0..16u32 {
            for x in 0..16u32 {
                let p = grain.get(x, y).unwrap();
                assert_eq!(p[0], p[1]);
                assert_eq!(p[1], p[2]);
            }
        }
        let speckle = add_noise(&flat, 99, 50, false);
        assert_ne!(
            speckle, grain,
            "color noise must differ from monochrome on the same seed"
        );
    }

    #[test]
    fn noise_preserves_alpha_and_dimensions() {
        let s = Surface::from_straight(
            4,
            2,
            &[
                255, 255, 255, 128, //
                10, 20, 30, 64, //
                0, 0, 0, 255, //
                200, 100, 50, 1, //
                1, 2, 3, 0, //
                90, 90, 90, 90, //
                5, 5, 5, 5, //
                250, 250, 250, 250,
            ],
        )
        .unwrap();
        let out = add_noise(&s, 3, 80, false);
        assert_eq!((out.width(), out.height()), (4, 2));
        for x in 0..4u32 {
            for y in 0..2u32 {
                assert_eq!(out.get(x, y).unwrap()[3], s.get(x, y).unwrap()[3]);
            }
        }
    }
}
