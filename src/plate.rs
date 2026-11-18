//! Generated "plates": a small line drawing (frame, construction grid, hatched
//! blocks, a dimension line) used for projects that have no images yet.
//! The same seed always gives the same drawing, so each project keeps its own.

use std::fmt::Write;

/// xorshift64*, seeded with the FNV-1a hash of a string.
struct Rng(u64);

impl Rng {
    fn new(seed: &str) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in seed.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Rng(h | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A number in `lo..hi`.
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next() % u64::from(hi - lo)) as u32
    }
}

/// Formats millimetres the way drawings do: 12600 → "12 600".
pub(crate) fn mm(v: u32) -> String {
    let s = v.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{2009}');
        }
        out.push(c);
    }
    out
}

/// An inline SVG drawing. `seed` must be unique per project (the slug), since
/// it also names the hatch pattern; `code` is the sheet number written on it.
pub fn plate(seed: &str, code: &str) -> String {
    let mut r = Rng::new(seed);
    let id = format!("hatch-{seed}");
    let mut s = String::new();

    let angle = [45, -45, 30, 60, 90][r.range(0, 5) as usize];
    let gap = r.range(5, 9);
    let _ = write!(
        s,
        r##"<svg class="plate" viewBox="0 0 400 300" role="img" aria-label="Drawing {code}" xmlns="http://www.w3.org/2000/svg" fill="none" stroke="currentColor"><defs><pattern id="{id}" width="{gap}" height="{gap}" patternUnits="userSpaceOnUse" patternTransform="rotate({angle})"><line x1="0" y1="0" x2="0" y2="{gap}" stroke="currentColor" stroke-width="0.6"/></pattern></defs><rect x="12" y="12" width="376" height="276" stroke-width="0.6"/>"##
    );

    // Construction grid.
    let cols = r.range(4, 8);
    for i in 1..cols {
        let x = 12.0 + 376.0 * f64::from(i) / f64::from(cols);
        let _ = write!(s, r#"<line class="faint" x1="{x:.1}" y1="12" x2="{x:.1}" y2="288"/>"#);
    }
    let rows = r.range(3, 6);
    for i in 1..rows {
        let y = 12.0 + 276.0 * f64::from(i) / f64::from(rows);
        let _ = write!(s, r#"<line class="faint" x1="12" y1="{y:.1}" x2="388" y2="{y:.1}"/>"#);
    }

    // Blocks on a 20-unit module, inside x 40..360, y 36..236.
    let n = r.range(3, 6);
    for k in 0..n {
        let w = 20 * r.range(2, 7);
        let h = 20 * r.range(2, 5);
        let x = 40 + 20 * r.range(0, (320 - w) / 20 + 1);
        let y = 36 + 20 * r.range(0, (200 - h) / 20 + 1);
        let fill = if r.range(0, 3) == 0 { format!(r#" fill="url(#{id})""#) } else { String::new() };
        let class = if k == n - 1 { r#" class="acc""# } else { "" };
        let _ = write!(s, r#"<rect{class} x="{x}" y="{y}" width="{w}" height="{h}" stroke-width="1.2"{fill}/>"#);
    }

    // Sometimes a column, tree or rotunda.
    if r.range(0, 2) == 0 {
        let cx = 60 + 20 * r.range(0, 15);
        let cy = 56 + 20 * r.range(0, 9);
        let rad = 8 + 4 * r.range(0, 6);
        let _ = write!(s, r#"<circle cx="{cx}" cy="{cy}" r="{rad}" stroke-width="0.8"/>"#);
    }

    // Dimension line with ticks and its measure.
    let a = r.range(0, 6);
    let b = r.range(4, 16 - a);
    let (x1, x2) = (40 + 20 * a, 40 + 20 * (a + b));
    let y = 262;
    let mid = (x1 + x2) / 2;
    let _ = write!(
        s,
        r#"<g class="dim-line" stroke-width="0.6"><line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}"/><line x1="{x1}" y1="{}" x2="{x1}" y2="{}"/><line x1="{x2}" y1="{}" x2="{x2}" y2="{}"/><line x1="{}" y1="{}" x2="{}" y2="{}"/><line x1="{}" y1="{}" x2="{}" y2="{}"/></g><text x="{mid}" y="{}" text-anchor="middle">{}</text>"#,
        y - 5, y + 5, y - 5, y + 5,
        x1 - 3, y + 3, x1 + 3, y - 3,
        x2 - 3, y + 3, x2 + 3, y - 3,
        y - 6,
        mm(b * 600),
    );

    let _ = write!(s, r#"<text x="380" y="282" text-anchor="end">{code}</text></svg>"#);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_drawing() {
        assert_eq!(plate("casa", "A-01"), plate("casa", "A-01"));
        assert_ne!(plate("casa", "A-01"), plate("tower", "A-01"));
    }

    #[test]
    fn drawing_is_closed_svg() {
        for seed in ["a", "coniugatio", "urbanitas", "x-1", "long-project-name-2026"] {
            let svg = plate(seed, "T-01");
            assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
            assert!(svg.contains(&format!(r#"id="hatch-{seed}""#)));
        }
    }

    #[test]
    fn mm_groups_thousands() {
        assert_eq!(mm(600), "600");
        assert_eq!(mm(12600), "12\u{2009}600");
    }
}
