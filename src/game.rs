//! The rules: one floor of bricks, the paddle on its path, the ball, lives and score.
//!
//! A fixed step of `STEP` seconds. The paddle moves first, then each ball is swept
//! along its motion, bouncing off whatever it touches first, until the step is used up.

use crate::levels::{self, Floor};
use crate::physics::{self, Hit, Path, V2};
use log::info;
use std::f32::consts::PI;

pub const STEP: f32 = 1.0 / 120.0;

/// The world: half-block pixels at the catalog's 100x30, less a HUD row and a help row.
pub const WORLD_W: f32 = 100.0;
pub const WORLD_H: f32 = 56.0;
/// The play field inside the walls.
pub const FIELD_X0: f32 = 11.0;
pub const FIELD_X1: f32 = 89.0;
pub const FIELD_TOP: f32 = 2.0;
pub const WALL: f32 = 2.0;
pub const BRICK_W: f32 = 6.0;
pub const BRICK_H: f32 = 3.0;
const BRICKS_TOP: f32 = 8.0;
/// The stone ledge along the bottom of every face of the tower.
pub const LEDGE: f32 = 54.0;
/// After a floor is cleared: a moment to see it, then the tower turns to the next.
pub const CLEAR_HOLD: f32 = 0.6;
pub const TURN_TIME: f32 = 1.8;
pub const PADDLE_Y: f32 = 52.0;
/// Half the paddle's length, and half its thickness.
pub const PADDLE_HALF: f32 = 7.0;
pub const PADDLE_TH: f32 = 1.2;
pub const BALL_R: f32 = 1.2;

/// Paddle speed along its path under keys, and the most it moves chasing the pointer.
const PADDLE_SPEED: f32 = 95.0;
const PADDLE_CHASE: f32 = 360.0;
const SPEED_START: f32 = 48.0;
const SPEED_MAX: f32 = 92.0;
const SPEED_PER_HIT: f32 = 0.6;
/// How far from the paddle's normal the ball leaves at the very end of the paddle.
const MAX_DEFLECT: f32 = PI / 3.0;
/// No velocity component smaller than this share of the speed (straight floors only),
/// so the ball never runs almost flat or almost vertical forever.
const MIN_SHARE: f32 = 0.18;
const AUTO_LAUNCH: f32 = 5.0;
const LIVES: u32 = 3;
const GRAVITY: f32 = 60.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    /// Ordinary floors: the paddle on a straight line.
    Line,
    /// Mini-boss floors: the paddle on a shallow arc.
    Arc,
    /// The roof: the paddle right round a ring, no walls.
    Circle,
}

pub struct Arena {
    pub mode: Mode,
    pub path: Path,
}

impl Arena {
    pub fn new(mode: Mode) -> Arena {
        let path = match mode {
            Mode::Line => Path::Line { y: PADDLE_Y, x0: FIELD_X0, x1: FIELD_X1 },
            Mode::Arc => {
                // a quarter circle whose ends meet the walls; the sweep is for playtesting
                let sweep = PI / 2.0;
                let half_w = (FIELD_X1 - FIELD_X0) / 2.0;
                let r = half_w / (sweep / 2.0).sin();
                let bottom = PADDLE_Y + 1.0;
                Path::Arc { c: V2::new(WORLD_W / 2.0, bottom - r), r, a0: PI / 2.0 - sweep / 2.0, a1: PI / 2.0 + sweep / 2.0 }
            }
            Mode::Circle => Path::Circle { c: V2::new(WORLD_W / 2.0, 29.0), r: 25.0 },
        };
        Arena { mode, path }
    }

    pub fn walls(&self) -> bool {
        self.mode != Mode::Circle
    }

