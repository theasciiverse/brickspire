//! Half-block rendering: every terminal cell is two roughly square pixels (`▀` with
//! the top pixel in the foreground colour and the bottom in the background). Each
//! pixel is shaded by asking the world what lies under its centre, so the picture
//! scales to any terminal; round things (ball, paddle) get analytic anti-aliasing.

use crate::game::{self, Game, Kind, Mode, LEDGE, BALL_R, BRICK_H, BRICK_W, FIELD_TOP, FIELD_X0, FIELD_X1, PADDLE_TH, WALL, WORLD_H};
use crate::boss::{self, Bell, Flight, Mason, MiniBoss, MiniKind, Rook, ShotKind, Storm, Weathercock};
use crate::levels::{self, RoofBoss, Theme};
use crate::map;
use crate::roof::{self, RoofView, Seen, Tilt};
use crate::game::Balcony;
use crate::physics::{self, V2};
use crate::power::{self, Power};
use crate::tower::{self, Faces};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

type Rgb = [f32; 3];

/// Where the world sits on screen: pixel = world * s + offset.
pub struct View {
    pub area: Rect,
    pub s: f32,
    pub ox: f32,
    pub oy: f32,
}

impl View {
    /// The box of world `size` round `centre` fitted to `area`, centred.
    pub fn new(area: Rect, (centre, size): (V2, V2)) -> View {
        let (w, h) = (area.width as f32, area.height as f32 * 2.0);
        let s = (w / size.x).min(h / size.y).max(0.01);
        View { area, s, ox: w / 2.0 - centre.x * s, oy: h / 2.0 - centre.y * s }
    }

    /// The world point under the middle of a terminal cell.
    pub fn cell_to_world(&self, col: u16, row: u16) -> V2 {
        let px = col as f32 - self.area.x as f32 + 0.5;
        let py = (row as f32 - self.area.y as f32) * 2.0 + 1.0;
        V2::new((px - self.ox) / self.s, (py - self.oy) / self.s)
    }
}

fn mix(a: Rgb, b: Rgb, k: f32) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
}

fn scale(a: Rgb, k: f32) -> Rgb {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn rgb(c: [u8; 3]) -> Rgb {
    [c[0] as f32, c[1] as f32, c[2] as f32]
}

fn hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h = h.wrapping_mul(0xcb1a_b31f);
    h ^ (h >> 16)
}

/// The tower's sky, darker at the top, with stars fixed to the screen, twinkling
/// (as many as its theme has).
fn sky(theme: &Theme, i: u32, j: u32, h_px: f32, time: f32) -> Rgb {
    let mut base = mix(rgb(theme.sky[0]), rgb(theme.sky[1]), j as f32 / h_px);
    if theme.rain {
        // storm clouds rolling by, and rain streaking down across them
        let (x, y) = (i as f32 / h_px, j as f32 / h_px);
        let cloud = (x * 3.1 + time * 0.03).sin() * (y * 7.0 + x * 2.0).cos() + (x * 7.3 - time * 0.05 + y * 3.0).sin() * 0.5;
        base = mix(base, [92.0, 98.0, 112.0], (cloud * 0.3).clamp(0.0, 0.25) * (1.0 - y));
        let streak = (i as f32 + j as f32 * 0.35 + time * 90.0).rem_euclid(37.0) < 1.0 && hash(i / 3, (j as f32 + time * 60.0) as u32 / 5) % 7 == 0;
        if streak {
            return mix(base, [170.0, 185.0, 210.0], 0.35);
        }
    }
    let h = hash(i, j);
    let starry = (j as f32 / h_px) < 0.25 + 0.75 * theme.stars;
    if h % 89 == 0 && starry && ((h >> 7) & 0xff) as f32 / 255.0 < theme.stars.max(0.5) {
        let tw = 0.55 + 0.45 * (time * (0.7 + (h >> 20) as f32 / 4096.0 * 2.0) + h as f32).sin();
        let k = (0.35 + ((h >> 8) & 0xff) as f32 / 255.0 * 0.65) * tw;
        return mix(base, [235.0, 235.0, 255.0], k);
    }
    base
}

/// The tower's own wall behind the bricks: faint masonry. `psx`/`psy` are a
/// pixel's size across and down, so mortar stays a pixel thick where the face is squashed.
fn masonry(theme: &Theme, p: V2, psx: f32, psy: f32) -> Rgb {
    let row = (p.y / 4.0).floor();
    let x = p.x + if row as i32 % 2 == 0 { 0.0 } else { 4.0 };
    let mortar = p.y.rem_euclid(4.0) < psy || x.rem_euclid(8.0) < psx;
    rgb(if mortar { theme.mortar } else { theme.wall })
}

/// Ivy on floor `n`'s wall at face point `p`: leaves in clumps, thickest hanging
/// from the lintel. `None` where the wall shows through.
pub fn ivy(n: i64, p: V2) -> Option<f32> {
    let seed = n as f32 * 3.7;
    let clumps = 0.5 + 0.5 * (p.x * 0.21 + (p.y * 0.13 + seed).sin() * 2.0 + seed).sin() * (p.y * 0.17 + p.x * 0.05 - seed).cos();
    let hanging = (1.0 - (p.y - FIELD_TOP) / 18.0).max(0.0) * 0.6;
    let (cx, cy) = ((p.x / 1.5).floor(), (p.y / 1.5).floor());
    let h = hash(cx as i32 as u32 ^ (n as u32).wrapping_mul(977), cy as i32 as u32);
    let k = (h & 0xff) as f32 / 255.0;
    (k < (clumps + hanging - 0.6) * 0.8).then_some(((h >> 8) & 0xff) as f32 / 255.0)
}

/// Steel pipe, lit across its width (`across` 0 to 1), with brass bands along it.
fn pipe(across: f32, along: f32, ps: f32) -> Rgb {
    if along.rem_euclid(9.0) < ps * 1.01 {
        return [210.0, 160.0, 60.0];
    }
    let lit = 1.0 - (across - 0.35).abs() * 1.3;
    mix([40.0, 50.0, 70.0], [170.0, 185.0, 210.0], lit)
}

