//! The tower: a cylinder of floor faces laid out as a spiral, four faces to a turn.
//!
//! Each face is one floor: lintel on top, a pilaster at each side (the field's walls),
//! a stone ledge along the bottom, and the floor's bricks on the masonry between.
//! Face `n` is centred at angle `n` quarter turns and sits `STEP_H` higher than face
//! `n - 1`; four steps make one face height, so face `n + 4` stands exactly on top of
//! face `n` and the spiral tiles the whole cylinder.
//!
//! The face turned to the camera is drawn flat and full width (it is the play field,
//! with ordinary flat physics). Past its edges the cylinder curves away to the
//! silhouette: the neighbouring faces are squashed and darkened there, the floor you
//! cleared round the left edge and lower, the next floor round the right and higher.
//! Turning to the next floor is a tween of the camera, in floors, from `n` to `n + 1`.

use crate::game::{self, Game, Kind, Mode, BRICK_H, BRICK_W, WORLD_H, WORLD_W};
use crate::levels;
use crate::physics::V2;
use std::f32::consts::FRAC_PI_4;

/// How much higher each floor's face sits than the one before.
pub const STEP_H: f32 = WORLD_H / 4.0;
/// Half the front face's width on screen (pilaster to pilaster, so x 9 to 91), drawn
/// flat; and half the tower's silhouette, where the curve ends.
const FLAT: f32 = 41.0;
const EDGE: f32 = 48.0;

/// What lies under a screen point on the tower.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    /// Which floor's face (0-based; negative is the plinth below the first floor).
    pub n: i64,
    /// Where on that face, in the same units as the play field (x 9 to 91, y 0 to 56).
    pub u: f32,
    pub v: f32,
    /// 1 on the front face, falling towards the silhouette.
    pub light: f32,
    /// How many face units one screen unit spans across here (more where squashed).
    pub squash: f32,
}

/// The tower under world point (`x`, `y`) with the camera facing floor `cam`.
pub fn sample(x: f32, y: f32, cam: f32) -> Option<Sample> {
    let dx = x - WORLD_W / 2.0;
    let ax = dx.abs();
    if ax > EDGE {
        return None;
    }
    // screen offset to angle: linear across the flat front face, then round the curve
    let (theta, light, squash) = if ax <= FLAT {
        (ax / FLAT * FRAC_PI_4, 1.0, 1.0)
    } else {
        let s = ((ax - FLAT) / (EDGE - FLAT)).min(0.999);
        let extra = s.asin() / 2.0;
        // d(face units)/d(screen units): 2 FLAT face units per quarter turn, and
        // d(theta)/dx = 1 / (2 (EDGE - FLAT) sqrt(1 - s^2))
        let squash = (4.0 * FLAT / std::f32::consts::PI) / (2.0 * (EDGE - FLAT) * (1.0 - s * s).sqrt()).max(0.05);
        (FRAC_PI_4 + extra, (2.0 * extra).cos(), squash)
    };
    // angle in quarter turns, measured from face 0
    let alpha = theta.copysign(dx) / (2.0 * FRAC_PI_4) + cam;
    let n0 = alpha.round();
    // the face at this angle that spans this height: n0, or one a whole turn above or below
    let v0 = y + (n0 - cam) * STEP_H;
    let k = -(v0 / WORLD_H).floor();
    Some(Sample {
        n: (n0 + 4.0 * k) as i64,
        u: WORLD_W / 2.0 + (alpha - n0) * 2.0 * FLAT,
        v: v0 + WORLD_H * k,
        light: 0.12 + 0.88 * light.max(0.0),
        squash,
    })
}

/// A brick on some face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrickHere {
    /// Floor and index: the same brick wherever it is sampled.
    pub id: (i64, usize),
    pub kind: Kind,
    pub hp: u8,
    pub flash: f32,
    /// Position inside the brick.
    pub lx: f32,
    pub ly: f32,
}

struct Grid {
    n: i64,
    boss: bool,
    origin: V2,
    cols: usize,
    rows: usize,
    cells: Vec<Option<Kind>>,
}

