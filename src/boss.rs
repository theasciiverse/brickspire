//! The bosses. The mini-bosses share one shape of fight, on the balcony: the first
//! tower's is the Gargoyle, a stone gargoyle come alive. It flaps to and fro above the
//! bricks, spits fireballs at the paddle (a hit stuns the paddle for a moment), and
//! every few hits lays a shield of stone bricks beneath itself. Those stay where they
//! are laid as it drifts on, so the field clutters up as the fight goes on. It speeds
//! up as it is hurt.
//!
//! The second tower's is the Spider, hanging on its thread from the lintel. It
//! creeps to and fro, rising and dropping on its thread; it spits web at the paddle
//! (a hit gums it up, slowing it for a while) and spins web bricks beneath itself.
//!
//! The third tower's is the Jack, the clock's iron striker come down off its bell.
//! It marches to and fro in ticks, like a clock's hand; on the hour it strikes a
//! volley of chimes at the paddle (one at first, up to three when hurt), and every
//! few hits it winds a row of clockwork bricks beneath itself.
//!
//! The fourth tower's is the sea serpent, its neck rising over the lintel from the
//! sea far below and its head swaying over the bricks. It spits freezing spray at
//! the paddle (a hit ices it over: it slides, slow to start and slow to stop) and
//! every few hits freezes a row of ice bricks beneath itself.
//!
//! The last tower's is the Mason's apprentice, on a scaffold plank slung from the
//! lintel on two ropes, a hod of bricks beside him. He throws bricks at the paddle
//! (a hit chips it: shorter for a while) and every few hits lays a row of bricks.
//!
//! The first tower's roof boss is the Storm Cloud: a thundercloud drifting over the
//! roof with a glowing core, the target. The ball always flies straight under it; its
//! threat is aimed at the paddle instead: it marks a stretch of the ring, and a moment
//! later lightning strikes there and stuns the paddle if it is still in the way. Every
//! few hits it rains, and bricks grow back round it.
//!
//! The second tower's roof boss is the Rook: a giant crow in a nest of twigs in the
//! middle of the roof. The ball has to break through the twig wall to reach it. It
//! hops about the nest, and now and then crouches, glaring at the paddle's spot on
//! the ring, then swoops out there and back: a paddle still on that spot is stunned.
//! Every few hits it patches its nest.
//!
//! The third tower's roof boss is the Great Bell, hanging over the middle of the
//! roof and swinging. Now and then it shivers, then tolls: a ring of sound spreads
//! out from it, loud only along its swing (two arcs, front and back), and stuns the
//! paddle where an arc crosses it. The ring is marked where the arcs will land, so
//! the paddle can step into the quiet between them. Its swing turns slowly round;
//! every few hits it lurches a quarter turn and tolls twice.
//!
//! The last tower's roof boss, the last of all, is the Mason, who built the tower. He
//! works inside a little fort of bricks in the middle of the roof, putting back
//! whatever the ball breaks, a brick at a time (three in a hurry, every few hits).
//! He lobs bricks at the paddle: the ring is marked where one will land, and a
//! paddle still there is stunned. Hurt, he lobs two at a time, one wide.
//!
//! The fourth tower's roof boss is the Weathercock: a giant iron weathervane in the
//! middle of the roof, roaming round it in a slow figure. Its arrow swings slowly
//! round and bats the ball away (it can't be hurt); the rooster at its hub is the
//! target. Now and then it blows a gust along
//! its arrow that bends the ball's path in flight, and every few hits it spins up and
//! sweeps the arrow round fast.

use crate::physics::V2;
use crate::game::WORLD_W;
pub use crate::levels::MiniKind;

/// Half the Gargoyle's body (its hit box), wingtip to wingtip and horn to claw; the
/// Spider's, leg tip to leg tip.
pub const HALF: V2 = V2::new(7.0, 3.0);
pub const SPIDER_HALF: V2 = V2::new(4.5, 3.0);
pub const SERPENT_HALF: V2 = V2::new(5.0, 3.6);
/// How much bigger the serpent's head is drawn than its sprite's own units.
pub const SERPENT_SCALE: f32 = 1.3;
const SPRAY_SPEED: f32 = 28.0;
pub const APPRENTICE_HALF: V2 = V2::new(7.2, 4.6);
/// How much bigger the apprentice and his scaffold are drawn than their own units.
pub const APPRENTICE_SCALE: f32 = 1.2;
const THROW_SPEED: f32 = 32.0;
pub const JACK_HALF: V2 = V2::new(5.4, 4.9);
/// How much bigger the Jack is drawn than its sprite's own units.
pub const JACK_SCALE: f32 = 1.4;
const CHIME_SPEED: f32 = 30.0;
/// Seconds between the chimes of one volley.
const CHIME_GAP: f32 = 0.45;
const WEB_SPEED: f32 = 26.0;
pub const SHOT_R: f32 = 1.0;
const SHOT_SPEED: f32 = 34.0;
const MAX_HP: u8 = 12;
/// Hits it takes between laying shields.
const SHIELD_EVERY: u8 = 3;
/// Seconds it can't be hurt again after a hit.
const HURT_TIME: f32 = 0.9;
/// Seconds its eyes glow before it spits.
pub const AIM_TIME: f32 = 0.6;
const SWAY: f32 = 30.0;
const HOVER_Y: f32 = 12.5;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShotKind {
    /// The Gargoyle's fireball: stuns the paddle.
    Fire,
    /// The Spider's web: slows the paddle.
    Web,
    /// The Jack's chime: stuns the paddle.
    Chime,
    /// The serpent's freezing spray: ices the paddle over.
    Spray,
    /// The apprentice's brick: chips the paddle shorter.
    Brick,
}

#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub p: V2,
    pub v: V2,
    pub kind: ShotKind,
}

/// What a hit did.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Hurt {
    /// Still glowing from the last hit: nothing.
    Shrugged,
    Hurt,
    /// Hurt, and lays a stone shield.
    Shield,
    Dead,
}

