//! Glyph mode: pipe graphics, no half-blocks. Everything is drawn in box-drawing
//! lines: bricks are lengths of pipe (`├──┤`, or boxes when they are taller than a
//! row), the paddle a heavy bar, the walls double pipes.
//!
//! A cell is about twice as tall as it is wide, so the scale is picked to make a brick
//! exactly whole cells: 4 cells by 1 row at the smallest size (the field is then
//! 67x19, a classic 80x24 terminal with room for the HUD), 8 by 2 at the next. The
//! view is nudged so the brick grid lands on cell edges. Same world, same physics as
//! pixel mode; only the drawing differs.

use crate::game::{self, Game, Kind, Mode, BALL_R, BRICK_H, LEDGE, FIELD_TOP, FIELD_X0, FIELD_X1, PADDLE_HALF};
use crate::boss::{self, Gargoyle, Weathercock};
use crate::roof::{self, RoofView, Seen};
use crate::game::{Balcony, State};
use crate::physics::{Path, V2};
use crate::tower::{self, BrickHere, Faces, Sample};
use crate::render::View;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

const SKY: Color = Color::Rgb(6, 8, 22);
const FIELD: Color = Color::Rgb(20, 22, 42);
const BALCONY: Color = Color::Rgb(13, 11, 22);
const STEEL: Color = Color::Rgb(150, 165, 190);
const BRASS: Color = Color::Rgb(220, 165, 60);
const PADDLE_END: Color = Color::Rgb(40, 210, 190);
const PADDLE_BODY: Color = Color::Rgb(240, 175, 60);
const RAIL: Color = Color::Rgb(70, 90, 125);
const FIRE: Color = Color::Rgb(255, 140, 40);
const GARGOYLE_STONE: Color = Color::Rgb(175, 170, 185);
const GARGOYLE_WING: Color = Color::Rgb(120, 112, 140);
const MERLON: Color = Color::Rgb(175, 168, 152);
const CRENEL: Color = Color::Rgb(52, 49, 50);
const ROOF_FLOOR: Color = Color::Rgb(34, 33, 42);
const PLINTH: Color = Color::Rgb(48, 50, 60);
const PIPE_BG: Color = Color::Rgb(38, 46, 64);
const LEDGE_STONE: Color = Color::Rgb(175, 165, 145);
const LEDGE_SHADOW: Color = Color::Rgb(55, 50, 56);

/// Pixels per world unit at size step `k`: a brick is `4k` cells by `k` rows.
fn scale_for(k: u32) -> f32 {
    2.0 * k as f32 / BRICK_H
}

/// The largest size at which the camera's box (`centre`, `size`) fits, aligned so
/// `align` (the brick grid's corner) sits on a cell corner.
pub fn view(area: Rect, align: V2, (centre, size): (V2, V2)) -> View {
    let (w, h) = (area.width as f32, area.height as f32 * 2.0);
    let fits = |k: u32| size.x * scale_for(k) <= w && size.y * scale_for(k) <= h;
    let k = (1..8).take_while(|&k| fits(k)).last().unwrap_or(1);
    let s = scale_for(k);
    let mut ox = w / 2.0 - centre.x * s;
    let mut oy = h / 2.0 - centre.y * s;
    let ax = align.x * s + ox;
    ox += ax.round() - ax;
    let ay = align.y * s + oy;
    oy += (ay / 2.0).round() * 2.0 - ay;
    View { area, s, ox, oy }
}

fn rgb(c: [u8; 3]) -> Color {
    Color::Rgb(c[0], c[1], c[2])
}

fn dim(c: [u8; 3], k: f32) -> Color {
    Color::Rgb((c[0] as f32 * k) as u8, (c[1] as f32 * k) as u8, (c[2] as f32 * k) as u8)
}

fn hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h = h.wrapping_mul(0xcb1a_b31f);
    h ^ (h >> 16)
}

struct Canvas<'a> {
    buf: &'a mut Buffer,
    view: &'a View,
}