/// The bricks of the floors near the camera: the live current floor, the full layouts
/// of the floors above, and only the gilt left on the floors already cleared.
pub struct Faces {
    floor: i64,
    grids: Vec<Grid>,
}

impl Faces {
    pub fn new(game: &Game) -> Faces {
        let floor = game.floor as i64;
        let grids = (floor - 8..=floor + 8)
            .filter(|&n| n >= 0 && n != floor)
            .map(|n| {
                let def = &levels::FLOORS[n as usize % levels::FLOORS.len()];
                let (cols, rows, mut cells) = game::parse(def);
                if n < floor {
                    for c in cells.iter_mut() {
                        if *c != Some(Kind::Gilt) {
                            *c = None;
                        }
                    }
                }
                Grid { n, boss: def.boss, origin: game::line_origin(cols), cols, rows, cells }
            })
            .collect();
        Faces { floor, grids }
    }

    /// Does floor `n`'s face have the boss balcony in front of it?
    pub fn balcony(&self, game: &Game, n: i64) -> bool {
        if n == self.floor {
            game.arena.mode == Mode::Arc
        } else {
            self.grids.iter().any(|g| g.n == n && g.boss)
        }
    }

    pub fn brick(&self, game: &Game, n: i64, u: f32, v: f32) -> Option<BrickHere> {
        let p = V2::new(u, v);
        if n == self.floor {
            let (i, b) = game.brick_at(p)?;
            let q = p - game.origin;
            return Some(BrickHere {
                id: (n, i),
                kind: b.kind,
                hp: b.hp,
                flash: b.flash,
                lx: q.x.rem_euclid(BRICK_W),
                ly: q.y.rem_euclid(BRICK_H),
            });
        }
        let g = self.grids.iter().find(|g| g.n == n)?;
        let q = p - g.origin;
        if q.x < 0.0 || q.y < 0.0 {
            return None;
        }
        let (c, r) = ((q.x / BRICK_W) as usize, (q.y / BRICK_H) as usize);
        if c >= g.cols || r >= g.rows {
            return None;
        }
        let i = r * g.cols + c;
        let kind = g.cells[i]?;
        Some(BrickHere {
            id: (n, i),
            kind,
            hp: if kind == Kind::Steel { game::steel_hp(n as usize) } else { 1 },
            flash: 0.0,
            lx: q.x.rem_euclid(BRICK_W),
            ly: q.y.rem_euclid(BRICK_H),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{FIELD_TOP, FIELD_X0, FIELD_X1};

    #[test]
    fn front_face_is_the_field() {
        for cam in [0.0, 3.0, 7.0] {
            for (x, y) in [(FIELD_X0, FIELD_TOP), (50.0, 30.0), (FIELD_X1 - 0.01, 55.9)] {
                let s = sample(x, y, cam).unwrap();
                assert_eq!(s.n, cam as i64);
                assert!((s.u - x).abs() < 1e-3 && (s.v - y).abs() < 1e-3, "{s:?} at {x},{y}");
                assert_eq!(s.light, 1.0);
            }
        }
    }

    #[test]
    fn neighbours_peek_round_the_edges() {
        // next floor on the right and higher, the last on the left and lower
        let r = sample(95.0, 20.0, 4.0).unwrap();
        assert_eq!(r.n, 5);
        assert!((r.v - (20.0 + STEP_H)).abs() < 1e-3);
        let l = sample(5.0, 20.0, 4.0).unwrap();
        assert_eq!(l.n, 3);
        assert!((l.v - (20.0 - STEP_H)).abs() < 1e-3);
        // and the spiral stacks: low on the right is floor n - 3
        assert_eq!(sample(95.0, 50.0, 4.0).unwrap().n, 1);
        assert!(sample(99.0, 20.0, 4.0).is_none());
    }

    #[test]
    fn halfway_through_a_turn_the_seam_is_in_the_middle() {
        let left = sample(49.9, 30.0, 0.5).unwrap();
        let right = sample(50.1, 30.0, 0.5).unwrap();
        assert_eq!((left.n, right.n), (0, 1));
    }
}