#[derive(Clone, Debug)]
pub struct MiniBoss {
    pub kind: MiniKind,
    pub p: V2,
    pub hp: u8,
    pub max: u8,
    /// Seconds of hurt flash left.
    pub hurt: f32,
    /// Seconds alive, for the wingbeat.
    pub t: f32,
    /// Where it is in its sway, separately from `t` so speeding up doesn't jump.
    phase: f32,
    /// Seconds until it next spits.
    pub reload: f32,
    since_shield: u8,
    shots_fired: u32,
    /// The Jack: chimes left in this volley, and seconds since its hammer last fell.
    volley: u8,
    pub swung: f32,
}

impl MiniBoss {
    pub fn new(kind: MiniKind) -> MiniBoss {
        MiniBoss {
            kind,
            p: V2::new(WORLD_W / 2.0, HOVER_Y),
            hp: MAX_HP,
            max: MAX_HP,
            hurt: 0.0,
            t: 0.0,
            phase: 0.0,
            reload: 3.0,
            since_shield: 0,
            shots_fired: 0,
            volley: 0,
            swung: 1.0,
        }
    }

    /// 1 when fresh, nearly 2 when nearly beaten.
    fn rage(&self) -> f32 {
        1.0 + (1.0 - self.hp as f32 / self.max as f32) * 0.9
    }

    pub fn half(&self) -> V2 {
        match self.kind {
            MiniKind::Gargoyle => HALF,
            MiniKind::Spider => SPIDER_HALF,
            MiniKind::Jack => JACK_HALF,
            MiniKind::Serpent => SERPENT_HALF,
            MiniKind::Apprentice => APPRENTICE_HALF,
        }
    }

    /// The serpent's neck, from over the lintel down to the back of its head, as
    /// points along a curve that sways.
    pub fn neck(&self) -> [V2; 10] {
        let anchor = V2::new(WORLD_W / 2.0 + 20.0 * (self.phase * 0.9 - 0.8).sin(), crate::game::FIELD_TOP - 1.0);
        let end = self.p - V2::new(0.0, SERPENT_HALF.y * 0.6);
        let mid = (anchor + end) * 0.5;
        let d = end - anchor;
        let side = V2::new(-d.y, d.x).norm();
        let ctrl = mid + side * (5.0 * (self.t * 1.4).sin());
        let mut out = [anchor; 10];
        for (i, q) in out.iter_mut().enumerate() {
            let k = i as f32 / 9.0;
            *q = anchor * ((1.0 - k) * (1.0 - k)) + ctrl * (2.0 * k * (1.0 - k)) + end * (k * k);
        }
        out
    }

    pub fn bounds(&self) -> (V2, V2) {
        (self.p - self.half(), self.p + self.half())
    }

    /// Wing angle for drawing: -1 (up) to 1 (down).
    pub fn flap(&self) -> f32 {
        (self.t * 6.0 * self.rage()).sin()
    }

    /// Is it about to spit (eyes glowing)?
    pub fn aiming(&self) -> bool {
        self.reload < AIM_TIME || self.volley > 0
    }

    /// Move for `dt`; while `live`, spit at `target` when reloaded.
    pub fn update(&mut self, dt: f32, live: bool, target: V2, shots: &mut Vec<Shot>) {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        self.phase += dt * 0.45 * self.rage();
        self.p = match self.kind {
            MiniKind::Gargoyle => V2::new(WORLD_W / 2.0 + SWAY * self.phase.sin(), HOVER_Y + 1.5 * (self.t * 1.3).sin()),
            // creeping along the lintel, letting its thread out and drawing it in
            MiniKind::Spider => V2::new(
                WORLD_W / 2.0 + (SWAY + 2.0) * (self.phase * 0.8).sin(),
                11.5 + 2.2 * (self.t * 0.7 * self.rage()).sin(),
            ),
            // marching in ticks, like a clock's hand: a quick step, then a pause
            // swaying on its neck, bobbing as if riding the swell
            MiniKind::Serpent => V2::new(
                WORLD_W / 2.0 + (SWAY + 1.0) * (self.phase * 0.9).sin(),
                12.0 + 2.5 * (self.t * 1.1 * self.rage()).sin(),
            ),
            // his scaffold swinging along on its ropes
            MiniKind::Apprentice => V2::new(WORLD_W / 2.0 + (SWAY - 3.0) * (self.phase * 0.7).sin(), 11.5 + 2.5 * (self.t * 0.6).sin()),
            MiniKind::Jack => {
                let ticks = self.phase * 2.2;
                let step = (ticks.fract() * 4.0).min(1.0);
                let stepped = ticks.floor() + step * step * (3.0 - 2.0 * step);
                V2::new(WORLD_W / 2.0 + SWAY * (stepped / 2.2).sin(), HOVER_Y - 0.6 * (1.0 - (step * std::f32::consts::PI).sin().abs()))
            }
        };
        self.swung += dt;
        if !live {
            self.volley = 0;
            return;
        }
        self.reload -= dt;
        if self.reload > 0.0 {
            return;
        }
        // (the Jack's chimes ring out from the little bell at its side)
        let mouth = self.p
            + match self.kind {
                MiniKind::Jack => V2::new(3.0, 0.8) * JACK_SCALE,
                MiniKind::Serpent => V2::new(0.0, 2.6),
                MiniKind::Apprentice => V2::new(0.5, -1.5) * APPRENTICE_SCALE,
                _ => V2::new(0.0, 1.0),
            };
        let (kind, speed) = match self.kind {
            MiniKind::Gargoyle => (ShotKind::Fire, SHOT_SPEED),
            MiniKind::Spider => (ShotKind::Web, WEB_SPEED),
            MiniKind::Jack => (ShotKind::Chime, CHIME_SPEED),
            MiniKind::Serpent => (ShotKind::Spray, SPRAY_SPEED),
            MiniKind::Apprentice => (ShotKind::Brick, THROW_SPEED),
        };
        shots.push(Shot { p: mouth, v: (target - mouth).norm() * speed, kind });
        self.shots_fired += 1;
        self.swung = 0.0;
        if self.kind == MiniKind::Jack {
            // on the hour: a volley, longer when hurt, each chime aimed afresh
            if self.volley == 0 {
                self.volley = 1 + (self.rage() > 1.3) as u8 + (self.rage() > 1.6) as u8;
            }
            self.volley -= 1;
            self.reload = if self.volley > 0 { CHIME_GAP } else { 3.6 / self.rage() };
            log::info!("jack strikes at {:.1},{:.1} ({} more)", target.x, target.y, self.volley);
            return;
        }
        // a little uneven, so the rhythm can't be learned to the frame
        let wobble = [0.0, 0.5, -0.3, 0.8, 0.2][self.shots_fired as usize % 5];
        self.reload = (2.4 + wobble) / self.rage();
        log::info!("{:?} spits at {:.1},{:.1}", self.kind, target.x, target.y);
    }

