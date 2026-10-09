//! Half-block rendering: every terminal cell is two roughly square pixels (`▀` with
//! the top pixel in the foreground colour and the bottom in the background). Each
//! pixel is shaded by asking the world what lies under its centre, so the picture
//! scales to any terminal; round things (ball, paddle) get analytic anti-aliasing.

use crate::game::{self, Game, Kind, Mode, LEDGE, BALL_R, BRICK_H, BRICK_W, FIELD_TOP, FIELD_X0, FIELD_X1, PADDLE_HALF, PADDLE_TH, WALL, WORLD_H, WORLD_W};
use crate::boss::{self, Gargoyle};
use crate::game::Balcony;
use crate::physics::{self, Path, V2};
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
    /// The world fitted to `area`, then pulled back by `zoom` (1 is a snug fit).
    pub fn new(area: Rect, zoom: f32) -> View {
        let (w, h) = (area.width as f32, area.height as f32 * 2.0);
        let s = (w / WORLD_W).min(h / WORLD_H).max(0.01) * zoom;
        View { area, s, ox: (w - WORLD_W * s) / 2.0, oy: (h - WORLD_H * s) / 2.0 }
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

/// Night sky with stars fixed to the screen, twinkling.
fn sky(i: u32, j: u32, h_px: f32, time: f32) -> Rgb {
    let base = mix([6.0, 8.0, 26.0], [24.0, 14.0, 44.0], j as f32 / h_px);
    let h = hash(i, j);
    if h % 89 == 0 {
        let tw = 0.55 + 0.45 * (time * (0.7 + (h >> 20) as f32 / 4096.0 * 2.0) + h as f32).sin();
        let k = (0.35 + ((h >> 8) & 0xff) as f32 / 255.0 * 0.65) * tw;
        return mix(base, [235.0, 235.0, 255.0], k);
    }
    base
}

/// The tower's own wall behind the bricks: faint masonry. `psx`/`psy` are a
/// pixel's size across and down, so mortar stays a pixel thick where the face is squashed.
fn masonry(p: V2, psx: f32, psy: f32) -> Rgb {
    let row = (p.y / 4.0).floor();
    let x = p.x + if row as i32 % 2 == 0 { 0.0 } else { 4.0 };
    let mortar = p.y.rem_euclid(4.0) < psy || x.rem_euclid(8.0) < psx;
    if mortar { [14.0, 16.0, 30.0] } else { [26.0, 28.0, 48.0] }
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

/// Straight floors: whichever face of the tower is under `p`.
fn tower_face(game: &Game, faces: &Faces, cam: f32, p: V2, ps: f32, bg: Rgb) -> Rgb {
    let Some(s) = tower::sample(p.x, p.y, cam) else { return bg };
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
    } else {
        masonry(V2::new(u, v), psx, psy)
    };
    // the plinth below the first floor is plain and darker
    let mut c = if s.n < 0 && v >= FIELD_TOP { scale(masonry(V2::new(u, v), psx, psy), 0.7) } else { c };
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

/// The arc and roof previews: a plain field, walls and bricks, no tower yet.
fn scenery(game: &Game, p: V2, ps: f32, bg: Rgb) -> Rgb {
    let mut c = bg;
    match game.arena.path {
        Path::Circle { c: centre, r } => {
            // the round roof under the ring, with the ring itself marked
            let d = (p - centre).len();
            if d < r + PADDLE_TH + 1.5 {
                c = masonry(p, ps, ps);
            }
            if (d - r).abs() < ps * 0.6 {
                c = mix(c, [70.0, 90.0, 120.0], 0.8);
            }
        }
        _ => {
            if p.x >= FIELD_X0 && p.x < FIELD_X1 && p.y >= FIELD_TOP && p.y < WORLD_H {
                c = masonry(p, ps, ps);
                if let Path::Arc { c: centre, r, .. } = game.arena.path {
                    let d = (p - centre).len();
                    if d > r + PADDLE_TH + 0.5 {
                        // the balcony floor, below the rail
                        c = mix([30.0, 26.0, 38.0], [14.0, 12.0, 20.0], (d - r) / 12.0);
                    } else if (d - r).abs() < ps * 0.6 {
                        c = mix(c, [70.0, 90.0, 120.0], 0.8);
                    }
                }
            }
        }
    }
    if game.arena.walls() {
        let in_side = (p.x >= FIELD_X0 - WALL && p.x < FIELD_X0) || (p.x >= FIELD_X1 && p.x < FIELD_X1 + WALL);
        let in_top = p.y >= FIELD_TOP - WALL && p.y < FIELD_TOP && p.x >= FIELD_X0 - WALL && p.x < FIELD_X1 + WALL;
        if in_top {
            c = pipe((p.y - (FIELD_TOP - WALL)) / WALL, p.x, ps);
        } else if in_side && p.y < WORLD_H {
            let across = if p.x < FIELD_X0 { (p.x - (FIELD_X0 - WALL)) / WALL } else { (p.x - FIELD_X1) / WALL };
            c = pipe(across, p.y, ps);
        }
    }
    if let Some((i, brick)) = game.brick_at(p) {
        let q = p - game.origin;
        let here = tower::BrickHere {
            id: (game.floor as i64, i),
            kind: brick.kind,
            hp: brick.hp,
            flash: brick.flash,
            lx: q.x.rem_euclid(BRICK_W),
            ly: q.y.rem_euclid(BRICK_H),
        };
        c = brick_colour(&here, q, game.time, ps, ps);
    }
    c
}

/// The Gargoyle, in its own units (`l` from its centre, y down): stone head with
/// horns and red eyes (glowing hot just before it spits), a squat body with claws,
/// and scalloped wings beating up and down.
fn gargoyle(g: &Gargoyle, l: V2) -> Option<Rgb> {
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
fn shade(game: &Game, faces: &Faces, cam: f32, balls: &[V2], p: V2, ps: f32, bg: Rgb) -> Rgb {
    let mut c = if game.arena.mode != Mode::Circle { tower_face(game, faces, cam, p, ps, bg) } else { scenery(game, p, ps, bg) };
    if let Some(g) = &game.boss {
        let l = p - g.p;
        if l.x.abs() < boss::HALF.x + 0.5 && l.y.abs() < boss::HALF.y + 1.0 {
            if let Some(gc) = gargoyle(g, l) {
                c = gc;
            }
        }
    }
    for shot in &game.shots {
        let d = (p - shot.p).len();
        if d < boss::SHOT_R + ps {
            let fire = mix([255.0, 240.0, 150.0], [230.0, 60.0, 20.0], d / boss::SHOT_R);
            c = mix(c, fire, (boss::SHOT_R - d) / ps + 0.5);
        }
    }
    // the paddle: a brass bar with teal ends, lit from above along its normal
    let (a, b, n) = game.paddle_segment();
    let q = physics::closest_on_segment(p, a, b);
    let off = p - q;
    let d = off.len();
    if d < PADDLE_TH + ps {
        let along = (q - (a + b) * 0.5).dot((b - a).norm()).abs();
        let across = off.dot(n) / PADDLE_TH; // +1 top face, -1 underside
        let mut body = if along > PADDLE_HALF - 2.0 { [40.0, 200.0, 185.0] } else { [235.0, 170.0, 55.0] };
        if game.stun > 0.0 {
            // stunned: greyed, flickering
            let flick = if (game.time * 20.0) as i32 % 2 == 0 { 0.35 } else { 0.6 };
            body = scale([170.0, 170.0, 180.0], flick + 0.3);
        }
        let lit = mix(scale(body, 0.5), mix(body, [255.0, 250.0, 220.0], 0.5), (across + 1.0) / 2.0);
        c = mix(c, lit, (PADDLE_TH - d) / ps + 0.5);
    }
    for &bp in balls {
        let d = (p - bp).len();
        if d < BALL_R + ps {
            let hi = (p - bp + V2::new(0.4, 0.4)).len() / BALL_R;
            let ball = mix([255.0, 255.0, 245.0], [150.0, 160.0, 190.0], hi - 0.2);
            c = mix(c, ball, (BALL_R - d) / ps + 0.5);
        }
    }
    c
}

pub fn draw(game: &Game, view: &View, buf: &mut Buffer) {
    let area = view.area;
    let (w, h) = (area.width as usize, area.height as usize * 2);
    if w == 0 || h == 0 {
        return;
    }
    let ps = 1.0 / view.s;
    let balls = game.shown_balls();
    let faces = Faces::new(game);
    let cam = game.camera();
    let mut px = vec![[0.0f32; 3]; w * h];
    for j in 0..h {
        for i in 0..w {
            let p = V2::new((i as f32 + 0.5 - view.ox) * ps, (j as f32 + 0.5 - view.oy) * ps);
            let bg = sky(i as u32, j as u32, h as f32, game.time);
            px[j * w + i] = shade(game, &faces, cam, &balls, p, ps, bg);
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
}
