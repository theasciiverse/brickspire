//! Vectors, the paths the paddle travels along, and swept collisions for the ball.
//!
//! World units are half-block pixels at the catalog's 100x30 size; x right, y down.
//! The ball is a circle swept along its motion each step, so it can't tunnel through
//! a thin brick however fast it goes. Everything it can hit is made of capsules
//! (a segment with a radius: the ball's own radius, plus the paddle's thickness for
//! the paddle) and planes (the walls).

use std::f32::consts::{FRAC_PI_2, TAU};
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const fn new(x: f32, y: f32) -> V2 {
        V2 { x, y }
    }
    pub fn dot(self, o: V2) -> f32 {
        self.x * o.x + self.y * o.y
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l > 0.0 { self * (1.0 / l) } else { self }
    }
    /// Rotated by `a` radians (clockwise on screen, since y points down).
    pub fn rotate(self, a: f32) -> V2 {
        let (s, c) = a.sin_cos();
        V2::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
    /// Mirrored off a surface with unit normal `n`.
    pub fn reflect(self, n: V2) -> V2 {
        self - n * (2.0 * self.dot(n))
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2::new(self.x + o.x, self.y + o.y)
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, o: V2) {
        *self = *self + o;
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2::new(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, k: f32) -> V2 {
        V2::new(self.x * k, self.y * k)
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        V2::new(-self.x, -self.y)
    }
}

/// The line the paddle's centre travels along. The position on it is an arc length
/// `s`, and increasing `s` always moves the paddle rightwards where you usually meet
/// the ball (along the bottom), so the right key is +s everywhere.
#[derive(Clone, Copy, Debug)]
pub enum Path {
    /// Ordinary floors: a horizontal line, as in Arkanoid.
    Line { y: f32, x0: f32, x1: f32 },
    /// Mini-boss floors: an arc of a circle round centre `c`, between screen angles
    /// `a0 < a1` (radians, y down, so the bottom of the circle is +90 degrees).
    Arc { c: V2, r: f32, a0: f32, a1: f32 },
    /// The roof: right round a circle.
    Circle { c: V2, r: f32 },
}

impl Path {
    pub fn len(&self) -> f32 {
        match *self {
            Path::Line { x0, x1, .. } => x1 - x0,
            Path::Arc { r, a0, a1, .. } => r * (a1 - a0),
            Path::Circle { r, .. } => r * TAU,
        }
    }

    pub fn wraps(&self) -> bool {
        matches!(self, Path::Circle { .. })
    }

    /// The screen angle at `s` on an arc or circle.
    fn angle(&self, s: f32) -> f32 {
        match *self {
            Path::Line { .. } => 0.0,
            Path::Arc { r, a1, .. } => a1 - s / r,
            Path::Circle { r, .. } => FRAC_PI_2 - s / r,
        }
    }

    pub fn point(&self, s: f32) -> V2 {
        match *self {
            Path::Line { y, x0, .. } => V2::new(x0 + s, y),
            Path::Arc { c, r, .. } | Path::Circle { c, r } => {
                let a = self.angle(s);
                c + V2::new(a.cos(), a.sin()) * r
            }
        }
    }

    /// Unit direction of increasing `s`: the way the paddle lies.
    pub fn tangent(&self, s: f32) -> V2 {
        match *self {
            Path::Line { .. } => V2::new(1.0, 0.0),
            _ => {
                let a = self.angle(s);
                V2::new(a.sin(), -a.cos())
            }
        }
    }

    /// Unit normal pointing into the play area: up from the line, in towards the
    /// centre from an arc or circle.
    pub fn normal(&self, s: f32) -> V2 {
        match *self {
            Path::Line { .. } => V2::new(0.0, -1.0),
            _ => {
                let a = self.angle(s);
                V2::new(-a.cos(), -a.sin())
            }
        }
    }

    /// The `s` of the point on the path nearest `p`.
    pub fn nearest(&self, p: V2) -> f32 {
        match *self {
            Path::Line { x0, x1, .. } => p.x.clamp(x0, x1) - x0,
            Path::Arc { c, r, a0, a1 } => {
                let d = p - c;
                let a = d.y.atan2(d.x);
                let gap = |to: f32| (a - to + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
                let a = if (a0..=a1).contains(&a) {
                    a
                } else if gap(a0).abs() < gap(a1).abs() {
                    a0
                } else {
                    a1
                };
                (a1 - a) * r
            }
            Path::Circle { c, r } => {
                let d = p - c;
                ((FRAC_PI_2 - d.y.atan2(d.x)) * r).rem_euclid(r * TAU)
            }
        }
    }

    /// Signed distance from `from` to `to` along the path: on a circle, the short
    /// way round.
    pub fn delta(&self, from: f32, to: f32) -> f32 {
        let d = to - from;
        if self.wraps() {
            let l = self.len();
            (d + l / 2.0).rem_euclid(l) - l / 2.0
        } else {
            d
        }
    }

    /// `s` kept on the path, with `margin` to spare at each end of an open path.
    pub fn keep(&self, s: f32, margin: f32) -> f32 {
        if self.wraps() { s.rem_euclid(self.len()) } else { s.clamp(margin, (self.len() - margin).max(margin)) }
    }
}

/// Where a sweep first touches something: the fraction `t` of the motion done, and
/// the surface normal there (pointing back out at the ball).
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub t: f32,
    pub n: V2,
}

const EPS: f32 = 1e-4;

fn earlier(a: Option<Hit>, b: Option<Hit>) -> Option<Hit> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if y.t < x.t { y } else { x }),
        (x, None) => x,
        (None, y) => y,
    }
}