    pub fn hit(&mut self) -> Hurt {
        if self.hurt > 0.0 {
            return Hurt::Shrugged;
        }
        self.hp = self.hp.saturating_sub(1);
        self.hurt = HURT_TIME;
        if self.hp == 0 {
            return Hurt::Dead;
        }
        self.since_shield += 1;
        if self.since_shield >= SHIELD_EVERY {
            self.since_shield = 0;
            return Hurt::Shield;
        }
        Hurt::Hurt
    }
}

/// The Storm Cloud's core (the target), and how far its puffs spread (drawing only:
/// the ball flies under the cloud).
pub const CORE_R: f32 = 3.0;
pub const CLOUD_R: f32 = 10.0;
const STORM_HP: u8 = 12;
/// Hits it takes between rains.
const RAIN_EVERY: u8 = 3;
/// Seconds between bolts (fewer when hurt), how long a bolt's mark shows before it
/// strikes, and how long the strike flashes.
const BOLT_EVERY: f32 = 4.5;
pub const BOLT_WARN: f32 = 1.2;
const BOLT_FLASH: f32 = 0.3;
/// Half the marked stretch of the ring, as an angle.
pub const BOLT_SPAN: f32 = 0.22;
const STORM_ROAM: V2 = V2::new(7.0, 5.5);

#[derive(Clone, Debug)]
pub struct Storm {
    /// Where it hangs now; it drifts round `home`, the middle of the roof.
    pub centre: V2,
    home: V2,
    roam: f32,
    pub hp: u8,
    pub max: u8,
    pub hurt: f32,
    pub t: f32,
    /// A bolt on its way: where round the ring (screen angle), and seconds until it
    /// strikes.
    pub charging: Option<(f32, f32)>,
    /// The bolt that just struck: where, and seconds of flash left.
    pub strike: Option<(f32, f32)>,
    next_bolt: f32,
    since_rain: u8,
}

impl Storm {
    pub fn new(centre: V2) -> Storm {
        Storm {
            centre,
            home: centre,
            roam: 0.0,
            hp: STORM_HP,
            max: STORM_HP,
            hurt: 0.0,
            t: 0.0,
            charging: None,
            strike: None,
            next_bolt: 3.0,
            since_rain: 0,
        }
    }

    fn rage(&self) -> f32 {
        1.0 + (1.0 - self.hp as f32 / self.max as f32) * 0.8
    }

    /// The cloud's puffs, for drawing: centre and radius each, slowly churning.
    pub fn puffs(&self) -> [(V2, f32); 8] {
        let mut out = [(self.centre, 5.0); 8];
        for (k, puff) in out.iter_mut().enumerate().skip(1) {
            let a = k as f32 * std::f32::consts::TAU / 7.0 + self.t * 0.15;
            let d = 5.2 + 1.2 * (k as f32 * 1.7).sin();
            *puff = (self.centre + V2::new(a.cos(), a.sin()) * d, 3.6 + 0.6 * (self.t * 0.8 + k as f32).sin());
        }
        out
    }

    /// The lightning bolt while it flashes: a jagged line from the cloud to the ring
    /// (centre `c`, radius `r`).
    pub fn bolt(&self, c: V2, r: f32) -> Option<Vec<V2>> {
        let (a, _) = self.strike?;
        let end = c + V2::new(a.cos(), a.sin()) * r;
        let d = end - self.centre;
        let side = V2::new(-d.y, d.x).norm();
        let n = 7;
        Some(
            (0..=n)
                .map(|i| {
                    let k = i as f32 / n as f32;
                    let jag = if i == 0 || i == n { 0.0 } else { ((a * 37.0 + i as f32 * 2.3).sin() * 43758.5).fract() * 4.0 - 2.0 };
                    self.centre + d * k + side * jag
                })
                .collect(),
        )
    }

    /// Drift for `dt`; while `live`, aim bolts at `target`, an angle round the ring.
    /// Returns where a bolt struck this step.
    pub fn update(&mut self, dt: f32, live: bool, target: f32) -> Option<f32> {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        self.strike = self.strike.map(|(a, f)| (a, f - dt)).filter(|&(_, f)| f > 0.0);
        self.roam += dt * 0.3 * self.rage();
        self.centre = self.home + V2::new(STORM_ROAM.x * self.roam.sin(), STORM_ROAM.y * (self.roam * 1.4).sin());
        if !live {
            self.charging = None;
            return None;
        }
        match self.charging {
            Some((a, left)) if left <= dt => {
                self.charging = None;
                self.strike = Some((a, BOLT_FLASH));
                self.next_bolt = BOLT_EVERY / self.rage();
                log::info!("lightning strikes at {:.0} degrees", a.to_degrees());
                return Some(a);
            }
            Some((a, left)) => self.charging = Some((a, left - dt)),
            None => {
                self.next_bolt -= dt;
                if self.next_bolt <= 0.0 {
                    self.charging = Some((target, BOLT_WARN));
                    log::info!("storm marks {:.0} degrees", target.to_degrees());
                }
            }
        }
        None
    }

