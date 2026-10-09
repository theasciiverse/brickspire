//! The rules: one floor of bricks, the paddle on its path, the ball, lives and score.
//!
//! A fixed step of `STEP` seconds. The paddle moves first, then each ball is swept
//! along its motion, bouncing off whatever it touches first, until the step is used up.

use crate::boss::{self, Flight, Hurt, MiniBoss, MiniKind, Rook, Shot, Storm, Weathercock};
use crate::levels::{self, Floor, FloorKind, RoofBoss};
use crate::physics::{self, Hit, Path, V2};
use crate::roof;
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
/// Climbing onto the roof: the camera rises and tilts to look straight down.
pub const TILT_TIME: f32 = 2.6;
/// How long the roof holds, beaten, before the next tower.
const ROOF_HOLD: f32 = 3.5;
const TOWER_BONUS: u32 = 5000;
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
/// On the roof the paddle may have to go half way round the ring while the ball
/// crosses it, so there it is longer and faster, and the ball slower. (A bigger ring
/// alone wouldn't help: both distances grow together.)
const ROOF_PADDLE_HALF: f32 = 9.0;
const ROOF_PADDLE_SPEED: f32 = 188.0;
const ROOF_SPEED_START: f32 = 40.0;
const ROOF_SPEED_MAX: f32 = 66.0;
const SPEED_PER_HIT: f32 = 0.6;
/// How far from the paddle's normal the ball leaves at the very end of the paddle.
const MAX_DEFLECT: f32 = PI / 3.0;
/// No velocity component smaller than this share of the speed (straight floors only),
/// so the ball never runs almost flat or almost vertical forever.
const MIN_SHARE: f32 = 0.18;
const AUTO_LAUNCH: f32 = 5.0;
const LIVES: u32 = 3;
const GRAVITY: f32 = 60.0;
/// Seconds the paddle is stunned by a fireball; seconds it is slowed by web, and to
/// what share of its speed.
const STUN: f32 = 1.0;
const WEBBED: f32 = 2.5;
const WEB_SLOW: f32 = 0.4;
/// How far the camera pulls back on a boss floor (pixel graphics).
const BOSS_ZOOM: f32 = 0.86;

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
                // high enough that the balcony walkway and its rail fit below it
                let bottom = PADDLE_Y - 2.0;
                Path::Arc { c: V2::new(WORLD_W / 2.0, bottom - r), r, a0: PI / 2.0 - sweep / 2.0, a1: PI / 2.0 + sweep / 2.0 }
            }
            // well inside the roof's parapet, leaving open roof between them
            Mode::Circle => Path::Circle { c: V2::new(WORLD_W / 2.0, 28.0), r: roof::RING },
        };
        Arena { mode, path }
    }

    pub fn walls(&self) -> bool {
        self.mode != Mode::Circle
    }

    /// Past the paddle for good: off the bottom, or out of the ring on the roof.
    pub fn lost(&self, p: V2) -> bool {
        match self.path {
            // off the roof: out through a gap in the parapet
            Path::Circle { c, .. } => (p - c).len() > roof::RIM + 1.0,
            _ => p.y > WORLD_H + 2.0,
        }
    }
}

/// The centre and radius of the boss balcony's arc (the paddle's path there).
pub fn balcony_arc() -> (V2, f32) {
    match Arena::new(Mode::Arc).path {
        Path::Arc { c, r, .. } => (c, r),
        _ => unreachable!(),
    }
}

/// Where a point on a boss face lies relative to the balcony, from its distance `d`
/// to the arc's centre.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Balcony {
    /// Above the paddle's path: the tower face as usual.
    Open,
    /// The stone walkway under the path, `k` 0 at its inner edge to 1 at the rail.
    Walkway(f32),
    /// The steel balustrade along its edge, `k` 0 to 1 across it.
    Rail(f32),
    /// The tower wall below, in the balcony's shadow.
    Shadow,
}

pub fn balcony_at(d: f32, r: f32) -> Balcony {
    let (walk0, rail0, rail1) = (r + PADDLE_TH + 0.3, r + 4.5, r + 5.7);
    if d < walk0 {
        Balcony::Open
    } else if d < rail0 {
        Balcony::Walkway((d - walk0) / (rail0 - walk0))
    } else if d < rail1 {
        Balcony::Rail((d - rail0) / (rail1 - rail0))
    } else {
        Balcony::Shadow
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
    /// Stone laid by the Gargoyle as a shield: one hit.
    Stone,
    /// Vine: slides along its row, one cell at a time, turning back when blocked.
    Vine,
    /// Web spun by the spider as a shield: one hit.
    Web,
}

#[derive(Clone, Copy, Debug)]
pub struct Brick {
    pub kind: Kind,
    pub hp: u8,
    /// Seconds of white flash left after a hit that didn't break it.
    pub flash: f32,
    /// How far it is drawn and struck from its cell across (a vine still sliding in
    /// from its last cell), and which way a vine slides next.
    pub slide: f32,
    pub dir: i8,
}

impl Brick {
    pub fn new(kind: Kind, hp: u8) -> Brick {
        Brick { kind, hp, flash: 0.0, slide: 0.0, dir: 1 }
    }
}

/// Seconds between a vine's steps, and how long a step takes.
const VINE_EVERY: f32 = 1.4;
const VINE_SLIDE: f32 = 0.35;

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
    /// The mini-boss on a boss floor (gone once beaten), its fireballs, and how long
    /// the paddle has left stunned by one.
    pub boss: Option<MiniBoss>,
    /// The roof's boss, likewise: the Storm Cloud, the Rook or the Weathercock.
    pub storm: Option<Storm>,
    pub rook: Option<Rook>,
    pub vane: Option<Weathercock>,
    /// A roof boss forced for every roof (`--roof`), instead of each roof's own.
    pub roof_boss: Option<RoofBoss>,
    pub boss_floor: bool,
    pub shots: Vec<Shot>,
    pub stun: f32,
    /// How long the paddle has left gummed up by the Spider's web (slow).
    pub webbed: f32,
    /// A paddle path forced for every floor (`--path`), instead of each floor's own.
    forced: Option<Mode>,
    seed: u32,
}