/// A brick, bevelled: lit top and left edges, shaded bottom and right, a sheen
/// sweeping over metal ones, white while flashing from a hit.
fn brick_colour(b: &tower::BrickHere, at: V2, time: f32, psx: f32, psy: f32) -> Rgb {
    let mut base = rgb(game::brick_rgb(b.kind));
    if b.kind == Kind::Vine {
        // a stem along the middle, leaves either side
        let stem = (b.ly - BRICK_H / 2.0 - (b.lx * 1.4).sin() * 0.4).abs() < psy * 0.6;
        let leaf = ((b.lx * 1.9).sin() * (b.ly * 2.6 + b.lx).cos()) > 0.15;
        base = if stem { [70.0, 60.0, 30.0] } else if leaf { [150.0, 235.0, 110.0] } else { [70.0, 175.0, 70.0] };
    } else if b.kind == Kind::Clock {
        // a brass plate with a cog turning in the middle
        let l = V2::new(b.lx - BRICK_W / 2.0, b.ly - BRICK_H / 2.0);
        let a = l.y.atan2(l.x) + time * 1.5;
        let teeth = if (a * 4.0).sin() > 0.0 { 1.25 } else { 0.95 };
        let d = V2::new(l.x, l.y * 1.15).len();
        base = if d < 0.35 {
            [255.0, 225.0, 140.0]
        } else if d < teeth {
            [125.0, 86.0, 38.0]
        } else {
            [205.0, 155.0, 72.0]
        };
    } else if b.kind == Kind::Ice {
        // pale and glassy, a streak of light across it; cracked once hit
        let glint = ((b.lx - b.ly * 1.5).rem_euclid(5.0) - 1.2).abs() < psx;
        base = if glint { [235.0, 250.0, 255.0] } else { mix([150.0, 210.0, 235.0], [200.0, 235.0, 250.0], b.ly / BRICK_H) };
        if b.hp < 2 {
            let crack = ((b.lx - 2.2) * 0.9 - (b.ly - 1.5) * 1.4 + ((b.ly * 3.0).sin() * 0.4)).abs() < psx * 0.7
                || ((b.lx - 3.6) * 0.6 + (b.ly - 1.0) * 1.2).abs() < psx * 0.6 && b.ly < 1.6;
            if crack {
                base = [70.0, 110.0, 140.0];
            }
        }
    } else if b.kind == Kind::Web {
        // silk strands criss-crossing, the wall dim behind
        let strand = (b.lx + b.ly * 1.6).rem_euclid(2.0) < psx || (b.lx - b.ly * 1.6).rem_euclid(2.0) < psx;
        base = if strand { [225.0, 225.0, 235.0] } else { [70.0, 70.0, 82.0] };
    }
    if b.kind == Kind::Steel || b.kind == Kind::Gilt {
        let sheen = ((at.x + at.y * 2.0 - time * 30.0).rem_euclid(60.0) - 30.0).abs() < 2.0;
        if sheen {
            base = mix(base, [255.0, 255.0, 255.0], 0.45);
        }
    }
    let mut c = if b.lx < psx || b.ly < psy {
        mix(base, [255.0, 255.0, 255.0], 0.45)
    } else if b.lx >= BRICK_W - psx || b.ly >= BRICK_H - psy {
        scale(base, 0.45)
    } else {
        base
    };
    if b.flash > 0.0 {
        c = mix(c, [255.0, 255.0, 255.0], b.flash / 0.15);
    }
    c
}

/// A broken clockwork brick winding back in (`k` of the way): its outline, filling
/// in from the left as it winds.
fn ghost_colour(c: Rgb, k: f32, l: V2, psx: f32, psy: f32) -> Rgb {
    let brass = [205.0, 155.0, 72.0];
    let edge = l.x < psx || l.y < psy || l.x >= BRICK_W - psx || l.y >= BRICK_H - psy;
    if edge && ((l.x + l.y) / psx.max(0.3)) as i32 % 2 == 0 {
        mix(c, brass, 0.55)
    } else if l.x / BRICK_W < k {
        mix(c, brass, 0.22)
    } else {
        c
    }
}

/// Straight floors: whichever face of the tower is under `p`.
fn tower_face(game: &Game, faces: &Faces, cam: f32, p: V2, ps: f32, bg: Rgb) -> Rgb {
    let Some(s) = tower::sample(p.x, p.y, cam) else { return bg };
    let theme = &levels::tower(game.floor).theme;
    let (u, v) = (s.u, s.v);
    let (psx, psy) = (ps * s.squash, ps);
    let c = if v < FIELD_TOP {
        pipe((v - (FIELD_TOP - WALL)) / WALL, u, psx)
    } else if u < FIELD_X0 || u >= FIELD_X1 {
        let across = if u < FIELD_X0 { (u - (FIELD_X0 - WALL)) / WALL } else { (u - FIELD_X1) / WALL };
        pipe(across, v, psy)
    } else if v >= LEDGE {
        // the stone ledge the spiral climbs by, lit along its top edge
        if v < LEDGE + psy {
            [190.0, 180.0, 160.0]
        } else {
            mix([120.0, 110.0, 100.0], [50.0, 46.0, 52.0], (v - LEDGE) / (WORLD_H - LEDGE))
        }
    } else if let Some(b) = faces.brick(game, s.n, u, v) {
        brick_colour(&b, V2::new(u, v), game.time, psx, psy)
    } else if let Some((k, l)) = game.ghost_at(V2::new(u, v)).filter(|_| s.n == game.floor as i64) {
        ghost_colour(masonry(theme, V2::new(u, v), psx, psy), k, l, psx, psy)
    } else if let Some(leaf) = ivy(s.n, V2::new(u, v)).filter(|_| theme.ivy && !faces.plinth(s.n)) {
        mix([26.0, 62.0, 34.0], [80.0, 140.0, 56.0], leaf)
    } else {
        masonry(theme, V2::new(u, v), psx, psy)
    };
    // the plinth below the first floor is plain and darker
    let mut c = if faces.plinth(s.n) && v >= FIELD_TOP { scale(masonry(theme, V2::new(u, v), psx, psy), 0.7) } else { c };
    if faces.balcony(game, s.n) {
        // the balcony curves round in front of this face, under the paddle's arc
        let (centre, r) = game::balcony_arc();
        let d = (V2::new(u, v) - centre).len();
        c = match game::balcony_at(d, r) {
            Balcony::Open if (d - r).abs() < psy * 0.6 => mix(c, [70.0, 90.0, 120.0], 0.6),
            Balcony::Open => c,
            Balcony::Walkway(k) if k * 4.0 < psy => [200.0, 190.0, 170.0],
            Balcony::Walkway(k) => mix([150.0, 140.0, 125.0], [90.0, 84.0, 82.0], k),
            Balcony::Rail(k) => pipe(k, u, psx),
            Balcony::Shadow => scale(c, 0.4),
        };
    }
    scale(c, s.light)
}

/// The ground far below, seen from above the tower: dark fields, specks of trees
/// and the odd light.
fn ground(theme: &Theme, i: u32, j: u32) -> Rgb {
    // fields in patches, darker specks of trees, and a rare lit window
    let field = hash(i / 7, j / 4);
    let base = mix(rgb(theme.fields[0]), rgb(theme.fields[1]), (field & 0xff) as f32 / 255.0);
    let speck = hash(i, j);
    if speck % 1013 == 0 {
        [150.0, 120.0, 60.0]
    } else if speck % 11 == 0 {
        scale(base, 0.55)
    } else {
        base
    }
}

/// The roof's top at roof point `q`: flagstones laid in rings, the paddle's ring
/// marked, an iron plinth in the middle, bricks, the boss, and the parapet with
/// its merlons round the rim.
fn roof_top(game: &Game, roof: &RoofView, q: V2, ps: f32) -> Rgb {
    let off = q - roof.centre;
    let d = off.len();
    let ang = off.y.atan2(off.x);
    if d > roof::RIM - roof::PARAPET {
        // merlons round the rim, lit on their outer faces
        let merlon = roof::merlon_at(ang);
        let edge = (d - (roof::RIM - roof::PARAPET)) / roof::PARAPET;
        return if merlon { mix([190.0, 182.0, 166.0], [120.0, 114.0, 108.0], edge) } else { mix([70.0, 66.0, 64.0], [40.0, 38.0, 40.0], edge) };
    }
    let mut c = {
        let ring = (d / 4.0).floor();
        let around = ang * (ring * 4.0 + 2.0) / 7.0 + ring * 0.5;
        let mortar = d.rem_euclid(4.0) < ps || around.rem_euclid(1.0) * 7.0 < ps;
        if mortar {
            [26.0, 25.0, 32.0]
        } else {
            mix([60.0, 58.0, 68.0], [48.0, 46.0, 56.0], (hash(ring as u32, around.floor() as i32 as u32) & 0xff) as f32 / 255.0)
        }
    };
    if d < 5.0 {
        c = mix([58.0, 60.0, 70.0], [34.0, 35.0, 42.0], d / 5.0);
    }
    if (d - roof.ring).abs() < ps * 0.6 {
        c = mix(c, [70.0, 90.0, 120.0], 0.8);
    }
    if let Some(b) = roof.brick(game, q) {
        c = brick_colour(&b, q, game.time, ps, ps);
    } else if let Some((k, l)) = game.ghost_at(q).filter(|_| game.arena.mode == Mode::Circle) {
        c = ghost_colour(c, k, l, ps, ps);
    }
    // still climbing: the roof's boss waits up there
    let (waiting_vane, waiting_storm, waiting_rook, waiting_bell, waiting_mason);
    let (mut vane, mut storm, mut rook, mut bell) = (game.vane.as_ref(), game.storm.as_ref(), game.rook.as_ref(), game.bell.as_ref());
    let mut mason = game.mason.as_ref();
    match game.waiting_roof_boss() {
        Some(RoofBoss::Mason) => {
            waiting_mason = Mason::new(roof.centre);
            mason = Some(&waiting_mason);
        }
        Some(RoofBoss::Bell) => {
            waiting_bell = Bell::new(roof.centre);
            bell = Some(&waiting_bell);
        }
        Some(RoofBoss::Rook) => {
            waiting_rook = Rook::new(roof.centre);
            rook = Some(&waiting_rook);
        }
        Some(RoofBoss::Weathercock) => {
            waiting_vane = Weathercock::new(roof.centre);
            vane = Some(&waiting_vane);
        }
        Some(RoofBoss::Storm) => {
            waiting_storm = Storm::new(roof.centre);
            storm = Some(&waiting_storm);
        }
        None => {}
    }
    if let Some(w) = vane {
        if let Some(wc) = weathercock(w, q - w.centre) {
            c = wc;
        }
    }
    if let Some(s) = storm {
        c = storm_cloud(s, roof, q, ps, game.time, c);
    }
    if let Some(r) = rook {
        c = rook_nest(r, roof, q, ps, game.time, c);
    }
    if let Some(b) = bell {
        c = great_bell(b, roof, q, ps, game.time, c);
    }
    if let Some(m) = mason {
        c = the_mason(m, roof, q, ps, game.time, c);
    }
    c
}

