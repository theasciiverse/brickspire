//! The map between towers: a wide landscape at dusk with all five towers standing
//! on it, side on. Towers already climbed fly a flag; the next one stands lit and
//! named; the rest are dark silhouettes, so their secrets stay hidden. A dotted
//! trail draws itself across the hills from the tower just climbed to the next.
//!
//! The camera comes with it: the map opens close on the top of the tower just
//! climbed and pulls back to the whole landscape; going on, it swoops in on the next
//! tower and fades. A new game opens on the map too, the whole landscape and no
//! tower climbed yet.
//!
//! Shared by both renderers: `look` says what lies at a screen point, each renderer
//! colours or draws it. Points are in screen heights: `y` 0 at the top to 1 at the
//! bottom, `x` 0 to `aspect` (the screen's width over its height, in square pixels).

use crate::levels;

/// The five towers as the map shows them, built or not yet.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Shape {
    Keep,
    Ivy,
    Bell,
    Lighthouse,
    Spire,
}

pub struct Landmark {
    pub name: &'static str,
    pub shape: Shape,
    /// Stone colour in daylight.
    pub stone: [u8; 3],
    /// Where it stands across the map (0 to 1 of the width), how wide and how tall
    /// its body is (in screen heights).
    pub at: f32,
    pub w: f32,
    pub h: f32,
}

pub const SKYLINE: [Landmark; 5] = [
    Landmark { name: "The Old Keep", shape: Shape::Keep, stone: [120, 124, 150], at: 0.11, w: 0.085, h: 0.26 },
    Landmark { name: "The Ivy Tower", shape: Shape::Ivy, stone: [110, 128, 112], at: 0.30, w: 0.075, h: 0.32 },
    Landmark { name: "The Bell Tower", shape: Shape::Bell, stone: [200, 160, 105], at: 0.50, w: 0.07, h: 0.34 },
    Landmark { name: "The Sea Tower", shape: Shape::Lighthouse, stone: [225, 222, 215], at: 0.69, w: 0.065, h: 0.38 },
    Landmark { name: "The Brickspire", shape: Shape::Spire, stone: [150, 140, 190], at: 0.88, w: 0.06, h: 0.48 },
];

/// Seconds before the trail starts, how long it takes to draw, and how long the map
/// stays before going on by itself (space goes on once the trail is drawn).
pub const TRAIL_START: f32 = 1.8;
pub const TRAIL_TIME: f32 = 2.2;
pub const MAP_TIME: f32 = 10.0;
/// The ending (the map after the last tower): when the Lamplighter's light
/// shows and the stars start coming back, when the sky is full again, and
/// when it moves on by itself.
pub const ENDING_LIGHT: f32 = 4.0;
pub const ENDING_FULL: f32 = 13.0;
pub const ENDING_TIME: f32 = 40.0;
/// The share of the sky's stars still shining with this many towers climbed:
/// the Mason is walling up the sky, and they go out as you climb.
const STARS_LEFT: [f32; 5] = [1.0, 0.75, 0.5, 0.3, 0.12];
/// How long the camera takes to pull back from the climbed tower's top, and to
/// swoop in on the next tower when going on; how close it starts and ends.
pub const ARRIVE: f32 = 1.6;
pub const LEAVE: f32 = 1.1;
const CLOSE: f32 = 0.12;

/// Where on the map the climb is.
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    /// The tower just climbed, and the next one.
    pub from: usize,
    pub next: usize,
    /// Seconds on the map, and the game's clock (for things that move).
    pub t: f32,
    pub time: f32,
    /// The start of a game: nothing climbed yet, no trail.
    pub start: bool,
    /// Seconds since going on to the next tower, while the camera swoops in.
    pub leave: Option<f32>,
}

impl Progress {
    pub fn climbed(&self, k: usize) -> bool {
        // round again after the last tower: every tower has its flag
        !self.start && (k <= self.from || self.next <= self.from)
    }

    /// How much of the trail is drawn, 0 to 1.
    pub fn trail(&self) -> f32 {
        if self.start {
            return 0.0;
        }
        ((self.t - TRAIL_START) / TRAIL_TIME).clamp(0.0, 1.0)
    }

    pub fn ready(&self) -> bool {
        if self.start {
            self.t > 0.8
        } else if self.ending() {
            self.t > ENDING_FULL
        } else {
            self.trail() >= 1.0
        }
    }