/// A point moving from `p` by `d` against a circle of radius `r` round `c`, from outside.
fn sweep_circle(p: V2, d: V2, c: V2, r: f32) -> Option<Hit> {
    let m = p - c;
    let b = m.dot(d);
    if b >= 0.0 {
        return None; // moving away
    }
    let cc = m.dot(m) - r * r;
    if cc < -EPS {
        return None; // already inside: overlap is resolved elsewhere
    }
    let a = d.dot(d);
    let disc = b * b - a * cc;
    if disc < 0.0 || a == 0.0 {
        return None;
    }
    let t = ((-b - disc.sqrt()) / a).max(0.0);
    (t <= 1.0).then(|| Hit { t, n: (p + d * t - c).norm() })
}

/// A point moving from `p` by `d` against the capsule round segment `a`-`b` with
/// radius `r`, from outside. The ball's centre against the segment grown by its radius.
pub fn sweep_capsule(p: V2, d: V2, a: V2, b: V2, r: f32) -> Option<Hit> {
    let ab = b - a;
    let l2 = ab.dot(ab);
    let mut best = None;
    if l2 > 0.0 {
        let n = V2::new(-ab.y, ab.x).norm();
        for side in [n, -n] {
            let rate = d.dot(side);
            if rate >= 0.0 {
                continue;
            }
            let d0 = (p - a).dot(side) - r;
            if d0 < -EPS {
                continue;
            }
            let t = (d0 / -rate).max(0.0);
            if t > 1.0 {
                continue;
            }
            let u = (p + d * t - a).dot(ab) / l2;
            if (0.0..=1.0).contains(&u) {
                best = earlier(best, Some(Hit { t, n: side }));
            }
        }
    }
    best = earlier(best, sweep_circle(p, d, a, r));
    earlier(best, sweep_circle(p, d, b, r))
}

/// A box from `min` to `max` (a brick), as its four edges.
pub fn sweep_box(p: V2, d: V2, min: V2, max: V2, r: f32) -> Option<Hit> {
    // broad phase: the swept ball's bounds against the grown box
    let (lo, hi) = (V2::new(p.x.min(p.x + d.x), p.y.min(p.y + d.y)), V2::new(p.x.max(p.x + d.x), p.y.max(p.y + d.y)));
    if hi.x < min.x - r || lo.x > max.x + r || hi.y < min.y - r || lo.y > max.y + r {
        return None;
    }
    let (tr, bl) = (V2::new(max.x, min.y), V2::new(min.x, max.y));
    let mut best = None;
    for (a, b) in [(min, tr), (tr, max), (max, bl), (bl, min)] {
        best = earlier(best, sweep_capsule(p, d, a, b, r));
    }
    best
}

/// A wall: the plane through `q` with unit normal `n` facing the play area.
pub fn sweep_plane(p: V2, d: V2, q: V2, n: V2, r: f32) -> Option<Hit> {
    let rate = d.dot(n);
    if rate >= 0.0 {
        return None;
    }
    let d0 = (p - q).dot(n) - r;
    let t = (d0 / -rate).max(0.0);
    (t <= 1.0).then_some(Hit { t, n })
}