impl Canvas<'_> {
    /// Cell column and row (relative to the area) holding world point `p`.
    fn cell(&self, p: V2) -> (i32, i32) {
        let px = p.x * self.view.s + self.view.ox;
        let py = p.y * self.view.s + self.view.oy;
        (px.floor() as i32, (py / 2.0).floor() as i32)
    }

    /// The world point at the middle of a cell.
    fn centre(&self, c: i32, r: i32) -> V2 {
        V2::new((c as f32 + 0.5 - self.view.ox) / self.view.s, (r as f32 * 2.0 + 1.0 - self.view.oy) / self.view.s)
    }

    fn put(&mut self, c: i32, r: i32, sym: &str, fg: Color, bg: Option<Color>) {
        let a = self.view.area;
        if c < 0 || r < 0 || c >= a.width as i32 || r >= a.height as i32 {
            return;
        }
        let cell = &mut self.buf[(a.x + c as u16, a.y + r as u16)];
        cell.set_symbol(sym).set_fg(fg);
        if let Some(bg) = bg {
            cell.set_bg(bg);
        }
    }

    fn bg(&mut self, c: i32, r: i32, bg: Color) {
        self.put(c, r, " ", bg, Some(bg));
    }
}

/// A line drawn along a path, one stroke per cell it passes through, each stroke
/// following the path's direction there.
fn trace(cv: &mut Canvas, path: &Path, fg: Color) {
    let n = (path.len() * 4.0) as usize;
    let cells = walk(cv, (0..=n).map(|i| path.point(path.len() * i as f32 / n as f32)));
    for (c, r, sym) in strokes(&cells, false, path.wraps()) {
        cv.put(c, r, sym, fg, None);
    }
}

/// The cells a line passes through, in order, each once.
fn walk(cv: &Canvas, points: impl Iterator<Item = V2>) -> Vec<(i32, i32)> {
    let mut cells: Vec<(i32, i32)> = Vec::new();
    for p in points {
        let cell = cv.cell(p);
        if cells.last() != Some(&cell) {
            cells.push(cell);
        }
    }
    cells
}

/// A connected line through `cells`: level runs as `─` (`━` if `heavy`), upright
/// runs as `│`, and wherever the line steps to another row, one `╱` or `╲` at the step,
/// so a gentle slope reads as one line rather than separate dashes. `closed` joins
/// the last cell back to the first (a ring).
fn strokes(cells: &[(i32, i32)], heavy: bool, closed: bool) -> Vec<(i32, i32, &'static str)> {
    let n = cells.len();
    let steep = match (cells.first(), cells.last()) {
        (Some(&(c0, r0)), Some(&(c1, r1))) => (r1 - r0).abs() > (c1 - c0).abs(),
        _ => false,
    };
    (0..n)
        .map(|i| {
            let (c, r) = cells[i];
            let at = |j: isize| -> Option<(i32, i32)> {
                let j = i as isize + j;
                if closed { Some(cells[j.rem_euclid(n as isize) as usize]) } else { cells.get(usize::try_from(j).ok()?).copied() }
            };
            // the rows this cell steps to on either side
            let step = |o: Option<(i32, i32)>| o.map(|(oc, or)| (oc - c, or - r));
            let (back, ahead) = (step(at(-1)), step(at(1)));
            let rises = |d: Option<(i32, i32)>| d.is_some_and(|(_, dr)| dr != 0);
            // upright where both neighbours are straight above and below, or at an
            // end of a line that is steep overall
            let upright = match (back, ahead) {
                (Some((bc, _)), Some((ac, _))) => bc == 0 && ac == 0,
                (Some((dc, _)), None) | (None, Some((dc, _))) => dc == 0 && steep,
                _ => false,
            };
            let sym = if upright {
                if heavy { "┃" } else { "│" }
            } else if rises(ahead) {
                // which way the line runs through here, left to right on screen
                let (dc, dr) = match (back, ahead) {
                    (Some((bc, br)), Some((ac, ar))) => (ac - bc, ar - br),
                    (Some((bc, br)), None) => (-bc, -br),
                    (None, Some(d)) => d,
                    _ => (1, 0),
                };
                if (dc > 0) == (dr > 0) { "╲" } else { "╱" }
            } else if heavy {
                "━"
            } else {
                "─"
            };
            (c, r, sym)
        })
        .collect()
}

