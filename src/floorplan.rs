//! Floor plans drawn from data: `content/plans/<name>.toml` → an inline SVG
//! (walls, openings, stairs, furniture, room names and areas, dimensions,
//! grid, north arrow, scale bar) and a room schedule.
//!
//! Everything is measured in millimetres, y pointing down (north is up).
//! The plan format is documented in `content/plans/README.md`.

use serde::Deserialize;
use std::{collections::BTreeMap, fmt::Write, fs};

// ------------------------------------------------------------------- data ---

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    N,
    E,
    S,
    W,
}

impl Dir {
    fn v(self) -> (f64, f64) {
        match self {
            Dir::N => (0.0, -1.0),
            Dir::E => (1.0, 0.0),
            Dir::S => (0.0, 1.0),
            Dir::W => (-1.0, 0.0),
        }
    }
    fn angle(self) -> f64 {
        match self {
            Dir::N => 0.0,
            Dir::E => 90.0,
            Dir::S => 180.0,
            Dir::W => 270.0,
        }
    }
    fn flip_x(self) -> Self {
        match self {
            Dir::E => Dir::W,
            Dir::W => Dir::E,
            d => d,
        }
    }
    fn flip_y(self) -> Self {
        match self {
            Dir::N => Dir::S,
            Dir::S => Dir::N,
            d => d,
        }
    }
}

fn d_wall() -> f64 {
    300.0
}
fn d_partition() -> f64 {
    120.0
}
fn d_column() -> f64 {
    400.0
}
fn d_true() -> bool {
    true
}
fn d_one() -> u32 {
    1
}
fn d_steps() -> u32 {
    14
}
fn d_origins() -> Vec<[f64; 2]> {
    vec![[0.0, 0.0]]
}

#[derive(Deserialize)]
struct Plan {
    #[serde(default = "d_wall")]
    wall: f64,
    #[serde(default = "d_partition")]
    partition: f64,
    #[serde(default = "d_column")]
    column: f64,
    level: Vec<Level>,
}

#[derive(Deserialize)]
struct Level {
    name: String,
    /// Closed polygon of exterior wall centrelines.
    #[serde(default)]
    outline: Vec<[f64; 2]>,
    #[serde(default)]
    grid: Option<Grid>,
    /// Extra dimension chains along the top / left (centreline positions).
    #[serde(default)]
    xdims: Vec<f64>,
    #[serde(default)]
    ydims: Vec<f64>,
    #[serde(default)]
    group: Vec<Group>,
    #[serde(flatten)]
    items: Items,
}

#[derive(Deserialize, Clone)]
struct Grid {
    #[serde(default)]
    x: Vec<f64>,
    #[serde(default)]
    y: Vec<f64>,
}

/// Items drawn several times: at each origin, `repeat = [count, dx, dy]`
/// times, and mirrored about `mirror_x` / `mirror_y` (in level coordinates).
#[derive(Deserialize)]
struct Group {
    #[serde(default = "d_origins")]
    origins: Vec<[f64; 2]>,
    #[serde(default)]
    repeat: Option<[f64; 3]>,
    #[serde(default)]
    mirror_x: Option<f64>,
    #[serde(default)]
    mirror_y: Option<f64>,
    /// First number for `{n}` / `{a}` in room names.
    #[serde(default = "d_one")]
    start: u32,
    #[serde(flatten)]
    items: Items,
}

#[derive(Deserialize, Default, Clone)]
struct Items {
    /// [x1, y1, x2, y2] or [x1, y1, x2, y2, thickness]; default `wall`.
    #[serde(default)]
    walls: Vec<Vec<f64>>,
    /// Same, default thickness `partition`.
    #[serde(default)]
    partitions: Vec<Vec<f64>>,
    #[serde(default)]
    columns: Vec<[f64; 2]>,
    #[serde(default)]
    doors: Vec<Door>,
    #[serde(default)]
    windows: Vec<Window>,
    #[serde(default)]
    stairs: Vec<Stair>,
    #[serde(default)]
    furniture: Vec<Furniture>,
    #[serde(default)]
    rooms: Vec<Room>,
    #[serde(default)]
    labels: Vec<Label>,
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(Deserialize, Clone)]
struct Door {
    /// Hinge point on the wall centreline.
    at: [f64; 2],
    w: f64,
    /// Direction from the hinge along the wall.
    run: Dir,
    /// The side the leaf opens to.
    #[serde(default)]
    swing: Option<Dir>,
    /// "swing" (default), "double", "slide" or "opening".
    #[serde(default)]
    style: Option<String>,
}

#[derive(Deserialize, Clone)]
struct Window {
    at: [f64; 2],
    w: f64,
    run: Dir,
}

#[derive(Deserialize, Clone)]
struct Stair {
    rect: [f64; 4],
    /// Direction of travel going up (of the first flight, for "u").
    dir: Dir,
    #[serde(default = "d_steps")]
    steps: u32,
    /// "straight" (default) or "u" (two flights with a half landing).
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    label: Option<String>,
    /// For "u": the first flight is on the left of `dir` instead of the right.
    #[serde(default)]
    left: bool,
}

#[derive(Deserialize, Clone)]
struct Furniture {
    kind: String,
    rect: [f64; 4],
    /// The side that is the piece's back (against the wall).
    #[serde(default)]
    side: Option<Dir>,
    #[serde(default)]
    seats: Option<u32>,
}

#[derive(Deserialize, Clone)]
struct Room {
    name: String,
    #[serde(default)]
    rect: Option<[f64; 4]>,
    #[serde(default)]
    rects: Vec<[f64; 4]>,
    /// Label position, if not the centre of the first rectangle.
    #[serde(default)]
    at: Option<[f64; 2]>,
    /// "tile", "paving", "timber" or "hatch".
    #[serde(default)]
    floor: Option<String>,
    /// "room" (name + area, default), "tag" (name only, accent) or "none".
    #[serde(default)]
    style: Option<String>,
    /// Listed in the room schedule and counted in the areas.
    #[serde(default = "d_true")]
    list: bool,
    /// Outdoor space: listed, but not counted in the floor area.
    #[serde(default)]
    outdoor: bool,
    /// Schedule name, if not the name without its number.
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Deserialize, Clone)]
struct Label {
    at: [f64; 2],
    text: String,
}

#[derive(Deserialize, Clone)]
struct Entry {
    at: [f64; 2],
    /// Direction of walking in.
    dir: Dir,
}

// -------------------------------------------------------------- transform ---

/// Translate, then optionally mirror about vertical / horizontal axes.
#[derive(Clone, Copy, Default)]
struct Tf {
    dx: f64,
    dy: f64,
    mx: Option<f64>,
    my: Option<f64>,
}

impl Tf {
    fn p(&self, x: f64, y: f64) -> [f64; 2] {
        let (mut x, mut y) = (x + self.dx, y + self.dy);
        if let Some(a) = self.mx {
            x = 2.0 * a - x;
        }
        if let Some(b) = self.my {
            y = 2.0 * b - y;
        }
        [x, y]
    }
    fn pt(&self, p: [f64; 2]) -> [f64; 2] {
        self.p(p[0], p[1])
    }
    fn rect(&self, r: [f64; 4]) -> [f64; 4] {
        let a = self.p(r[0], r[1]);
        let b = self.p(r[0] + r[2], r[1] + r[3]);
        [a[0].min(b[0]), a[1].min(b[1]), (a[0] - b[0]).abs(), (a[1] - b[1]).abs()]
    }
    fn dir(&self, d: Dir) -> Dir {
        let mut d = d;
        if self.mx.is_some() {
            d = d.flip_x();
        }
        if self.my.is_some() {
            d = d.flip_y();
        }
        d
    }
    fn mirrored(&self) -> bool {
        self.mx.is_some() != self.my.is_some()
    }
}