    /// The core was hit. `Hurt::Shield` means it rains.
    pub fn hit(&mut self) -> Hurt {
        if self.hurt > 0.0 {
            return Hurt::Shrugged;
        }
        self.hp = self.hp.saturating_sub(1);
        self.hurt = HURT_TIME;
        if self.hp == 0 {
            return Hurt::Dead;
        }
        self.since_rain += 1;
        if self.since_rain >= RAIN_EVERY {
            self.since_rain = 0;
            return Hurt::Shield;
        }
        Hurt::Hurt
    }
}

/// The Rook's nest: the twig wall's radius, how many twigs, how thick; the crow's
/// body.
pub const NEST_R: f32 = 7.5;
pub const TWIGS: usize = 14;
pub const TWIG_TH: f32 = 0.7;
pub const CROW_R: f32 = 2.6;
const ROOK_HP: u8 = 14;
/// Hits it takes between patching its nest, and how many twigs it puts back.
const REBUILD_EVERY: u8 = 3;
const REBUILD_TWIGS: usize = 4;
/// Seconds between swoops (fewer when hurt), how long it crouches first, how long
/// a swoop takes out and back, and how often it hops about the nest.
const SWOOP_EVERY: f32 = 6.0;
pub const SWOOP_WARN: f32 = 1.1;
const SWOOP_TIME: f32 = 1.6;
const HOP_EVERY: f32 = 1.1;
const HOP_TIME: f32 = 0.25;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Flight {
    Nesting,
    /// Glaring at angle `a` round the ring, `left` seconds before it goes.
    Crouching { a: f32, left: f32 },
    /// Out to angle `a` on the ring and back, `t` 0 to 1; `struck` once it has hit
    /// the paddle on this swoop.
    Swooping { a: f32, t: f32, from: V2, struck: bool },
}

#[derive(Clone, Debug)]
pub struct Rook {
    pub nest: V2,
    pub p: V2,
    /// Which way it faces (screen angle).
    pub facing: f32,
    pub hp: u8,
    pub max: u8,
    pub hurt: f32,
    pub t: f32,
    pub twigs: [bool; TWIGS],
    pub flight: Flight,
    next_swoop: f32,
    hop: (V2, V2, f32),
    hops: u32,
    since_rebuild: u8,
}

impl Rook {
    pub fn new(nest: V2) -> Rook {
        Rook {
            nest,
            p: nest,
            facing: std::f32::consts::FRAC_PI_2,
            hp: ROOK_HP,
            max: ROOK_HP,
            hurt: 0.0,
            t: 0.0,
            twigs: [true; TWIGS],
            flight: Flight::Nesting,
            next_swoop: 4.0,
            hop: (nest, nest, 1.0),
            hops: 0,
            since_rebuild: 0,
        }
    }

    fn rage(&self) -> f32 {
        1.0 + (1.0 - self.hp as f32 / self.max as f32) * 0.8
    }

    /// Twig `k` as a segment: a chord of the nest's wall.
    pub fn twig(&self, k: usize) -> (V2, V2) {
        let step = std::f32::consts::TAU / TWIGS as f32;
        let at = |a: f32| self.nest + V2::new(a.cos(), a.sin()) * NEST_R;
        (at(k as f32 * step), at((k + 1) as f32 * step))
    }

    /// Fly and hop for `dt`; while `live`, swoop at `target` (an angle round the ring
    /// of radius `ring`).
    pub fn update(&mut self, dt: f32, live: bool, ring: f32, target: f32) {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        let towards = |a: f32| self.nest + V2::new(a.cos(), a.sin()) * ring;
        match self.flight {
            Flight::Nesting => {
                let (from, to, k) = self.hop;
                let k = (k + dt / HOP_TIME).min(1.0);
                self.p = from + (to - from) * (k * k * (3.0 - 2.0 * k));
                self.hop.2 = k;
                if (self.t / HOP_EVERY).floor() != ((self.t - dt) / HOP_EVERY).floor() {
                    self.hops += 1;
                    let a = self.hops as f32 * 2.4;
                    let to = self.nest + V2::new(a.cos(), a.sin()) * (2.5 * ((self.hops as f32 * 1.7).sin() * 0.5 + 0.5));
                    if to != self.p {
                        self.facing = (to - self.p).y.atan2((to - self.p).x);
                    }
                    self.hop = (self.p, to, 0.0);
                }
                if live {
                    self.next_swoop -= dt;
                    if self.next_swoop <= 0.0 {
                        self.flight = Flight::Crouching { a: target, left: SWOOP_WARN };
                        let d = towards(target) - self.p;
                        self.facing = d.y.atan2(d.x);
                        log::info!("rook crouches towards {:.0} degrees", target.to_degrees());
                    }
                }
            }
            Flight::Crouching { a, left } => {
                if !live {
                    self.flight = Flight::Nesting;
                } else if left <= dt {
                    self.flight = Flight::Swooping { a, t: 0.0, from: self.p, struck: false };
                    log::info!("rook swoops");
                } else {
                    self.flight = Flight::Crouching { a, left: left - dt };
                }
            }
            Flight::Swooping { a, t, from, struck } => {
                let t = t + dt / SWOOP_TIME;
                let out = if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 };
                let out = out.clamp(0.0, 1.0);
                let end = towards(a);
                self.p = from + (end - from) * (out * out * (3.0 - 2.0 * out));
                let d = if t < 0.5 { end - from } else { from - end };
                self.facing = d.y.atan2(d.x);
                if t >= 1.0 {
                    self.p = from;
                    self.flight = Flight::Nesting;
                    self.hop = (from, from, 1.0);
                    self.next_swoop = SWOOP_EVERY / self.rage();
                } else {
                    self.flight = Flight::Swooping { a, t, from, struck };
                }
            }
        }
    }

    pub fn hit(&mut self) -> Hurt {
        if self.hurt > 0.0 {
            return Hurt::Shrugged;
        }
        self.hp = self.hp.saturating_sub(1);
        self.hurt = HURT_TIME;
        if self.hp == 0 {
            return Hurt::Dead;
        }
        self.since_rebuild += 1;
        if self.since_rebuild >= REBUILD_EVERY {
            self.since_rebuild = 0;
            return Hurt::Shield;
        }
        Hurt::Hurt
    }

    /// Put back up to a few missing twigs (not where `clear` says a ball is).
    pub fn rebuild(&mut self, clear: impl Fn(V2, V2) -> bool) -> usize {
        let mut put = 0;
        for k in 0..TWIGS {
            if put < REBUILD_TWIGS && !self.twigs[k] && clear(self.twig(k).0, self.twig(k).1) {
                self.twigs[k] = true;
                put += 1;
            }
        }
        put
    }
}

