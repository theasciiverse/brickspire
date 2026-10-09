//! Half-block rendering: every terminal cell is two roughly square pixels (`▀` with
//! the top pixel in the foreground colour and the bottom in the background). Each
//! pixel is shaded by asking the world what lies under its centre, so the picture
//! scales to any terminal; round things (ball, paddle) get analytic anti-aliasing.

use crate::game::{self, Game, Kind, BALL_R, BRICK_H, BRICK_W, FIELD_TOP, FIELD_X0, FIELD_X1, PADDLE_HALF, PADDLE_TH, WALL, WORLD_H, WORLD_W};
use crate::physics::{self, Path, V2};
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
    pub fn new(area: Rect) -> View {
        let (w, h) = (area.width as f32, area.height as f32 * 2.0);
        let s = (w / WORLD_W).min(h / WORLD_H).max(0.01);
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

/// The tower's own wall behind the bricks: faint masonry.
fn masonry(p: V2, ps: f32) -> Rgb {
    let row = (p.y / 4.0).floor();
    let x = p.x + if row as i32 % 2 == 0 { 0.0 } else { 4.0 };
    let mortar = p.y.rem_euclid(4.0) < ps || x.rem_euclid(8.0) < ps;
    if mortar { [14.0, 16.0, 30.0] } else { [26.0, 28.0, 48.0] }
}

/// What lies under world point `p` (pixel size `ps` in world units).
fn shade(game: &Game, balls: &[V2], p: V2, ps: f32, bg: Rgb) -> Rgb {
    let mut c = bg;
    let walls = game.arena.walls();
    match game.arena.path {
        Path::Circle { c: centre, r } => {
            // the round roof under the ring, with the ring itself marked
            let d = (p - centre).len();
            if d < r + PADDLE_TH + 1.5 {
                c = masonry(p, ps);
            }
            if (d - r).abs() < ps * 0.6 {
                c = mix(c, [70.0, 90.0, 120.0], 0.8);
            }
        }
        _ => {
            if p.x >= FIELD_X0 && p.x < FIELD_X1 && p.y >= FIELD_TOP && p.y < WORLD_H {
                c = masonry(p, ps);
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
    if walls {
        let in_side = (p.x >= FIELD_X0 - WALL && p.x < FIELD_X0) || (p.x >= FIELD_X1 && p.x < FIELD_X1 + WALL);
        let in_top = p.y >= FIELD_TOP - WALL && p.y < FIELD_TOP && p.x >= FIELD_X0 - WALL && p.x < FIELD_X1 + WALL;
        if (in_side && p.y >= FIELD_TOP - WALL && p.y < WORLD_H) || in_top {
            // steel pipe, lit across its width, with brass rivets
            let across = if in_top && !in_side { (p.y - (FIELD_TOP - WALL)) / WALL } else if p.x < FIELD_X0 { (p.x - (FIELD_X0 - WALL)) / WALL } else { (p.x - FIELD_X1) / WALL };
            let along = if in_top && !in_side { p.x } else { p.y };
            let lit = 1.0 - (across - 0.35).abs() * 1.3;
            c = mix([40.0, 50.0, 70.0], [170.0, 185.0, 210.0], lit);
            if along.rem_euclid(9.0) < ps * 1.01 {
                c = [210.0, 160.0, 60.0];
            }
        }
    }
    if let Some((_, brick)) = game.brick_at(p) {
        let q = p - game.origin;
        let (lx, ly) = (q.x.rem_euclid(BRICK_W), q.y.rem_euclid(BRICK_H));
        let mut base = rgb(game::brick_rgb(brick.kind));
        if brick.kind == Kind::Steel || brick.kind == Kind::Gilt {
            // a sheen sweeping across metal bricks
            let sheen = ((q.x + q.y * 2.0 - game.time * 30.0).rem_euclid(60.0) - 30.0).abs() < 2.0;
            if sheen {
                base = mix(base, [255.0, 255.0, 255.0], 0.45);
            }
        }
        c = if lx < ps || ly < ps {
            mix(base, [255.0, 255.0, 255.0], 0.45)
        } else if lx >= BRICK_W - ps || ly >= BRICK_H - ps {
            scale(base, 0.45)
        } else {
            base
        };
        if brick.flash > 0.0 {
            c = mix(c, [255.0, 255.0, 255.0], brick.flash / 0.15);
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
        let body = if along > PADDLE_HALF - 2.0 { [40.0, 200.0, 185.0] } else { [235.0, 170.0, 55.0] };
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
    let mut px = vec![[0.0f32; 3]; w * h];
    for j in 0..h {
        for i in 0..w {
            let p = V2::new((i as f32 + 0.5 - view.ox) * ps, (j as f32 + 0.5 - view.oy) * ps);
            let bg = sky(i as u32, j as u32, h as f32, game.time);
            px[j * w + i] = shade(game, &balls, p, ps, bg);
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