/// The box-drawing stroke for a line running along cell-space direction `t`.
fn line(t: V2, heavy: bool) -> &'static str {
    let slope = t.y.abs() / t.x.abs().max(1e-6);
    if slope < 0.45 {
        if heavy { "━" } else { "─" }
    } else if slope > 2.2 {
        if heavy { "┃" } else { "│" }
    } else if (t.x > 0.0) == (t.y > 0.0) {
        "╲"
    } else {
        "╱"
    }
}

/// A ring of pipe round `p`, `r` world units in radius, at its true size in cells.
/// Returns the cells it covers (left, top, width, height).
fn pipe_ring(cv: &mut Canvas, p: V2, r: f32, fg: Color) -> (i32, i32, i32, i32) {
    let cols = ((2.0 * r * cv.view.s).round() as i32).max(2);
    let rows = ((r * cv.view.s).round() as i32).max(1);
    let c0 = (p.x * cv.view.s + cv.view.ox - cols as f32 / 2.0).round() as i32;
    let r0 = ((p.y * cv.view.s + cv.view.oy) / 2.0 - rows as f32 / 2.0).round() as i32;
    for y in 0..rows {
        for x in 0..cols {
            let (left, right, top, bottom) = (x == 0, x == cols - 1, y == 0, y == rows - 1);
            let sym = if rows == 1 {
                if left { "(" } else if right { ")" } else { "─" }
            } else {
                match (left, right, top, bottom) {
                    (true, _, true, _) => "╭",
                    (_, true, true, _) => "╮",
                    (true, _, _, true) => "╰",
                    (_, true, _, true) => "╯",
                    (true, _, _, _) | (_, true, _, _) => "│",
                    (_, _, true, _) | (_, _, _, true) => "─",
                    _ => " ",
                }
            };
            cv.put(c0 + x, r0 + y, sym, fg, None);
        }
    }
    (c0, r0, cols, rows)
}

/// The Weathercock: its arrow a heavy iron line with a brass head pointing its way
/// and a fletched tail, the rooster on the hub a verdigris ring with a red comb. It
/// shudders before a gust and blinks white after a hit.
fn draw_weathercock(cv: &mut Canvas, w: &Weathercock) {
    let blink = w.hurt > 0.0 && (w.hurt * 12.0) as i32 % 2 == 0;
    let shake = if w.warning() { V2::new((w.t * 70.0).sin() * 0.6, 0.0) } else { V2::default() };
    let (tail, head) = w.arrow();
    let (tail, head) = (tail + shake, head + shake);
    let iron = if blink { Color::Rgb(255, 255, 255) } else { Color::Rgb(150, 152, 168) };
    let cells = walk(cv, (0..=64).map(|i| tail + (head - tail) * (i as f32 / 64.0)));
    for (c, r, sym) in strokes(&cells, true, false) {
        cv.put(c, r, sym, iron, None);
    }
    // the head: an arrow pointing the way the vane points (screen octant)
    let d = w.dir();
    let octant = ((d.y.atan2(d.x) / (std::f32::consts::TAU / 8.0)).round() as i32).rem_euclid(8) as usize;
    let (hc, hr) = cv.cell(head);
    cv.put(hc, hr, ["→", "↘", "↓", "↙", "←", "↖", "↑", "↗"][octant], if blink { iron } else { BRASS }, None);
    let (tc, tr) = cv.cell(tail);
    cv.put(tc, tr, "≡", iron, None);
    let fg = if blink { Color::Rgb(255, 255, 255) } else { Color::Rgb(110, 200, 165) };
    let (c0, r0, cols, _) = pipe_ring(cv, w.centre + shake, boss::ROOSTER_R, fg);
    if cols >= 3 {
        cv.put(c0 + cols / 2, r0, "▲", if blink { fg } else { Color::Rgb(225, 50, 40) }, None);
    }
}