/// The Great Bell: its mouth's radius (the target), how far it swings either way,
/// and how fast.
pub const BELL_R: f32 = 4.2;
const BELL_HP: u8 = 16;
const BELL_SWING: f32 = 5.0;
const SWING_RATE: f32 = 1.3;
/// How fast its swing turns round the roof, radians per second.
const AXIS_TURN: f32 = 0.12;
/// Seconds between tolls (fewer when hurt), and how long it shivers first.
const TOLL_EVERY: f32 = 4.5;
pub const TOLL_WARN: f32 = 1.1;
/// The ring of sound: how fast it spreads, half of each loud arc (radians, front
/// and back along the swing), and how thick it is.
pub const WAVE_SPEED: f32 = 14.0;
pub const WAVE_SPAN: f32 = 0.7;
pub const WAVE_TH: f32 = 1.3;
/// Hits it takes between lurches (a quarter turn and a double toll).
const LURCH_EVERY: u8 = 3;

/// A ring of sound spreading from where the bell tolled.
#[derive(Clone, Copy, Debug)]
pub struct Wave {
    pub c: V2,
    pub r: f32,
    /// The swing's direction when it tolled: the loud arcs face this way and back.
    pub axis: f32,
    /// Has it stunned the paddle already (once per wave).
    pub struck: bool,
}

impl Wave {
    /// Is point `q` in one of the loud arcs' directions from the wave's centre?
    pub fn loud_towards(&self, q: V2) -> bool {
        loud(self.c, self.axis, q)
    }
}

/// Is `q` within the loud arcs (along `axis`, front and back) seen from `c`?
pub fn loud(c: V2, axis: f32, q: V2) -> bool {
    let d = q - c;
    let off = (d.y.atan2(d.x) - axis).rem_euclid(std::f32::consts::PI);
    off < WAVE_SPAN || off > std::f32::consts::PI - WAVE_SPAN
}

/// Where a ray from `from` at angle `a` crosses the ring (centre `c`, radius `r`),
/// as an angle round the ring; `from` must be inside it.
pub fn ring_cross(from: V2, a: f32, c: V2, r: f32) -> f32 {
    let u = V2::new(a.cos(), a.sin());
    let w = from - c;
    let b = w.dot(u);
    let t = -b + (b * b - (w.dot(w) - r * r)).max(0.0).sqrt();
    let q = w + u * t;
    q.y.atan2(q.x)
}

#[derive(Clone, Debug)]
pub struct Bell {
    home: V2,
    /// Where its mouth hangs now, swung along `axis`.
    pub p: V2,
    pub axis: f32,
    axis_to: f32,
    pub swing: f32,
    pub hp: u8,
    pub max: u8,
    pub hurt: f32,
    pub t: f32,
    /// Shivering before a toll: seconds left.
    pub tolling: Option<f32>,
    next_toll: f32,
    /// Extra tolls to follow quickly (after a lurch).
    queued: u8,
    pub waves: Vec<Wave>,
    since_lurch: u8,
}

impl Bell {
    pub fn new(home: V2) -> Bell {
        Bell {
            home,
            p: home,
            axis: 0.0,
            axis_to: 0.0,
            swing: 0.0,
            hp: BELL_HP,
            max: BELL_HP,
            hurt: 0.0,
            t: 0.0,
            tolling: None,
            next_toll: 3.5,
            queued: 0,
            waves: Vec::new(),
            since_lurch: 0,
        }
    }

    fn rage(&self) -> f32 {
        1.0 + (1.0 - self.hp as f32 / self.max as f32) * 0.8
    }

    pub fn dir(&self) -> V2 {
        V2::new(self.axis.cos(), self.axis.sin())
    }

    /// Swing for `dt` and spread its rings out to `rim`; while `live`, toll.
    /// Returns true when it tolls.
    pub fn update(&mut self, dt: f32, live: bool, rim: f32) -> bool {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        self.axis_to += dt * AXIS_TURN * self.rage();
        self.axis += (self.axis_to - self.axis) * (dt * 3.0).min(1.0);
        // it swings harder while it shivers
        let hard = if self.tolling.is_some() { 1.6 } else { 1.0 };
        self.swing += dt * SWING_RATE * self.rage().sqrt() * hard;
        self.p = self.home + self.dir() * (BELL_SWING * self.swing.sin());
        for w in self.waves.iter_mut() {
            w.r += WAVE_SPEED * dt;
        }
        self.waves.retain(|w| w.r < rim + 2.0);
        if !live {
            self.tolling = None;
            return false;
        }
        match self.tolling {
            Some(left) if left <= dt => {
                self.tolling = None;
                self.waves.push(Wave { c: self.p, r: BELL_R, axis: self.axis, struck: false });
                self.next_toll = if self.queued > 0 { 1.2 } else { TOLL_EVERY / self.rage() };
                self.queued = self.queued.saturating_sub(1);
                log::info!("the bell tolls along {:.0} degrees", self.axis.to_degrees());
                return true;
            }
            Some(left) => self.tolling = Some(left - dt),
            None => {
                self.next_toll -= dt;
                if self.next_toll <= 0.0 {
                    self.tolling = Some(TOLL_WARN);
                }
            }
        }
        false
    }