fn letter(n: u32) -> String {
    let mut n = n.max(1);
    let mut s = Vec::new();
    while n > 0 {
        n -= 1;
        s.push(b'A' + (n % 26) as u8);
        n /= 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

impl Room {
    fn key(&self) -> String {
        if let Some(k) = &self.kind {
            return k.clone();
        }
        let s = self.name.replace("{n}", "").replace("{a}", "");
        s.split_whitespace().collect::<Vec<_>>().join(" ").trim_end_matches(['·', '-', '—']).trim().to_string()
    }
    fn all_rects(&self) -> Vec<[f64; 4]> {
        self.rect.into_iter().chain(self.rects.iter().copied()).collect()
    }
    /// Area in m².
    fn area(&self) -> f64 {
        self.all_rects().iter().map(|r| r[2] * r[3]).sum::<f64>() / 1e6
    }
}

impl Items {
    fn transformed(&self, tf: Tf, n: u32) -> Items {
        let seg = |v: &Vec<f64>| {
            let a = tf.p(v[0], v[1]);
            let b = tf.p(v[2], v[3]);
            let mut out = vec![a[0], a[1], b[0], b[1]];
            out.extend(v.get(4));
            out
        };
        Items {
            walls: self.walls.iter().map(seg).collect(),
            partitions: self.partitions.iter().map(seg).collect(),
            columns: self.columns.iter().map(|c| tf.pt(*c)).collect(),
            doors: self
                .doors
                .iter()
                .map(|d| Door { at: tf.pt(d.at), run: tf.dir(d.run), swing: d.swing.map(|s| tf.dir(s)), ..d.clone() })
                .collect(),
            windows: self.windows.iter().map(|w| Window { at: tf.pt(w.at), run: tf.dir(w.run), ..w.clone() }).collect(),
            stairs: self
                .stairs
                .iter()
                .map(|s| Stair { rect: tf.rect(s.rect), dir: tf.dir(s.dir), left: s.left != tf.mirrored(), ..s.clone() })
                .collect(),
            furniture: self
                .furniture
                .iter()
                .map(|f| Furniture { rect: tf.rect(f.rect), side: Some(tf.dir(f.side.unwrap_or(Dir::N))), ..f.clone() })
                .collect(),
            rooms: self
                .rooms
                .iter()
                .map(|r| Room {
                    name: r.name.replace("{n}", &n.to_string()).replace("{a}", &letter(n)),
                    kind: Some(r.key()),
                    rect: r.rect.map(|x| tf.rect(x)),
                    rects: r.rects.iter().map(|x| tf.rect(*x)).collect(),
                    at: r.at.map(|p| tf.pt(p)),
                    ..r.clone()
                })
                .collect(),
            labels: self.labels.iter().map(|l| Label { at: tf.pt(l.at), ..l.clone() }).collect(),
            entries: self.entries.iter().map(|e| Entry { at: tf.pt(e.at), dir: tf.dir(e.dir) }).collect(),
        }
    }

    fn extend(&mut self, o: Items) {
        self.walls.extend(o.walls);
        self.partitions.extend(o.partitions);
        self.columns.extend(o.columns);
        self.doors.extend(o.doors);
        self.windows.extend(o.windows);
        self.stairs.extend(o.stairs);
        self.furniture.extend(o.furniture);
        self.rooms.extend(o.rooms);
        self.labels.extend(o.labels);
        self.entries.extend(o.entries);
    }
}

impl Group {
    fn transforms(&self) -> Vec<Tf> {
        let [count, dx, dy] = self.repeat.unwrap_or([1.0, 0.0, 0.0]);
        let mut base = Vec::new();
        for o in &self.origins {
            for i in 0..(count.max(1.0) as u32) {
                let i = f64::from(i);
                base.push(Tf { dx: o[0] + i * dx, dy: o[1] + i * dy, ..Tf::default() });
            }
        }
        let mut all = base.clone();
        if let Some(a) = self.mirror_x {
            all.extend(base.iter().map(|t| Tf { mx: Some(a), ..*t }));
        }
        if let Some(b) = self.mirror_y {
            all.extend(base.iter().map(|t| Tf { my: Some(b), ..*t }));
        }
        all
    }
}

// ---------------------------------------------------------------- resolve ---

#[derive(Clone, Copy)]
struct Seg {
    a: [f64; 2],
    b: [f64; 2],
    t: f64,
}

struct Lv<'a> {
    src: &'a Level,
    segs: Vec<Seg>,
    it: Items,
    /// Outer bounding box: min x, min y, max x, max y.
    bb: [f64; 4],
}

impl Lv<'_> {
    fn w(&self) -> f64 {
        self.bb[2] - self.bb[0]
    }
    fn h(&self) -> f64 {
        self.bb[3] - self.bb[1]
    }
    fn area(&self) -> f64 {
        self.it.rooms.iter().filter(|r| r.list && !r.outdoor).map(Room::area).sum()
    }
    /// Thickness of the wall an opening sits in (thickest wall through its middle).
    fn thickness_at(&self, p: [f64; 2], fallback: f64) -> f64 {
        let mut best: Option<f64> = None;
        for s in &self.segs {
            let (minx, maxx) = (s.a[0].min(s.b[0]), s.a[0].max(s.b[0]));
            let (miny, maxy) = (s.a[1].min(s.b[1]), s.a[1].max(s.b[1]));
            let on = if (s.a[1] - s.b[1]).abs() < 1.0 {
                (p[1] - s.a[1]).abs() < 1.0 && p[0] >= minx - 1.0 && p[0] <= maxx + 1.0
            } else if (s.a[0] - s.b[0]).abs() < 1.0 {
                (p[0] - s.a[0]).abs() < 1.0 && p[1] >= miny - 1.0 && p[1] <= maxy + 1.0
            } else {
                false
            };
            if on {
                best = Some(best.map_or(s.t, |b: f64| b.max(s.t)));
            }
        }
        best.unwrap_or(fallback)
    }
}

fn resolve<'a>(plan: &Plan, l: &'a Level) -> Result<Lv<'a>, String> {
    let mut it = l.items.transformed(Tf::default(), 1);
    // Keep original names on ungrouped rooms ({n} means nothing there).
    for (r, src) in it.rooms.iter_mut().zip(&l.items.rooms) {
        r.kind = src.kind.clone().or_else(|| Some(src.key()));
    }
    for g in &l.group {
        for (i, tf) in g.transforms().into_iter().enumerate() {
            it.extend(g.items.transformed(tf, g.start + i as u32));
        }
    }

    let mut segs = Vec::new();
    let n = l.outline.len();
    for i in 0..n {
        segs.push(Seg { a: l.outline[i], b: l.outline[(i + 1) % n], t: plan.wall });
    }
    for (list, default) in [(&it.walls, plan.wall), (&it.partitions, plan.partition)] {
        for v in list {
            if v.len() != 4 && v.len() != 5 {
                return Err(format!("level {:?}: a wall needs 4 or 5 numbers, got {:?}", l.name, v));
            }
            segs.push(Seg { a: [v[0], v[1]], b: [v[2], v[3]], t: v.get(4).copied().unwrap_or(default) });
        }
    }
    if segs.is_empty() {
        return Err(format!("level {:?} has no walls", l.name));
    }
    let mut bb = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for s in &segs {
        for p in [s.a, s.b] {
            bb[0] = bb[0].min(p[0] - s.t / 2.0);
            bb[1] = bb[1].min(p[1] - s.t / 2.0);
            bb[2] = bb[2].max(p[0] + s.t / 2.0);
            bb[3] = bb[3].max(p[1] + s.t / 2.0);
        }
    }
    Ok(Lv { src: l, segs, it, bb })
}