/// A brick, `l` from its middle, `w` by `h`, turned by `spin`: red with pale mortar
/// round its edge.
fn small_brick(l: V2, w: f32, h: f32, spin: f32, ps: f32) -> Option<Rgb> {
    let l = l.rotate(-spin);
    if l.x.abs() > w / 2.0 || l.y.abs() > h / 2.0 {
        return None;
    }
    let edge = l.x.abs() > w / 2.0 - ps * 0.5 || l.y.abs() > h / 2.0 - ps * 0.5;
    Some(if edge { [215.0, 190.0, 160.0] } else { mix([210.0, 85.0, 60.0], [140.0, 50.0, 40.0], l.y / h + 0.5) })
}

/// The Mason from above, over roof point `q` on top of `c`: the bricks he lobs,
/// arcing up over the roof with their shadows below and the marks where they will
/// land; and the Mason, a broad-brimmed hat over blue shoulders, his trowel
/// flashing as he lays a brick.
fn the_mason(m: &Mason, roof: &RoofView, q: V2, ps: f32, time: f32, mut c: Rgb) -> Rgb {
    for lob in &m.lobs {
        c = ring_mark(roof, q, lob.a, boss::LOB_SPAN, lob.left, boss::LOB_TIME, time, [255.0, 140.0, 80.0], c);
    }
    let l = q - m.p;
    let (ax, d) = (l.x.abs(), l.len());
    let mut fig = None;
    if ax < 3.2 && l.y.abs() < 1.4 && (l.x / 3.2).powi(2) + (l.y / 1.4).powi(2) < 1.0 {
        fig = Some(mix([110.0, 140.0, 225.0], [50.0, 70.0, 140.0], l.y / 2.8 + 0.5));
    }
    if (l - V2::new(3.3, 0.9)).len() < 0.75 {
        fig = Some(if m.laid < 0.3 { [255.0, 255.0, 255.0] } else { [190.0, 195.0, 205.0] });
    }
    if d < 2.3 {
        // a straw hat: lit crown, red band, brim
        let lit = 0.5 - l.dot(V2::new(-0.6, -0.8)) / 2.3 * 0.5;
        fig = Some(if d < 1.3 {
            mix([240.0, 205.0, 135.0], [175.0, 135.0, 80.0], lit)
        } else if d < 1.6 {
            [150.0, 45.0, 40.0]
        } else {
            mix([215.0, 175.0, 105.0], [140.0, 105.0, 60.0], lit + (d - 1.6) / 1.4)
        });
    }
    if let Some(f) = fig {
        c = if m.hurt > 0.0 && (m.hurt * 12.0) as i32 % 2 == 0 { mix(f, [255.0, 255.0, 255.0], 0.75) } else { f };
    }
    for lob in &m.lobs {
        let (at, high) = lob.at(roof.centre, roof.ring);
        // its shadow on the roof, then the brick itself up in the air
        if (q - at).len() < 1.4 {
            c = scale(c, 0.6);
        }
        if let Some(b) = small_brick(q - at + V2::new(0.0, high * 3.5), 3.0 + high * 1.5, 1.5 + high * 0.75, time * 7.0, ps) {
            c = b;
        }
    }
    c
}

/// The Great Bell seen from above, over roof point `q` on top of `c`: its rings of
/// sound spreading (bright along the loud arcs, faint between), the marks on the
/// paddle's ring where they will land, the bronze bell with its rim, shoulder and
/// crown, the oak headstock across it, shivering before it tolls.
fn great_bell(b: &Bell, roof: &RoofView, q: V2, ps: f32, time: f32, mut c: Rgb) -> Rgb {
    let warn = [255.0, 105.0, 150.0];
    // where the loud arcs of a ring from `from` along `axis` will cross the paddle's
    // ring: a dashed line, so it never reads as the paddle
    let mark = |from: V2, axis: f32, urgent: f32, c: Rgb| {
        let off = q - roof.centre;
        let dash = (off.y.atan2(off.x) * roof.ring / 1.6).rem_euclid(2.0) < 1.0;
        if (off.len() - roof.ring).abs() < 0.9 && dash && boss::loud(from, axis, q) {
            let pulse = 0.5 + 0.5 * (time * (10.0 + 30.0 * urgent)).sin();
            mix(c, warn, 0.45 + 0.5 * pulse)
        } else {
            c
        }
    };
    if let Some(left) = b.tolling {
        c = mark(b.p, b.axis, 1.0 - left / boss::TOLL_WARN, c);
    }
    for w in &b.waves {
        if w.r < roof.ring + 1.0 {
            c = mark(w.c, w.axis, (w.r / roof.ring).min(1.0), c);
        }
        let d = ((q - w.c).len() - w.r).abs();
        if d < boss::WAVE_TH {
            let fade = (1.0 - w.r / (roof::RIM + 2.0)).clamp(0.5, 1.0);
            let k = (1.0 - d / boss::WAVE_TH) * fade;
            c = if w.loud_towards(q) { mix(c, [255.0, 190.0, 215.0], 0.9 * k) } else { mix(c, [200.0, 190.0, 200.0], 0.15 * k) };
        }
    }
    // shivering before a toll
    let shake = if b.tolling.is_some() { V2::new((b.t * 60.0).sin() * 0.3, (b.t * 47.0).cos() * 0.3) } else { V2::default() };
    let l = q - b.p - shake;
    let dir = b.dir();
    let side = V2::new(-dir.y, dir.x);
    let blink = b.hurt > 0.0 && (b.hurt * 12.0) as i32 % 2 == 0;
    // the headstock: a dark oak beam across the swing, iron-strapped, over the crown
    let (along, across) = (l.dot(side), l.dot(dir));
    if along.abs() < boss::BELL_R + 1.6 && across.abs() < 0.8 {
        let strap = (along.abs() - 1.6).abs() < 0.3;
        let wood = if strap { [60.0, 60.0, 70.0] } else { mix([110.0, 72.0, 40.0], [50.0, 32.0, 18.0], across.abs() / 0.8) };
        return if blink { mix(wood, [255.0, 255.0, 255.0], 0.75) } else { wood };
    }
    let d = l.len();
    if d < boss::BELL_R + ps {
        // tipped along its swing: lit on the side swinging up
        let tip = b.swing.sin() * 0.6;
        let k = d / boss::BELL_R;
        let mut bronze = if k > 0.84 {
            // the lip, bright
            mix([255.0, 215.0, 120.0], [190.0, 135.0, 55.0], (k - 0.84) / 0.16)
        } else if k > 0.5 {
            mix([175.0, 120.0, 50.0], [225.0, 165.0, 75.0], (k - 0.5) / 0.34)
        } else if k > 0.28 {
            [150.0, 100.0, 42.0]
        } else {
            [85.0, 58.0, 30.0]
        };
        bronze = scale(bronze, 1.0 + 0.25 * (l.dot(dir) / boss::BELL_R) * tip);
        if (k - 0.5).abs() < ps * 0.6 / boss::BELL_R {
            bronze = scale(bronze, 0.6);
        }
        if blink {
            bronze = mix(bronze, [255.0, 255.0, 255.0], 0.75);
        }
        c = mix(c, bronze, (boss::BELL_R - d) / ps + 0.5);
    }
    c
}

