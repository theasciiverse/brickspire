//! The towers, each a theme and a list of floors; floor layouts as small text grids,
//! one character per brick.
//!
//! `.` empty; colours `w` white, `o` orange, `c` cyan, `g` green, `r` red, `b` blue,
//! `m` magenta, `y` yellow; `S` steel (takes several hits); `G` gilt (can't be broken,
//! and doesn't count towards clearing the floor); `s` stone (what the Gargoyle lays);
//! `v` vine (slides along its row); `W` web (what the spider spins); `k` clockwork
//! (winds itself back in a while after it is broken); `i` ice (two hits, and the
//! ball skids off it at a shallower angle).
//!
//! Floors are numbered straight through all the towers (0 is the first tower's ground
//! floor); after the last tower the climb starts again from the first.
//!
//! A boss floor is cleared by beating its boss, not its bricks: the balcony puts the
//! paddle on an arc against the Gargoyle, the roof on a full ring round the roof's boss.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FloorKind {
    /// A flat floor: paddle on a straight line, clear the bricks.
    Plain,
    /// The mini-boss balcony: paddle on an arc, beat the Gargoyle.
    Balcony,
    /// The roof: looking straight down, paddle right round a ring, beat its boss.
    Roof(RoofBoss),
}

/// Who waits on a roof.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RoofBoss {
    Storm,
    Rook,
    Bell,
    Weathercock,
}

/// Who waits on a balcony.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MiniKind {
    Gargoyle,
    Spider,
    Jack,
    Serpent,
}

/// A tower's look: sky, stone, the ground far below.
pub struct Theme {
    /// Sky at the top of the screen and at the bottom, and how many stars show
    /// (0 none, 1 a full night sky).
    pub sky: [[u8; 3]; 2],
    pub stars: f32,
    /// The tower's wall behind the bricks, and its mortar.
    pub wall: [u8; 3],
    pub mortar: [u8; 3],
    /// The ground seen from the roof: two field colours.
    pub fields: [[u8; 3]; 2],
    /// Ivy growing over the walls.
    pub ivy: bool,
    /// Rain streaking across the sky.
    pub rain: bool,
}

pub struct Tower {
    pub name: &'static str,
    pub theme: Theme,
    pub mini: MiniKind,
    pub floors: &'static [Floor],
}

pub const TOWERS: &[Tower] = &[
    Tower {
        name: "The Old Keep",
        theme: Theme {
            sky: [[6, 8, 26], [24, 14, 44]],
            stars: 1.0,
            wall: [26, 28, 48],
            mortar: [14, 16, 30],
            fields: [[10, 18, 14], [24, 28, 17]],
            ivy: false,
            rain: false,
        },
        mini: MiniKind::Gargoyle,
        floors: OLD_KEEP,
    },
    Tower {
        name: "The Ivy Tower",
        theme: Theme {
            sky: [[34, 26, 64], [150, 78, 70]],
            stars: 0.35,
            wall: [34, 40, 38],
            mortar: [18, 22, 22],
            fields: [[8, 26, 12], [14, 40, 16]],
            ivy: true,
            rain: false,
        },
        mini: MiniKind::Spider,
        floors: IVY_TOWER,
    },
    Tower {
        name: "The Bell Tower",
        theme: Theme {
            sky: [[52, 34, 78], [235, 128, 62]],
            stars: 0.1,
            wall: [72, 54, 38],
            mortar: [40, 29, 21],
            fields: [[58, 46, 20], [96, 76, 30]],
            ivy: false,
            rain: false,
        },
        mini: MiniKind::Jack,
        floors: BELL_TOWER,
    },
    Tower {
        name: "The Sea Tower",
        theme: Theme {
            sky: [[16, 20, 32], [64, 74, 88]],
            stars: 0.0,
            wall: [64, 66, 76],
            mortar: [38, 40, 48],
            fields: [[12, 34, 58], [26, 60, 88]],
            ivy: false,
            rain: true,
        },
        mini: MiniKind::Serpent,
        floors: SEA_TOWER,
    },
];

/// How many floors in all the towers together.
pub fn total() -> usize {
    TOWERS.iter().map(|t| t.floors.len()).sum()
}

/// Which tower floor `n` is in, and that tower's first floor.
pub fn tower_of(n: usize) -> (usize, usize) {
    let lap = n / total() * total();
    let mut first = lap;
    for (i, t) in TOWERS.iter().enumerate() {
        if n < first + t.floors.len() {
            return (i, first);
        }
        first += t.floors.len();
    }
    unreachable!()
}