    /// Its mouth was hit. Every few hits it lurches a quarter turn and tolls twice.
    pub fn hit(&mut self) -> Hurt {
        if self.hurt > 0.0 {
            return Hurt::Shrugged;
        }
        self.hp = self.hp.saturating_sub(1);
        self.hurt = HURT_TIME;
        if self.hp == 0 {
            return Hurt::Dead;
        }
        self.since_lurch += 1;
        if self.since_lurch >= LURCH_EVERY {
            self.since_lurch = 0;
            self.axis_to += std::f32::consts::FRAC_PI_2;
            self.queued = 1;
            if self.tolling.is_none() {
                self.next_toll = self.next_toll.min(0.6);
            }
            log::info!("the bell lurches");
            return Hurt::Shield;
        }
        Hurt::Hurt
    }
}

/// The Mason: his size (the target), and how far he works about inside his fort.
pub const MASON_R: f32 = 3.0;
const MASON_HP: u8 = 20;
const MASON_ROAM: V2 = V2::new(5.5, 1.2);
/// Seconds between lobs (fewer when hurt), how long a brick is in the air (the
/// mark on the ring shows meanwhile), and half the marked stretch, as an angle.
const LOB_EVERY: f32 = 4.0;
pub const LOB_TIME: f32 = 1.3;
pub const LOB_SPAN: f32 = 0.2;
/// Seconds between putting bricks back (fewer when hurt), and hits between hurrying
/// (three at once).
const BUILD_EVERY: f32 = 3.0;
const HURRY_EVERY: u8 = 4;

/// A brick in the air: where round the ring it lands, seconds left, where from.
#[derive(Clone, Copy, Debug)]
pub struct Lob {
    pub a: f32,
    pub left: f32,
    pub from: V2,
}

impl Lob {
    /// Where it is now, and how high (0 at either end, 1 at the top of its arc).
    pub fn at(&self, c: V2, r: f32) -> (V2, f32) {
        let k = 1.0 - self.left / LOB_TIME;
        let to = c + V2::new(self.a.cos(), self.a.sin()) * r;
        (self.from + (to - self.from) * k, 4.0 * k * (1.0 - k))
    }
}

#[derive(Clone, Debug)]
pub struct Mason {
    home: V2,
    pub p: V2,
    pub hp: u8,
    pub max: u8,
    pub hurt: f32,
    pub t: f32,
    pub lobs: Vec<Lob>,
    next_lob: f32,
    next_build: f32,
    /// Seconds since he last laid a brick (his trowel's flourish).
    pub laid: f32,
    lobbed: u32,
    since_hurry: u8,
}

impl Mason {
    pub fn new(home: V2) -> Mason {
        Mason {
            home,
            p: home,
            hp: MASON_HP,
            max: MASON_HP,
            hurt: 0.0,
            t: 0.0,
            lobs: Vec::new(),
            next_lob: 3.0,
            next_build: BUILD_EVERY,
            laid: 1.0,
            lobbed: 0,
            since_hurry: 0,
        }
    }

    fn rage(&self) -> f32 {
        1.0 + (1.0 - self.hp as f32 / self.max as f32) * 0.8
    }

    /// Work for `dt`; while `live`, lob at `target` (an angle round the ring). Returns
    /// where bricks landed this step, and whether it is time to put one back.
    pub fn update(&mut self, dt: f32, live: bool, target: f32) -> (Vec<f32>, bool) {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        self.laid += dt;
        self.p = self.home + V2::new(MASON_ROAM.x * (self.t * 0.5).sin(), MASON_ROAM.y * (self.t * 0.8).sin());
        if !live {
            self.lobs.clear();
            return (Vec::new(), false);
        }
        let mut landed = Vec::new();
        for l in self.lobs.iter_mut() {
            l.left -= dt;
            if l.left <= 0.0 {
                landed.push(l.a);
            }
        }
        self.lobs.retain(|l| l.left > 0.0);
        self.next_lob -= dt;
        if self.next_lob <= 0.0 {
            self.lobbed += 1;
            self.lobs.push(Lob { a: target, left: LOB_TIME, from: self.p });
            if self.hp <= self.max / 2 {
                // a second, wide to one side or the other
                let wide = if self.lobbed % 2 == 0 { 1.0 } else { -1.0 };
                self.lobs.push(Lob { a: target + wide * 1.1, left: LOB_TIME, from: self.p });
            }
            self.next_lob = LOB_EVERY / self.rage();
            log::info!("mason lobs at {:.0} degrees", target.to_degrees());
        }
        self.next_build -= dt;
        let build = self.next_build <= 0.0;
        if build {
            self.next_build = BUILD_EVERY / self.rage();
        }
        (landed, build)
    }

    /// He was hit. `Hurt::Shield` means he hurries: three bricks at once.
    pub fn hit(&mut self) -> Hurt {
        if self.hurt > 0.0 {
            return Hurt::Shrugged;
        }
        self.hp = self.hp.saturating_sub(1);
        self.hurt = HURT_TIME;
        if self.hp == 0 {
            return Hurt::Dead;
        }
        self.since_hurry += 1;
        if self.since_hurry >= HURRY_EVERY {
            self.since_hurry = 0;
            return Hurt::Shield;
        }
        Hurt::Hurt
    }
}