/// The Gargoyle, two rows tall (smallest size) or four, wings up and wings down.
/// Spaces are see-through; columns `HEAD` are head and body, the rest wings.
const GARGOYLE_SMALL: [[&str; 2]; 2] = [["╲╲(●▾●)╱╱", "  ╰┴─┴╯  "], ["─╮(●▾●)╭─", "╱╱╰┴─┴╯╲╲"]];
const GARGOYLE_BIG: [[&str; 4]; 2] = [
    ["╲╲      ╲ ╱      ╱╱", " ╲╲╲   (●▾●)   ╱╱╱ ", "   ╲╲─╮│   │╭─╱╱   ", "       ╰┴───┴╯     "],
    ["        ╲ ╱        ", "───────(●▾●)───────", "╱╱╱  ╭─│   │─╮  ╲╲╲", "       ╰┴───┴╯     "],
];

fn draw_gargoyle(cv: &mut Canvas, g: &Gargoyle) {
    let rows_fit = (boss::HALF.y * cv.view.s).round() as i32;
    let up = g.flap() < 0.0;
    let sprite: Vec<&str> = if rows_fit >= 4 {
        GARGOYLE_BIG[if up { 0 } else { 1 }].to_vec()
    } else {
        GARGOYLE_SMALL[if up { 0 } else { 1 }].to_vec()
    };
    let width = sprite.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i32;
    let head = if rows_fit >= 4 { 7..=11 } else { 2..=6 };
    let blink = g.hurt > 0.0 && (g.hurt * 12.0) as i32 % 2 == 0;
    let px = g.p.x * cv.view.s + cv.view.ox;
    let py = (g.p.y * cv.view.s + cv.view.oy) / 2.0;
    let (c0, r0) = ((px - width as f32 / 2.0).round() as i32, (py - sprite.len() as f32 / 2.0).round() as i32);
    for (r, line) in sprite.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let fg = if blink {
                Color::Rgb(255, 255, 255)
            } else if ch == '●' {
                if g.aiming() { Color::Rgb(255, 220, 120) } else { Color::Rgb(255, 60, 40) }
            } else if head.contains(&(c as i32)) {
                GARGOYLE_STONE
            } else {
                GARGOYLE_WING
            };
            let mut buf = [0u8; 4];
            cv.put(c0 + c as i32, r0 + r as i32, ch.encode_utf8(&mut buf), fg, None);
        }
    }
}

/// What a cell of the tower shows.
#[derive(Clone, Copy, PartialEq)]
enum Part {
    Sky,
    Lintel,
    Pilaster,
    Ledge,
    Wall,
    Brick(BrickHere),
    /// The boss balcony: walkway, rail (with its direction in cells), and the shadow
    /// it casts on the wall below.
    Walkway,
    Rail(V2),
    Shadow,
    /// The roof, seen from above: merlons and the gaps between them round the rim,
    /// flagstones, the Weathercock's iron plinth; and the parapet's outer wall.
    Merlon,
    Crenel,
    RoofFloor,
    Plinth,
    ParapetWall,
}

/// What part of the roof lies at roof point `q`.
fn roof_part(game: &Game, roof: &RoofView, q: V2) -> Part {
    let off = q - roof.centre;
    let d = off.len();
    if d > roof::RIM - roof::PARAPET {
        let merlon = roof::merlon_at(off.y.atan2(off.x));
        return if merlon { Part::Merlon } else { Part::Crenel };
    }
    if let Some(b) = roof.brick(game, q) {
        return Part::Brick(b);
    }
    if d < 5.0 { Part::Plinth } else { Part::RoofFloor }
}

fn lit(c: Color, k: f32) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb((r as f32 * k) as u8, (g as f32 * k) as u8, (b as f32 * k) as u8),
        c => c,
    }
}