pub fn tower(n: usize) -> &'static Tower {
    &TOWERS[tower_of(n).0]
}

/// Floor `n`, counted through all the towers.
pub fn floor(n: usize) -> &'static Floor {
    let (t, first) = tower_of(n);
    &TOWERS[t].floors[n - first]
}

pub struct Floor {
    pub name: &'static str,
    pub rows: &'static [&'static str],
    pub kind: FloorKind,
}

/// The first tower: flat floors 13 bricks across, the balcony partway up, the roof
/// on top (a few groups of bricks round the middle, where the Storm Cloud drifts).
pub const OLD_KEEP: &[Floor] = &[
    Floor {
        name: "Ground Floor",
        rows: &["SSSSSSSSSSSSS", "rrrrrrrrrrrrr", "yyyyyyyyyyyyy", "bbbbbbbbbbbbb", "mmmmmmmmmmmmm", "ggggggggggggg"],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Spire",
        rows: &[
            "......w......",
            ".....wcw.....",
            "....wcccw....",
            "...wcbbbcw...",
            "..wcbmmmbcw..",
            ".wcbmGGGmbcw.",
            "SSSSS...SSSSS",
            "ooooo...ooooo",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "Chequers",
        rows: &[
            "r.o.y.g.c.b.m",
            ".S.S.S.S.S.S.",
            "m.b.c.g.y.o.r",
            ".............",
            "GGG..ggg..GGG",
            "yyy..SSS..yyy",
            "ccc..www..ccc",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "Twin Lanterns",
        rows: &[
            "..c.......c..",
            ".cbc.....cbc.",
            "cbmbc...cbmbc",
            ".cbc..S..cbc.",
            "..c..SyS..c..",
            "......S......",
            "oo.........oo",
            "GoG.......GoG",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Gargoyle's Balcony",
        // the top rows stay open: the Gargoyle flies there, and lays its stone in row 4
        rows: &[
            ".............",
            ".............",
            ".............",
            ".............",
            "G...........G",
            "ccc..yyy..ccc",
            "..b.......b..",
        ],
        kind: FloorKind::Balcony,
    },
    Floor {
        name: "The Storm Cloud's Roof",
        // a few small groups, leaving the roof open under the cloud
        rows: &[
            ".......",
            "..cyc..",
            ".......",
            ".......",
            ".b...b.",
            ".S...S.",
            ".b...b.",
            ".......",
            ".......",
            "..cyc..",
            ".......",
        ],
        kind: FloorKind::Roof(RoofBoss::Storm),
    },
];

/// The second tower, overgrown: vines slide along their rows, the spider hangs over
/// the balcony, the rook nests on the roof (the middle of the roof is its nest).
pub const IVY_TOWER: &[Floor] = &[
    Floor {
        name: "The Ivy Gate",
        rows: &["ggggggggggggg", "yyyyyyyyyyyyy", "vvvv.....vvvv", ".............", "oooooo.oooooo", "SS.........SS"],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "Creepers",
        rows: &[
            "G...........G",
            "vvv...vvv....",
            "....vvv...vvv",
            "ggggggggggggg",
            "..vvv....vvv.",
            "rrrrrrrrrrrrr",
            "SSS.SSSSS.SSS",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Trellis",
        rows: &[
            "S.S.S.S.S.S.S",
            "ggggg...ggggg",
            ".vvvv...vvvv.",
            "ccccccccccccc",
            ".v.v.v.v.v.v.",
            "bbbbbbbbbbbbb",
            "vvvv.....vvvv",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "Overgrown",
        rows: &[
            "..ggggggggg..",
            ".gyyyyyyyyyg.",
            "gy.vvv.vvv.yg",
            "gy..SS.SS..yg",
            "gy.vvv.vvv.yg",
            ".gyyyyyyyyyg.",
            "..ggggggggg..",
            "G...........G",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Spider's Balcony",
        // the top rows stay open: the spider hangs there, and spins its web in row 4
        rows: &[
            ".............",
            ".............",
            ".............",
            ".............",
            "S...........S",
            "vvv..ggg..vvv",
            "..g.......g..",
        ],
        kind: FloorKind::Balcony,
    },
    Floor {
        name: "The Rook's Roof",
        // round the edge of the roof: the rook's nest fills the middle
        rows: &[
            ".......",
            "g.vvv.g",
            ".......",
            ".......",
            "y.....y",
            "S.....S",
            "y.....y",
            ".......",
            ".......",
            "g.vvv.g",
            ".......",
        ],
        kind: FloorKind::Roof(RoofBoss::Rook),
    },
];

/// The third tower, warm sandstone at sunset: clockwork bricks wind back in, the
/// clock's jack strikes on the balcony, the Great Bell swings on the roof (in the
/// middle, so the roof's bricks keep to the edges).
pub const BELL_TOWER: &[Floor] = &[
    Floor {
        name: "The Bell Gate",
        rows: &["ooooooooooooo", "yyyyyyyyyyyyy", "..k..k.k..k..", "rrrrr...rrrrr", ".............", "SS..S...S..SS"],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Escapement",
        rows: &[
            "S.k.k.k.k.k.S",
            "yyyyyyyyyyyyy",
            ".ooooo.ooooo.",
            "..rrr...rrr..",
            "k...........k",
            "SSS..yyy..SSS",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Clock Face",
        rows: &[
            "....yyyyy....",
            "..yy..k..yy..",
            ".y....k....y.",
            ".y....kkk..y.",
            ".y.........y.",
            "..yy.....yy..",
            "....yyyyy....",
            "G...........G",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "Gears",
        rows: &[
            ".kkk.....kkk.",
            "kooo.....oook",
            ".kkk.SSS.kkk.",
            ".....rrr.....",
            "bbbbbbbbbbbbb",
            "S.S.S...S.S.S",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Jack's Balcony",
        // the top rows stay open: the jack marches there, and winds its gears in row 4
        rows: &[
            ".............",
            ".............",
            ".............",
            ".............",
            "S...........S",
            "kkk..ooo..kkk",
            "..o.......o..",
        ],
        kind: FloorKind::Balcony,
    },
    Floor {
        name: "The Great Bell's Roof",
        // round the edge: the bell swings through the middle
        rows: &[
            ".......",
            "..kok..",
            ".......",
            "o.....o",
            "y.....y",
            ".......",
            "y.....y",
            "o.....o",
            ".......",
            "..kok..",
            ".......",
        ],
        kind: FloorKind::Roof(RoofBoss::Bell),
    },
];

/// The fourth tower, a lighthouse on a cliff in a storm: ice bricks, the sea serpent
/// on the balcony, the Weathercock on the roof (it roams the middle, so the roof's
/// bricks keep to small groups round it).
pub const SEA_TOWER: &[Floor] = &[
    Floor {
        name: "The Lamp Gate",
        rows: &["ccccccccccccc", "wwwwwwwwwwwww", "iiii.....iiii", ".............", "bbb.SSSSS.bbb", "iiiiiiiiiiiii"],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "Breakers",
        rows: &[
            "i.i.i.i.i.i.i",
            ".c.c.c.c.c.c.",
            "b.b.b.b.b.b.b",
            ".............",
            "SSiiiSSSiiiSS",
            "wwwwwwwwwwwww",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Lens",
        rows: &[
            ".....iii.....",
            "...iiyyyii...",
            "..iyywwwyyi..",
            "..iyywwwyyi..",
            "...iiyyyii...",
            ".....iii.....",
            "G...........G",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "Pack Ice",
        rows: &[
            "iiiiiiiiiiiii",
            "iSiiiiSiiiiSi",
            "ccccccccccccc",
            ".............",
            "bbbbb...bbbbb",
            "..iii...iii..",
        ],
        kind: FloorKind::Plain,
    },
    Floor {
        name: "The Serpent's Balcony",
        // the top rows stay open: the serpent's head sways there, and freezes ice in row 4
        rows: &[
            ".............",
            ".............",
            ".............",
            ".............",
            "S...........S",
            "iii..ccc..iii",
            "..c.......c..",
        ],
        kind: FloorKind::Balcony,
    },
    Floor {
        name: "The Weathercock's Roof",
        rows: &[
            ".......",
            "..cic..",
            ".......",
            ".......",
            ".i...i.",
            ".S...S.",
            ".i...i.",
            ".......",
            ".......",
            "..cic..",
            ".......",
        ],
        kind: FloorKind::Roof(RoofBoss::Weathercock),
    },
];

/// A small block for trying the roof's full-circle paddle (`--path circle`), which
/// needs its bricks in the middle of the ring.
pub const ROOF_TEST: Floor = Floor {
    name: "Roof (preview)",
    rows: &[".SS.", "SrrS", "ryyr", "yggy", "ryyr", "SrrS", ".SS."],
    kind: FloorKind::Plain,
};
