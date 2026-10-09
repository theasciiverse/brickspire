//! Power-up capsules: the classic set under our own names. A broken brick now and
//! then lets one fall (on the roof, drift outward towards the ring); catch it on the
//! paddle for its effect. Three last until the floor ends, a ball is lost or another
//! of them is caught (the plank, the mortar and the chisel); the rest act at once.

use crate::physics::V2;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Power {
    /// A longer paddle.
    Plank,
    /// The ball sticks to the paddle until fire lets it go.
    Mortar,
    /// Fire shoots a pair of chisels up from the paddle's ends to break bricks.
    Chisel,
    /// The ball slows back to its starting speed.
    Hourglass,
    /// Every ball in play becomes three.
    Flock,
    /// An extra paddle (life).
    Spare,
    /// A stair opens: on to the next floor.
    Stair,
}

/// How likely a broken brick is to let a capsule fall.
pub const DROP_CHANCE: f32 = 0.14;
/// Capsule size (half across, half down), and how fast it falls.
pub const CAP_HALF: V2 = V2::new(2.2, 1.0);
pub const FALL: f32 = 16.0;
/// Seconds between chisel volleys, and their speed.
pub const CHISEL_EVERY: f32 = 0.3;
pub const CHISEL_SPEED: f32 = 95.0;
/// Seconds a ball stays caught in mortar before it lets go by itself.
pub const MORTAR_HOLD: f32 = 3.0;
/// The plank's length against the paddle's own.
pub const PLANK_SHARE: f32 = 1.5;

impl Power {
    pub const ALL: [Power; 7] = [Power::Plank, Power::Mortar, Power::Chisel, Power::Hourglass, Power::Flock, Power::Spare, Power::Stair];

    /// How often each falls, against the others.
    fn weight(self) -> f32 {
        match self {
            Power::Plank => 20.0,
            Power::Mortar => 15.0,
            Power::Chisel => 15.0,
            Power::Hourglass => 18.0,
            Power::Flock => 18.0,
            Power::Spare => 5.0,
            Power::Stair => 4.0,
        }
    }

    /// The one a roll `k` (0 to 1) picks; `stair` says whether a stair may fall
    /// (not on boss floors).
    pub fn pick(k: f32, stair: bool) -> Power {
        let all = Power::ALL.iter().filter(|&&p| stair || p != Power::Stair);
        let total: f32 = all.clone().map(|p| p.weight()).sum();
        let mut at = k * total;
        for &p in all {
            at -= p.weight();
            if at < 0.0 {
                return p;
            }
        }
        Power::Plank
    }

    /// The letter on its capsule.
    pub fn letter(self) -> &'static str {
        match self {
            Power::Plank => "P",
            Power::Mortar => "M",
            Power::Chisel => "C",
            Power::Hourglass => "H",
            Power::Flock => "F",
            Power::Spare => "+",
            Power::Stair => "S",
        }
    }

    pub fn rgb(self) -> [u8; 3] {
        match self {
            Power::Plank => [70, 120, 245],
            Power::Mortar => [175, 175, 180],
            Power::Chisel => [235, 65, 60],
            Power::Hourglass => [250, 150, 40],
            Power::Flock => [60, 210, 230],
            Power::Spare => [225, 90, 205],
            Power::Stair => [90, 215, 85],
        }
    }

    /// What catching it says.
    pub fn told(self) -> &'static str {
        match self {
            Power::Plank => "PLANK · a longer paddle",
            Power::Mortar => "MORTAR · the ball sticks, space lets it go",
            Power::Chisel => "CHISEL · space fires",
            Power::Hourglass => "HOURGLASS · the ball slows",
            Power::Flock => "FLOCK · three balls",
            Power::Spare => "SPARE · an extra paddle",
            Power::Stair => "STAIR · on to the next floor",
        }
    }

    /// Does it last (until replaced, a ball lost, or the floor's end)?
    pub fn lasts(self) -> bool {
        matches!(self, Power::Plank | Power::Mortar | Power::Chisel)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Capsule {
    pub p: V2,
    pub v: V2,
    pub kind: Power,
}

/// A chisel flying from the paddle.
#[derive(Clone, Copy, Debug)]
pub struct Bolt {
    pub p: V2,
    pub v: V2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_power_can_fall_and_stairs_only_where_allowed() {
        let rolls: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
        for p in Power::ALL {
            assert!(rolls.iter().any(|&k| Power::pick(k, true) == p), "{p:?} never falls");
        }
        assert!(rolls.iter().all(|&k| Power::pick(k, false) != Power::Stair));
    }
}