/// Straight floors: the tower's faces, each cell drawn from what it shows and what
/// its neighbours show, so bricks keep their ends even squashed round the curve.
fn draw_tower(cv: &mut Canvas, game: &Game) {
    let cam = game.camera();
    let faces = Faces::new(game);
    let roof = RoofView::new(game);
    // near the roof, the camera tilts: each cell sees roof, parapet, side or beyond
    let tilt = game.tilt().zip(roof.as_ref()).map(|(k, rv)| roof::tilt(k, rv.centre));
    let ground_y = roof::ground_below(game.floor);
    let (w, h) = (cv.view.area.width as i32, cv.view.area.height as i32);
    let seen: Vec<Seen> = (0..h)
        .flat_map(|r| (0..w).map(move |c| (c, r)))
        .map(|(c, r)| {
            let p = cv.centre(c, r);
            tilt.map_or(Seen::Side(p), |t| t.seen(p, ground_y))
        })
        .collect();
    let samples: Vec<Option<Sample>> = seen
        .iter()
        .map(|v| match *v {
            Seen::Side(sp) => tower::sample(sp.x, sp.y, cam),
            _ => None,
        })
        .collect();
    let parts: Vec<Part> = samples
        .iter()
        .map(|s| match *s {
            None => Part::Sky,
            Some(s) if s.v < FIELD_TOP => Part::Lintel,
            Some(s) if s.u < FIELD_X0 || s.u >= FIELD_X1 => Part::Pilaster,
            Some(s) if s.v >= LEDGE => Part::Ledge,
            Some(s) => faces.brick(game, s.n, s.u, s.v).map_or(Part::Wall, Part::Brick),
        })
        .zip(samples.iter())
        .map(|(part, s)| {
            let Some(s) = s.filter(|s| part != Part::Sky && faces.balcony(game, s.n)) else { return part };
            let (centre, rad) = game::balcony_arc();
            let off = V2::new(s.u, s.v) - centre;
            match game::balcony_at(off.len(), rad) {
                Balcony::Open => part,
                Balcony::Walkway(_) => Part::Walkway,
                Balcony::Rail(_) => Part::Rail(V2::new(-off.y * cv.view.s, off.x * cv.view.s / 2.0)),
                Balcony::Shadow => Part::Shadow,
            }
        })
        .zip(seen.iter())
        .map(|(part, v)| match (*v, &roof) {
            (Seen::Roof(q), Some(rv)) => roof_part(game, rv, q),
            (Seen::Parapet(_), _) => Part::ParapetWall,
            _ => part,
        })
        .collect();
    let at = |c: i32, r: i32| if c < 0 || r < 0 || c >= w || r >= h { Part::Sky } else { parts[(r * w + c) as usize] };
    let same = |a: Part, b: Part| match (a, b) {
        (Part::Brick(x), Part::Brick(y)) => x.id == y.id,
        _ => false,
    };
    for r in 0..h {
        for c in 0..w {
            let here = samples[(r * w + c) as usize];
            let k = here.map_or(1.0, |s| s.light);
            let wall = lit(if here.is_some_and(|s| s.n < 0) { BALCONY } else { FIELD }, k);
            match at(c, r) {
                Part::Sky => {}
                Part::Merlon => cv.put(c, r, "▒", MERLON, Some(CRENEL)),
                Part::Crenel => cv.bg(c, r, CRENEL),
                Part::RoofFloor => cv.bg(c, r, ROOF_FLOOR),
                Part::Plinth => cv.bg(c, r, PLINTH),
                Part::ParapetWall => cv.put(c, r, "░", MERLON, Some(CRENEL)),
                Part::Wall => cv.bg(c, r, wall),
                Part::Lintel => {
                    let Some(s) = here else { continue };
                    let left = samples.get((r * w + c - 1) as usize).copied().flatten();
                    let band = at(c - 1, r) == Part::Lintel && left.is_some_and(|l| (l.u / 9.0).floor() != (s.u / 9.0).floor());
                    let below_edge = at(c, r + 1) == Part::Pilaster && (at(c - 1, r + 1) != Part::Pilaster || at(c + 1, r + 1) != Part::Pilaster);
                    let (sym, fg) = if below_edge {
                        ("╦", STEEL)
                    } else if band {
                        ("╪", BRASS)
                    } else {
                        ("═", STEEL)
                    };
                    cv.put(c, r, sym, lit(fg, k), Some(wall));
                }
                Part::Pilaster => {
                    // a column is a pipe: lines down its edges, steel between
                    let edge = at(c - 1, r) != Part::Pilaster || at(c + 1, r) != Part::Pilaster;
                    if edge {
                        let Some(s) = here else { continue };
                        let above = if r > 0 { samples[((r - 1) * w + c) as usize] } else { None };
                        let band = at(c, r - 1) == Part::Pilaster && above.is_some_and(|a| (a.v / 9.0).floor() != (s.v / 9.0).floor());
                        let (sym, fg) = if band { ("╫", BRASS) } else { ("║", STEEL) };
                        cv.put(c, r, sym, lit(fg, k), Some(lit(PIPE_BG, k)));
                    } else {
                        cv.bg(c, r, lit(PIPE_BG, k));
                    }
                }
                Part::Ledge => cv.put(c, r, "▒", lit(LEDGE_STONE, k), Some(lit(LEDGE_SHADOW, k))),
                Part::Walkway => cv.put(c, r, "░", lit(LEDGE_STONE, k), Some(lit(LEDGE_SHADOW, k))),
                Part::Rail(t) => cv.put(c, r, line(t, false), lit(STEEL, k), Some(lit(PIPE_BG, k * 0.6))),
                Part::Shadow => cv.bg(c, r, lit(wall, 0.4)),
                Part::Brick(b) => {
                    let here = at(c, r);
                    let (left, right) = (!same(here, at(c - 1, r)), !same(here, at(c + 1, r)));
                    let (top, bottom) = (!same(here, at(c, r - 1)), !same(here, at(c, r + 1)));
                    let base = game::brick_rgb(b.kind);
                    let (bg, fg) = if b.flash > 0.0 {
                        (Color::Rgb(255, 255, 255), dim(base, 0.5))
                    } else {
                        (lit(dim(base, 0.3), k), lit(rgb(base), k))
                    };
                    let double = match b.kind {
                        Kind::Plain(_) | Kind::Stone => false,
                        Kind::Steel => b.hp > 1,
                        Kind::Gilt => true,
                    };
                    let sym = brick_stroke(b.kind, double, left, right, top, bottom);
                    cv.put(c, r, sym, fg, Some(bg));
                }
            }
        }
    }
}