/// A warning on the ring: the stretch `span` either side of angle `a`, pulsing in
/// `rgb`, faster as `left` of `warn` seconds runs out.
fn ring_mark(roof: &RoofView, q: V2, a: f32, span: f32, left: f32, warn: f32, time: f32, rgb: Rgb, c: Rgb) -> Rgb {
    let off = q - roof.centre;
    let gap = (off.y.atan2(off.x) - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    if gap.abs() < span && (off.len() - roof.ring).abs() < 1.8 {
        let pulse = 0.5 + 0.5 * (time * (10.0 + 30.0 * (1.0 - left / warn))).sin();
        mix(c, rgb, 0.35 + 0.5 * pulse)
    } else {
        c
    }
}

/// The Rook's nest at roof point `q`, over `c`: a woven straw floor ringed by twigs
/// (gaps where the ball broke them), and the crow. While it crouches, a dashed line
/// and a mark on the ring show where it is about to swoop. It spreads its wings in
/// flight and blinks white after a hit.
fn rook_nest(r: &Rook, roof: &RoofView, q: V2, ps: f32, time: f32, mut c: Rgb) -> Rgb {
    let off = q - r.nest;
    if off.len() < boss::NEST_R - 0.4 {
        let weave = (off.x * 1.7 + off.y * 0.9).sin() * (off.y * 1.9 - off.x * 0.6).sin();
        c = mix([96.0, 74.0, 42.0], [140.0, 112.0, 64.0], weave * 0.5 + 0.5);
    }
    for k in (0..boss::TWIGS).filter(|&k| r.twigs[k]) {
        let (a, b) = r.twig(k);
        let near = physics::closest_on_segment(q, a, b);
        let d = (q - near).len();
        if d < boss::TWIG_TH + ps * 0.5 {
            let along = (near - a).len();
            let bark = (along * 2.3 + k as f32).sin() > 0.6;
            let wood = mix([150.0, 105.0, 58.0], [70.0, 46.0, 26.0], d / boss::TWIG_TH);
            c = if bark { scale(wood, 0.7) } else { wood };
        }
    }
    if let Flight::Crouching { a, left } = r.flight {
        let end = r.nest + V2::new(a.cos(), a.sin()) * roof.ring;
        let near = physics::closest_on_segment(q, r.p, end);
        if (q - near).len() < 0.5 && ((near - r.p).len() / 1.5) as i32 % 2 == 0 && (near - r.p).len() > boss::CROW_R + 1.0 {
            c = mix(c, [255.0, 90.0, 60.0], 0.7);
        }
        c = ring_mark(roof, q, a, (boss::CROW_R + 1.5) / roof.ring, left, boss::SWOOP_WARN, time, [255.0, 90.0, 60.0], c);
    }
    // the crow, in its own frame: along its facing and across
    let l = q - r.p;
    let dir = V2::new(r.facing.cos(), r.facing.sin());
    let side = V2::new(-dir.y, dir.x);
    let (along, across) = (l.dot(dir), l.dot(side));
    let flying = !matches!(r.flight, Flight::Nesting);
    let mut crow = None;
    if flying {
        let beat = if matches!(r.flight, Flight::Swooping { .. }) { (time * 14.0).sin() * 0.6 } else { 0.0 };
        let reach = boss::CROW_R + 4.0 + beat;
        if across.abs() < reach && (along + across.abs() * 0.35).abs() < 1.3 {
            crow = Some(mix([60.0, 60.0, 80.0], [16.0, 16.0, 24.0], across.abs() / reach));
        }
    }
    let d = l.len();
    if d < boss::CROW_R {
        let eye = (l - dir * 1.0 - side * 0.9).len() < 0.4 || (l - dir * 1.0 + side * 0.9).len() < 0.4;
        crow = Some(if eye { [235.0, 220.0, 120.0] } else { mix([70.0, 70.0, 95.0], [18.0, 18.0, 26.0], (d / boss::CROW_R + l.dot(V2::new(0.6, 0.8)) / boss::CROW_R) * 0.5 + 0.3) });
    } else if along > 0.0 && along < boss::CROW_R + 1.6 && across.abs() < (boss::CROW_R + 1.6 - along) * 0.45 {
        crow = Some(mix([210.0, 190.0, 110.0], [120.0, 100.0, 50.0], across.abs()));
    }
    if let Some(k) = crow {
        c = if r.hurt > 0.0 && (r.hurt * 12.0) as i32 % 2 == 0 { mix(k, [255.0, 255.0, 255.0], 0.75) } else { k };
    }
    c
}

/// The Storm Cloud over roof point `q`, on top of `c`: the mark it puts on the ring
/// before a bolt, then the bolt itself; churning grey puffs you can see through; and
/// its glowing core, which blinks white after a hit.
fn storm_cloud(s: &Storm, roof: &RoofView, q: V2, ps: f32, time: f32, mut c: Rgb) -> Rgb {
    if let Some((a, left)) = s.charging {
        c = ring_mark(roof, q, a, boss::BOLT_SPAN, left, boss::BOLT_WARN, time, [255.0, 225.0, 90.0], c);
    }
    // the cloud: the most each puff covers here, lit from the top left
    let mut cover = 0.0f32;
    let mut lit = 0.0f32;
    for (p, r) in s.puffs() {
        let l = q - p;
        let k = ((1.0 - l.len() / r) * 3.0).clamp(0.0, 1.0);
        if k > cover {
            cover = k;
            lit = (0.5 - l.dot(V2::new(-0.6, -0.8)) / r * 0.5).clamp(0.0, 1.0);
        }
    }
    if cover > 0.0 {
        let mut cloud = mix([165.0, 168.0, 186.0], [62.0, 64.0, 82.0], lit);
        if s.charging.is_some() && (time * 9.0).sin() > 0.6 {
            cloud = mix(cloud, [255.0, 240.0, 170.0], 0.3);
        }
        c = mix(c, cloud, cover * 0.62);
    }
    let d = (q - s.centre).len();
    if d < boss::CORE_R + ps {
        let glow = 0.85 + 0.15 * (time * 4.0).sin();
        let mut core = scale(mix([255.0, 252.0, 215.0], [240.0, 190.0, 50.0], d / boss::CORE_R), glow);
        if s.hurt > 0.0 && (s.hurt * 12.0) as i32 % 2 == 0 {
            core = [255.0, 255.0, 255.0];
        }
        c = mix(c, core, (boss::CORE_R - d) / ps + 0.5);
    }
    if let Some(bolt) = s.bolt(roof.centre, roof.ring) {
        let near = bolt.windows(2).map(|w| (q - physics::closest_on_segment(q, w[0], w[1])).len()).fold(f32::MAX, f32::min);
        if near < 0.5 + ps {
            c = [255.0, 255.0, 235.0];
        } else if near < 2.0 {
            c = mix(c, [200.0, 210.0, 255.0], 0.5 * (1.0 - near / 2.0));
        }
    }
    c
}

/// The Weathercock, in its own units (`l` from its hub): an iron arrow with a brass
/// head and a fletched tail, and a verdigris rooster on the hub with a red comb. It
/// shudders before a gust and blinks white after a hit.
fn weathercock(w: &Weathercock, l: V2) -> Option<Rgb> {
    let l = if w.warning() { l + V2::new((w.t * 70.0).sin() * 0.35, 0.0) } else { l };
    let dir = w.dir();
    let side = V2::new(-dir.y, dir.x);
    let (along, across) = (l.dot(dir), l.dot(side));
    let mut c = None;
    if along.abs() < boss::ARROW_HALF + 1.6 && across.abs() < 3.0 {
        let head = boss::ARROW_HALF - 2.5;
        if along > head && across.abs() < (boss::ARROW_HALF + 1.6 - along) * 0.75 {
            c = Some(mix([255.0, 215.0, 110.0], [170.0, 120.0, 40.0], across.abs() / 2.0));
        } else if along < -boss::ARROW_HALF + 3.5 && along > -boss::ARROW_HALF - 1.0 && across.abs() < 2.4 && (along * 1.6).rem_euclid(1.0) < 0.6 {
            c = Some([110.0, 112.0, 126.0]);
        } else if across.abs() < boss::ARROW_TH && along.abs() < boss::ARROW_HALF {
            c = Some(mix([150.0, 152.0, 165.0], [50.0, 52.0, 62.0], (across / boss::ARROW_TH + 1.0) / 2.0));
        }
    }
    let d = l.len();
    if d < boss::ROOSTER_R {
        let comb = (l - dir * 1.2 - side * 1.3).len() < 1.1;
        let eye = (l - dir * 1.6 + side * 0.2).len() < 0.45;
        c = Some(if eye {
            [20.0, 20.0, 24.0]
        } else if comb {
            [225.0, 50.0, 40.0]
        } else {
            mix([110.0, 200.0, 165.0], [40.0, 110.0, 90.0], d / boss::ROOSTER_R)
        });
    }
    if w.hurt > 0.0 && (w.hurt * 12.0) as i32 % 2 == 0 {
        c = c.map(|c| mix(c, [255.0, 255.0, 255.0], 0.75));
    }
    c
}

/// The Spider, in its own units (`l` from its middle, y down): a round abdomen with
/// a red hourglass, a smaller head below with eyes (glowing before it spits), and
/// eight jointed legs that creep as it moves.
fn spider(g: &MiniBoss, l: V2) -> Option<Rgb> {
    let seg = |a: V2, b: V2, r: f32| (l - physics::closest_on_segment(l, a, b)).len() < r;
    let abdomen = l - V2::new(0.0, -0.7);
    let head = l - V2::new(0.0, 1.5);
    let ax = l.x.abs();
    let eye = (V2::new(ax, head.y) - V2::new(0.45, 0.1)).len() < 0.32;
    let mut c = if eye {
        Some(if g.aiming() { [255.0, 220.0, 120.0] } else { [235.0, 50.0, 50.0] })
    } else if (abdomen.x / 2.1).powi(2) + (abdomen.y / 1.8).powi(2) < 1.0 {
        let hourglass = abdomen.x.abs() < 0.15 + abdomen.y.abs() * 0.45 && abdomen.y.abs() < 0.9;
        Some(if hourglass { [235.0, 50.0, 40.0] } else { mix([170.0, 110.0, 195.0], [70.0, 40.0, 95.0], (abdomen.y + abdomen.x * 0.3 + 1.8) / 3.6) })
    } else if head.len() < 1.0 {
        Some([120.0, 80.0, 140.0])
    } else {
        None
    };
    if c.is_none() {
        let creep = g.t * 6.0;
        for side in [-1.0f32, 1.0] {
            for k in 0..4 {
                let kf = k as f32;
                let wiggle = (creep + kf * 1.6 + side).sin() * 0.35;
                let hip = V2::new(side * 1.2, -1.4 + kf * 0.75);
                let knee = V2::new(side * 3.1, -2.6 + kf * 1.2 + wiggle);
                let foot = V2::new(side * 4.3, -0.9 + kf * 1.3 + wiggle * 0.5);
                if seg(hip, knee, 0.35) || seg(knee, foot, 0.3) {
                    c = Some([120.0, 95.0, 135.0]);
                }
            }
        }
    }
    if g.hurt > 0.0 && (g.hurt * 12.0) as i32 % 2 == 0 {
        c = c.map(|c| mix(c, [255.0, 255.0, 255.0], 0.75));
    }
    c
}

/// The Mason's apprentice on his scaffold, in his own units (`l` from the middle, y
/// down): the plank, a hod of bricks at one end, the lad in a blue cap and apron, a
/// brick raised to throw while he aims.
fn apprentice(g: &MiniBoss, l: V2, ps: f32) -> Option<Rgb> {
    let seg = |a: V2, b: V2, r: f32| (l - physics::closest_on_segment(l, a, b)).len() < r;
    let c = if l.y > 2.3 && l.y < 3.3 && l.x.abs() < 6.0 {
        let grain = ((l.x * 1.7).sin() * 0.5 + 0.5) * 0.15;
        Some(mix([170.0, 120.0, 70.0], [100.0, 68.0, 40.0], (l.y - 2.3) + grain))
    } else if l.x > 2.0 && l.x < 4.8 && l.y > 0.7 && l.y <= 2.3 {
        // the hod: two courses of bricks
        let mortar = (l.y - 0.7).rem_euclid(0.8) < ps || (l.x - 2.0 + if l.y < 1.5 { 0.7 } else { 0.0 }).rem_euclid(1.4) < ps;
        Some(if mortar { [200.0, 180.0, 150.0] } else { [190.0, 75.0, 55.0] })
    } else if (l - V2::new(-1.5, -2.3)).len() < 0.85 {
        let eye = (l - V2::new(-1.2, -2.3)).len() < 0.2;
        Some(if eye { [40.0, 30.0, 30.0] } else { [235.0, 185.0, 145.0] })
    } else if l.y < -2.5 && l.y > -3.4 && (l.x + 1.5).abs() < 1.0 + if l.y > -2.8 { 0.4 } else { 0.0 } {
        Some([70.0, 100.0, 175.0])
    } else if (l.x + 1.5).abs() < 0.95 && l.y > -1.5 && l.y < 0.7 {
        Some(if (l.x + 1.5).abs() < 0.6 && l.y > -0.9 { [215.0, 205.0, 175.0] } else { [70.0, 100.0, 175.0] })
    } else if seg(V2::new(-2.0, 0.7), V2::new(-2.1, 2.3), 0.3) || seg(V2::new(-1.0, 0.7), V2::new(-0.9, 2.3), 0.3) {
        Some([60.0, 60.0, 90.0])
    } else if g.aiming() && (seg(V2::new(-0.7, -1.1), V2::new(0.2, -2.6), 0.28) || (l - V2::new(0.4, -3.1)).len() < 0.6) {
        Some(if (l - V2::new(0.4, -3.1)).len() < 0.6 { [190.0, 75.0, 55.0] } else { [235.0, 185.0, 145.0] })
    } else if !g.aiming() && seg(V2::new(-0.7, -1.1), V2::new(0.3, 0.3), 0.28) {
        Some([235.0, 185.0, 145.0])
    } else {
        None
    };
    if g.hurt > 0.0 && (g.hurt * 12.0) as i32 % 2 == 0 {
        return c.map(|c| mix(c, [255.0, 255.0, 255.0], 0.75));
    }
    c
}

/// The sea serpent's head, in its own units (`l` from its middle, y down): a long
/// green-teal head facing down, a paler snout and jaw (open, with fangs, while it
/// aims), yellow eyes and red fins swept back either side.
fn serpent(g: &MiniBoss, l: V2) -> Option<Rgb> {
    let ax = l.x.abs();
    let body = (l.x / 3.0).powi(2) + ((l.y + 0.2) / 2.4).powi(2) < 1.0;
    let snout = (l.x / 1.7).powi(2) + ((l.y - 1.6) / 1.1).powi(2) < 1.0;
    let fin = ax > 2.0 && ax < 4.0 && l.y > -2.6 + (ax - 2.0) * 0.2 && l.y < -0.6 - (ax - 2.0) * 0.6 && ((ax * 3.0).sin() > -0.4);
    let eye = (V2::new(ax, l.y) - V2::new(1.3, -0.1)).len() < 0.42;
    let open = g.aiming();
    let c = if eye {
        Some(if open { [255.0, 240.0, 140.0] } else { [235.0, 200.0, 60.0] })
    } else if open && snout && l.y > 1.4 && ax < 1.0 {
        // the open jaw: dark inside, two white fangs
        Some(if (ax - 0.55).abs() < 0.2 && l.y < 2.1 { [240.0, 240.0, 230.0] } else { [70.0, 20.0, 30.0] })
    } else if snout {
        Some(mix([150.0, 200.0, 170.0], [80.0, 140.0, 120.0], (l.y - 1.0) / 1.5))
    } else if body {
        let scale_ = ((l.x * 2.2).sin() * (l.y * 2.2).sin()).abs() < 0.15;
        let base = mix([60.0, 150.0, 140.0], [20.0, 70.0, 70.0], (l.y + 2.6) / 5.0 + ax * 0.08);
        Some(if scale_ { scale(base, 0.75) } else { base })
    } else if fin {
        Some(mix([220.0, 80.0, 70.0], [140.0, 40.0, 50.0], (ax - 2.0) / 2.0))
    } else {
        None
    };
    if g.hurt > 0.0 && (g.hurt * 12.0) as i32 % 2 == 0 {
        return c.map(|c| mix(c, [255.0, 255.0, 255.0], 0.75));
    }
    c
}

/// The Jack, in its own units (`l` from its middle, y down): an iron figure in a
/// helmet, marching, its hammer raised over its shoulder and brought down on the
/// little gold bell at its side when it strikes (raised higher, eyes lit, just before).
fn jack(g: &MiniBoss, l: V2) -> Option<Rgb> {
    let seg = |a: V2, b: V2, r: f32| (l - physics::closest_on_segment(l, a, b)).len() < r;
    let iron = |y: f32| mix([150.0, 155.0, 175.0], [60.0, 62.0, 78.0], (y + 3.5) / 7.0);
    let brass = [230.0, 175.0, 70.0];
    // the hammer: raised (wound back further while it aims), down on the bell as it strikes
    let fall = (g.swung / 0.35).min(1.0);
    let raised = if g.aiming() && g.swung > 0.35 { -2.3 } else { -1.8 };
    let ang = 0.35 + (raised - 0.35) * (fall * fall * (3.0 - 2.0 * fall));
    let shoulder = V2::new(1.0, -0.9);
    let reach = V2::new(ang.cos(), ang.sin());
    let head = shoulder + reach * 2.4;
    let eye = (V2::new(l.x.abs(), l.y) - V2::new(0.35, -2.2)).len() < 0.28;
    let march = (g.t * 6.0).sin() * 0.4;
    let c = if eye {
        Some(if g.aiming() { [255.0, 220.0, 120.0] } else { [255.0, 90.0, 50.0] })
    } else if (l - head).len() < 0.75 || seg(head - V2::new(-reach.y, reach.x) * 0.8, head + V2::new(-reach.y, reach.x) * 0.8, 0.45) {
        Some(mix([175.0, 178.0, 195.0], [70.0, 72.0, 88.0], (l - head).dot(V2::new(0.6, 0.8)) + 0.5))
    } else if seg(shoulder, head, 0.26) {
        Some([165.0, 110.0, 55.0])
    } else if (l - V2::new(3.0, 0.4)).len() < 0.95 && l.y < 1.2 || seg(V2::new(2.1, 1.1), V2::new(3.9, 1.1), 0.25) {
        Some(if g.swung < 0.15 { [255.0, 250.0, 200.0] } else { brass })
    } else if (l - V2::new(0.0, -2.2)).len() < 0.95 || seg(V2::new(-0.9, -2.5), V2::new(0.9, -2.5), 0.3) {
        Some(iron(l.y - 1.0))
    } else if l.x.abs() < 1.1 && l.y > -1.3 && l.y < 1.4 {
        Some(if (l.y - 0.6).abs() < 0.25 { brass } else { iron(l.y) })
    } else if seg(V2::new(-0.5, 1.3), V2::new(-0.6 + march, 3.3), 0.3) || seg(V2::new(0.5, 1.3), V2::new(0.6 - march, 3.3), 0.3) {
        Some(iron(l.y))
    } else if seg(V2::new(-1.0, -0.9), V2::new(-1.6, 0.8), 0.28) {
        Some(iron(l.y))
    } else {
        None
    };
    if g.hurt > 0.0 && (g.hurt * 12.0) as i32 % 2 == 0 {
        return c.map(|c| mix(c, [255.0, 255.0, 255.0], 0.75));
    }
    c
}

/// The Gargoyle, in its own units (`l` from its centre, y down): stone head with
/// horns and red eyes (glowing hot just before it spits), a squat body with claws,
/// and scalloped wings beating up and down.
fn gargoyle(g: &MiniBoss, l: V2) -> Option<Rgb> {
    let stone = |y: f32| mix([175.0, 170.0, 185.0], [80.0, 76.0, 92.0], (y + 3.0) / 6.0);
    let seg = |a: V2, b: V2, r: f32| (l - physics::closest_on_segment(l, a, b)).len() < r;
    let ax = l.x.abs();
    let eye = (V2::new(ax, l.y) - V2::new(0.6, -1.3)).len() < 0.45;
    let mut c = if eye {
        Some(if g.aiming() { [255.0, 220.0, 120.0] } else { [255.0, 60.0, 40.0] })
    } else if (l - V2::new(0.0, -1.2)).len() < 1.5 || seg(V2::new(0.8, -2.2), V2::new(1.8, -3.3), 0.4) && l.x > 0.0 || seg(V2::new(-0.8, -2.2), V2::new(-1.8, -3.3), 0.4) && l.x < 0.0 {
        Some(stone(l.y - 1.0))
    } else if ((l.x / 2.4).powi(2) + ((l.y - 0.9) / 2.0).powi(2)) < 1.0 {
        Some(stone(l.y))
    } else if seg(V2::new(1.2, 2.4), V2::new(1.5, 3.1), 0.35) || seg(V2::new(-1.2, 2.4), V2::new(-1.5, 3.1), 0.35) {
        Some([60.0, 56.0, 66.0])
    } else if (1.6..7.0).contains(&ax) {
        // a wing: from the shoulder out to a tip that rises and falls with the beat
        let k = (ax - 1.6) / 5.4;
        let tip = -1.8 + g.flap() * 1.6;
        let top = -1.2 + (tip - -1.2) * k;
        let scallop = 0.7 * (1.0 - ((ax - 1.6) * 2.2).cos().abs());
        let bottom = 1.2 + (tip + 0.8 - 1.2) * k - scallop;
        if l.y > top && l.y < bottom {
            let rib = ((ax - 1.6) * 2.2 / std::f32::consts::PI).fract() < 0.12;
            Some(if rib { [60.0, 56.0, 72.0] } else { mix([110.0, 104.0, 125.0], [70.0, 66.0, 84.0], (l.y - top) / (bottom - top).max(0.1)) })
        } else {
            None
        }
    } else {
        None
    };
    if g.hurt > 0.0 {
        // blinks white while hits don't count
        let blink = if (g.hurt * 12.0) as i32 % 2 == 0 { 0.75 } else { 0.0 };
        c = c.map(|c| mix(c, [255.0, 255.0, 255.0], blink));
    }
    c
}

/// What lies under world point `p` (pixel size `ps` in world units).
/// Everything a frame needs to know once, rather than per pixel.
struct Scene<'a> {
    faces: Faces,
    cam: f32,
    balls: Vec<V2>,
    tilt: Option<(Tilt, f32)>,
    roof: Option<RoofView>,
    game: &'a Game,
}