    /// Every tower climbed: the Mason beaten and the Lamplighter free.
    pub fn ending(&self) -> bool {
        !self.start && self.next <= self.from
    }

    /// How long the map waits before going on by itself.
    pub fn wait(&self) -> f32 {
        if self.ending() { ENDING_TIME } else { MAP_TIME }
    }

    /// The share of the sky's stars shining, 0 to 1: fewer after every tower,
    /// and in the ending, coming back one by one until the sky is full.
    pub fn stars(&self) -> f32 {
        if self.start {
            return STARS_LEFT[0];
        }
        if self.ending() {
            let k = ((self.t - ENDING_LIGHT) / (ENDING_FULL - ENDING_LIGHT)).clamp(0.0, 1.0);
            return 0.05 + 0.95 * k;
        }
        STARS_LEFT[(self.from + 1).min(STARS_LEFT.len() - 1)]
    }

    /// The camera: 0 close on a tower, 1 the whole landscape; and the tower it is
    /// close on (and where on it: 0 its foot, 1 its top).
    pub fn pull(&self) -> (f32, usize, f32) {
        let ease = |k: f32| {
            let k = k.clamp(0.0, 1.0);
            k * k * (3.0 - 2.0 * k)
        };
        match self.leave {
            Some(l) => (1.0 - ease(l / LEAVE), self.next, 0.35),
            None if self.start => (1.0, self.next, 0.35),
            None => (ease(self.t / ARRIVE), self.from, 0.85),
        }
    }

    /// Has the camera settled on the whole landscape?
    pub fn settled(&self) -> bool {
        self.pull().0 >= 1.0
    }

    /// How bright: fading in at first and out while leaving.
    pub fn light(&self) -> f32 {
        let fade_in = if self.start { (self.t / 0.6).min(1.0) } else { (self.t / 0.4).min(1.0) };
        let fade_out = self.leave.map_or(1.0, |l| (1.0 - (l - LEAVE * 0.55) / (LEAVE * 0.45)).clamp(0.0, 1.0));
        fade_in * fade_out
    }
}

/// The height of the hills at `x` (0 to 1 across).
pub fn ground(x: f32) -> f32 {
    0.78 + 0.03 * (x * 7.0 + 1.0).sin() + 0.012 * (x * 19.0).sin()
}

/// Which part of a tower.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Part {
    Wall,
    /// A stone band between floors.
    Ledge,
    Window,
    Battlement,
    Leaves,
    /// The lighthouse's red bands.
    Stripe,
    /// The lighthouse's lamp, the bell in its belfry.
    Lamp,
    Bell,
    /// A pointed roof or the spire's needle.
    Roof,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Look {
    /// `k` 0 at the top of the screen to 1 at the horizon.
    Sky(f32),
    Mountain,
    /// `k` 0 at the hilltop, more further down.
    Hill(f32),
    /// Tower `k`: which part, how lit across (0 shadow to 1 lit), whether it is
    /// known (climbed or next: drawn in colour) and whether it is the next.
    Tower { k: usize, part: Part, lit: f32, known: bool, next: bool },
    Pole,
    Flag,
    Trail,
    /// The marker walking the trail.
    Walker,
}

/// Where tower `k`'s base sits, its body's half width, height, and the very top of it
/// (roof, battlements or needle included), all in screen heights.
pub fn place(k: usize, aspect: f32) -> (f32, f32, f32, f32, f32) {
    let l = &SKYLINE[k];
    let x = l.at * aspect;
    let base = ground(l.at) + 0.01;
    let extra = match l.shape {
        Shape::Keep | Shape::Ivy => 0.05,
        Shape::Bell => 0.24,
        Shape::Lighthouse => 0.2,
        Shape::Spire => 0.4,
    };
    (x, base, l.w / 2.0, l.h, base - l.h * (1.0 + extra))
}

/// The trail from tower `a`'s foot to tower `b`'s, `k` of the way, in screen heights.
pub fn trail_point(a: usize, b: usize, k: f32, aspect: f32) -> (f32, f32) {
    let (xa, xb) = (SKYLINE[a].at, SKYLINE[b].at);
    let x = xa + (xb - xa) * k;
    (x * aspect, ground(x) + 0.045 + 0.02 * (k * std::f32::consts::PI).sin())
}

