//! The roof, and the camera tilting up onto it.
//!
//! On the roof the camera looks straight down onto the round tower top: a stone disc
//! ringed by a crenellated parapet, with the paddle's ring inside it. Getting there
//! is one camera move, `k` from 0 to 1: the camera rises and tilts from the tower's
//! side to straight down. The roof shows as an ellipse that opens into a circle
//! (its height is `sin` of the tilt), growing from the tower's full width to the
//! roof's own size, while the tower's side below foreshortens away (`cos` of the
//! tilt) until only the ground far below is left round the roof.
//!
//! Screen points here are in world units, as everywhere else; roof points are in the
//! roof floor's own world (its ring centred where the arena puts it).

use crate::game::{self, BRICK_H, BRICK_W, Game, Kind, WORLD_W};
use crate::physics::V2;
use crate::tower::{self, BrickHere, STEP_H};
use std::f32::consts::FRAC_PI_2;

/// The roof's outer radius (parapet included) and the parapet's thickness. The
/// paddle's ring sits well inside, leaving open roof between it and the parapet: a
/// ball that gets past the paddle bounces off the merlons back into play, and is lost
/// only through the gaps between them.
pub const RIM: f32 = 33.0;
pub const PARAPET: f32 = 2.4;
/// The paddle's ring: as far out as leaves room behind it for a ball to get past and
/// bounce off the merlons (3.4 between the paddle's back and the parapet).
pub const RING: f32 = 26.0;
/// How many merlons, and how much of the rim they cover (the rest is gaps).
pub const MERLONS: f32 = 16.0;
pub const MERLON_SHARE: f32 = 0.65;

/// Is the parapet solid (a merlon) at screen angle `a`?
pub fn merlon_at(a: f32) -> bool {
    (a / (std::f32::consts::TAU / MERLONS)).rem_euclid(1.0) < MERLON_SHARE
}

/// The merlons as angle spans, for the physics.
pub fn merlons() -> impl Iterator<Item = (f32, f32)> {
    let step = std::f32::consts::TAU / MERLONS;
    (0..MERLONS as usize).map(move |k| (k as f32 * step, (k as f32 + MERLON_SHARE) * step))
}
/// The tilt when the camera starts up: just above level with the tower top.
const START_TILT: f32 = 0.12;
/// How tall the parapet's outer wall stands, in side-view units.
const PARAPET_WALL: f32 = 3.0;

/// The camera part-way up onto the roof.
#[derive(Clone, Copy, Debug)]
pub struct Tilt {
    k: f32,
    /// The roof's centre in its own world.
    centre: V2,
    /// Screen scale of the roof, its ellipse's height (`sin` of the tilt) and the
    /// side's foreshortening (`cos`).
    m: f32,
    pub e: f32,
    f: f32,
    /// Screen height of the ellipse's centre, and of its nearest edge (the lip where
    /// the tower's side begins).
    yc: f32,
    lip: f32,
}

pub fn tilt(k: f32, centre: V2) -> Tilt {
    let k = k.clamp(0.0, 1.0);
    let phi = START_TILT + (FRAC_PI_2 - START_TILT) * k;
    let (e, f) = (phi.sin(), phi.cos());
    let m0 = tower::EDGE / RIM;
    let m = m0 + (1.0 - m0) * k;
    // starts with the roof's near edge on the top of the face in view, ends centred
    let (yc0, yc1) = (-RIM * m0 * START_TILT.sin(), centre.y);
    let yc = yc0 + (yc1 - yc0) * k;
    Tilt { k, centre, m, e, f, yc, lip: yc + RIM * m * e }
}

/// What lies under a screen point while tilting.
#[derive(Clone, Copy, Debug)]
pub enum Seen {
    /// The roof's top, at this roof point.
    Roof(V2),
    /// The parapet's outer wall, between the roof's rim and the tower's faces; `k`
    /// 0 at its top to 1 at its foot.
    Parapet(f32),
    /// The tower's side, at this point of the ordinary side view.
    Side(V2),
    /// Past the tower: sky or the ground far below.
    Beyond,
}