/// The Weathercock's arrow: half its length and its thickness; its rooster's radius.
pub const ARROW_HALF: f32 = 9.5;
pub const ARROW_TH: f32 = 0.8;
pub const ROOSTER_R: f32 = 3.2;
const VANE_HP: u8 = 16;
/// Hits it takes between spin-ups, how long a spin-up lasts, and how fast it turns.
const SPIN_EVERY: u8 = 4;
const SPIN_TIME: f32 = 2.5;
const SPIN_RATE: f32 = 3.2;
const DRIFT_RATE: f32 = 0.3;
/// How far it roams from the middle of the roof, across and down.
const ROAM: V2 = V2::new(6.5, 5.0);
/// A gust: how long, how hard it pushes (world units per second squared), how often.
const GUST_TIME: f32 = 1.6;
pub const GUST_PUSH: f32 = 55.0;
const GUST_EVERY: f32 = 6.0;
/// Seconds before a gust when the vane shudders, as a warning.
pub const GUST_WARN: f32 = 0.8;

#[derive(Clone, Debug)]
pub struct Weathercock {
    /// Where it stands now; it roams round `home`, the middle of the roof.
    pub centre: V2,
    home: V2,
    /// How far along its roaming figure it is.
    roam: f32,
    /// Which way the arrow points (radians, screen angle).
    pub angle: f32,
    /// How fast it is turning now (radians per second).
    pub turn: f32,
    pub hp: u8,
    pub max: u8,
    pub hurt: f32,
    pub t: f32,
    /// Seconds left in the current gust, and until the next.
    pub gust: f32,
    pub next_gust: f32,
    pub spinning: f32,
    since_spin: u8,
}

impl Weathercock {
    pub fn new(centre: V2) -> Weathercock {
        Weathercock {
            centre,
            home: centre,
            roam: 0.0,
            angle: -std::f32::consts::FRAC_PI_2,
            turn: DRIFT_RATE,
            hp: VANE_HP,
            max: VANE_HP,
            hurt: 0.0,
            t: 0.0,
            gust: 0.0,
            next_gust: GUST_EVERY,
            spinning: 0.0,
            since_spin: 0,
        }
    }

    fn rage(&self) -> f32 {
        1.0 + (1.0 - self.hp as f32 / self.max as f32) * 0.8
    }

    pub fn dir(&self) -> V2 {
        V2::new(self.angle.cos(), self.angle.sin())
    }

    /// The arrow as a segment: tail to head.
    pub fn arrow(&self) -> (V2, V2) {
        (self.centre - self.dir() * ARROW_HALF, self.centre + self.dir() * ARROW_HALF)
    }

    /// The wind on the roof right now: a push along the arrow during a gust.
    pub fn wind(&self) -> Option<V2> {
        (self.gust > 0.0).then(|| self.dir() * GUST_PUSH)
    }

    /// Shuddering just before a gust.
    pub fn warning(&self) -> bool {
        self.gust <= 0.0 && self.next_gust < GUST_WARN
    }

    pub fn update(&mut self, dt: f32, live: bool) {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        self.spinning = (self.spinning - dt).max(0.0);
        // turn rate eases towards drifting or spinning, so the arrow never jumps
        let want = if self.spinning > 0.0 { SPIN_RATE } else { DRIFT_RATE * self.rage() };
        self.turn += (want - self.turn) * (dt * 3.0).min(1.0);
        // it holds still while it blows
        let turning = if self.gust > 0.0 && self.spinning <= 0.0 { 0.0 } else { self.turn };
        self.angle = (self.angle + turning * dt).rem_euclid(std::f32::consts::TAU);
        // it glides round the middle of the roof in a slow figure, faster when hurt
        // (and stays put while it blows)
        if self.gust <= 0.0 {
            self.roam += dt * 0.35 * self.rage();
        }
        self.centre = self.home + V2::new(ROAM.x * self.roam.sin(), ROAM.y * (self.roam * 1.6).sin());
        if !live {
            return;
        }
        if self.gust > 0.0 {
            self.gust = (self.gust - dt).max(0.0);
        } else {
            self.next_gust -= dt;
            if self.next_gust <= 0.0 {
                self.gust = GUST_TIME;
                self.next_gust = GUST_EVERY / self.rage();
                log::info!("weathercock gust towards {:.0} degrees", self.angle.to_degrees());
            }
        }
    }