// ----------------------------------------------------------------- output ---

pub struct Drawing {
    pub svg: String,
    /// HTML table of rooms and areas.
    pub schedule: String,
    /// Extra title-block rows: (label, value).
    pub facts: Vec<(String, String)>,
}

/// Reads and draws `content/plans/<name>.toml`. `id` must be unique per page
/// (the project slug): it prefixes the SVG pattern ids.
/// `lang` is a language code ("es", "en", "ko"): room names, level names
/// and the drawing's own words are translated with `content/plans/words.toml`.
pub fn load(name: &str, id: &str, lang: &str) -> Result<Drawing, String> {
    let path = format!("content/plans/{name}.toml");
    let text = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let mut plan: Plan = toml::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    let dict = Dict::load(lang)?;
    localize(&mut plan, &dict);
    render(&plan, id, &dict).map_err(|e| format!("{path}: {e}"))
}

// ------------------------------------------------------------ translation ---

const WORDS: &str = "content/plans/words.toml";

/// Words the drawing and schedule use themselves, besides the plans' names.
#[cfg(test)]
const UI_WORDS: [&str; 16] = [
    "Floor plan",
    "Room schedule",
    "Level",
    "Room",
    "No.",
    "Each",
    "Total",
    "Net floor area",
    "Levels",
    "Footprint",
    "Net area",
    "rooms",
    "net",
    "Up",
    "Dn",
    "Dimensions in metres",
];

/// English words of the plans → their translations, from `words.toml`:
/// `"Kitchen" = { es = "Cocina", ko = "주방" }`.
pub struct Dict {
    lang: String,
    words: BTreeMap<String, BTreeMap<String, String>>,
}

impl Dict {
    fn load(lang: &str) -> Result<Dict, String> {
        let words = match fs::read_to_string(WORDS) {
            Ok(text) => toml::from_str(&text).map_err(|e| format!("{WORDS}: {e}"))?,
            Err(_) => BTreeMap::new(),
        };
        Ok(Dict { lang: lang.to_string(), words })
    }

    fn lookup(&self, s: &str) -> Option<&String> {
        self.words.get(s).and_then(|m| m.get(&self.lang))
    }

    /// "Bedroom 2" is translated as "Bedroom" plus its number.
    fn numbered<'a>(&self, s: &'a str) -> Option<(&'a str, &'a str)> {
        let (head, tail) = s.rsplit_once(' ')?;
        (!tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())).then_some((head, tail))
    }

    fn t(&self, s: &str) -> String {
        if let Some(w) = self.lookup(s) {
            return w.clone();
        }
        if self.lang != "en"
            && let Some((head, n)) = self.numbered(s)
            && self.lookup(head).is_some()
        {
            return format!("{} {n}", self.t(head));
        }
        s.to_string()
    }

    /// Whether `s` has a translation (English always has).
    #[cfg(test)]
    fn knows(&self, s: &str) -> bool {
        self.lang == "en" || self.lookup(s).is_some() || self.numbered(s).is_some_and(|(h, _)| self.lookup(h).is_some())
    }
}

fn localize_items(it: &mut Items, d: &Dict) {
    for r in &mut it.rooms {
        // Numbered names keep their schedule name before they are translated.
        if r.kind.is_none() && r.name.contains('{') {
            r.kind = Some(r.key());
        }
        r.kind = r.kind.as_deref().map(|k| d.t(k));
        r.name = d.t(&r.name);
    }
    for l in &mut it.labels {
        l.text = d.t(&l.text);
    }
    for st in &mut it.stairs {
        st.label = Some(d.t(st.label.as_deref().unwrap_or("Up")));
    }
}

fn localize(plan: &mut Plan, d: &Dict) {
    for l in &mut plan.level {
        l.name = d.t(&l.name);
        localize_items(&mut l.items, d);
        for g in &mut l.group {
            localize_items(&mut g.items, d);
        }
    }
}