/// The scenery under world point `p`: the tower's side, or, near the roof, whatever
/// the tilting camera sees there.
fn scenery(sc: &Scene, p: V2, ps: f32, sky: Rgb, ground: Rgb) -> Rgb {
    let game = sc.game;
    let Some((t, ground_y)) = &sc.tilt else { return tower_face(game, &sc.faces, sc.cam, p, ps, sky) };
    // sky gives way to the ground far below as the camera turns to look down
    let bg = mix(sky, ground, (t.e - 0.3) / 0.6);
    match (t.seen(p, *ground_y), &sc.roof) {
        (Seen::Roof(q), Some(roof)) => roof_top(game, roof, q, ps),
        (Seen::Parapet(k), _) => mix([150.0, 142.0, 132.0], [70.0, 66.0, 64.0], k),
        (Seen::Side(sp), _) => tower_face(game, &sc.faces, sc.cam, sp, ps, bg),
        _ => bg,
    }
}

fn shade(sc: &Scene, p: V2, ps: f32, sky: Rgb, ground: Rgb) -> Rgb {
    let game = sc.game;
    let mut c = scenery(sc, p, ps, sky, ground);
    if let Some(g) = &game.boss {
        let l = p - g.p;
        let half = g.half();
        if g.kind == MiniKind::Spider && l.x.abs() < ps * 0.5 + 0.1 && p.y > FIELD_TOP && l.y < -1.5 {
            // its thread, up to the lintel
            c = mix(c, [200.0, 200.0, 215.0], 0.6);
        }
        if g.kind == MiniKind::Apprentice && p.y > FIELD_TOP && l.y < 2.3 * boss::APPRENTICE_SCALE && ((l.x.abs() - 5.5 * boss::APPRENTICE_SCALE).abs() < ps * 0.5 + 0.12) {
            // the scaffold's ropes, up to the lintel
            c = mix(c, [190.0, 170.0, 125.0], 0.85);
        }
        if g.kind == MiniKind::Serpent && p.y > FIELD_TOP {
            // its neck, up over the lintel: scaled, a pale belly down one side
            let neck = g.neck();
            let (mut best, mut at) = (f32::MAX, 0.0);
            for (k, w) in neck.windows(2).enumerate() {
                let q = physics::closest_on_segment(p, w[0], w[1]);
                let d = (p - q).len();
                if d < best {
                    best = d;
                    at = k as f32 + (q - w[0]).len() / (w[1] - w[0]).len().max(1e-3);
                }
            }
            let r = 1.6 - at * 0.04;
            if best < r + ps * 0.5 {
                let scales = ((at * 4.0).fract() - 0.5).abs() < 0.12;
                let base = mix([40.0, 120.0, 112.0], [18.0, 60.0, 62.0], best / r);
                c = if scales { scale(base, 0.7) } else { base };
            }
        }
        if l.x.abs() < half.x + 0.5 && l.y.abs() < half.y + 1.0 {
            let drawn = match g.kind {
                MiniKind::Gargoyle => gargoyle(g, l),
                MiniKind::Spider => spider(g, l),
                MiniKind::Jack => jack(g, l * (1.0 / boss::JACK_SCALE)),
                MiniKind::Serpent => serpent(g, l * (1.0 / boss::SERPENT_SCALE)),
                MiniKind::Apprentice => apprentice(g, l * (1.0 / boss::APPRENTICE_SCALE), ps),
            };
            if let Some(gc) = drawn {
                c = gc;
            }
        }
    }
    for shot in &game.shots {
        let d = (p - shot.p).len();
        let reach = if shot.kind == ShotKind::Brick { boss::SHOT_R * 1.4 } else { boss::SHOT_R };
        if d < reach + ps {
            let blob = match shot.kind {
                // a tangle of silk: strands across a pale glob
                ShotKind::Web => {
                    let l = p - shot.p;
                    let strand = (l.x * 3.0).sin().abs() < 0.3 || (l.y * 3.0).sin().abs() < 0.3;
                    if strand { [240.0, 240.0, 250.0] } else { [150.0, 150.0, 165.0] }
                }
                // a ringing chime: a bright ring with a hollow middle
                ShotKind::Chime => {
                    if d > boss::SHOT_R * 0.45 { mix([255.0, 250.0, 210.0], [240.0, 180.0, 60.0], d / boss::SHOT_R) } else { mix(c, [255.0, 230.0, 140.0], 0.3) }
                }
                // a thrown brick, tumbling
                ShotKind::Brick => match small_brick(p - shot.p, 2.4, 1.2, game.time * 9.0, ps) {
                    Some(b) => b,
                    None => continue,
                },
                // freezing spray: a pale ball of droplets
                ShotKind::Spray => {
                    let l = p - shot.p;
                    if ((l.x * 5.0).sin() * (l.y * 5.0).sin()).abs() > 0.5 { [240.0, 252.0, 255.0] } else { [130.0, 200.0, 235.0] }
                }
                ShotKind::Fire => mix([255.0, 240.0, 150.0], [230.0, 60.0, 20.0], d / boss::SHOT_R),
            };
            c = mix(c, blob, (boss::SHOT_R - d) / ps + 0.5);
        }
    }
    // the paddle: a brass bar with teal ends, lit from above along its normal
    let (a, b, n) = game.paddle_segment();
    let q = physics::closest_on_segment(p, a, b);
    let off = p - q;
    let d = off.len();
    let climbing = sc.tilt.is_some() && game.arena.mode != Mode::Circle;
    if d < PADDLE_TH + ps && !climbing {
        let along = (q - (a + b) * 0.5).dot((b - a).norm()).abs();
        let across = off.dot(n) / PADDLE_TH; // +1 top face, -1 underside
        let tip = if game.power == Some(Power::Chisel) { [235.0, 70.0, 60.0] } else { [40.0, 200.0, 185.0] };
        let mut body = if along > game.paddle_half() - 2.0 { tip } else { [235.0, 170.0, 55.0] };
        if game.power == Some(Power::Mortar) && across > 0.2 {
            // a layer of mortar along its top
            body = [200.0, 195.0, 185.0];
        }
        if game.webbed > 0.0 && ((q - a).len() * 1.3 + across * 2.0).sin() > 0.3 {
            // gummed up: silk wound round it
            body = [225.0, 225.0, 235.0];
        }
        if game.iced > 0.0 && ((q - a).len() * 0.8 - across).rem_euclid(1.6) < 0.5 {
            // iced over: frosted, glinting
            body = [200.0, 235.0, 250.0];
        }
        if game.stun > 0.0 {
            // stunned: greyed, flickering
            let flick = if (game.time * 20.0) as i32 % 2 == 0 { 0.35 } else { 0.6 };
            body = scale([170.0, 170.0, 180.0], flick + 0.3);
        }
        let lit = mix(scale(body, 0.5), mix(body, [255.0, 250.0, 220.0], 0.5), (across + 1.0) / 2.0);
        c = mix(c, lit, (PADDLE_TH - d) / ps + 0.5);
    }
    for cap in &game.capsules {
        // a pill in its colour, lit along the top, banded at its ends
        let l = p - cap.p;
        let h = power::CAP_HALF;
        let core = (h.x - h.y).max(0.0);
        let d = V2::new((l.x.abs() - core).max(0.0), l.y).len();
        if d < h.y + ps * 0.5 {
            let base = rgb(cap.kind.rgb());
            let lit = mix(mix(base, [255.0, 255.0, 255.0], 0.45), scale(base, 0.55), l.y / h.y * 0.5 + 0.5);
            let band = (l.x.abs() - core).abs() < ps * 0.6;
            c = mix(c, if band { scale(base, 0.5) } else { lit }, (h.y - d) / ps + 0.5);
        }
    }
    for bolt in &game.bolts {
        let tail = bolt.p - bolt.v.norm() * 1.8;
        if (p - physics::closest_on_segment(p, tail, bolt.p)).len() < 0.35 + ps * 0.5 {
            c = mix([255.0, 230.0, 160.0], [235.0, 90.0, 60.0], (p - bolt.p).len() / 1.8);
        }
    }
    for &bp in &sc.balls {
        let d = (p - bp).len();
        if d < BALL_R + ps {
            let hi = (p - bp + V2::new(0.4, 0.4)).len() / BALL_R;
            let ball = mix([255.0, 255.0, 245.0], [150.0, 160.0, 190.0], hi - 0.2);
            c = mix(c, ball, (BALL_R - d) / ps + 0.5);
        }
    }
    c
}