/// What lies at screen point (`x`, `y`); `ps` is a pixel's size. The camera may be
/// close on a tower: the point is moved to where it lands in the landscape first.
pub fn look(p: &Progress, x: f32, y: f32, aspect: f32, ps: f32) -> Look {
    let (k, tower, up) = p.pull();
    if k < 1.0 {
        let (tx, base, _, _, top) = place(tower, aspect);
        let focus = (tx, base + (top - base) * up);
        let screen = (aspect / 2.0, 0.5);
        let z = CLOSE + (1.0 - CLOSE) * k;
        let f = (focus.0 + (screen.0 - focus.0) * k, focus.1 + (screen.1 - focus.1) * k);
        return look_at(p, f.0 + (x - screen.0) * z, f.1 + (y - screen.1) * z, aspect, ps * z);
    }
    look_at(p, x, y, aspect, ps)
}

fn look_at(p: &Progress, x: f32, y: f32, aspect: f32, ps: f32) -> Look {
    // the trail and its walker, in front of everything
    let (a, b) = (p.from, p.next);
    let drawn = p.trail();
    if p.start {
        // the climber waits at the first tower's foot
        let (hx, hy) = trail_point(p.next, p.next, 0.0, aspect);
        let bob = (p.time * 3.0).sin().abs() * 0.008;
        if ((x - hx).powi(2) + (y - hy + bob).powi(2)).sqrt() < 0.014 {
            return Look::Walker;
        }
    }
    if drawn > 0.0 {
        let (hx, hy) = trail_point(a, b, drawn, aspect);
        if ((x - hx).powi(2) + (y - hy).powi(2)).sqrt() < 0.014 {
            return Look::Walker;
        }
        let (xa, xb) = (SKYLINE[a].at * aspect, SKYLINE[b].at * aspect);
        let k = (x - xa) / (xb - xa);
        if (0.0..=drawn).contains(&k) {
            let (_, ty) = trail_point(a, b, k, aspect);
            if (y - ty).abs() < ps * 0.8 && ((x - xa).abs() / 0.022) as i32 % 2 == 0 {
                return Look::Trail;
            }
        }
    }
    for k in 0..SKYLINE.len() {
        if let Some(look) = tower(p, k, x, y, aspect, ps) {
            return look;
        }
    }
    let g = ground(x / aspect);
    if y >= g {
        return Look::Hill(y - g);
    }
    let fx = x / aspect;
    if y > 0.62 + 0.06 * (fx * 9.0).sin().abs() * (fx * 3.0 + 0.5).cos() + 0.02 * (fx * 31.0).sin() {
        return Look::Mountain;
    }
    Look::Sky((y / g).clamp(0.0, 1.0))
}