/// Nearest point to `p` on segment `a`-`b`.
pub fn closest_on_segment(p: V2, a: V2, b: V2) -> V2 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 == 0.0 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
}

/// If a ball of radius `r` at `p` overlaps the box, how far and which way to push it out.
pub fn box_overlap(p: V2, min: V2, max: V2, r: f32) -> Option<(f32, V2)> {
    let q = V2::new(p.x.clamp(min.x, max.x), p.y.clamp(min.y, max.y));
    let off = p - q;
    let dist = off.len();
    if dist >= r {
        return None;
    }
    if dist > 0.0 {
        return Some((r - dist, off * (1.0 / dist)));
    }
    // centre inside the box: out through the nearest side
    let sides = [
        (p.x - min.x, V2::new(-1.0, 0.0)),
        (max.x - p.x, V2::new(1.0, 0.0)),
        (p.y - min.y, V2::new(0.0, -1.0)),
        (max.y - p.y, V2::new(0.0, 1.0)),
    ];
    let (depth, n) = sides.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
    Some((depth + r, n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_4;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn capsule_face_and_cap() {
        // straight down onto the top face
        let h = sweep_capsule(V2::new(5.0, 0.0), V2::new(0.0, 10.0), V2::new(0.0, 5.0), V2::new(10.0, 5.0), 1.0).unwrap();
        assert!(close(h.t, 0.4) && close(h.n.y, -1.0));
        // past the end, onto the rounded cap
        let h = sweep_capsule(V2::new(10.5, 0.0), V2::new(0.0, 10.0), V2::new(0.0, 5.0), V2::new(10.0, 5.0), 1.0).unwrap();
        assert!(h.n.x > 0.0 && h.n.y < 0.0);
        // clear miss
        assert!(sweep_capsule(V2::new(12.0, 0.0), V2::new(0.0, 10.0), V2::new(0.0, 5.0), V2::new(10.0, 5.0), 1.0).is_none());
        // moving away
        assert!(sweep_capsule(V2::new(5.0, 0.0), V2::new(0.0, -10.0), V2::new(0.0, 5.0), V2::new(10.0, 5.0), 1.0).is_none());
    }

    #[test]
    fn box_side_by_direction() {
        let (min, max) = (V2::new(0.0, 0.0), V2::new(6.0, 3.0));
        let h = sweep_box(V2::new(-5.0, 1.5), V2::new(10.0, 0.0), min, max, 1.0).unwrap();
        assert!(close(h.n.x, -1.0) && close(h.t, 0.4));
        let h = sweep_box(V2::new(3.0, 10.0), V2::new(0.0, -10.0), min, max, 1.0).unwrap();
        assert!(close(h.n.y, 1.0));
    }

    #[test]
    fn paths_are_consistent() {
        let paths = [
            Path::Line { y: 50.0, x0: 10.0, x1: 90.0 },
            Path::Arc { c: V2::new(50.0, 0.0), r: 55.0, a0: FRAC_PI_4, a1: 3.0 * FRAC_PI_4 },
            Path::Circle { c: V2::new(50.0, 30.0), r: 25.0 },
        ];
        for p in paths {
            let l = p.len();
            for i in 0..=20 {
                let s = l * i as f32 / 20.0 * 0.999;
                let (t, n) = (p.tangent(s), p.normal(s));
                assert!(close(t.len(), 1.0) && close(n.len(), 1.0) && close(t.dot(n), 0.0));
                // the tangent really is the direction of increasing s
                let step = (p.point(s + 0.01) - p.point(s)).norm();
                assert!(step.dot(t) > 0.99, "{p:?} at {s}");
                assert!((p.nearest(p.point(s)) - s).abs() < 1e-2 || p.wraps() && (p.nearest(p.point(s)) - s).abs() > l - 1e-2);
            }
        }
    }

    #[test]
    fn right_is_right_at_the_bottom() {
        let arc = Path::Arc { c: V2::new(50.0, 0.0), r: 55.0, a0: FRAC_PI_4, a1: 3.0 * FRAC_PI_4 };
        let mid = arc.len() / 2.0;
        assert!(close(arc.tangent(mid).x, 1.0) && close(arc.normal(mid).y, -1.0));
        let circle = Path::Circle { c: V2::new(50.0, 30.0), r: 25.0 };
        assert!(close(circle.point(0.0).y, 55.0) && close(circle.tangent(0.0).x, 1.0));
        assert!(close(circle.delta(1.0, circle.len() - 1.0), -2.0));
    }
}