/// The colour of a map point (see `map::look`), faded in over its first moments.
pub fn map_colour(p: &map::Progress, look: map::Look, i: u32, j: u32) -> Rgb {
    use map::{Look, Part};
    let c = match look {
        Look::Sky(k) => {
            let base = mix([10.0, 10.0, 34.0], [125.0, 72.0, 82.0], k * k);
            let h = hash(i, j);
            if k < 0.55 && h % 97 == 0 { mix(base, [235.0, 235.0, 255.0], 0.6 * (1.0 - k)) } else { base }
        }
        Look::Mountain => [44.0, 38.0, 70.0],
        Look::Hill(d) => {
            let base = mix([30.0, 52.0, 34.0], [10.0, 22.0, 16.0], d / 0.2);
            if hash(i, j) % 13 == 0 { scale(base, 0.6) } else { base }
        }
        Look::Tower { k, part, lit, known, next } => {
            if !known {
                [24.0, 21.0, 40.0]
            } else {
                let stone = rgb(map::SKYLINE[k].stone);
                let glow = if next { 0.85 + 0.15 * (p.time * 3.0).sin() } else { 1.0 };
                let c = match part {
                    Part::Wall => stone,
                    Part::Ledge => mix(stone, [255.0, 255.0, 255.0], 0.25),
                    Part::Battlement => scale(stone, 0.9),
                    Part::Window if p.climbed(k) => [255.0, 200.0, 110.0],
                    Part::Window => [30.0, 28.0, 44.0],
                    Part::Leaves => [70.0, 140.0, 60.0],
                    Part::Stripe => [200.0, 55.0, 48.0],
                    Part::Lamp => mix([255.0, 230.0, 140.0], [255.0, 255.0, 230.0], 0.5 + 0.5 * (p.time * 4.0).sin()),
                    Part::Bell => [235.0, 185.0, 60.0],
                    Part::Roof => match map::SKYLINE[k].shape {
                        map::Shape::Bell => [170.0, 80.0, 60.0],
                        map::Shape::Lighthouse => [140.0, 40.0, 40.0],
                        _ => [95.0, 92.0, 135.0],
                    },
                };
                let lamp = matches!(part, Part::Lamp | Part::Window);
                scale(c, if lamp { glow } else { lit * glow })
            }
        }
        Look::Pole => [210.0, 210.0, 215.0],
        Look::Flag => [230.0, 70.0, 50.0],
        Look::Trail => [240.0, 220.0, 160.0],
        Look::Walker => [255.0, 245.0, 210.0],
    };
    scale(c, p.light())
}

