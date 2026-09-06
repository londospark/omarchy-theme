use std::fmt;

/// A RGBA color with 8 bits per channel.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const BLACK: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Rgba = Rgba {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const TRANSPARENT: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    /// `0xRRGGBBAA` — the packing shared by the C ABI struct, ImNodes, and
    /// raylib's `Color`.
    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | self.a as u32
    }

    pub const fn from_u32(v: u32) -> Rgba {
        Rgba {
            r: (v >> 24) as u8,
            g: (v >> 16) as u8,
            b: (v >> 8) as u8,
            a: v as u8,
        }
    }

    pub const fn to_f32_array(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// `#rrggbb` (alpha ignored), lowercase — the representation Omarchy's
    /// own resolver and templates use on disk.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// `#rrggbbaa` (alpha included), lowercase.
    pub fn to_hex8(self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }

    /// Parse `#rgb`, `#rrggbb`, or `#rrggbbaa`. Also accepts `rgb()` /
    /// `rgba()` decimal triplets as used by some theme files.
    pub fn parse(s: &str) -> Option<Rgba> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix('#') {
            let digit = |c: u8| match c {
                b'0'..=b'9' => Some(c - b'0'),
                b'a'..=b'f' | b'A'..=b'F' => Some((c | 0x20) - b'a' + 10),
                _ => None,
            };
            let pair = |i: usize| -> Option<u8> {
                Some(digit(hex.as_bytes()[i])? << 4 | digit(hex.as_bytes()[i + 1])?)
            };
            let single = |i: usize| -> Option<u8> {
                let d = digit(hex.as_bytes()[i])?;
                Some(d << 4 | d)
            };
            let bytes = hex.as_bytes();
            return match bytes.len() {
                3 => Some(Rgba {
                    r: single(0)?,
                    g: single(1)?,
                    b: single(2)?,
                    a: 255,
                }),
                6 => Some(Rgba {
                    r: pair(0)?,
                    g: pair(2)?,
                    b: pair(4)?,
                    a: 255,
                }),
                8 => Some(Rgba {
                    r: pair(0)?,
                    g: pair(2)?,
                    b: pair(4)?,
                    a: pair(6)?,
                }),
                _ => None,
            };
        }
        if let Some(rest) = s.strip_prefix("rgb(").or_else(|| s.strip_prefix("rgba(")) {
            let rest = rest.strip_suffix(')')?;
            let mut parts = rest
                .split(|c| c == ',' || c == ' ')
                .filter(|p| !p.is_empty());
            let num = |p: &str| p.parse::<f64>().ok();
            let r = num(parts.next()?)? as u32;
            let g = num(parts.next()?)? as u32;
            let b = num(parts.next()?)? as u32;
            let a = match parts.next() {
                Some(p) => (num(p)? * 255.0 + 0.5) as u32,
                None => 255,
            };
            let cl = |v: u32| v.min(255) as u8;
            return Some(Rgba {
                r: cl(r),
                g: cl(g),
                b: cl(b),
                a: cl(a),
            });
        }
        None
    }

    /// Perceptually-flat sum-of-channels luminance used by Omarchy for
    /// light/dark auto-detection: `r + g + b > 382` means light.
    pub fn omarchy_luminance(self) -> u32 {
        self.r as u32 + self.g as u32 + self.b as u32
    }

    pub fn is_light_by_omarchy(self) -> bool {
        self.omarchy_luminance() > 382
    }

    /// Linear blend toward `other`. `amount` accepts fractions (`0.25`) and
    /// percentages (`25%`), matching `omarchy-theme-color`'s `mix` template
    /// helper; values above 1 that arrive as bare numbers are divided by 100
    /// exactly like the awk implementation, and the result is rounded
    /// half-up per channel.
    pub fn mix(self, other: Rgba, amount: f64) -> Rgba {
        let mut a = amount;
        if a > 1.0 {
            a /= 100.0;
        }
        let a = a.clamp(0.0, 1.0);
        let ch = |s: u8, d: u8| -> u8 { ((s as f64) * (1.0 - a) + (d as f64) * a + 0.5) as u8 };
        Rgba {
            r: ch(self.r, other.r),
            g: ch(self.g, other.g),
            b: ch(self.b, other.b),
            a: ch(self.a, other.a),
        }
    }

    pub fn with_alpha(self, alpha: f32) -> Rgba {
        Rgba {
            a: (alpha.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            ..self
        }
    }
}

impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex8())
    }
}

/// Parse a mix amount the way the oracle's awk does: `25%` -> 0.25,
/// `0.25` -> 0.25, bare `25` -> 0.25, junk -> 0.
pub fn parse_mix_amount(s: &str) -> f64 {
    let s = s.trim();
    let v = match s.strip_suffix('%') {
        Some(rest) => rest.trim().parse::<f64>().unwrap_or(0.0) / 100.0,
        None => {
            let v = s.parse::<f64>().unwrap_or(0.0);
            if v > 1.0 {
                v / 100.0
            } else {
                v
            }
        }
    };
    v.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_forms() {
        assert_eq!(
            Rgba::parse("#1e1e2e"),
            Some(Rgba {
                r: 30,
                g: 30,
                b: 46,
                a: 255
            })
        );
        assert_eq!(Rgba::parse("#fff"), Some(Rgba::WHITE));
        assert_eq!(
            Rgba::parse("#ff880080"),
            Some(Rgba {
                r: 255,
                g: 136,
                b: 0,
                a: 128
            })
        );
        assert_eq!(
            Rgba::parse("#FFC94D"),
            Some(Rgba {
                r: 255,
                g: 201,
                b: 77,
                a: 255
            })
        );
        assert_eq!(Rgba::parse("not a color"), None);
        assert_eq!(Rgba::parse("#12345"), None);
    }

    #[test]
    fn parse_rgb_function() {
        assert_eq!(
            Rgba::parse("rgb(30, 30, 46)"),
            Some(Rgba {
                r: 30,
                g: 30,
                b: 46,
                a: 255
            })
        );
        assert_eq!(
            Rgba::parse("rgba(0, 0, 0, 0.5)"),
            Some(Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 128
            })
        );
    }

    #[test]
    fn u32_roundtrip() {
        let c = Rgba {
            r: 0x89,
            g: 0xb4,
            b: 0xfa,
            a: 0x80,
        };
        assert_eq!(Rgba::from_u32(c.to_u32()), c);
    }

    #[test]
    fn mix_matches_oracle_rounding() {
        // omarchy-theme-color: mix #1e1e2e with #000000 at 25%
        let bg = Rgba::parse("#1e1e2e").unwrap();
        let dark = bg.mix(Rgba::BLACK, 0.25);
        assert_eq!(dark.to_hex(), "#171723"); // 30*0.75=22.5 -> int(23), 46*0.75=34.5 -> int(35)
        let pct = bg.mix(Rgba::BLACK, parse_mix_amount("25%"));
        assert_eq!(pct, dark);
        let bare = bg.mix(Rgba::BLACK, parse_mix_amount("25"));
        assert_eq!(bare, dark);
    }

    #[test]
    fn luminance_boundary() {
        assert!(!Rgba::parse("#80807e").unwrap().is_light_by_omarchy()); // 128+128+126 = 382
        assert!(Rgba::parse("#80807f").unwrap().is_light_by_omarchy()); // 383 > 382
    }
}