impl Tilt {
    /// `ground` is how far below the top of the face in view the tower meets the ground.
    pub fn seen(&self, p: V2, ground: f32) -> Seen {
        let dx = p.x - WORLD_W / 2.0;
        let half = RIM * self.m;
        if dx.abs() > half {
            return Seen::Beyond;
        }
        let q = V2::new(dx / self.m, (p.y - self.yc) / (self.m * self.e));
        if q.len() <= RIM {
            return Seen::Roof(self.centre + q);
        }
        // the side below each column starts at the roof's near edge above it, so the
        // faces' tops curve round with the cylinder
        // (bending in over the start of the climb, so the lintels don't jump)
        let bend = (self.k * 4.0).min(1.0);
        let edge = self.lip - RIM * self.m * self.e * (1.0 - (1.0 - (dx / half).powi(2)).max(0.0).sqrt()) * bend;
        if p.y < edge {
            return Seen::Beyond;
        }
        let ys = (p.y - edge) / (self.f * self.m * RIM / tower::EDGE).max(1e-4);
        // the parapet's outer wall grows in as the camera starts up, so nothing jumps
        let wall = PARAPET_WALL * (self.k * 4.0).min(1.0);
        if ys < wall {
            Seen::Parapet(ys / wall.max(1e-4))
        } else if ys > ground {
            Seen::Beyond
        } else {
            Seen::Side(V2::new(WORLD_W / 2.0 + dx * tower::EDGE / half, ys))
        }
    }
}

/// How far below the top of floor `floor`'s face the tower meets the ground.
pub fn ground_below(floor: usize) -> f32 {
    game::WORLD_H + floor as f32 * STEP_H
}

/// What is on the roof: its bricks (live once you are up there, the layout before),
/// and its boss.
pub struct RoofView {
    live: bool,
    origin: V2,
    cols: usize,
    rows: usize,
    cells: Vec<Option<Kind>>,
    pub centre: V2,
    pub ring: f32,
}

impl RoofView {
    /// The roof of `game`'s tower: the current floor if it is the roof, else the next.
    pub fn new(game: &Game) -> Option<RoofView> {
        let (n, live) = if game.arena.mode == game::Mode::Circle {
            (game.floor, true)
        } else if game.roof_next() {
            (game.floor + 1, false)
        } else {
            return None;
        };
        let (centre, ring) = match game::Arena::new(game::Mode::Circle).path {
            crate::physics::Path::Circle { c, r } => (c, r),
            _ => unreachable!(),
        };
        let (cols, rows, cells) = game::parse(game.floor_def(n));
        let origin = if live { game.origin } else { centre - V2::new(cols as f32 * BRICK_W / 2.0, rows as f32 * BRICK_H / 2.0) };
        Some(RoofView { live, origin, cols, rows, cells, centre, ring })
    }

    pub fn brick(&self, game: &Game, q: V2) -> Option<BrickHere> {
        let local = q - self.origin;
        if local.x < 0.0 || local.y < 0.0 {
            return None;
        }
        let (c, r) = ((local.x / BRICK_W) as usize, (local.y / BRICK_H) as usize);
        if c >= self.cols || r >= self.rows {
            return None;
        }
        let i = r * self.cols + c;
        let (kind, hp, flash) = if self.live {
            let b = game.bricks[i]?;
            (b.kind, b.hp, b.flash)
        } else {
            let k = self.cells[i]?;
            (k, if k == Kind::Steel { 2 } else { 1 }, 0.0)
        };
        Some(BrickHere { id: (-1, i), kind, hp, flash, lx: local.x.rem_euclid(BRICK_W), ly: local.y.rem_euclid(BRICK_H) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: V2 = V2::new(50.0, 29.0);

    #[test]
    fn starts_as_the_side_view_and_ends_as_the_roof() {
        let t = tilt(0.0, C);
        // well below the lip, the side view is unchanged
        for (x, y) in [(20.0, 30.0), (50.0, 50.0), (80.0, 10.0)] {
            match t.seen(V2::new(x, y), 200.0) {
                // the faces' tops follow the curve a little even here, most at the sides
                Seen::Side(s) => assert!((s.x - x).abs() < 0.01 && (s.y - y).abs() < 2.0, "{s:?} for {x},{y}"),
                other => panic!("{other:?} at {x},{y}"),
            }
        }
        let t = tilt(1.0, C);
        for (x, y) in [(50.0, 29.0), (30.0, 20.0), (60.0, 45.0)] {
            match t.seen(V2::new(x, y), 200.0) {
                Seen::Roof(q) => assert!((q - V2::new(x, y)).len() < 0.01),
                other => panic!("{other:?} at {x},{y}"),
            }
        }
        // and nothing of the tower's side is left, at the middle or the corners
        for (x, y) in [(50.0, 29.0 + RIM + 1.5), (25.0, 55.0), (75.0, 55.0), (22.0, 52.0)] {
            assert!(matches!(t.seen(V2::new(x, y), 200.0), Seen::Beyond), "{x},{y}");
        }
    }
}