/// The stroke for one cell of a brick, from which of its sides are the brick's edge.
fn brick_stroke(kind: Kind, double: bool, left: bool, right: bool, top: bool, bottom: bool) -> &'static str {
    let (ends, run, side, corners) = match (kind, double) {
        (Kind::Gilt, _) => (["╠", "╣"], "═", "║", ["╔", "╗", "╚", "╝"]),
        (_, true) => (["╞", "╡"], "═", "│", ["╒", "╕", "╘", "╛"]),
        _ => (["├", "┤"], "─", "│", ["┌", "┐", "└", "┘"]),
    };
    if top && bottom {
        // one row tall: a length of pipe; one cell wide (squashed): just its side
        return match (left, right) {
            (true, true) => side,
            (true, false) => ends[0],
            (false, true) => ends[1],
            _ => run,
        };
    }
    match (left, right, top, bottom) {
        (true, _, true, _) => corners[0],
        (_, true, true, _) => corners[1],
        (true, _, _, true) => corners[2],
        (_, true, _, true) => corners[3],
        (true, _, _, _) | (_, true, _, _) => side,
        (_, _, true, _) | (_, _, _, true) => run,
        _ => " ",
    }
}

pub fn draw(game: &Game, view: &View, buf: &mut Buffer) {
    let area = view.area;
    let mut cv = Canvas { buf, view };
    let (w, h) = (area.width as i32, area.height as i32);

    // sky with stars fixed to the screen, twinkling; or, once the camera looks down
    // from the roof, the ground far below
    let looking_down = game.tilt().is_some_and(|k| k > 0.6);
    for r in 0..h {
        for c in 0..w {
            let hv = hash(c as u32, r as u32);
            if looking_down {
                let field = hash(c as u32 / 4, r as u32 / 2) & 0xff;
                let bg = Color::Rgb(10 + (field / 24) as u8, 18 + (field / 30) as u8, 14);
                if hv % 29 == 0 {
                    cv.put(c, r, ".", Color::Rgb(32, 46, 30), Some(bg));
                } else {
                    cv.bg(c, r, bg);
                }
            } else if hv % 53 == 0 {
                let tw = 0.5 + 0.5 * (game.time * (0.6 + (hv >> 22) as f32 / 1024.0 * 1.5) + hv as f32).sin();
                let sym = ["·", ".", "+", "*"][(hv >> 6) as usize % 4];
                let k = 0.3 + 0.7 * tw;
                cv.put(c, r, sym, Color::Rgb((200.0 * k) as u8, (205.0 * k) as u8, (235.0 * k) as u8), Some(SKY));
            } else {
                cv.bg(c, r, SKY);
            }
        }
    }

    draw_tower(&mut cv, game);
    // the paddle's path along the balcony or round the roof, once the camera is still
    if game.arena.mode != Mode::Line && !matches!(game.state, State::Clear { .. }) {
        trace(&mut cv, &game.arena.path, RAIL);
    }
    if let Some(w) = &game.vane {
        draw_weathercock(&mut cv, w);
    }

    for part in &game.particles {
        let (c, r) = cv.cell(part.p);
        let sym = if part.life > 0.35 { "*" } else { "." };
        cv.put(c, r, sym, dim(part.rgb, (part.life / 0.5).min(1.0)), None);
    }

    if let Some(g) = &game.boss {
        draw_gargoyle(&mut cv, g);
    }
    for shot in &game.shots {
        let (c, r) = cv.cell(shot.p);
        cv.put(c, r, "*", FIRE, None);
    }

    // the paddle: a heavy bar, one stroke per cell along it, teal at the ends (grey
    // and flickering while stunned)
    let (body, ends) = if game.stun > 0.0 {
        let grey = if (game.time * 20.0) as i32 % 2 == 0 { Color::Rgb(90, 90, 100) } else { Color::Rgb(160, 160, 170) };
        (grey, grey)
    } else {
        (PADDLE_BODY, PADDLE_END)
    };
    let climbing = game.tilt().is_some() && game.arena.mode != Mode::Circle;
    let (a, b, _) = game.paddle_segment();
    let steps = if climbing { 0 } else { (PADDLE_HALF * 8.0) as usize };
    let cells = walk(&cv, (0..=steps).map(|i| a + (b - a) * (i as f32 / steps as f32)));
    // the teal ends: the last 2 units at each end of the bar
    let bar: Vec<_> = strokes(&cells, true, false)
        .into_iter()
        .map(|(c, r, sym)| {
            let p = cv.centre(c, r);
            (c, r, sym, (p - a).len().min((p - b).len()) < 2.0)
        })
        .collect();
    for (c, r, sym, tip) in bar {
        cv.put(c, r, sym, if tip { ends } else { body }, None);
    }
    // cap a level paddle with half-strokes so its ends read as rounded
    let ((ca, ra), (cb, rb)) = (cv.cell(a), cv.cell(b));
    if ra == rb && !climbing {
        cv.put(ca.min(cb), ra, "╺", ends, None);
        cv.put(ca.max(cb), rb, "╸", ends, None);
    }

    // balls: a little pipe ring at the ball's true size (`╭─╮ ╰─╯` when it is two
    // rows tall, `()` at the smallest size), centred on the ball
    for p in game.shown_balls() {
        pipe_ring(&mut cv, p, BALL_R, Color::Rgb(255, 255, 240));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gargoyle_sprites_are_square() {
        for frame in GARGOYLE_SMALL {
            assert!(frame.iter().all(|l| l.chars().count() == 9), "{frame:?}");
        }
        for frame in GARGOYLE_BIG {
            assert!(frame.iter().all(|l| l.chars().count() == 19), "{frame:?}");
        }
    }
}