/// Every word of a plan that needs a translation.
#[cfg(test)]
fn plan_words(plan: &Plan) -> Vec<String> {
    let mut out = Vec::new();
    let items = |it: &Items, out: &mut Vec<String>| {
        for r in &it.rooms {
            if !r.name.contains('{') {
                out.push(r.name.clone());
            }
            out.extend(r.kind.clone());
        }
        out.extend(it.labels.iter().map(|l| l.text.clone()));
        out.extend(it.stairs.iter().filter_map(|s| s.label.clone()));
    };
    for l in &plan.level {
        out.push(l.name.clone());
        items(&l.items, &mut out);
        for g in &l.group {
            items(&g.items, &mut out);
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn area_text(a: f64) -> String {
    format!("{a:.2}\u{a0}m²")
}

/// Millimetre coordinates → SVG pixels for one level.
struct Pen {
    s: f64,
    ox: f64,
    oy: f64,
    k: f64,
}

impl Pen {
    fn x(&self, v: f64) -> f64 {
        self.ox + v * self.s
    }
    fn y(&self, v: f64) -> f64 {
        self.oy + v * self.s
    }
    fn l(&self, v: f64) -> f64 {
        v * self.s
    }
}

const LEFT: f64 = 84.0;
const RIGHT: f64 = 60.0;
const TOP: f64 = 84.0;
const BOTTOM: f64 = 100.0;
const GAP: f64 = 120.0;
const WIDTH: f64 = 1000.0;

fn render(plan: &Plan, id: &str, d: &Dict) -> Result<Drawing, String> {
    if plan.level.is_empty() {
        return Err("a plan needs at least one [[level]]".into());
    }
    let levels: Vec<Lv> = plan.level.iter().map(|l| resolve(plan, l)).collect::<Result<_, _>>()?;
    let n = levels.len() as f64;
    let content: f64 = levels.iter().map(Lv::w).sum();
    let s = (WIDTH - LEFT - RIGHT - GAP * (n - 1.0)) / content;
    // Labels grow a little on small buildings, which are drawn larger.
    let k = (s / 0.02).powf(0.3).clamp(1.0, 1.55);
    let hmax = levels.iter().map(Lv::h).fold(0.0, f64::max);
    let height = (TOP + hmax * s + BOTTOM).round();

    let mut o = String::new();
    let _ = write!(
        o,
        r##"<svg class="plate plan" viewBox="0 0 {WIDTH} {height}" role="img" aria-label="{aria}" xmlns="http://www.w3.org/2000/svg" fill="none" stroke="currentColor"><defs>"##,
        aria = esc(&d.t("Floor plan")),
    );
    let tile = 600.0 * s;
    let _ = write!(
        o,
        r##"<pattern id="{id}-tile" width="{tile:.2}" height="{tile:.2}" patternUnits="userSpaceOnUse"><path class="fl" d="M0 0H{tile:.2}M0 0V{tile:.2}"/></pattern><pattern id="{id}-paving" width="{pw:.2}" height="{ph:.2}" patternUnits="userSpaceOnUse"><path class="fl" d="M0 0H{pw:.2}M0 0V{ph:.2}"/></pattern><pattern id="{id}-timber" width="{pw:.2}" height="{tb:.2}" patternUnits="userSpaceOnUse"><path class="fl" d="M0 0H{pw:.2}"/></pattern><pattern id="{id}-hatch" width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path class="fl" d="M0 0V5"/></pattern></defs>"##,
        pw = 900.0 * s,
        ph = 450.0 * s,
        tb = (180.0 * s).max(2.5),
    );

    let mut x0 = LEFT;
    for (n, lv) in levels.iter().enumerate() {
        let pen = Pen { s, ox: x0 - lv.bb[0] * s, oy: TOP - lv.bb[1] * s, k };
        draw_level(&mut o, plan, lv, &pen, id, d, n + 1);
        x0 += lv.w() * s + GAP;
    }
    north_arrow(&mut o, WIDTH - 34.0, 40.0);
    scale_bar(&mut o, s, WIDTH - RIGHT, height - 30.0);
    o.push_str("</svg>");

    // Schedule: one row per (level, room type, area), counting repeats.
    let mut rows = String::new();
    let mut total = 0.0;
    for lv in &levels {
        let mut agg: BTreeMap<(String, i64), (usize, f64, bool, usize)> = BTreeMap::new();
        for (i, r) in lv.it.rooms.iter().filter(|r| r.list).enumerate() {
            let a = r.area();
            let e = agg.entry((r.kind.clone().unwrap_or_else(|| r.key()), (a * 100.0).round() as i64)).or_insert((0, a, r.outdoor, i));
            e.0 += 1;
        }
        let mut list: Vec<_> = agg.into_iter().collect();
        list.sort_by_key(|(_, v)| v.3);
        for ((name, _), (count, a, outdoor, _)) in list {
            let tot = a * count as f64;
            if !outdoor {
                total += tot;
            }
            let _ = write!(
                rows,
                r#"<tr{cls}><td>{lvl}</td><td>{name}</td><td class="mono">{count}</td><td class="mono">{each}</td><td class="mono">{tot}</td></tr>"#,
                cls = if outdoor { r#" class="out""# } else { "" },
                lvl = esc(&lv.src.name),
                name = esc(&name),
                each = area_text(a),
                tot = area_text(tot),
            );
        }
    }
    let schedule = format!(
        r#"<table class="schedule"><caption class="mono">{}</caption><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>{rows}</tbody><tfoot><tr><td colspan="4">{}</td><td class="mono">{}</td></tr></tfoot></table>"#,
        d.t("Room schedule"),
        d.t("Level"),
        d.t("Room"),
        d.t("No."),
        d.t("Each"),
        d.t("Total"),
        d.t("Net floor area"),
        area_text(total)
    );

    let mut facts = Vec::new();
    if levels.len() > 1 {
        facts.push((d.t("Levels"), levels.len().to_string()));
    }
    let fp = &levels[0];
    facts.push((
        d.t("Footprint"),
        format!("{:.2} × {:.2} m", fp.w() / 1000.0, fp.h() / 1000.0),
    ));
    facts.push((d.t("Net area"), area_text(total)));

    Ok(Drawing { svg: o, schedule, facts })
}

#[allow(clippy::too_many_arguments)]
fn draw_level(o: &mut String, plan: &Plan, lv: &Lv, p: &Pen, id: &str, d: &Dict, number: usize) {
    let it = &lv.it;
    let k = p.k;
    let (bx0, by0, bx1, by1) = (p.x(lv.bb[0]), p.y(lv.bb[1]), p.x(lv.bb[2]), p.y(lv.bb[3]));

    // Floor finishes.
    for r in &it.rooms {
        if let Some(f) = &r.floor {
            for q in r.all_rects() {
                let _ = write!(
                    o,
                    r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="url(#{id}-{f})" stroke="none"/>"##,
                    p.x(q[0]),
                    p.y(q[1]),
                    p.l(q[2]),
                    p.l(q[3])
                );
            }
        }
    }

    // Structural grid, bubbles at the bottom and right.
    if let Some(g) = &lv.src.grid {
        for (i, gx) in g.x.iter().enumerate() {
            let x = p.x(*gx);
            let _ = write!(o, r#"<line class="gd" x1="{x:.1}" y1="{:.1}" x2="{x:.1}" y2="{:.1}"/>"#, by0 - 12.0, by1 + 26.0);
            bubble(o, x, by1 + 34.0, &(i + 1).to_string());
        }
        for (i, gy) in g.y.iter().enumerate() {
            let y = p.y(*gy);
            let _ = write!(o, r#"<line class="gd" x1="{:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}"/>"#, bx0 - 12.0, bx1 + 26.0);
            bubble(o, bx1 + 34.0, y, &letter(i as u32 + 1));
        }
    }

    // Walls in section: a Chak outline (A-WALL) under K'ak' poché
    // (A-WALL-PATT). Each layer is drawn whole, so junctions merge.
    for (class, grow) in [("w wo", 1.3), ("w wi", -1.3)] {
        let _ = write!(o, r#"<g class="{class}">"#);
        for sg in &lv.segs {
            let _ = write!(
                o,
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke-width="{:.2}"/>"#,
                p.x(sg.a[0]),
                p.y(sg.a[1]),
                p.x(sg.b[0]),
                p.y(sg.b[1]),
                (p.l(sg.t) + grow).max(0.6)
            );
        }
        o.push_str("</g>");
    }
    for c in &it.columns {
        let h = p.l(plan.column);
        let _ = write!(o, r#"<rect class="col" x="{:.1}" y="{:.1}" width="{h:.1}" height="{h:.1}"/>"#, p.x(c[0]) - h / 2.0, p.y(c[1]) - h / 2.0);
    }

    // Openings.
    for d in &it.doors {
        let (rx, ry) = d.run.v();
        let mid = [d.at[0] + rx * d.w / 2.0, d.at[1] + ry * d.w / 2.0];
        let t = lv.thickness_at(mid, plan.wall);
        door(o, p, d, t);
    }
    for w in &it.windows {
        let (rx, ry) = w.run.v();
        let mid = [w.at[0] + rx * w.w / 2.0, w.at[1] + ry * w.w / 2.0];
        let t = lv.thickness_at(mid, plan.wall);
        window(o, p, w, t);
    }

    // Stairs and furniture.
    for st in &it.stairs {
        stair(o, p, st);
    }
    for f in &it.furniture {
        let side = f.side.unwrap_or(Dir::N);
        let (cx, cy) = (p.x(f.rect[0] + f.rect[2] / 2.0), p.y(f.rect[1] + f.rect[3] / 2.0));
        let (w, h) = match side {
            Dir::N | Dir::S => (p.l(f.rect[2]), p.l(f.rect[3])),
            _ => (p.l(f.rect[3]), p.l(f.rect[2])),
        };
        let _ = write!(o, r#"<g class="fx" transform="translate({cx:.1} {cy:.1}) rotate({})">"#, side.angle());
        symbol(o, &f.kind, w, h, p.s, f.seats);
        o.push_str("</g>");
    }

    // Entrances.
    for e in &it.entries {
        let (dx, dy) = e.dir.v();
        let (x, y) = (p.x(e.at[0]), p.y(e.at[1]));
        let (nx, ny) = (-dy, dx);
        let a = 7.0 * k;
        let (tx, ty) = (x - dx * a * 0.6, y - dy * a * 0.6);
        let (bx, by) = (x - dx * a * 2.0, y - dy * a * 2.0);
        let _ = write!(
            o,
            r#"<path class="acf" d="M{tx:.1} {ty:.1}L{:.1} {:.1}L{:.1} {:.1}Z"/>"#,
            bx + nx * a * 0.75,
            by + ny * a * 0.75,
            bx - nx * a * 0.75,
            by - ny * a * 0.75
        );
    }

    // Room names and areas.
    for r in &it.rooms {
        let style = r.style.as_deref().unwrap_or("room");
        if style == "none" {
            continue;
        }
        let rects = r.all_rects();
        let at = r.at.unwrap_or_else(|| {
            let q = rects.first().copied().unwrap_or([0.0; 4]);
            [q[0] + q[2] / 2.0, q[1] + q[3] / 2.0]
        });
        let (x, y) = (p.x(at[0]), p.y(at[1]));
        if style == "tag" {
            let _ = write!(o, r#"<text class="lbl tag" x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="{:.1}">{}</text>"#, y + 4.0 * k, 10.5 * k, esc(&r.name.to_uppercase()));
        } else {
            let _ = write!(
                o,
                r#"<text class="lbl rn" x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="{:.1}">{}</text>"#,
                y - 1.0 * k,
                8.0 * k,
                esc(&r.name.to_uppercase())
            );
            if r.list {
                let _ = write!(
                    o,
                    r#"<text class="lbl ra" x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="{:.1}">{}</text>"#,
                    y + 9.0 * k,
                    6.6 * k,
                    area_text(r.area())
                );
            }
        }
    }
    for l in &it.labels {
        let _ = write!(
            o,
            r#"<text class="lbl note" x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{:.1}">{}</text>"#,
            p.x(l.at[0]),
            p.y(l.at[1]) + 3.0 * k,
            6.6 * k,
            esc(&l.text.to_uppercase())
        );
    }

    // Dimensions: overall outer size, plus optional chains.
    let chains = |vals: &Vec<f64>| -> Vec<f64> {
        let mut v = vals.clone();
        v.sort_by(f64::total_cmp);
        v.dedup();
        v
    };
    let xs = chains(&lv.src.xdims);
    let ys = chains(&lv.src.ydims);
    let top_overall = if xs.len() > 1 { by0 - 48.0 } else { by0 - 28.0 };
    if xs.len() > 1 {
        for w in xs.windows(2) {
            dim(o, p.x(w[0]), by0 - 28.0, p.x(w[1]), by0 - 28.0, w[1] - w[0], false, k);
        }
    }
    dim(o, bx0, top_overall, bx1, top_overall, lv.w(), false, k);
    let left_overall = if ys.len() > 1 { bx0 - 48.0 } else { bx0 - 28.0 };
    if ys.len() > 1 {
        for w in ys.windows(2) {
            dim(o, bx0 - 28.0, p.y(w[0]), bx0 - 28.0, p.y(w[1]), w[1] - w[0], true, k);
        }
    }
    dim(o, left_overall, by0, left_overall, by1, lv.h(), true, k);

    // Level title.
    let ty = by1 + if lv.src.grid.is_some() { 66.0 } else { 44.0 };
    let tx = bx0 + 22.0;
    let _ = write!(
        o,
        r#"<circle class="tagc" cx="{:.1}" cy="{:.1}" r="9"/><text class="tagn" x="{:.1}" y="{:.1}" text-anchor="middle" font-size="9">{number}</text><text class="title" x="{tx:.1}" y="{ty:.1}" font-size="11">{}</text><text class="ra" x="{tx:.1}" y="{:.1}" font-size="7.5">{} {} · {} {} · {}</text>"#,
        bx0 + 9.0,
        ty + 1.5,
        bx0 + 9.0,
        ty + 4.6,
        esc(&lv.src.name.to_uppercase()),
        ty + 13.0,
        lv.it.rooms.iter().filter(|r| r.list && !r.outdoor).count(),
        esc(&d.t("rooms").to_uppercase()),
        esc(&d.t("net").to_uppercase()),
        area_text(lv.area()).to_uppercase(),
        esc(&d.t("Dimensions in metres").to_uppercase())
    );
}

fn bubble(o: &mut String, x: f64, y: f64, t: &str) {
    let _ = write!(
        o,
        r#"<circle class="th" cx="{x:.1}" cy="{y:.1}" r="7.5"/><text class="gt" x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="7.5">{t}</text>"#,
        y + 2.7
    );
}

/// A dimension line with tick marks; `vertical` writes the text sideways.
#[allow(clippy::too_many_arguments)]
fn dim(o: &mut String, x1: f64, y1: f64, x2: f64, y2: f64, mm: f64, vertical: bool, k: f64) {
    let t = 3.0;
    let _ = write!(
        o,
        r#"<g class="dm"><line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#,
        x1 - if vertical { 0.0 } else { 6.0 },
        y1 - if vertical { 6.0 } else { 0.0 },
        x2 + if vertical { 0.0 } else { 6.0 },
        y2 + if vertical { 6.0 } else { 0.0 }
    );
    for (x, y) in [(x1, y1), (x2, y2)] {
        let _ = write!(o, r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#, x - t, y + t, x + t, y - t);
        if vertical {
            let _ = write!(o, r#"<line x1="{:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}"/>"#, x - 5.0, x + 5.0);
        } else {
            let _ = write!(o, r#"<line x1="{x:.1}" y1="{:.1}" x2="{x:.1}" y2="{:.1}"/>"#, y - 5.0, y + 5.0);
        }
    }
    o.push_str("</g>");
    let label = format!("{:.2}", mm.max(0.0) / 1000.0);
    let fs = 6.6 * k.min(1.3);
    if vertical {
        let (x, y) = (x1 - 4.0, (y1 + y2) / 2.0);
        let _ = write!(o, r#"<text class="dt" transform="translate({x:.1} {y:.1}) rotate(-90)" text-anchor="middle" font-size="{fs:.1}">{label}</text>"#);
    } else {
        let _ = write!(o, r#"<text class="dt" x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{fs:.1}">{label}</text>"#, (x1 + x2) / 2.0, y1 - 4.0);
    }
}

fn door(o: &mut String, p: &Pen, d: &Door, t: f64) {
    let (rx, ry) = d.run.v();
    // The swing side defaults to the right of the run.
    let (sx, sy) = d.swing.map_or((-ry, rx), Dir::v);
    let (hx, hy) = (p.x(d.at[0]), p.y(d.at[1]));
    let w = p.l(d.w);
    let th = p.l(t) + 0.6;
    // Cut the wall.
    let (ax, ay) = (hx - sx * th / 2.0, hy - sy * th / 2.0);
    let (bx, by) = (hx + rx * w + sx * th / 2.0, hy + ry * w + sy * th / 2.0);
    let _ = write!(
        o,
        r#"<rect class="cut" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}"/>"#,
        ax.min(bx),
        ay.min(by),
        (ax - bx).abs(),
        (ay - by).abs()
    );
    // Jambs.
    for (jx, jy) in [(hx, hy), (hx + rx * w, hy + ry * w)] {
        let _ = write!(
            o,
            r#"<line class="th" x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#,
            jx - sx * th / 2.0,
            jy - sy * th / 2.0,
            jx + sx * th / 2.0,
            jy + sy * th / 2.0
        );
    }
    let face = th / 2.0;
    let leaf = |o: &mut String, hx: f64, hy: f64, rx: f64, ry: f64, w: f64| {
        let (fx, fy) = (hx + sx * face, hy + sy * face);
        let (ox, oy) = (fx + sx * w, fy + sy * w);
        let (cx, cy) = (fx + rx * w, fy + ry * w);
        let sweep = if sx * ry - sy * rx > 0.0 { 1 } else { 0 };
        let _ = write!(
            o,
            r#"<line class="leaf" x1="{fx:.1}" y1="{fy:.1}" x2="{ox:.1}" y2="{oy:.1}"/><path class="arc" d="M{ox:.1} {oy:.1}A{w:.1} {w:.1} 0 0 {sweep} {cx:.1} {cy:.1}"/>"#
        );
    };
    match d.style.as_deref().unwrap_or("swing") {
        "double" => {
            leaf(o, hx, hy, rx, ry, w / 2.0);
            leaf(o, hx + rx * w, hy + ry * w, -rx, -ry, w / 2.0);
        }
        "slide" => {
            for (a, b, off) in [(0.0, 0.55, -0.18), (0.45, 1.0, 0.18)] {
                let _ = write!(
                    o,
                    r#"<line class="leaf" x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#,
                    hx + rx * w * a + sx * th * off,
                    hy + ry * w * a + sy * th * off,
                    hx + rx * w * b + sx * th * off,
                    hy + ry * w * b + sy * th * off
                );
            }
        }
        "opening" => {}
        _ => leaf(o, hx, hy, rx, ry, w),
    }
}

fn window(o: &mut String, p: &Pen, win: &Window, t: f64) {
    let (rx, ry) = win.run.v();
    let (nx, ny) = (-ry, rx);
    let (hx, hy) = (p.x(win.at[0]), p.y(win.at[1]));
    let w = p.l(win.w);
    let th = p.l(t);
    let (ax, ay) = (hx - nx * th / 2.0, hy - ny * th / 2.0);
    let (bx, by) = (hx + rx * w + nx * th / 2.0, hy + ry * w + ny * th / 2.0);
    let (x, y, ww, hh) = (ax.min(bx), ay.min(by), (ax - bx).abs(), (ay - by).abs());
    let _ = write!(o, r#"<rect class="cut" x="{x:.1}" y="{y:.1}" width="{ww:.1}" height="{hh:.1}"/><rect class="th" x="{x:.1}" y="{y:.1}" width="{ww:.1}" height="{hh:.1}"/>"#);
    for off in [-0.12, 0.12] {
        let _ = write!(
            o,
            r#"<line class="gl" x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#,
            hx + nx * th * off,
            hy + ny * th * off,
            hx + rx * w + nx * th * off,
            hy + ry * w + ny * th * off
        );
    }
}

fn arrowhead(o: &mut String, x: f64, y: f64, dx: f64, dy: f64) {
    let a = 4.0;
    let (nx, ny) = (-dy, dx);
    let _ = write!(
        o,
        r#"<path class="inkf" d="M{x:.1} {y:.1}L{:.1} {:.1}L{:.1} {:.1}Z"/>"#,
        x - dx * a * 1.6 + nx * a * 0.7,
        y - dy * a * 1.6 + ny * a * 0.7,
        x - dx * a * 1.6 - nx * a * 0.7,
        y - dy * a * 1.6 - ny * a * 0.7
    );
}

fn stair(o: &mut String, p: &Pen, st: &Stair) {
    let r = st.rect;
    let (cx, cy) = (p.x(r[0] + r[2] / 2.0), p.y(r[1] + r[3] / 2.0));
    // Local frame: going up is towards -y.
    let (w, h) = match st.dir {
        Dir::N | Dir::S => (p.l(r[2]), p.l(r[3])),
        _ => (p.l(r[3]), p.l(r[2])),
    };
    let ang = st.dir.angle();
    let n = st.steps.max(2) as f64;
    let mut g = String::new();
    let _ = write!(g, r#"<rect class="f" x="{:.1}" y="{:.1}" width="{w:.1}" height="{h:.1}"/>"#, -w / 2.0, -h / 2.0);
    // Where the label goes, in the local frame.
    let start: (f64, f64) = if st.kind.as_deref() == Some("u") {
        let land = w / 2.0;
        let run = h - land;
        let side = if st.left { -1.0 } else { 1.0 };
        let (c1, c2) = (side * w / 4.0, -side * w / 4.0);
        let step = run / n;
        for i in 1..(n as u32) {
            let y = h / 2.0 - step * f64::from(i);
            let _ = write!(g, r#"<line class="th" x1="{:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}"/>"#, -w / 2.0, w / 2.0);
        }
        let ly = -h / 2.0 + land;
        let _ = write!(
            g,
            r#"<line class="th" x1="{:.1}" y1="{ly:.1}" x2="{:.1}" y2="{ly:.1}"/><line x1="0" y1="{ly:.1}" x2="0" y2="{:.1}" stroke-width="1.4"/>"#,
            -w / 2.0,
            w / 2.0,
            h / 2.0
        );
        let my = -h / 2.0 + land / 2.0;
        let (y0, y3) = (h / 2.0 - step / 2.0, h / 2.0 - step);
        let _ = write!(g, r#"<path class="th" d="M{c1:.1} {y0:.1}V{my:.1}H{c2:.1}V{y3:.1}"/><circle class="inkf" cx="{c1:.1}" cy="{y0:.1}" r="1.6"/>"#);
        arrowhead(&mut g, c2, y3, 0.0, 1.0);
        (c1, y0 + 8.0)
    } else {
        let step = h / n;
        for i in 1..(n as u32) {
            let y = h / 2.0 - step * f64::from(i);
            let _ = write!(g, r#"<line class="th" x1="{:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}"/>"#, -w / 2.0, w / 2.0);
        }
        let (y0, y1) = (h / 2.0 - step / 2.0, -h / 2.0 + 3.0);
        let _ = write!(g, r#"<line class="th" x1="0" y1="{y0:.1}" x2="0" y2="{y1:.1}"/><circle class="inkf" cx="0" cy="{y0:.1}" r="1.6"/>"#);
        arrowhead(&mut g, 0.0, y1, 0.0, -1.0);
        (0.0, y0 + 8.0)
    };
    let _ = write!(o, r#"<g class="st" transform="translate({cx:.1} {cy:.1}) rotate({ang})">{g}</g>"#);
    let (sa, ca) = (ang.to_radians().sin(), ang.to_radians().cos());
    let (lx, ly) = (cx + start.0 * ca - start.1 * sa, cy + start.0 * sa + start.1 * ca);
    let label = st.label.as_deref().unwrap_or("Up");
    let _ = write!(
        o,
        r#"<text class="lbl dt" x="{lx:.1}" y="{:.1}" text-anchor="middle" font-size="6">{}</text>"#,
        ly + 2.0,
        esc(&label.to_uppercase())
    );
}

/// Draws a piece of furniture centred on the origin, `w`×`h` pixels, its
/// back towards -y. `s` converts millimetres to pixels.
fn symbol(o: &mut String, kind: &str, w: f64, h: f64, s: f64, seats: Option<u32>) {
    let (x0, y0) = (-w / 2.0, -h / 2.0);
    let rect = |o: &mut String, x: f64, y: f64, w: f64, h: f64, rx: f64| {
        let _ = write!(o, r#"<rect class="f" x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" rx="{rx:.1}"/>"#);
    };
    let line = |o: &mut String, x1: f64, y1: f64, x2: f64, y2: f64| {
        let _ = write!(o, r#"<line class="th" x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}"/>"#);
    };
    let (cw, cd, gap) = (450.0 * s, 420.0 * s, 70.0 * s);
    let chair = |o: &mut String, x: f64, y: f64, rot: f64| {
        let _ = write!(
            o,
            r#"<g transform="translate({x:.1} {y:.1}) rotate({rot:.0})"><rect class="f" x="{:.1}" y="{:.1}" width="{cw:.1}" height="{cd:.1}" rx="1"/><line class="th" x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/></g>"#,
            -cw / 2.0,
            -cd / 2.0,
            -cw / 2.0,
            -cd / 2.0 + cd * 0.22,
            cw / 2.0,
            -cd / 2.0 + cd * 0.22
        );
    };
    match kind {
        "bed" => {
            rect(o, x0, y0, w, h, 1.0);
            let n = if w > 1300.0 * s { 2.0 } else { 1.0 };
            let m = w * 0.06;
            let pw = (w - m * (n + 1.0)) / n;
            for i in 0..(n as u32) {
                rect(o, x0 + m + f64::from(i) * (pw + m), y0 + h * 0.04, pw, h * 0.13, 1.5);
            }
            line(o, x0, y0 + h * 0.3, x0 + w, y0 + h * 0.3);
            line(o, x0 + w * 0.62, y0 + h * 0.3, x0 + w, y0 + h * 0.42);
        }
        "sofa" => {
            rect(o, x0, y0, w, h, 1.5);
            let arm = (200.0 * s).min(w * 0.15);
            line(o, x0 + arm, y0 + h * 0.28, x0 + w - arm, y0 + h * 0.28);
            line(o, x0 + arm, y0 + h * 0.28, x0 + arm, y0 + h);
            line(o, x0 + w - arm, y0 + h * 0.28, x0 + w - arm, y0 + h);
            let seats = ((w - 2.0 * arm) / (650.0 * s)).round().max(1.0);
            for i in 1..(seats as u32) {
                let x = x0 + arm + (w - 2.0 * arm) * f64::from(i) / seats;
                line(o, x, y0 + h * 0.28, x, y0 + h);
            }
        }
        "chair" => {
            rect(o, x0, y0, w, h, 1.5);
            line(o, x0 + w * 0.12, y0 + h * 0.25, x0 + w * 0.88, y0 + h * 0.25);
        }
        "table" | "desk" => {
            if kind == "desk" {
                chair(o, 0.0, y0 + h + gap + cd / 2.0, 180.0);
            } else {
                let horizontal = w >= h;
                let len = if horizontal { w } else { h };
                let n = seats.unwrap_or_else(|| (len / (650.0 * s)).floor().max(1.0) as u32);
                for i in 0..n {
                    let t = -len / 2.0 + len * (f64::from(i) + 0.5) / f64::from(n);
                    if horizontal {
                        chair(o, t, y0 - gap - cd / 2.0, 0.0);
                        chair(o, t, -y0 + gap + cd / 2.0, 180.0);
                    } else {
                        chair(o, x0 - gap - cd / 2.0, t, 270.0);
                        chair(o, -x0 + gap + cd / 2.0, t, 90.0);
                    }
                }
            }
            rect(o, x0, y0, w, h, 0.5);
        }
        "round" => {
            let r = w.min(h) / 2.0;
            let n = seats.unwrap_or(4);
            for i in 0..n {
                let a = 360.0 * f64::from(i) / f64::from(n);
                let rad = (a - 90.0).to_radians();
                let d = r + gap + cd / 2.0;
                chair(o, d * rad.cos(), d * rad.sin(), a);
            }
            let _ = write!(o, r#"<circle class="f" cx="0" cy="0" r="{r:.1}"/>"#);
        }
        "wc" => {
            rect(o, x0, y0, w, h * 0.28, 1.0);
            let _ = write!(o, r#"<ellipse class="f" cx="0" cy="{:.1}" rx="{:.1}" ry="{:.1}"/>"#, y0 + h * 0.64, w * 0.4, h * 0.36);
        }
        "basin" => {
            rect(o, x0, y0, w, h, 2.0);
            let _ = write!(o, r#"<ellipse class="f" cx="0" cy="{:.1}" rx="{:.1}" ry="{:.1}"/><circle class="inkf" cx="0" cy="{:.1}" r="0.9"/>"#, h * 0.06, w * 0.34, h * 0.3, y0 + h * 0.13);
        }
        "bath" => {
            rect(o, x0, y0, w, h, 1.0);
            let m = 70.0 * s;
            rect(o, x0 + m, y0 + m, w - 2.0 * m, h - 2.0 * m, (h.min(w) - 2.0 * m) * 0.35);
            let _ = write!(o, r#"<circle class="th" cx="{:.1}" cy="0" r="1.4"/>"#, x0 + w * 0.15);
        }
        "shower" => {
            rect(o, x0, y0, w, h, 0.5);
            line(o, x0, y0, x0 + w, y0 + h);
            line(o, x0 + w, y0, x0, y0 + h);
            let _ = write!(o, r#"<circle class="f" cx="0" cy="0" r="2"/>"#);
        }
        "sink" => {
            rect(o, x0, y0, w, h, 1.0);
            let m = w * 0.06;
            let bw = (w - 3.0 * m) / 2.0;
            rect(o, x0 + m, y0 + h * 0.2, bw, h * 0.7, 1.5);
            rect(o, x0 + 2.0 * m + bw, y0 + h * 0.2, bw, h * 0.7, 1.5);
        }
        "hob" => {
            rect(o, x0, y0, w, h, 0.5);
            let r = (w.min(h) / 2.0) * 0.32;
            for (fx, fy) in [(-0.25, -0.25), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)] {
                let _ = write!(o, r#"<circle class="th" cx="{:.1}" cy="{:.1}" r="{r:.1}"/>"#, w * fx, h * fy);
            }
        }
        "wardrobe" | "rack" => {
            if kind == "wardrobe" {
                rect(o, x0, y0, w, h, 0.0);
            }
            line(o, x0 + 2.0, 0.0, x0 + w - 2.0, 0.0);
            let n = (w / 9.0).floor().max(2.0) as u32;
            for i in 1..n {
                let x = x0 + w * f64::from(i) / f64::from(n);
                line(o, x - 1.2, -h * 0.3, x + 1.2, h * 0.3);
            }
        }
        "lift" => {
            rect(o, x0, y0, w, h, 0.0);
            let m = w.min(h) * 0.1;
            rect(o, x0 + m, y0 + m, w - 2.0 * m, h - 2.0 * m, 0.0);
            line(o, x0 + m, y0 + m, x0 + w - m, y0 + h - m);
            line(o, x0 + w - m, y0 + m, x0 + m, y0 + h - m);
        }
        "tree" => {
            let r = w.min(h) / 2.0;
            let _ = write!(o, r#"<circle class="canopy" cx="0" cy="0" r="{r:.1}"/><circle class="th" cx="0" cy="0" r="{:.1}"/><circle class="inkf" cx="0" cy="0" r="1.6"/>"#, r * 0.62);
        }
        "plant" => {
            let r = w.min(h) / 2.0;
            let _ = write!(o, r#"<circle class="f" cx="0" cy="0" r="{r:.1}"/><circle class="th" cx="0" cy="0" r="{:.1}"/>"#, r * 0.45);
        }
        "curtain" => {
            let n = (w / 6.0).floor().max(2.0) as u32;
            let mut d = format!("M{x0:.1} 0");
            for i in 0..n {
                let xa = x0 + w * (f64::from(i) + 0.5) / f64::from(n);
                let xb = x0 + w * (f64::from(i) + 1.0) / f64::from(n);
                let dy = if i % 2 == 0 { -2.0 } else { 2.0 };
                let _ = write!(d, "Q{xa:.1} {dy:.1} {xb:.1} 0");
            }
            let _ = write!(o, r#"<path class="th" d="{d}"/>"#);
        }
        "counter" => {
            rect(o, x0, y0, w, h, 0.0);
            let _ = write!(o, r#"<line class="hid" x1="{x0:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#, y0 + h * 0.6, -x0, y0 + h * 0.6);
        }
        "display" => {
            rect(o, x0, y0, w, h, 0.0);
            let m = w.min(h) * 0.14;
            rect(o, x0 + m, y0 + m, w - 2.0 * m, h - 2.0 * m, 0.0);
        }
        "car" => {
            rect(o, x0, y0, w, h, w * 0.18);
            rect(o, x0 + w * 0.12, y0 + h * 0.28, w * 0.76, h * 0.4, 2.0);
        }
        _ => rect(o, x0, y0, w, h, 0.0),
    }
}

/// North: La Flama, the flame from the studio's symbol, pointing north.
fn north_arrow(o: &mut String, x: f64, y: f64) {
    // Drawn in a 40 × 32 box centred on (x, y), its point at the top.
    let pts = [
        (17.0, 0.0),
        (23.0, 0.0),
        (25.5, 10.5),
        (39.0, 21.0),
        (40.0, 30.5),
        (33.5, 31.5),
        (20.0, 21.5),
        (6.5, 31.5),
        (0.0, 30.5),
        (1.0, 21.0),
        (14.5, 10.5),
    ];
    let scale = 0.62;
    let mut d = String::new();
    for (i, (px, py)) in pts.iter().enumerate() {
        let _ = write!(d, "{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, x + (px - 20.0) * scale, y + (py - 16.0) * scale);
    }
    let _ = write!(
        o,
        r#"<g class="na"><path class="flama" d="{d}Z"/><text class="gt" x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="8">N</text></g>"#,
        y - 16.0 * scale - 5.0
    );
}

/// A bar of five segments, right-aligned at `right`.
fn scale_bar(o: &mut String, s: f64, right: f64, y: f64) {
    let step_m = [1.0, 2.0, 5.0, 10.0].into_iter().find(|m| m * 1000.0 * s * 5.0 >= 90.0).unwrap_or(10.0);
    let seg = step_m * 1000.0 * s;
    let x0 = right - seg * 5.0;
    let _ = write!(o, r#"<g class="sb">"#);
    for i in 0..5 {
        let x = x0 + seg * f64::from(i);
        let cls = if i % 2 == 0 { "inkf" } else { "f" };
        let _ = write!(o, r#"<rect class="{cls} th" x="{x:.1}" y="{y:.1}" width="{seg:.1}" height="4"/>"#);
    }
    o.push_str("</g>");
    for i in [0, 1, 5] {
        let x = x0 + seg * f64::from(i);
        let v = step_m * f64::from(i);
        let t = if i == 5 { format!("{v} m") } else { format!("{v}") };
        let _ = write!(o, r#"<text class="dt" x="{x:.1}" y="{:.1}" text-anchor="middle" font-size="7">{t}</text>"#, y + 15.0);
    }
}

// ------------------------------------------------------------------ tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters() {
        assert_eq!(letter(1), "A");
        assert_eq!(letter(4), "D");
        assert_eq!(letter(27), "AA");
    }

    #[test]
    fn mirror_flips_points_and_directions() {
        let tf = Tf { dx: 100.0, mx: Some(1000.0), ..Tf::default() };
        assert_eq!(tf.p(0.0, 5.0), [1900.0, 5.0]);
        assert_eq!(tf.dir(Dir::E), Dir::W);
        assert_eq!(tf.dir(Dir::N), Dir::N);
        assert_eq!(tf.rect([0.0, 0.0, 200.0, 50.0]), [1700.0, 0.0, 200.0, 50.0]);
        assert!(tf.mirrored());
    }

    #[test]
    fn groups_number_their_rooms() {
        let src = r#"
            [[level]]
            name = "Typical"
            outline = [[0, 0], [8000, 0], [8000, 4000], [0, 4000]]
            [[level.group]]
            repeat = [2, 4000, 0]
            mirror_y = 4000
            start = 101
            rooms = [{ name = "Room {n}", rect = [0, 0, 4000, 2000] }]
        "#;
        let plan: Plan = toml::from_str(src).unwrap();
        let lv = resolve(&plan, &plan.level[0]).unwrap();
        let names: Vec<_> = lv.it.rooms.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Room 101", "Room 102", "Room 103", "Room 104"]);
        assert_eq!(lv.it.rooms[3].rect, Some([4000.0, 6000.0, 4000.0, 2000.0]));
        assert!(lv.it.rooms.iter().all(|r| r.key() == "Room"));
        let d = render(&plan, "t", &Dict::load("en").unwrap()).unwrap();
        assert!(d.schedule.contains("<td>Room</td><td class=\"mono\">4</td>"));
    }

    #[test]
    fn opening_takes_the_thickness_of_its_wall() {
        let src = r#"
            [[level]]
            name = "G"
            outline = [[0, 0], [5000, 0], [5000, 5000], [0, 5000]]
            partitions = [[2500, 0, 2500, 5000]]
        "#;
        let plan: Plan = toml::from_str(src).unwrap();
        let lv = resolve(&plan, &plan.level[0]).unwrap();
        assert_eq!(lv.thickness_at([1000.0, 0.0], 0.0), 300.0);
        assert_eq!(lv.thickness_at([2500.0, 2000.0], 0.0), 120.0);
        assert_eq!(lv.thickness_at([1000.0, 1000.0], 7.0), 7.0);
    }

    #[test]
    fn every_plan_draws() {
        let mut found = 0;
        for entry in fs::read_dir("content/plans").unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") || path.ends_with("words.toml") {
                continue;
            }
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            for lang in ["es", "en", "ko"] {
                let d = load(&name, &name, lang).unwrap_or_else(|e| panic!("{e}"));
                assert!(d.svg.starts_with("<svg") && d.svg.ends_with("</svg>"));
                assert!(!d.svg.contains("NaN"), "{name}: NaN in drawing");
            }
            let d = load(&name, &name, "en").unwrap();
            assert!(d.schedule.contains("Net floor area"));
            found += 1;
        }
        assert!(found >= 6, "expected the six sample plans, found {found}");
    }

    #[test]
    fn every_word_is_translated() {
        for lang in ["es", "ko"] {
            let d = Dict::load(lang).unwrap();
            let mut missing: Vec<String> = UI_WORDS.iter().filter(|w| !d.knows(w)).map(|w| w.to_string()).collect();
            for entry in fs::read_dir("content/plans").unwrap() {
                let path = entry.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") || path.ends_with("words.toml") {
                    continue;
                }
                let plan: Plan = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
                missing.extend(plan_words(&plan).into_iter().filter(|w| !d.knows(w)));
            }
            missing.sort();
            missing.dedup();
            assert!(missing.is_empty(), "{WORDS} has no {lang} for: {missing:?}");
        }
    }

    #[test]
    fn numbered_names_are_translated() {
        let mut words = BTreeMap::new();
        words.insert("Bedroom".to_string(), BTreeMap::from([("es".to_string(), "Dormitorio".to_string())]));
        let d = Dict { lang: "es".into(), words };
        assert_eq!(d.t("Bedroom 2"), "Dormitorio 2");
        assert_eq!(d.t("Kitchen"), "Kitchen");
        assert!(d.knows("Bedroom 3") && !d.knows("Kitchen"));
    }
}