    /// The rooster was hit.
    pub fn hit(&mut self) -> Hurt {
        if self.hurt > 0.0 {
            return Hurt::Shrugged;
        }
        self.hp = self.hp.saturating_sub(1);
        self.hurt = HURT_TIME;
        if self.hp == 0 {
            return Hurt::Dead;
        }
        self.since_spin += 1;
        if self.since_spin >= SPIN_EVERY {
            self.since_spin = 0;
            self.spinning = SPIN_TIME;
            log::info!("weathercock spins up");
        }
        Hurt::Hurt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_between_the_walls() {
        let mut g = MiniBoss::new(MiniKind::Gargoyle);
        let mut shots = Vec::new();
        for _ in 0..20_000 {
            g.update(1.0 / 120.0, true, V2::new(50.0, 50.0), &mut shots);
            let (min, max) = g.bounds();
            assert!(min.x >= crate::game::FIELD_X0 && max.x <= crate::game::FIELD_X1 && min.y >= crate::game::FIELD_TOP);
        }
        assert!(shots.len() > 20);
    }

    #[test]
    fn weathercock_gusts_spins_and_falls() {
        let mut w = Weathercock::new(V2::new(50.0, 29.0));
        let mut gusts = 0;
        for _ in 0..(30.0 * 120.0) as usize {
            let before = w.gust > 0.0;
            w.update(1.0 / 120.0, true);
            if !before && w.gust > 0.0 {
                gusts += 1;
                assert!(w.wind().is_some());
            }
        }
        assert!(gusts >= 4, "{gusts} gusts in 30 s");
        let mut spins = 0;
        while w.hp > 0 {
            w.hurt = 0.0;
            w.spinning = 0.0;
            let was = w.spinning > 0.0;
            w.hit();
            if !was && w.spinning > 0.0 {
                spins += 1;
            }
        }
        assert_eq!(spins, 3);
    }

    #[test]
    fn the_storm_marks_then_strikes() {
        let mut s = Storm::new(V2::new(50.0, 28.0));
        let (mut marks, mut strikes) = (0, 0);
        for _ in 0..(30.0 * 120.0) as usize {
            let was = s.charging.is_some();
            if let Some(a) = s.update(1.0 / 120.0, true, 1.0) {
                assert_eq!(a, 1.0);
                assert!(s.bolt(V2::new(50.0, 28.0), 26.0).is_some());
                strikes += 1;
            }
            if !was && s.charging.is_some() {
                marks += 1;
            }
        }
        assert!(strikes >= 5 && marks >= strikes, "{marks} marks, {strikes} strikes");
        let mut rains = 0;
        while s.hp > 0 {
            s.hurt = 0.0;
            if s.hit() == Hurt::Shield {
                rains += 1;
            }
        }
        assert_eq!(rains, 3);
    }

    #[test]
    fn the_rook_crouches_swoops_and_comes_home() {
        let nest = V2::new(50.0, 28.0);
        let mut r = Rook::new(nest);
        let (mut swoops, mut furthest) = (0, 0.0f32);
        for _ in 0..(30.0 * 120.0) as usize {
            let was = matches!(r.flight, Flight::Swooping { .. });
            r.update(1.0 / 120.0, true, 26.0, 0.5);
            if !was && matches!(r.flight, Flight::Swooping { .. }) {
                swoops += 1;
            }
            furthest = furthest.max((r.p - nest).len());
            if r.flight == Flight::Nesting {
                // at home it stays inside its twig wall
                assert!((r.p - nest).len() + CROW_R < NEST_R - TWIG_TH, "{:?}", r.p);
            }
        }
        assert!(swoops >= 3, "{swoops} swoops");
        assert!(furthest > 25.0, "never reached the ring: {furthest}");
        r.twigs = [false; TWIGS];
        r.hp = 10;
        assert_eq!((r.hit(), r.hit()), (Hurt::Hurt, Hurt::Shrugged));
        assert_eq!(r.rebuild(|_, _| true), 4);
    }

    #[test]
    fn the_jack_strikes_volleys_when_hurt() {
        let mut j = MiniBoss::new(MiniKind::Jack);
        let mut shots = Vec::new();
        for _ in 0..(20.0 * 120.0) as usize {
            j.update(1.0 / 120.0, true, V2::new(50.0, 50.0), &mut shots);
            let (min, max) = j.bounds();
            assert!(min.x >= crate::game::FIELD_X0 && max.x <= crate::game::FIELD_X1);
        }
        let calm = shots.len();
        assert!(calm >= 4 && shots.iter().all(|s| s.kind == ShotKind::Chime), "{calm} chimes");
        shots.clear();
        j.hp = 2;
        for _ in 0..(20.0 * 120.0) as usize {
            j.update(1.0 / 120.0, true, V2::new(50.0, 50.0), &mut shots);
        }
        assert!(shots.len() > calm * 2, "{} chimes when hurt, {calm} calm", shots.len());
    }

    #[test]
    fn the_bell_tolls_loud_along_its_swing() {
        let c = V2::new(50.0, 28.0);
        let mut b = Bell::new(c);
        let mut tolls = 0;
        for _ in 0..(30.0 * 120.0) as usize {
            if b.update(1.0 / 120.0, true, 33.0) {
                tolls += 1;
            }
            assert!((b.p - c).len() <= BELL_SWING + 1e-3);
        }
        assert!(tolls >= 5, "{tolls} tolls");
        let w = Wave { c, r: 10.0, axis: 0.0, struck: false };
        assert!(w.loud_towards(c + V2::new(10.0, 1.0)) && w.loud_towards(c - V2::new(10.0, 1.0)));
        assert!(!w.loud_towards(c + V2::new(0.0, 10.0)));
        // straight across from the middle, the ring is crossed where you'd expect
        assert!((ring_cross(c, 0.5, c, 26.0) - 0.5).abs() < 1e-4);
        let mut lurches = 0;
        while b.hp > 0 {
            b.hurt = 0.0;
            if b.hit() == Hurt::Shield {
                lurches += 1;
            }
        }
        assert_eq!(lurches, (BELL_HP - 1) / LURCH_EVERY);
    }

    #[test]
    fn the_mason_lobs_and_builds() {
        let c = V2::new(50.0, 28.0);
        let mut m = Mason::new(c);
        let (mut landed, mut builds) = (0, 0);
        for _ in 0..(30.0 * 120.0) as usize {
            let (l, b) = m.update(1.0 / 120.0, true, 1.0);
            landed += l.len();
            builds += b as usize;
            assert!((m.p.x - c.x).abs() <= MASON_ROAM.x + 1e-3 && (m.p.y - c.y).abs() <= MASON_ROAM.y + 1e-3);
        }
        assert!(landed >= 6 && builds >= 9, "{landed} landed, {builds} builds");
        // hurt, he lobs two at once
        m.hp = 5;
        m.lobs.clear();
        while m.lobs.is_empty() {
            m.update(1.0 / 120.0, true, 1.0);
        }
        assert_eq!(m.lobs.len(), 2);
        let (at, high) = m.lobs[0].at(c, 26.0);
        assert!((at - m.lobs[0].from).len() < 1.0 && high < 0.1);
    }

    #[test]
    fn shields_every_third_hit_and_dies() {
        let mut g = MiniBoss::new(MiniKind::Gargoyle);
        let mut seen = Vec::new();
        while g.hp > 0 {
            g.hurt = 0.0;
            seen.push(g.hit());
        }
        assert_eq!(seen.iter().filter(|&&h| h == Hurt::Shield).count(), 3);
        assert_eq!(seen.last(), Some(&Hurt::Dead));
        g.hp = 5;
        g.hurt = 0.1;
        assert_eq!(g.hit(), Hurt::Shrugged);
    }
}
