//! Floor layouts as small text grids, one character per brick.
//!
//! `.` empty; colours `w` white, `o` orange, `c` cyan, `g` green, `r` red, `b` blue,
//! `m` magenta, `y` yellow; `S` steel (takes several hits); `G` gilt (can't be broken,
//! and doesn't count towards clearing the floor); `s` stone (what the Gargoyle lays).
//!
//! A boss floor is cleared by beating its boss, not its bricks: the balcony puts the
//! paddle on an arc against the Gargoyle, the roof on a full ring round the Weathercock.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FloorKind {
    /// A flat floor: paddle on a straight line, clear the bricks.
    Plain,
    /// The mini-boss balcony: paddle on an arc, beat the Gargoyle.
    Balcony,
    /// The roof: looking straight down, paddle right round a ring, beat the Weathercock.
    Roof,
}

pub struct Floor {
    pub name: &'static str,
    pub rows: &'static [&'static str],
    pub kind: FloorKind,
}

/// The first tower: flat floors 13 bricks across, the balcony partway up, the roof
/// on top (its bricks a ring round the middle, where the Weathercock stands).
pub const FLOORS: &[Floor] = &[
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
        name: "The Weathercock's Roof",
        rows: &[
            "..ccc..",
            ".cbbbc.",
            "cb...bc",
            "b.....b",
            "y.....y",
            "S.....S",
            "y.....y",
            "b.....b",
            "cb...bc",
            ".cbbbc.",
            "..ccc..",
        ],
        kind: FloorKind::Roof,
    },
];

/// A small block for trying the roof's full-circle paddle (`--path circle`), which
/// needs its bricks in the middle of the ring.
pub const ROOF_TEST: Floor = Floor {
    name: "Roof (preview)",
    rows: &[".SS.", "SrrS", "ryyr", "yggy", "ryyr", "SrrS", ".SS."],
    kind: FloorKind::Plain,
};