/// What a ball touched.
#[derive(Clone, Copy, Debug)]
enum Thing {
    Wall,
    Brick(usize),
    Paddle,
    Boss,
    /// The Weathercock's arrow (bounces, can't be hurt) and its rooster (the target).
    Arrow,
    Rooster,
    /// The Storm Cloud's core.
    Core,
    /// A twig of the Rook's nest, and the Rook.
    Twig(usize),
    Crow,
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
        Kind::Stone => [150, 135, 118],
        Kind::Vine => [70, 170, 70],
        Kind::Web => [215, 215, 225],
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
                    's' => Some(Kind::Stone),
                    'v' => Some(Kind::Vine),
                    'W' => Some(Kind::Web),
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
    /// A new game; `forced` puts the paddle on one kind of path for every floor.
    pub fn new(forced: Option<Mode>) -> Game {
        let mut g = Game {
            arena: Arena::new(forced.unwrap_or(Mode::Line)),
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
            boss: None,
            storm: None,
            rook: None,
            vane: None,
            roof_boss: None,
            boss_floor: false,
            shots: Vec::new(),
            stun: 0.0,
            webbed: 0.0,
            forced,
            seed: 0x9e37_79b9,
        };
        g.load_floor(0);
        g
    }

    pub fn floor_def(&self, n: usize) -> &'static Floor {
        if self.forced == Some(Mode::Circle) { &levels::ROOF_TEST } else { levels::floor(n) }
    }

    /// The paddle's path on floor `n`: the balcony's arc on a boss floor.
    pub fn floor_mode(&self, n: usize) -> Mode {
        self.forced.unwrap_or(match self.floor_def(n).kind {
            FloorKind::Plain => Mode::Line,
            FloorKind::Balcony => Mode::Arc,
            FloorKind::Roof(_) => Mode::Circle,
        })
    }

    pub fn load_floor(&mut self, n: usize) {
        let def = self.floor_def(n);
        self.floor = n;
        self.floor_name = def.name;
        self.arena = Arena::new(self.floor_mode(n));
        self.boss_floor = def.kind != FloorKind::Plain;
        self.boss = (def.kind == FloorKind::Balcony).then(|| MiniBoss::new(levels::tower(n).mini));
        let roof = match (def.kind, self.arena.path) {
            (FloorKind::Roof(who), Path::Circle { c, .. }) => Some((self.roof_boss.unwrap_or(who), c)),
            _ => None,
        };
        self.storm = roof.and_then(|(who, c)| (who == RoofBoss::Storm).then(|| Storm::new(c)));
        self.rook = roof.and_then(|(who, c)| (who == RoofBoss::Rook).then(|| Rook::new(c)));
        self.vane = roof.and_then(|(who, c)| (who == RoofBoss::Weathercock).then(|| Weathercock::new(c)));
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
            .enumerate()
            .map(|(i, k)| {
                k.map(|kind| Brick {
                    // vines in alternate rows set off opposite ways
                    dir: if (i / cols) % 2 == 0 { 1 } else { -1 },
                    ..Brick::new(kind, if kind == Kind::Steel { steel_hp } else { 1 })
                })
            })
            .collect();
        self.particles.clear();
        self.serve();
        info!("floor {} start: {} ({} bricks, {:?})", n + 1, def.name, self.left(), self.arena.mode);
    }

    fn serve(&mut self) {
        self.balls.clear();
        self.shots.clear();
        self.stun = 0.0;
        self.webbed = 0.0;
        self.speed = self.speed_start();
        if self.arena.path.wraps() {
            self.paddle.s = 0.0;
        } else {
            self.paddle.s = self.arena.path.len() / 2.0;
        }
        self.paddle.vel = 0.0;
        self.state = State::Serve { wait: 0.0 };
    }

    /// Half the paddle's length: longer on the roof.
    pub fn paddle_half(&self) -> f32 {
        if self.arena.mode == Mode::Circle { ROOF_PADDLE_HALF } else { PADDLE_HALF }
    }

    fn paddle_speed(&self) -> f32 {
        if self.arena.mode == Mode::Circle { ROOF_PADDLE_SPEED } else { PADDLE_SPEED }
    }

    fn speed_start(&self) -> f32 {
        if self.arena.mode == Mode::Circle { ROOF_SPEED_START } else { SPEED_START }
    }

    fn speed_max(&self) -> f32 {
        if self.arena.mode == Mode::Circle { ROOF_SPEED_MAX } else { SPEED_MAX }
    }

    /// Breakable bricks still standing.
    pub fn left(&self) -> usize {
        self.bricks.iter().flatten().filter(|b| b.kind != Kind::Gilt).count()
    }

    /// Brick `i`'s box: its cell, shifted while it slides.
    pub fn brick_box(&self, i: usize) -> (V2, V2) {
        let (c, r) = (i % self.cols, i / self.cols);
        let slide = self.bricks.get(i).copied().flatten().map_or(0.0, |b| b.slide);
        let min = self.origin + V2::new(c as f32 * BRICK_W + slide, r as f32 * BRICK_H);
        (min, min + V2::new(BRICK_W, BRICK_H))
    }

    /// The brick under world point `p`, if any, and where in it (from its top left).
    pub fn brick_at(&self, p: V2) -> Option<(usize, &Brick, V2)> {
        let q = p - self.origin;
        if q.y < 0.0 || q.x < -BRICK_W {
            return None;
        }
        let (c, r) = ((q.x / BRICK_W).floor() as i64, (q.y / BRICK_H) as usize);
        if r >= self.rows {
            return None;
        }
        // its own cell first, then a neighbour sliding over
        for c in [c, c - 1, c + 1] {
            if c < 0 || c as usize >= self.cols {
                continue;
            }
            let i = r * self.cols + c as usize;
            let Some(b) = &self.bricks[i] else { continue };
            let (min, max) = self.brick_box(i);
            if p.x >= min.x && p.x < max.x {
                return Some((i, b, p - min));
            }
        }
        None
    }

    /// Vines step along their rows into empty cells, turning back at anything in
    /// the way (a brick, the edge, a ball), and glide over from the cell they left.
    fn grow_vines(&mut self) {
        let glide = BRICK_W / VINE_SLIDE * STEP;
        for b in self.bricks.iter_mut().flatten() {
            b.slide = if b.slide > 0.0 { (b.slide - glide).max(0.0) } else { (b.slide + glide).min(0.0) };
        }
        let before = self.time - STEP;
        if (self.time / VINE_EVERY).floor() == (before / VINE_EVERY).floor() || self.state != State::Play {
            return;
        }
        let mut moved = vec![false; self.bricks.len()];
        for i in 0..self.bricks.len() {
            let Some(b) = self.bricks[i] else { continue };
            if b.kind != Kind::Vine || moved[i] {
                continue;
            }
            let (c, r) = (i % self.cols, i / self.cols);
            let to = c as i64 + b.dir as i64;
            let free = to >= 0 && (to as usize) < self.cols && {
                let j = r * self.cols + to as usize;
                let min = self.origin + V2::new(to as f32 * BRICK_W, r as f32 * BRICK_H);
                let max = min + V2::new(BRICK_W, BRICK_H);
                self.bricks[j].is_none() && !self.balls.iter().any(|ball| physics::box_overlap(ball.p, min, max, BALL_R + 1.0).is_some())
            };
            if free {
                let j = r * self.cols + to as usize;
                self.bricks[i] = None;
                self.bricks[j] = Some(Brick { slide: -(b.dir as f32) * BRICK_W, ..b });
                moved[j] = true;
            } else {
                self.bricks[i] = Some(Brick { dir: -b.dir, ..b });
            }
        }
    }

    /// The paddle as a segment, and the path's normal at its centre.
    pub fn paddle_segment(&self) -> (V2, V2, V2) {
        let path = &self.arena.path;
        let (c, t) = (path.point(self.paddle.s), path.tangent(self.paddle.s));
        let half = self.paddle_half();
        (c - t * half, c + t * half, path.normal(self.paddle.s))
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
        self.grow_vines();
        let gravity = if self.arena.mode == Mode::Circle { 0.0 } else { GRAVITY };
        self.particles.retain_mut(|p| {
            p.v.y += gravity * STEP;
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
                if t >= self.clear_time() {
                    info!("on to floor {}", self.floor + 2);
                    self.load_floor(self.floor + 1);
                } else {
                    if t > CLEAR_HOLD && t - STEP <= CLEAR_HOLD {
                        info!("{}", if self.roof_next() { "camera climbing onto the roof" } else { "tower turning" });
                    }
                    self.state = State::Clear { t };
                }
                return;
            }
            _ => {}
        }
        self.stun = (self.stun - STEP).max(0.0);
        self.webbed = (self.webbed - STEP).max(0.0);
        self.move_paddle(steer);
        self.update_boss();
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
            self.shots.clear();
            self.state = State::Lost { t: 1.2 };
        } else if self.boss_floor && !self.boss_left() {
            info!("floor {} clear, boss beaten: score {}", self.floor + 1, self.score);
            if self.arena.mode == Mode::Circle {
                info!("tower climbed: bonus {TOWER_BONUS}");
                self.score += TOWER_BONUS;
            }
            self.clear_floor(0.0);
        } else if !self.boss_floor && self.left() == 0 {
            info!("floor {} clear: score {}", self.floor + 1, self.score);
            self.clear_floor(0.0);
        }
    }

    /// Who waits on the roof above, while climbing to it.
    pub fn waiting_roof_boss(&self) -> Option<RoofBoss> {
        match self.floor_def(self.floor + 1).kind {
            FloorKind::Roof(who) if self.roof_next() => Some(self.roof_boss.unwrap_or(who)),
            _ => None,
        }
    }

    /// Is a boss still alive on this floor?
    pub fn boss_left(&self) -> bool {
        self.boss.is_some() || self.vane.is_some() || self.storm.is_some() || self.rook.is_some()
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
        let slow = if self.webbed > 0.0 { WEB_SLOW } else { 1.0 };
        let chase = |to: f32| (path.delta(self.paddle.s, to) / STEP).clamp(-PADDLE_CHASE * slow, PADDLE_CHASE * slow);
        self.paddle.vel = if self.stun > 0.0 {
            0.0
        } else {
            match steer {
            Steer::Move(k) => k.clamp(-1.0, 1.0) * self.paddle_speed() * slow,
            Steer::Toward(p) => chase(path.nearest(p)),
            Steer::Aim(dir) => match path {
                Path::Circle { c, r } if dir.len() > 0.0 => chase(path.nearest(c + dir.norm() * r)),
                _ => dir.x.clamp(-1.0, 1.0) * self.paddle_speed() * slow,
            },
            }
        };
        self.paddle.s = path.keep(self.paddle.s + self.paddle.vel * STEP, self.paddle_half());
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
        let u = ((physics::closest_on_segment(p, a, b) - mid).dot(t) / self.paddle_half()).clamp(-1.0, 1.0);
        self.balls[i].v = n.rotate(u * MAX_DEFLECT) * self.speed;
    }

    fn move_ball(&mut self, i: usize) {
        // the Weathercock's gusts bend the ball's path (its speed stays the same)
        if let Some(wind) = self.vane.as_ref().and_then(|w| w.wind()) {
            let v = self.balls[i].v + wind * STEP;
            self.balls[i].v = v.norm() * self.speed;
        }
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
                Thing::Boss => {
                    self.balls[i].v = ball.v.reflect(hit.n);
                    self.hit_boss();
                }
                Thing::Arrow => self.balls[i].v = ball.v.reflect(hit.n),
                Thing::Rooster => {
                    self.balls[i].v = ball.v.reflect(hit.n);
                    self.hit_vane();
                }
                Thing::Core => {
                    self.balls[i].v = ball.v.reflect(hit.n);
                    self.hit_storm();
                }
                Thing::Twig(k) => {
                    self.balls[i].v = ball.v.reflect(hit.n);
                    self.break_twig(k);
                }
                Thing::Crow => {
                    self.balls[i].v = ball.v.reflect(hit.n);
                    self.hit_rook();
                }
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
        // (a column wider each side, for vines sliding over from the next cell)
        let (c0, c1) = (cell(lo.x - BALL_R - BRICK_W, BRICK_W, self.cols), cell(hi.x + BALL_R + BRICK_W, BRICK_W, self.cols.saturating_sub(1)));
        let (r0, r1) = (cell(lo.y - BALL_R, BRICK_H, self.rows), cell(hi.y + BALL_R, BRICK_H, self.rows.saturating_sub(1)));
        if hi.x + BALL_R + BRICK_W >= 0.0 && hi.y + BALL_R >= 0.0 {
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
        if let Some(g) = &self.boss {
            let (min, max) = g.bounds();
            take(physics::sweep_box(p, d, min, max, BALL_R), Thing::Boss);
        }
        if let Path::Circle { c, .. } = self.arena.path {
            // the parapet's merlons: an inner face and two sides each
            if (p - c).len() + d.len() > roof::RIM - roof::PARAPET - BALL_R - 0.5 {
                let (r0, r1) = (roof::RIM - roof::PARAPET, roof::RIM + 1.0);
                let at = |a: f32, r: f32| c + V2::new(a.cos(), a.sin()) * r;
                for (a0, a1) in roof::merlons() {
                    let mid = (a0 + a1) / 2.0;
                    take(physics::sweep_capsule(p, d, at(a0, r0), at(mid, r0), BALL_R), Thing::Wall);
                    take(physics::sweep_capsule(p, d, at(mid, r0), at(a1, r0), BALL_R), Thing::Wall);
                    take(physics::sweep_capsule(p, d, at(a0, r0), at(a0, r1), BALL_R), Thing::Wall);
                    take(physics::sweep_capsule(p, d, at(a1, r0), at(a1, r1), BALL_R), Thing::Wall);
                }
            }
        }
        if let Some(w) = &self.vane {
            let (tail, head) = w.arrow();
            take(physics::sweep_capsule(p, d, tail, head, boss::ARROW_TH + BALL_R), Thing::Arrow);
            take(physics::sweep_capsule(p, d, w.centre, w.centre, boss::ROOSTER_R + BALL_R), Thing::Rooster);
        }
        if let Some(s) = &self.storm {
            take(physics::sweep_capsule(p, d, s.centre, s.centre, boss::CORE_R + BALL_R), Thing::Core);
        }
        if let Some(r) = &self.rook {
            for k in (0..boss::TWIGS).filter(|&k| r.twigs[k]) {
                let (a, b) = r.twig(k);
                take(physics::sweep_capsule(p, d, a, b, boss::TWIG_TH + BALL_R), Thing::Twig(k));
            }
            take(physics::sweep_capsule(p, d, r.p, r.p, boss::CROW_R + BALL_R), Thing::Crow);
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
        self.speed = (self.speed + SPEED_PER_HIT).min(self.speed_max());
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
            Kind::Stone | Kind::Web => 10,
            Kind::Vine => 80,
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

    fn burst(&mut self, at: V2, spread: V2, count: usize, rgb: [u8; 3], speed: f32) {
        for _ in 0..count {
            let p = at + V2::new((self.rand() - 0.5) * spread.x, (self.rand() - 0.5) * spread.y);
            let a = self.rand() * 2.0 * PI;
            let v = V2::new(a.cos(), a.sin()) * (speed * (0.3 + self.rand()));
            let life = 0.4 + self.rand() * 0.6;
            self.particles.push(Particle { p, v, life, rgb });
        }
    }

    /// The boss flies and spits; its fireballs fly and may stun the paddle; and if
    /// it flew into a ball, the ball is pushed back out.
    fn update_boss(&mut self) {
        self.update_vane();
        self.update_storm();
        self.update_rook();
        let Some(mut g) = self.boss.take() else { return };
        let live = self.state == State::Play;
        let target = self.arena.path.point(self.paddle.s);
        g.update(STEP, live, target, &mut self.shots);
        let (min, max) = g.bounds();
        for b in self.balls.iter_mut() {
            if let Some((depth, n)) = physics::box_overlap(b.p, min, max, BALL_R) {
                b.p += n * (depth + 1e-3);
                if b.v.dot(n) < 0.0 {
                    b.v = b.v.reflect(n);
                }
            }
        }
        self.boss = Some(g);
        let (a, b, _) = self.paddle_segment();
        let mut stunned = None;
        let arena = &self.arena;
        self.shots.retain_mut(|s| {
            s.p += s.v * STEP;
            if (s.p - physics::closest_on_segment(s.p, a, b)).len() < PADDLE_TH + boss::SHOT_R {
                stunned = Some((s.p, s.web));
                return false;
            }
            !arena.lost(s.p) && s.p.y < WORLD_H + 4.0
        });
        match stunned {
            Some((at, false)) => {
                info!("paddle stunned");
                self.stun = STUN;
                self.burst(at, V2::new(2.0, 1.0), 10, [255, 150, 40], 22.0);
            }
            Some((at, true)) => {
                info!("paddle webbed");
                self.webbed = WEBBED;
                self.burst(at, V2::new(3.0, 1.0), 10, [215, 215, 225], 12.0);
            }
            None => {}
        }
    }

    /// The Weathercock turns and blows; a ball its arrow or rooster swung into is
    /// pushed back out (and batted, if it was heading in); gusts streak the roof.
    fn update_vane(&mut self) {
        let Some(mut w) = self.vane.take() else { return };
        w.update(STEP, self.state == State::Play);
        let (tail, head) = w.arrow();
        for b in self.balls.iter_mut() {
            for (a, z, reach) in [(tail, head, boss::ARROW_TH + BALL_R), (w.centre, w.centre, boss::ROOSTER_R + BALL_R)] {
                let q = physics::closest_on_segment(b.p, a, z);
                let off = b.p - q;
                if off.len() < reach && off.len() > 0.0 {
                    let n = off.norm();
                    b.p = q + n * (reach + 1e-3);
                    if b.v.dot(n) < 0.0 {
                        b.v = b.v.reflect(n);
                    }
                }
            }
        }
        if let (Some(wind), Path::Circle { c, r }) = (w.wind(), self.arena.path) {
            let a = self.rand() * 2.0 * PI;
            let p = c + V2::new(a.cos(), a.sin()) * (self.rand().sqrt() * r);
            let life = 0.3 + self.rand() * 0.3;
            self.particles.push(Particle { p, v: wind * 0.9, life, rgb: [200, 215, 235] });
        }
        self.vane = Some(w);
    }

    /// The Storm Cloud drifts and drizzles, marks the ring and strikes; a strike
    /// stuns the paddle if any of it is on the marked stretch. A ball its core drifted
    /// into is pushed back out.
    fn update_storm(&mut self) {
        let Some(mut s) = self.storm.take() else { return };
        let Path::Circle { c, r } = self.arena.path else {
            self.storm = Some(s);
            return;
        };
        let paddle = self.arena.path.point(self.paddle.s);
        let at_paddle = (paddle - c).y.atan2((paddle - c).x);
        let struck = s.update(STEP, self.state == State::Play, at_paddle);
        for b in self.balls.iter_mut() {
            let off = b.p - s.centre;
            let reach = boss::CORE_R + BALL_R;
            if off.len() < reach && off.len() > 0.0 {
                let n = off.norm();
                b.p = s.centre + n * (reach + 1e-3);
                if b.v.dot(n) < 0.0 {
                    b.v = b.v.reflect(n);
                }
            }
        }
        let centre = s.centre;
        self.storm = Some(s);
        if self.rand() < 0.35 {
            let a = self.rand() * 2.0 * PI;
            let p = centre + V2::new(a.cos(), a.sin()) * (self.rand().sqrt() * boss::CLOUD_R * 0.8);
            let life = 0.2 + self.rand() * 0.2;
            self.particles.push(Particle { p, v: V2::new(0.0, 6.0), life, rgb: [110, 150, 225] });
        }
        if let Some(a) = struck {
            let at = c + V2::new(a.cos(), a.sin()) * r;
            self.burst(at, V2::new(3.0, 3.0), 14, [255, 245, 170], 26.0);
            // how far round the ring the paddle's middle is from the strike
            let off = (at_paddle - a + PI).rem_euclid(2.0 * PI) - PI;
            if off.abs() < boss::BOLT_SPAN + self.paddle_half() / r {
                info!("paddle struck by lightning");
                self.stun = STUN;
                self.burst(paddle, V2::new(2.0, 2.0), 10, [190, 210, 255], 22.0);
            }
        }
    }

    /// The Rook hops and swoops; a swoop that reaches the paddle stuns it, and a ball
    /// the crow ran into is pushed back out.
    fn update_rook(&mut self) {
        let Some(mut rook) = self.rook.take() else { return };
        let Path::Circle { c, r } = self.arena.path else {
            self.rook = Some(rook);
            return;
        };
        let paddle = self.arena.path.point(self.paddle.s);
        let at_paddle = (paddle - c).y.atan2((paddle - c).x);
        rook.update(STEP, self.state == State::Play, r, at_paddle);
        for b in self.balls.iter_mut() {
            let off = b.p - rook.p;
            let reach = boss::CROW_R + BALL_R;
            if off.len() < reach && off.len() > 0.0 {
                let n = off.norm();
                b.p = rook.p + n * (reach + 1e-3);
                if b.v.dot(n) < 0.0 {
                    b.v = b.v.reflect(n);
                }
            }
        }
        let mut stunned = false;
        if let Flight::Swooping { a, t, from, struck: false } = rook.flight {
            let (pa, pb, _) = self.paddle_segment();
            if (rook.p - physics::closest_on_segment(rook.p, pa, pb)).len() < boss::CROW_R + PADDLE_TH {
                rook.flight = Flight::Swooping { a, t, from, struck: true };
                stunned = true;
            }
        }
        let at = rook.p;
        self.rook = Some(rook);
        if stunned {
            info!("paddle struck by the rook");
            self.stun = STUN;
            self.burst(at, V2::new(3.0, 3.0), 12, [40, 40, 55], 22.0);
        }
    }

    fn break_twig(&mut self, k: usize) {
        let Some(r) = self.rook.as_mut() else { return };
        if !r.twigs[k] {
            return;
        }
        r.twigs[k] = false;
        let (a, b) = r.twig(k);
        self.score += 20;
        self.burst((a + b) * 0.5, V2::new(2.0, 2.0), 8, [140, 100, 55], 16.0);
    }

    fn hit_rook(&mut self) {
        let Some(r) = self.rook.as_mut() else { return };
        let hurt = r.hit();
        let at = r.p;
        let hp = r.hp;
        match hurt {
            Hurt::Shrugged => return,
            Hurt::Hurt | Hurt::Shield => {
                info!("rook hit: {hp} left");
                self.score += 150;
                self.burst(at, V2::new(3.0, 3.0), 8, [30, 30, 45], 18.0);
                if hurt == Hurt::Shield {
                    let balls: Vec<V2> = self.balls.iter().map(|b| b.p).collect();
                    let r = self.rook.as_mut().unwrap();
                    let put = r.rebuild(|a, b| balls.iter().all(|&p| (p - physics::closest_on_segment(p, a, b)).len() > boss::TWIG_TH + BALL_R + 1.0));
                    info!("rook patches its nest: {put} twigs");
                }
            }
            Hurt::Dead => {
                info!("rook beaten");
                self.score += 4000;
                self.rook = None;
                self.burst(at, V2::new(6.0, 6.0), 50, [30, 30, 45], 30.0);
                self.burst(at, V2::new(16.0, 16.0), 40, [140, 100, 55], 26.0);
            }
        }
        self.speed = (self.speed + SPEED_PER_HIT).min(self.speed_max());
    }

    fn hit_storm(&mut self) {
        let Some(s) = self.storm.as_mut() else { return };
        let hurt = s.hit();
        let (at, hp) = (s.centre, s.hp);
        match hurt {
            Hurt::Shrugged => return,
            Hurt::Hurt | Hurt::Shield => {
                info!("storm cloud hit: {hp} left");
                self.score += 150;
                self.burst(at, V2::new(3.0, 3.0), 8, [255, 235, 140], 16.0);
                if hurt == Hurt::Shield {
                    self.rain(at);
                }
            }
            Hurt::Dead => {
                info!("storm cloud beaten");
                self.score += 4000;
                self.storm = None;
                self.burst(at, V2::new(18.0, 18.0), 50, [120, 125, 145], 30.0);
                self.burst(at, V2::new(4.0, 4.0), 25, [255, 235, 140], 26.0);
                self.burst(at, V2::new(10.0, 10.0), 25, [110, 150, 225], 20.0);
            }
        }
        self.speed = (self.speed + SPEED_PER_HIT).min(self.speed_max());
    }

    /// Rain round the cloud: a few empty cells near it (not under its core, and not
    /// where a ball is) grow cyan bricks back.
    fn rain(&mut self, at: V2) {
        let mut cells: Vec<(f32, usize)> = (0..self.bricks.len())
            .filter(|&i| self.bricks[i].is_none())
            .filter_map(|i| {
                let (min, max) = self.brick_box(i);
                let d = ((min + max) * 0.5 - at).len();
                let in_the_way = self.balls.iter().any(|b| physics::box_overlap(b.p, min, max, BALL_R + 1.0).is_some());
                (d > boss::CORE_R + 3.5 && d < boss::CLOUD_R + 1.0 && !in_the_way).then_some((d, i))
            })
            .collect();
        // nearest first, shuffled a little so it isn't always the same cells
        for c in cells.iter_mut() {
            c.0 += self.rand() * 4.0;
        }
        cells.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(_, i) in cells.iter().take(3) {
            self.bricks[i] = Some(Brick { flash: 0.15, ..Brick::new(Kind::Plain(2), 1) });
        }
        info!("storm rains: {} bricks grow back", cells.len().min(3));
    }

    fn hit_vane(&mut self) {
        let Some(w) = self.vane.as_mut() else { return };
        let hurt = w.hit();
        let (at, hp) = (w.centre, w.hp);
        match hurt {
            Hurt::Shrugged => return,
            Hurt::Hurt | Hurt::Shield => {
                info!("weathercock hit: {hp} left");
                self.score += 150;
                self.burst(at, V2::new(3.0, 3.0), 8, [235, 70, 50], 16.0);
            }
            Hurt::Dead => {
                info!("weathercock beaten");
                self.score += 4000;
                self.vane = None;
                self.burst(at, V2::new(16.0, 16.0), 50, [90, 90, 100], 30.0);
                self.burst(at, V2::new(4.0, 4.0), 25, [235, 70, 50], 26.0);
                self.burst(at, V2::new(6.0, 6.0), 20, [240, 185, 45], 24.0);
            }
        }
        self.speed = (self.speed + SPEED_PER_HIT).min(self.speed_max());
    }

    fn hit_boss(&mut self) {
        let Some(g) = self.boss.as_mut() else { return };
        let hurt = g.hit();
        let (at, hp, kind, half) = (g.p, g.hp, g.kind, g.half());
        let body = if kind == MiniKind::Spider { [70, 40, 80] } else { [150, 140, 160] };
        match hurt {
            Hurt::Shrugged => return,
            Hurt::Hurt | Hurt::Shield => {
                info!("{kind:?} hit: {hp} left");
                self.score += 100;
                self.burst(at, half, 8, body, 16.0);
                if hurt == Hurt::Shield {
                    self.lay_shield(at, half, kind);
                }
            }
            Hurt::Dead => {
                info!("{kind:?} beaten");
                self.score += 2000;
                self.boss = None;
                self.shots.clear();
                self.burst(at, half * 2.0, 40, body, 30.0);
                self.burst(at, half, 20, if kind == MiniKind::Spider { [215, 215, 225] } else { [255, 120, 40] }, 24.0);
            }
        }
        self.speed = (self.speed + SPEED_PER_HIT).min(self.speed_max());
    }

    /// A row of bricks under the mini-boss, wherever there is room for them: stone
    /// from the Gargoyle, web from the Spider.
    fn lay_shield(&mut self, at: V2, half: V2, kind: MiniKind) {
        let brick = if kind == MiniKind::Spider { Kind::Web } else { Kind::Stone };
        let y = at.y + half.y + 2.5 - self.origin.y;
        let r = (y / BRICK_H).floor() as i32;
        if r < 0 || r as usize >= self.rows {
            return;
        }
        let c0 = ((at.x - 9.0 - self.origin.x) / BRICK_W).floor().max(0.0) as usize;
        let c1 = (((at.x + 9.0 - self.origin.x) / BRICK_W).floor().max(0.0) as usize).min(self.cols - 1);
        let mut laid = 0;
        for c in c0..=c1 {
            let i = r as usize * self.cols + c;
            let (min, max) = self.brick_box(i);
            let in_the_way = self.balls.iter().any(|b| physics::box_overlap(b.p, min, max, BALL_R + 0.5).is_some());
            if self.bricks[i].is_none() && !in_the_way {
                self.bricks[i] = Some(Brick { flash: 0.15, ..Brick::new(brick, 1) });
                laid += 1;
            }
        }
        info!("{kind:?} lays a shield of {laid} bricks");
    }

    /// What the camera frames: the middle and size (world units) of the box fitted to
    /// the screen. On the tower's side, the whole 100x56 world (pulled back a little
    /// on the balcony); on the roof, just the roof, so it fills the screen however the
    /// screen is shaped. Eased across during a turn or the climb onto the roof.
    pub fn frame(&self) -> (V2, V2) {
        let world = V2::new(WORLD_W, WORLD_H);
        let side = |n: usize| (world * 0.5, world * (1.0 / if self.floor_def(n).kind == FloorKind::Balcony { BOSS_ZOOM } else { 1.0 }));
        let (centre, _) = balcony_arc();
        let roof_centre = match Arena::new(Mode::Circle).path {
            Path::Circle { c, .. } => c,
            _ => centre,
        };
        let roof = (roof_centre, V2::new(1.0, 1.0) * (2.0 * roof::RIM + 2.0));
        let blend = |a: (V2, V2), b: (V2, V2), k: f32| (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
        if self.arena.mode == Mode::Circle {
            return roof;
        }
        if let Some(k) = self.tilt() {
            return blend(side(self.floor), roof, k);
        }
        match self.state {
            State::Clear { t } => blend(side(self.floor), side(self.floor + 1), smoothstep((t - CLEAR_HOLD) / TURN_TIME)),
            _ => side(self.floor),
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
        self.boss = None;
        self.vane = None;
        self.storm = None;
        self.rook = None;
        if self.arena.mode != Mode::Circle {
            self.clear_floor(CLEAR_HOLD);
        } else {
            self.load_floor(self.floor + 1);
        }
    }

    /// The floor is done: the balls go, and the tower turns (straight floors) or the
    /// next floor simply loads (the arc and roof previews).
    fn clear_floor(&mut self, t: f32) {
        self.balls.clear();
        self.shots.clear();
        self.state = State::Clear { t };
    }

    /// Where the camera faces, in floors: the current floor, plus how far through
    /// the turn to the next one.
    pub fn camera(&self) -> f32 {
        let turn = match self.state {
            State::Clear { t } if self.arena.mode != Mode::Circle && !self.roof_next() => smoothstep((t - CLEAR_HOLD) / TURN_TIME),
            _ => 0.0,
        };
        self.floor as f32 + turn
    }

    /// Is the next floor the roof (and this one not)?
    pub fn roof_next(&self) -> bool {
        self.arena.mode != Mode::Circle && self.floor_mode(self.floor + 1) == Mode::Circle
    }

    /// How far the camera has tilted from looking at the tower's side (0) to looking
    /// straight down on its roof (1); `None` when it isn't anywhere near the roof.
    pub fn tilt(&self) -> Option<f32> {
        if self.arena.mode == Mode::Circle {
            return Some(1.0);
        }
        match self.state {
            State::Clear { t } if self.roof_next() => Some(smoothstep((t - CLEAR_HOLD) / TILT_TIME)),
            _ => None,
        }
    }

    /// How long a cleared floor waits before the next one loads.
    fn clear_time(&self) -> f32 {
        if self.arena.mode == Mode::Circle {
            ROOF_HOLD
        } else if self.roof_next() {
            CLEAR_HOLD + TILT_TIME
        } else {
            CLEAR_HOLD + TURN_TIME
        }
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
        let mut g = Game::new(Some(Mode::Line));
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
            let mut g = Game::new(Some(mode));
            g.state = State::Play;
            let (a, b, n) = g.paddle_segment();
            let mid = (a + b) * 0.5;
            let t = (b - a).norm();
            for (u, want) in [(0.0, 0.0), (1.0, MAX_DEFLECT), (-0.5, -MAX_DEFLECT / 2.0)] {
                let at = mid + t * (u * g.paddle_half()) + n * 10.0;
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
            let mut g = Game::new(Some(mode));
            let start = g.left();
            play(&mut g, 120.0);
            assert!(g.floor > 0 || g.left() < start, "{mode:?}: no progress");
        }
    }

    #[test]
    fn floors_parse() {
        for t in levels::TOWERS {
            assert!(matches!(t.floors.last().unwrap().kind, FloorKind::Roof(_)), "{} has no roof on top", t.name);
            assert_eq!(t.floors.iter().filter(|f| f.kind == FloorKind::Balcony).count(), 1, "{} needs one balcony", t.name);
            for f in t.floors {
                let width = if matches!(f.kind, FloorKind::Roof(_)) { 7 } else { 13 };
                assert!(f.rows.iter().all(|r| r.len() == width), "{} is not {width} wide", f.name);
                assert!(f.rows.iter().flat_map(|r| r.chars()).all(|c| ".wocgrbmySGv".contains(c)), "{} has a stray character", f.name);
            }
        }
        // numbered straight through, and round again after the last tower
        assert_eq!(levels::tower_of(levels::TOWERS[0].floors.len()), (1, levels::TOWERS[0].floors.len()));
        assert_eq!(levels::tower_of(levels::total()), (0, levels::total()));
    }

    /// Every floor of the given kind, through all the towers.
    fn floors_where(want: impl Fn(FloorKind) -> bool) -> Vec<usize> {
        (0..levels::total()).filter(|&n| want(levels::floor(n).kind)).collect()
    }

    #[test]
    fn every_balcony_is_an_arc_and_its_boss_can_be_beaten() {
        for boss in floors_where(|k| k == FloorKind::Balcony) {
            let mut g = Game::new(None);
            g.load_floor(boss);
            assert_eq!(g.arena.mode, Mode::Arc);
            assert_eq!(g.boss.as_ref().map(|b| b.kind), Some(levels::tower(boss).mini));
            // plenty of lives: this is about the fight being winnable, not the pilot
            g.lives = 99;
            let mut pilot = Pilot::new();
            for _ in 0..(240.0 / STEP) as usize {
                let s = pilot.steer(&g);
                g.step(s, true);
                if g.floor != boss {
                    break;
                }
            }
            assert_eq!(g.floor, boss + 1, "the {:?} on floor {boss} was never beaten", levels::tower(boss).mini);
            // up onto the roof, and its boss
            assert_eq!(g.arena.mode, Mode::Circle);
            assert!(g.boss_left());
        }
    }

    fn roof_floor() -> usize {
        floors_where(|k| matches!(k, FloorKind::Roof(_)))[0]
    }

    #[test]
    fn the_rook_can_be_beaten_and_swoops() {
        let roof = floors_where(|k| k == FloorKind::Roof(RoofBoss::Rook))[0];
        let mut g = Game::new(None);
        g.load_floor(roof);
        assert!(g.rook.is_some());
        g.lives = 99;
        let mut pilot = Pilot::new();
        let mut swoops = 0;
        for _ in 0..(400.0 / STEP) as usize {
            let s = pilot.steer(&g);
            let was = g.rook.as_ref().is_some_and(|r| matches!(r.flight, Flight::Swooping { .. }));
            g.step(s, true);
            if !was && g.rook.as_ref().is_some_and(|r| matches!(r.flight, Flight::Swooping { .. })) {
                swoops += 1;
            }
            if g.floor != roof {
                break;
            }
        }
        assert!(swoops > 0, "the rook never swooped");
        assert_eq!(g.floor, roof + 1, "the rook was never beaten");
        // the next tower's ground floor
        assert_eq!(levels::tower_of(g.floor).1, g.floor);
    }

    #[test]
    fn vines_slide_without_overlapping_or_trapping_the_ball() {
        let floor = (0..levels::total()).find(|&n| levels::floor(n).rows.iter().any(|r| r.contains('v'))).unwrap();
        let mut g = Game::new(None);
        g.load_floor(floor);
        g.lives = 99;
        let start: Vec<usize> = (0..g.bricks.len()).filter(|&i| g.bricks[i].is_some_and(|b| b.kind == Kind::Vine)).collect();
        let mut pilot = Pilot::new();
        let mut moved = false;
        for _ in 0..(30.0 / STEP) as usize {
            let s = pilot.steer(&g);
            g.step(s, true);
            if g.floor != floor {
                break;
            }
            let now: Vec<usize> = (0..g.bricks.len()).filter(|&i| g.bricks[i].is_some_and(|b| b.kind == Kind::Vine)).collect();
            moved |= now.iter().any(|i| !start.contains(i));
            for b in &g.balls {
                for i in 0..g.bricks.len() {
                    if g.bricks[i].is_some() {
                        let (min, max) = g.brick_box(i);
                        assert!(physics::box_overlap(b.p, min, max, BALL_R - 0.05).is_none(), "ball inside brick {i}");
                    }
                }
            }
        }
        assert!(moved, "no vine moved");
    }

    #[test]
    fn the_storm_cloud_can_be_beaten() {
        let mut g = Game::new(None);
        let roof = roof_floor();
        g.load_floor(roof);
        assert!(g.storm.is_some() && g.vane.is_none());
        g.lives = 99;
        let mut pilot = Pilot::new();
        let mut strikes = 0;
        for _ in 0..(400.0 / STEP) as usize {
            let s = pilot.steer(&g);
            let was = g.storm.as_ref().is_some_and(|s| s.strike.is_some());
            g.step(s, true);
            if !was && g.storm.as_ref().is_some_and(|s| s.strike.is_some()) {
                strikes += 1;
            }
            if g.floor != roof {
                break;
            }
        }
        assert!(strikes > 0, "no lightning");
        assert_eq!(g.floor, roof + 1, "the storm cloud was never beaten");
    }

    #[test]
    fn a_strike_off_the_paddle_misses() {
        let mut g = Game::new(None);
        g.load_floor(roof_floor());
        g.state = State::Play;
        g.balls = vec![Ball { p: V2::new(50.0, 28.0), v: V2::new(0.0, 0.0) }];
        let Path::Circle { c, r } = g.arena.path else { panic!() };
        // paddle at the bottom, the mark at the top
        g.paddle.s = g.arena.path.nearest(c + V2::new(0.0, r));
        g.storm.as_mut().unwrap().charging = Some((-PI / 2.0, 0.0));
        g.update_storm();
        assert!(g.storm.as_ref().unwrap().strike.is_some());
        assert_eq!(g.stun, 0.0);
        // and on it, it stuns
        g.storm.as_mut().unwrap().charging = Some((PI / 2.0, 0.0));
        g.update_storm();
        assert!(g.stun > 0.0);
    }

    #[test]
    fn the_roof_is_a_ring_and_the_weathercock_can_be_beaten() {
        let mut g = Game::new(None);
        g.roof_boss = Some(RoofBoss::Weathercock);
        let roof = roof_floor();
        g.load_floor(roof);
        assert_eq!(g.arena.mode, Mode::Circle);
        g.lives = 99;
        let mut pilot = Pilot::new();
        let mut gusts = 0;
        for _ in 0..(400.0 / STEP) as usize {
            let s = pilot.steer(&g);
            let was = g.vane.as_ref().is_some_and(|w| w.wind().is_some());
            g.step(s, true);
            if !was && g.vane.as_ref().is_some_and(|w| w.wind().is_some()) {
                gusts += 1;
            }
            if g.floor != roof {
                break;
            }
        }
        assert!(gusts > 0, "no gusts");
        assert_eq!(g.floor, roof + 1, "the weathercock was never beaten");
    }

    /// A roof with one ball heading straight out from the middle at angle `a`.
    fn roof_shot(a: f32) -> Game {
        let mut g = Game::new(None);
        g.load_floor(roof_floor());
        g.storm = None;
        g.bricks.iter_mut().for_each(|b| *b = None);
        g.state = State::Play;
        let Path::Circle { c, .. } = g.arena.path else { panic!() };
        let dir = V2::new(a.cos(), a.sin());
        g.balls = vec![Ball { p: c + dir * 10.0, v: dir * g.speed }];
        // the paddle well away, on the far side
        g.paddle.s = g.arena.path.nearest(c - dir * 20.0);
        g
    }

    #[test]
    fn merlons_bounce_and_gaps_lose() {
        let step = std::f32::consts::TAU / roof::MERLONS;
        // at a merlon's middle: bounces back towards the middle
        let mut g = roof_shot(step * roof::MERLON_SHARE / 2.0);
        for _ in 0..(1.5 / STEP) as usize {
            g.move_ball(0);
        }
        let Path::Circle { c, .. } = g.arena.path else { panic!() };
        assert!((g.balls[0].p - c).len() < roof::RIM - roof::PARAPET, "went through a merlon");
        // at a gap's middle: out and off the roof
        let mut g = roof_shot(step * (1.0 + roof::MERLON_SHARE) / 2.0);
        for _ in 0..(1.5 / STEP) as usize {
            g.move_ball(0);
        }
        assert!(g.arena.lost(g.balls[0].p), "didn't go out through the gap");
    }

    #[test]
    fn losing_every_life_ends_the_game() {
        let mut g = Game::new(Some(Mode::Line));
        for _ in 0..(60.0 / STEP) as usize {
            // the paddle hides in the corner
            g.step(Steer::Move(-1.0), true);
        }
        assert_eq!(g.state, State::Over);
    }
}
