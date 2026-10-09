//! Floor layouts as small text grids, one character per brick.
//!
//! `.` empty; colours `w` white, `o` orange, `c` cyan, `g` green, `r` red, `b` blue,
//! `m` magenta, `y` yellow; `S` steel (takes several hits); `G` gilt (can't be broken,
//! and doesn't count towards clearing the floor); `s` stone (what the Gargoyle lays).
//!
//! A boss floor is cleared by beating its boss, not its bricks, and puts the paddle on
//! the balcony's arc.

pub struct Floor {
    pub name: &'static str,
    pub rows: &'static [&'static str],
    /// The Gargoyle's balcony: paddle on the arc, beat the boss to clear it.
    pub boss: bool,
}

/// The flat floors of the first tower, 13 bricks across.
pub const FLOORS: &[Floor] = &[
    Floor {
        name: "Ground Floor",
        rows: &["SSSSSSSSSSSSS", "rrrrrrrrrrrrr", "yyyyyyyyyyyyy", "bbbbbbbbbbbbb", "mmmmmmmmmmmmm", "ggggggggggggg"],
        boss: false,
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
        boss: false,
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
        boss: false,
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
        boss: false,
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
        boss: true,
    },
];

/// A small block for trying the roof's full-circle paddle (`--path circle`), which
/// needs its bricks in the middle of the ring.
pub const ROOF_TEST: Floor = Floor {
    name: "Roof (preview)",
    rows: &[".SS.", "SrrS", "ryyr", "yggy", "ryyr", "SrrS", ".SS."],
    boss: false,
};