/// The map between towers, filling the area.
fn draw_map(p: &map::Progress, area: Rect, buf: &mut Buffer) {
    let (w, h) = (area.width as usize, area.height as usize * 2);
    let ps = 1.0 / h as f32;
    let aspect = w as f32 / h as f32;
    let col = |c: Rgb| Color::Rgb(c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8);
    let at = |i: usize, j: usize| {
        let look = map::look(p, (i as f32 + 0.5) * ps, (j as f32 + 0.5) * ps, aspect, ps);
        col(map_colour(p, look, i as u32, j as u32))
    };
    for y in 0..area.height as usize {
        for x in 0..w {
            let cell = &mut buf[(area.x + x as u16, area.y + y as u16)];
            cell.set_symbol("▀");
            cell.set_fg(at(x, 2 * y));
            cell.set_bg(at(x, 2 * y + 1));
        }
    }
}

pub fn draw(game: &Game, view: &View, buf: &mut Buffer) {
    let area = view.area;
    let (w, h) = (area.width as usize, area.height as usize * 2);
    if w == 0 || h == 0 {
        return;
    }
    if let Some(p) = game.map() {
        draw_map(&p, area, buf);
        return;
    }
    let ps = 1.0 / view.s;
    let theme = &levels::tower(game.floor).theme;
    let roof = RoofView::new(game);
    let centre = roof.as_ref().map_or(V2::default(), |r| r.centre);
    let sc = Scene {
        faces: Faces::new(game),
        cam: game.camera(),
        balls: game.shown_balls(),
        tilt: game.tilt().map(|k| (roof::tilt(k, centre), roof::ground_below(game.floor))),
        roof,
        game,
    };
    let mut px = vec![[0.0f32; 3]; w * h];
    for j in 0..h {
        for i in 0..w {
            let p = V2::new((i as f32 + 0.5 - view.ox) * ps, (j as f32 + 0.5 - view.oy) * ps);
            let bg = sky(theme, i as u32, j as u32, h as f32, game.time);
            let below = if sc.tilt.is_some() { ground(theme, i as u32, j as u32) } else { bg };
            px[j * w + i] = shade(&sc, p, ps, bg, below);
        }
    }
    for part in &game.particles {
        let (i, j) = (part.p.x * view.s + view.ox, part.p.y * view.s + view.oy);
        if i >= 0.0 && j >= 0.0 && (i as usize) < w && (j as usize) < h {
            let k = (part.life / 0.5).min(1.0);
            let at = j as usize * w + i as usize;
            px[at] = mix(px[at], rgb(part.rgb), k);
        }
    }
    let col = |c: Rgb| Color::Rgb(c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8);
    for y in 0..area.height as usize {
        for x in 0..w {
            let cell = &mut buf[(area.x + x as u16, area.y + y as u16)];
            cell.set_symbol("▀");
            cell.set_fg(col(px[(2 * y) * w + x]));
            cell.set_bg(col(px[(2 * y + 1) * w + x]));
        }
    }
    // each capsule's letter, in the cell at its middle, if it is big enough to carry one
    if power::CAP_HALF.x * 2.0 * view.s >= 3.0 {
        for cap in &game.capsules {
            let (i, j) = (cap.p.x * view.s + view.ox, (cap.p.y * view.s + view.oy) / 2.0);
            if i >= 0.0 && j >= 0.0 && (i as usize) < w && (j as u16) < area.height {
                let cell = &mut buf[(area.x + i as u16, area.y + j as u16)];
                cell.set_symbol(cap.kind.letter()).set_fg(Color::Rgb(20, 18, 30)).set_bg(col(rgb(cap.kind.rgb())));
            }
        }
    }
}