fn tower(p: &Progress, k: usize, x: f32, y: f32, aspect: f32, ps: f32) -> Option<Look> {
    let l = &SKYLINE[k];
    let (cx, base, half, h, top) = place(k, aspect);
    let climbed = p.climbed(k);
    let next = k == p.next;
    // a flag on each tower climbed, waving
    if climbed {
        let pole_top = top - 0.09;
        if (x - cx).abs() < ps * 0.6 && y >= pole_top && y < top {
            return Some(Look::Pole);
        }
        let fx = (x - cx) / 0.05;
        let wave = (fx * 5.0 - p.time * 6.0).sin() * 0.006 * fx;
        if (0.0..1.0).contains(&fx) && y >= pole_top + wave && y < pole_top + 0.032 + wave {
            return Some(Look::Flag);
        }
    }
    if x < cx - half * 1.3 || x > cx + half * 1.3 || y < top - ps || y > base {
        return None;
    }
    let ly = (base - y) / h;
    let known = climbed || next;
    let taper = match l.shape {
        Shape::Lighthouse => 1.0 - ly.min(1.0) * 0.35,
        Shape::Spire => 1.0 - ly.min(1.0) * 0.5,
        _ => 1.0,
    };
    let lx = (x - cx) / (half * taper);
    let lit = (1.0 - (lx + 0.5).powi(2) * 0.35).clamp(0.2, 1.0);
    let mk = |part: Part| Some(Look::Tower { k, part, lit, known, next });
    if ly > 1.0 {
        let up = ly - 1.0;
        return match l.shape {
            Shape::Keep | Shape::Ivy => (up < 0.05 && lx.abs() <= 1.0 && ((lx + 1.0) * 3.5) as i32 % 2 == 0).then(|| mk(Part::Battlement)).flatten(),
            Shape::Bell => (lx.abs() < 1.05 * (1.0 - up / 0.24)).then(|| mk(Part::Roof)).flatten(),
            Shape::Lighthouse if up < 0.1 => (lx.abs() < 0.75).then(|| mk(Part::Lamp)).flatten(),
            Shape::Lighthouse => (lx.abs() < 0.9 * (1.0 - (up - 0.1) / 0.1)).then(|| mk(Part::Roof)).flatten(),
            // in the ending, the Lamplighter's light at the needle's tip
            Shape::Spire if p.ending() && p.t > ENDING_LIGHT && up > 0.29 && up < 0.4 => (lx.abs() < 0.9).then(|| mk(Part::Lamp)).flatten(),
            Shape::Spire => (lx.abs() < 0.5 * (1.0 - up / 0.4)).then(|| mk(Part::Roof)).flatten(),
        };
    }
    if lx.abs() > 1.0 {
        return None;
    }
    if !known {
        return mk(Part::Wall);
    }
    let floors = 6.0;
    let band = (ly * floors).fract();
    match l.shape {
        Shape::Bell if ly > 0.78 && ly < 0.95 && lx.abs() < 0.6 => {
            // the open belfry, and the bell hanging in it
            let bl = (ly - 0.8) / 0.12;
            return if lx.abs() < 0.12 + (1.0 - bl).max(0.0) * 0.3 && bl > 0.0 && bl < 1.0 { mk(Part::Bell) } else { mk(Part::Window) };
        }
        Shape::Lighthouse => {
            return if (ly * 5.0) as i32 % 2 == 1 { mk(Part::Stripe) } else { mk(Part::Wall) };
        }
        _ => {}
    }
    if band < 0.08 {
        return mk(Part::Ledge);
    }
    if lx.abs() < 0.16 && (0.35..0.7).contains(&band) {
        return mk(Part::Window);
    }
    if l.shape == Shape::Ivy {
        let leaf = ((x * 410.0).sin() * (y * 530.0 + x * 90.0).cos() + (ly * 9.0 + lx).sin() * 0.6) > 0.45;
        if leaf {
            return mk(Part::Leaves);
        }
    }
    mk(Part::Wall)
}

/// Where the map stands after floor `n` (the roof just beaten).
pub fn progress_after(n: usize, t: f32, time: f32, leave: Option<f32>) -> Progress {
    let from = levels::tower_of(n).0;
    let next = levels::tower_of(n + 1).0;
    Progress { from, next, t, time, start: false, leave }
}

/// The map at the start of a game: nothing climbed, the first tower next.
pub fn opening(t: f32, time: f32, leave: Option<f32>) -> Progress {
    Progress { from: 0, next: 0, t, time, start: true, leave }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn towers_stand_on_the_hills_and_the_trail_joins_them() {
        let aspect = 2.5;
        let p = Progress { from: 0, next: 1, t: 10.0, time: 0.0, start: false, leave: None };
        for k in 0..SKYLINE.len() {
            let (x, base, _, h, top) = place(k, aspect);
            assert!(top > 0.02, "tower {k} runs off the top");
            assert!(matches!(look(&p, x, base - h * 0.5, aspect, 0.004), Look::Tower { .. } | Look::Trail | Look::Walker), "no tower {k}");
        }
        assert!(p.climbed(0) && !p.climbed(1) && !p.climbed(4));
        // the walker ends at the next tower's foot
        let (x, y) = trail_point(0, 1, 1.0, aspect);
        assert_eq!(look(&p, x, y, aspect, 0.004), Look::Walker);
        // at first the camera is close on the climbed tower's top: its middle is that
        let close = Progress { t: 0.0, ..p };
        assert!(matches!(look(&close, aspect / 2.0, 0.5, aspect, 0.004), Look::Tower { k: 0, .. } | Look::Pole | Look::Flag));
        assert!(!close.settled() && p.settled());
        // a new game: nothing climbed, the walker at the first tower
        let start = opening(1.0, 0.0, None);
        assert!(!start.climbed(0) && start.ready());
        let (x, y) = trail_point(0, 0, 0.0, aspect);
        assert_eq!(look(&start, x, y, aspect, 0.004), Look::Walker);
    }
}
