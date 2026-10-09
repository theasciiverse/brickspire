//! The bosses. The mini-boss of the first tower is the Gargoyle: a stone gargoyle come alive on the
//! balcony floor. It flaps to and fro above the bricks, spits fireballs at the paddle
//! (a hit stuns the paddle for a moment), and every few hits lays a shield of stone
//! bricks beneath itself. Those stay where they are laid as it drifts on, so the
//! field clutters up as the fight goes on. It speeds up as it is hurt.
//!
//! The first tower's roof boss is the Storm Cloud: a thundercloud drifting over the
//! roof with a glowing core, the target. The ball always flies straight under it; its
//! threat is aimed at the paddle instead: it marks a stretch of the ring, and a moment
//! later lightning strikes there and stuns the paddle if it is still in the way. Every
//! few hits it rains, and bricks grow back round it.
//!
//! The Weathercock (saved for a later tower's roof) is a giant iron weathervane in the
//! middle of the roof, roaming round it in a slow figure. Its arrow swings slowly
//! round and bats the ball away (it can't be hurt); the rooster at its hub is the
//! target. Now and then it blows a gust along
//! its arrow that bends the ball's path in flight, and every few hits it spins up and
//! sweeps the arrow round fast.

use crate::physics::V2;
use crate::game::WORLD_W;

/// Half the Gargoyle's body (its hit box), wingtip to wingtip and horn to claw.
pub const HALF: V2 = V2::new(7.0, 3.0);
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

#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub p: V2,
    pub v: V2,
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
pub struct Gargoyle {
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
}

impl Gargoyle {
    pub fn new() -> Gargoyle {
        Gargoyle {
            p: V2::new(WORLD_W / 2.0, HOVER_Y),
            hp: MAX_HP,
            max: MAX_HP,
            hurt: 0.0,
            t: 0.0,
            phase: 0.0,
            reload: 3.0,
            since_shield: 0,
            shots_fired: 0,
        }
    }

    /// 1 when fresh, nearly 2 when nearly beaten.
    fn rage(&self) -> f32 {
        1.0 + (1.0 - self.hp as f32 / self.max as f32) * 0.9
    }

    pub fn bounds(&self) -> (V2, V2) {
        (self.p - HALF, self.p + HALF)
    }

    /// Wing angle for drawing: -1 (up) to 1 (down).
    pub fn flap(&self) -> f32 {
        (self.t * 6.0 * self.rage()).sin()
    }

    /// Is it about to spit (eyes glowing)?
    pub fn aiming(&self) -> bool {
        self.reload < AIM_TIME
    }

    /// Move for `dt`; while `live`, spit at `target` when reloaded.
    pub fn update(&mut self, dt: f32, live: bool, target: V2, shots: &mut Vec<Shot>) {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        self.phase += dt * 0.45 * self.rage();
        self.p = V2::new(WORLD_W / 2.0 + SWAY * self.phase.sin(), HOVER_Y + 1.5 * (self.t * 1.3).sin());
        if !live {
            return;
        }
        self.reload -= dt;
        if self.reload <= 0.0 {
            let mouth = self.p + V2::new(0.0, 1.0);
            shots.push(Shot { p: mouth, v: (target - mouth).norm() * SHOT_SPEED });
            self.shots_fired += 1;
            // a little uneven, so the rhythm can't be learned to the frame
            let wobble = [0.0, 0.5, -0.3, 0.8, 0.2][self.shots_fired as usize % 5];
            self.reload = (2.4 + wobble) / self.rage();
            log::info!("gargoyle spits at {:.1},{:.1}", target.x, target.y);
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
        let mut g = Gargoyle::new();
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
    fn shields_every_third_hit_and_dies() {
        let mut g = Gargoyle::new();
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