    /// Past the paddle for good: off the bottom, or out of the ring on the roof.
    pub fn lost(&self, p: V2) -> bool {
        match self.path {
            Path::Circle { c, r } => (p - c).len() > r + 4.0,
            _ => p.y > WORLD_H + 2.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// An ordinary brick in one of eight colours.
    Plain(u8),
    /// Steel: several hits.
    Steel,
    /// Gilt: can't be broken.
    Gilt,
}

#[derive(Clone, Copy, Debug)]
pub struct Brick {
    pub kind: Kind,
    pub hp: u8,
    /// Seconds of white flash left after a hit that didn't break it.
    pub flash: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Ball {
    pub p: V2,
    pub v: V2,
}

#[derive(Clone, Copy, Debug)]
pub struct Paddle {
    /// Position along the arena's path.
    pub s: f32,
    pub vel: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    /// The ball sits on the paddle until you launch it.
    Serve { wait: f32 },
    Play,
    /// The last ball is gone; a moment before the next serve.
    Lost { t: f32 },
    /// Every breakable brick is gone: `t` seconds since, holding for `CLEAR_HOLD`
    /// and then turning the tower for `TURN_TIME`.
    Clear { t: f32 },
    Over,
}

/// What the player wants the paddle to do this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Steer {
    /// Keys or d-pad: -1 to 1 along the path (+1 is right at the bottom).
    Move(f32),
    /// Mouse: chase the nearest point of the path to this world point.
    Toward(V2),
    /// Stick on the roof: go to where the stick points round the ring.
    Aim(V2),
}

pub struct Particle {
    pub p: V2,
    pub v: V2,
    pub life: f32,
    pub rgb: [u8; 3],
}

pub struct Game {
    pub arena: Arena,
    pub floor: usize,
    pub floor_name: &'static str,
    pub origin: V2,
    pub cols: usize,
    pub rows: usize,
    pub bricks: Vec<Option<Brick>>,
    pub balls: Vec<Ball>,
    pub paddle: Paddle,
    pub state: State,
    pub lives: u32,
    pub score: u32,
    pub speed: f32,
    pub time: f32,
    pub particles: Vec<Particle>,
    seed: u32,
}

/// What a ball touched.
#[derive(Clone, Copy, Debug)]
enum Thing {
    Wall,
    Brick(usize),
    Paddle,
}

pub fn brick_rgb(kind: Kind) -> [u8; 3] {
    match kind {
        Kind::Plain(c) => [
            [236, 236, 228],
            [255, 140, 40],
            [60, 210, 230],
            [90, 210, 80],
            [230, 60, 60],
            [70, 110, 240],
            [220, 80, 200],
            [245, 215, 60],
        ][c as usize % 8],
        Kind::Steel => [150, 160, 175],
        Kind::Gilt => [240, 185, 45],
    }
}

/// A floor's bricks as a grid, `cols` wide.
pub fn parse(def: &Floor) -> (usize, usize, Vec<Option<Kind>>) {
    let cols = def.rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let cells = def
        .rows
        .iter()
        .flat_map(|row| {
            let mut cells: Vec<Option<Kind>> = row
                .chars()
                .map(|ch| match ch {
                    'S' => Some(Kind::Steel),
                    'G' => Some(Kind::Gilt),
                    _ => Some(Kind::Plain("wocgrbmy".find(ch)? as u8)),
                })
                .collect();
            cells.resize(cols, None);
            cells
        })
        .collect();
    (cols, def.rows.len(), cells)
}

/// Where a straight floor's brick grid starts: centred, below the lintel.
pub fn line_origin(cols: usize) -> V2 {
    V2::new(WORLD_W / 2.0 - cols as f32 * BRICK_W / 2.0, BRICKS_TOP)
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Game {
    pub fn new(mode: Mode) -> Game {
        let mut g = Game {
            arena: Arena::new(mode),
            floor: 0,
            floor_name: "",
            origin: V2::default(),
            cols: 0,
            rows: 0,
            bricks: Vec::new(),
            balls: Vec::new(),
            paddle: Paddle { s: 0.0, vel: 0.0 },
            state: State::Serve { wait: 0.0 },
            lives: LIVES,
            score: 0,
            speed: SPEED_START,
            time: 0.0,
            particles: Vec::new(),
            seed: 0x9e37_79b9,
        };
        g.load_floor(0);
        g
    }

    fn floor_def(&self, n: usize) -> &'static Floor {
        if self.arena.mode == Mode::Circle { &levels::ROOF_TEST } else { &levels::FLOORS[n % levels::FLOORS.len()] }
    }

    pub fn load_floor(&mut self, n: usize) {
        let def = self.floor_def(n);
        self.floor = n;
        self.floor_name = def.name;
        let (cols, rows, cells) = parse(def);
        (self.cols, self.rows) = (cols, rows);
        let (w, h) = (self.cols as f32 * BRICK_W, self.rows as f32 * BRICK_H);
        self.origin = match self.arena.path {
            Path::Circle { c, .. } => c - V2::new(w / 2.0, h / 2.0),
            _ => line_origin(cols),
        };
        let steel_hp = steel_hp(n);
        self.bricks = cells
            .into_iter()
            .map(|k| k.map(|kind| Brick { kind, hp: if kind == Kind::Steel { steel_hp } else { 1 }, flash: 0.0 }))
            .collect();
        self.particles.clear();
        self.serve();
        info!("floor {} start: {} ({} bricks, {:?})", n + 1, def.name, self.left(), self.arena.mode);
    }

    fn serve(&mut self) {
        self.balls.clear();
        self.speed = SPEED_START;
        if self.arena.path.wraps() {
            self.paddle.s = 0.0;
        } else {
            self.paddle.s = self.arena.path.len() / 2.0;
        }
        self.paddle.vel = 0.0;
        self.state = State::Serve { wait: 0.0 };
    }

    /// Breakable bricks still standing.
    pub fn left(&self) -> usize {
        self.bricks.iter().flatten().filter(|b| b.kind != Kind::Gilt).count()
    }

    pub fn brick_box(&self, i: usize) -> (V2, V2) {
        let (c, r) = (i % self.cols, i / self.cols);
        let min = self.origin + V2::new(c as f32 * BRICK_W, r as f32 * BRICK_H);
        (min, min + V2::new(BRICK_W, BRICK_H))
    }

    /// The brick under world point `p`, if any.
    pub fn brick_at(&self, p: V2) -> Option<(usize, &Brick)> {
        let q = p - self.origin;
        if q.x < 0.0 || q.y < 0.0 {
            return None;
        }
        let (c, r) = ((q.x / BRICK_W) as usize, (q.y / BRICK_H) as usize);
        if c >= self.cols || r >= self.rows {
            return None;
        }
        let i = r * self.cols + c;
        self.bricks[i].as_ref().map(|b| (i, b))
    }

    /// The paddle as a segment, and the path's normal at its centre.
    pub fn paddle_segment(&self) -> (V2, V2, V2) {
        let path = &self.arena.path;
        let (c, t) = (path.point(self.paddle.s), path.tangent(self.paddle.s));
        (c - t * PADDLE_HALF, c + t * PADDLE_HALF, path.normal(self.paddle.s))
    }

    /// Where a served ball sits: on the paddle's face, a little right of centre.
    fn held_ball(&self) -> V2 {
        let path = &self.arena.path;
        path.point(self.paddle.s) + path.tangent(self.paddle.s) * 2.0 + path.normal(self.paddle.s) * (PADDLE_TH + BALL_R + 0.05)
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn step(&mut self, steer: Steer, fire: bool) {
        self.time += STEP;
        for b in self.bricks.iter_mut().flatten() {
            b.flash = (b.flash - STEP).max(0.0);
        }
        self.particles.retain_mut(|p| {
            p.v.y += GRAVITY * STEP;
            p.p += p.v * STEP;
            p.life -= STEP;
            p.life > 0.0
        });
        match self.state {
            State::Over => return,
            State::Lost { t } => {
                let t = t - STEP;
                if t <= 0.0 {
                    if self.lives == 0 {
                        info!("game over: score {}", self.score);
                        self.state = State::Over;
                    } else {
                        self.serve();
                    }
                } else {
                    self.state = State::Lost { t };
                }
                return;
            }
            State::Clear { t } => {
                let t = t + STEP;
                // the paddle glides back to the middle while the tower turns
                let mid = self.arena.path.len() / 2.0;
                self.paddle.s += (mid - self.paddle.s) * (STEP * 3.0).min(1.0);
                if t >= CLEAR_HOLD + TURN_TIME {
                    info!("tower turned to floor {}", self.floor + 2);
                    self.load_floor(self.floor + 1);
                } else {
                    if t > CLEAR_HOLD && t - STEP <= CLEAR_HOLD {
                        info!("tower turning");
                    }
                    self.state = State::Clear { t };
                }
                return;
            }
            _ => {}
        }
        self.move_paddle(steer);
        if let State::Serve { wait } = self.state {
            let wait = wait + STEP;
            if fire || wait >= AUTO_LAUNCH {
                self.launch();
            } else {
                self.state = State::Serve { wait };
            }
            return;
        }
        for i in 0..self.balls.len() {
            self.move_ball(i);
        }
        let before = self.balls.len();
        self.balls.retain(|b| !self.arena.lost(b.p));
        if self.balls.len() < before {
            info!("ball lost ({} left in play)", self.balls.len());
        }
        if self.balls.is_empty() {
            self.lives = self.lives.saturating_sub(1);
            info!("life lost: {} left", self.lives);
            self.state = State::Lost { t: 1.2 };
        } else if self.left() == 0 {
            info!("floor {} clear: score {}", self.floor + 1, self.score);
            self.clear_floor(0.0);
        }
    }

    fn launch(&mut self) {
        let path = &self.arena.path;
        let dir = path.normal(self.paddle.s).rotate(0.35);
        self.balls.push(Ball { p: self.held_ball(), v: dir * self.speed });
        self.state = State::Play;
        info!("launch at s {:.1}", self.paddle.s);
    }

    fn move_paddle(&mut self, steer: Steer) {
        let path = self.arena.path;
        let chase = |to: f32| (path.delta(self.paddle.s, to) / STEP).clamp(-PADDLE_CHASE, PADDLE_CHASE);
        self.paddle.vel = match steer {
            Steer::Move(k) => k.clamp(-1.0, 1.0) * PADDLE_SPEED,
            Steer::Toward(p) => chase(path.nearest(p)),
            Steer::Aim(dir) => match path {
                Path::Circle { c, r } if dir.len() > 0.0 => chase(path.nearest(c + dir.norm() * r)),
                _ => dir.x.clamp(-1.0, 1.0) * PADDLE_SPEED,
            },
        };
        self.paddle.s = path.keep(self.paddle.s + self.paddle.vel * STEP, PADDLE_HALF);
        // a paddle moving into a ball pushes it out and bats it
        let (a, b, n) = self.paddle_segment();
        for i in 0..self.balls.len() {
            let p = self.balls[i].p;
            let q = physics::closest_on_segment(p, a, b);
            let off = p - q;
            let reach = PADDLE_TH + BALL_R;
            if off.len() < reach && off.dot(n) > 0.0 {
                self.balls[i].p = q + off.norm() * (reach + 1e-3);
                if self.balls[i].v.dot(n) < 0.0 {
                    self.bat(i);
                }
            }
        }
    }

    /// The ball leaves the paddle along the path's normal, turned towards whichever end
    /// it struck, up to `MAX_DEFLECT` at the very end (Doh It Again style).
    fn bat(&mut self, i: usize) {
        let (a, b, n) = self.paddle_segment();
        let mid = (a + b) * 0.5;
        let t = (b - a).norm();
        let p = self.balls[i].p;
        let u = ((physics::closest_on_segment(p, a, b) - mid).dot(t) / PADDLE_HALF).clamp(-1.0, 1.0);
        self.balls[i].v = n.rotate(u * MAX_DEFLECT) * self.speed;
    }

    fn move_ball(&mut self, i: usize) {
        let mut left = 1.0f32;
        for _ in 0..8 {
            if left <= 0.0 {
                break;
            }
            let ball = self.balls[i];
            let d = ball.v * (STEP * left);
            let Some((hit, thing)) = self.first_hit(ball.p, d) else {
                self.balls[i].p += d;
                break;
            };
            self.balls[i].p = ball.p + d * hit.t + hit.n * 1e-3;
            left *= 1.0 - hit.t;
            match thing {
                Thing::Wall => self.balls[i].v = ball.v.reflect(hit.n),
                Thing::Brick(b) => {
                    self.balls[i].v = ball.v.reflect(hit.n);
                    self.hit_brick(b);
                }
                Thing::Paddle => self.bat(i),
            }
            self.balls[i].v = self.tidy(self.balls[i].v);
        }
        // a ball left inside a brick (never expected; insurance) is pushed back out
        let p = self.balls[i].p;
        for b in 0..self.bricks.len() {
            if self.bricks[b].is_none() {
                continue;
            }
            let (min, max) = self.brick_box(b);
            if let Some((depth, n)) = physics::box_overlap(p, min, max, BALL_R) {
                self.balls[i].p = p + n * (depth + 1e-3);
                if self.balls[i].v.dot(n) < 0.0 {
                    self.balls[i].v = self.balls[i].v.reflect(n);
                }
            }
        }
    }

    /// The first thing a ball at `p` moving by `d` touches.
    fn first_hit(&self, p: V2, d: V2) -> Option<(Hit, Thing)> {
        let mut best: Option<(Hit, Thing)> = None;
        let mut take = |h: Option<Hit>, thing: Thing| {
            if let Some(h) = h {
                if best.is_none_or(|(b, _)| h.t < b.t) {
                    best = Some((h, thing));
                }
            }
        };
        if self.arena.walls() {
            take(physics::sweep_plane(p, d, V2::new(FIELD_X0, 0.0), V2::new(1.0, 0.0), BALL_R), Thing::Wall);
            take(physics::sweep_plane(p, d, V2::new(FIELD_X1, 0.0), V2::new(-1.0, 0.0), BALL_R), Thing::Wall);
            take(physics::sweep_plane(p, d, V2::new(0.0, FIELD_TOP), V2::new(0.0, 1.0), BALL_R), Thing::Wall);
        }
        // only the grid cells the swept ball can reach
        let lo = V2::new(p.x.min(p.x + d.x), p.y.min(p.y + d.y)) - self.origin;
        let hi = V2::new(p.x.max(p.x + d.x), p.y.max(p.y + d.y)) - self.origin;
        let cell = |v: f32, size: f32, n: usize| ((v / size).floor().max(0.0) as usize).min(n);
        let (c0, c1) = (cell(lo.x - BALL_R, BRICK_W, self.cols), cell(hi.x + BALL_R, BRICK_W, self.cols.saturating_sub(1)));
        let (r0, r1) = (cell(lo.y - BALL_R, BRICK_H, self.rows), cell(hi.y + BALL_R, BRICK_H, self.rows.saturating_sub(1)));
        if hi.x + BALL_R >= 0.0 && hi.y + BALL_R >= 0.0 {
            for r in r0..=r1.min(self.rows.saturating_sub(1)) {
                for c in c0..=c1.min(self.cols.saturating_sub(1)) {
                    let b = r * self.cols + c;
                    if self.bricks[b].is_some() {
                        let (min, max) = self.brick_box(b);
                        take(physics::sweep_box(p, d, min, max, BALL_R), Thing::Brick(b));
                    }
                }
            }
        }
        let (a, b, n) = self.paddle_segment();
        let h = physics::sweep_capsule(p, d, a, b, PADDLE_TH + BALL_R).filter(|h| h.n.dot(n) > -0.3);
        take(h, Thing::Paddle);
        best
    }

    /// Keep the speed, and on straight floors keep both components at `MIN_SHARE`.
    fn tidy(&self, v: V2) -> V2 {
        let v = v.norm() * self.speed;
        if self.arena.mode == Mode::Circle {
            return v;
        }
        let min = MIN_SHARE * self.speed;
        let sign = |x: f32| if x < 0.0 { -1.0 } else { 1.0 };
        let other = |m: f32| (self.speed * self.speed - m * m).sqrt();
        if v.x.abs() < min {
            V2::new(sign(v.x) * min, sign(v.y) * other(min))
        } else if v.y.abs() < min {
            V2::new(sign(v.x) * other(min), sign(v.y) * min)
        } else {
            v
        }
    }

    fn hit_brick(&mut self, i: usize) {
        let Some(mut brick) = self.bricks[i] else { return };
        self.speed = (self.speed + SPEED_PER_HIT).min(SPEED_MAX);
        if brick.kind == Kind::Gilt {
            brick.flash = 0.12;
            self.bricks[i] = Some(brick);
            return;
        }
        brick.hp = brick.hp.saturating_sub(1);
        if brick.hp > 0 {
            brick.flash = 0.15;
            self.bricks[i] = Some(brick);
            return;
        }
        self.bricks[i] = None;
        self.score += match brick.kind {
            Kind::Plain(c) => 50 + 10 * c as u32,
            _ => 50 * (self.floor as u32 + 1),
        };
        let (min, max) = self.brick_box(i);
        let rgb = brick_rgb(brick.kind);
        for _ in 0..7 {
            let p = V2::new(min.x + self.rand() * (max.x - min.x), min.y + self.rand() * (max.y - min.y));
            let a = self.rand() * 2.0 * PI;
            let v = V2::new(a.cos(), a.sin()) * (8.0 + self.rand() * 18.0);
            let life = 0.4 + self.rand() * 0.5;
            self.particles.push(Particle { p, v, life, rgb });
        }
    }

    /// Skip to the next floor (debug key).
    pub fn skip_floor(&mut self) {
        if matches!(self.state, State::Clear { .. } | State::Over) {
            return;
        }
        info!("skip floor {}", self.floor + 1);
        // as if cleared: only the gilt stays, so the turn shows the floor as it will be
        for b in self.bricks.iter_mut() {
            if b.is_some_and(|b| b.kind != Kind::Gilt) {
                *b = None;
            }
        }
        if self.arena.mode == Mode::Line {
            self.clear_floor(CLEAR_HOLD);
        } else {
            self.load_floor(self.floor + 1);
        }
    }

    /// The floor is done: the balls go, and the tower turns (straight floors) or the
    /// next floor simply loads (the arc and roof previews).
    fn clear_floor(&mut self, t: f32) {
        self.balls.clear();
        self.state = State::Clear { t };
    }

    /// Where the camera faces, in floors: the current floor, plus how far through
    /// the turn to the next one.
    pub fn camera(&self) -> f32 {
        let turn = match self.state {
            State::Clear { t } if self.arena.mode == Mode::Line => smoothstep((t - CLEAR_HOLD) / TURN_TIME),
            _ => 0.0,
        };
        self.floor as f32 + turn
    }

    /// A served ball is drawn on the paddle before it exists.
    pub fn shown_balls(&self) -> Vec<V2> {
        match self.state {
            State::Serve { .. } => vec![self.held_ball()],
            _ => self.balls.iter().map(|b| b.p).collect(),
        }
    }
}

/// Hits a steel brick takes on floor `n` (0-based).
pub fn steel_hp(n: usize) -> u8 {
    2 + (n / 8) as u8
}

/// Autoplay for snapshots and tests: keep the paddle under the ball, hitting it
/// off-centre by a slowly wandering amount so it doesn't settle into one loop.
pub struct Pilot {
    t: f32,
}

impl Pilot {
    pub fn new() -> Pilot {
        Pilot { t: 0.0 }
    }

    pub fn steer(&mut self, game: &Game) -> Steer {
        self.t += STEP;
        let Some(ball) = game.balls.first() else { return Steer::Move(0.0) };
        let (a, b, _) = game.paddle_segment();
        let t = (b - a).norm();
        Steer::Toward(ball.p - t * ((self.t * 0.7).sin() * PADDLE_HALF * 0.6))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(game: &mut Game, secs: f32) {
        let mut pilot = Pilot::new();
        for _ in 0..(secs / STEP) as usize {
            let s = pilot.steer(game);
            game.step(s, true);
            for b in &game.balls {
                if game.arena.walls() {
                    assert!(b.p.x >= FIELD_X0 && b.p.x <= FIELD_X1 && b.p.y >= FIELD_TOP, "ball escaped the field at {:?}", b.p);
                }
                assert!(game.brick_at(b.p).is_none(), "ball inside a brick at {:?}", b.p);
            }
        }
    }

    #[test]
    fn no_tunnelling_through_a_column() {
        let mut g = Game::new(Mode::Line);
        g.state = State::Play;
        // absurdly fast, straight up a column of bricks
        let x = g.origin.x + BRICK_W * 6.5;
        g.balls = vec![Ball { p: V2::new(x, 46.0), v: V2::new(0.0, -6000.0) }];
        g.speed = 6000.0;
        let before = g.left();
        g.move_ball(0);
        assert_eq!(before - g.left(), 1, "one step broke more than the first brick");
        assert!(g.balls[0].v.y > 0.0);
    }

    #[test]
    fn paddle_aims_by_where_it_is_hit() {
        for mode in [Mode::Line, Mode::Arc, Mode::Circle] {
            let mut g = Game::new(mode);
            g.state = State::Play;
            let (a, b, n) = g.paddle_segment();
            let mid = (a + b) * 0.5;
            let t = (b - a).norm();
            for (u, want) in [(0.0, 0.0), (1.0, MAX_DEFLECT), (-0.5, -MAX_DEFLECT / 2.0)] {
                let at = mid + t * (u * PADDLE_HALF) + n * 10.0;
                g.balls = vec![Ball { p: at, v: -n * g.speed }];
                for _ in 0..300 {
                    g.move_ball(0);
                    if g.balls[0].v.dot(n) > 0.0 {
                        break;
                    }
                }
                let v = g.balls[0].v.norm();
                let angle = v.dot(t).atan2(v.dot(n));
                // dead centre on a straight floor, the minimum share tilts it a little
                let slack = if mode != Mode::Circle && want == 0.0 { MIN_SHARE.asin() + 0.01 } else { 0.05 };
                assert!((angle - want).abs() < slack, "{mode:?} u {u}: angle {angle} want {want}");
            }
        }
    }

    #[test]
    fn autoplay_stays_in_bounds_and_clears() {
        for mode in [Mode::Line, Mode::Arc, Mode::Circle] {
            let mut g = Game::new(mode);
            let start = g.left();
            play(&mut g, 120.0);
            assert!(g.floor > 0 || g.left() < start, "{mode:?}: no progress");
        }
    }

    #[test]
    fn floors_parse() {
        for (i, f) in levels::FLOORS.iter().enumerate() {
            assert!(f.rows.iter().all(|r| r.len() == 13), "floor {i} is not 13 wide");
            assert!(f.rows.iter().flat_map(|r| r.chars()).all(|c| ".wocgrbmySG".contains(c)), "floor {i} has a stray character");
        }
    }

    #[test]
    fn losing_every_life_ends_the_game() {
        let mut g = Game::new(Mode::Line);
        for _ in 0..(60.0 / STEP) as usize {
            // the paddle hides in the corner
            g.step(Steer::Move(-1.0), true);
        }
        assert_eq!(g.state, State::Over);
    }
}
