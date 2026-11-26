// SPDX-License-Identifier: GPL-3.0-or-later
//! Binary libraries made byte by byte for the tests. They are invented here and come from no
//! file of any program.

/// One record being filled.
#[derive(Clone, Copy)]
pub struct R(pub [u8; 24]);

impl R {
    pub fn new(kind: u8) -> R {
        let mut r = [0u8; 24];
        r[0] = kind;
        R(r)
    }

    pub fn b(mut self, at: usize, value: u8) -> R {
        self.0[at] = value;
        self
    }

    pub fn u16(mut self, at: usize, value: u16) -> R {
        self.0[at..at + 2].copy_from_slice(&value.to_le_bytes());
        self
    }

    pub fn u32(mut self, at: usize, value: u32) -> R {
        self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
        self
    }

    pub fn i32(mut self, at: usize, value: i32) -> R {
        self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
        self
    }

    pub fn text(mut self, at: usize, value: &[u8]) -> R {
        self.0[at..at + value.len()].copy_from_slice(value);
        self
    }

    /// Marks the field at `at` as a long string.
    pub fn long(self, at: usize) -> R {
        self.b(at, 0x7F)
    }

    /// The four coordinates of a line, at 4, 8, 12 and 16.
    pub fn ends(self, x1: i32, y1: i32, x2: i32, y2: i32) -> R {
        self.i32(4, x1).i32(8, y1).i32(12, x2).i32(16, y2)
    }

    /// One coordinate of the centre of a general arc, in the high bytes of the first three.
    pub fn centre(self, value: i32) -> R {
        let c = value.to_le_bytes();
        self.b(7, c[0]).b(11, c[1]).b(15, c[2])
    }
}

/// A drawing record: its own records and how many of them follow it.
pub fn drawing(head: R, children: Vec<R>) -> Vec<R> {
    let mut out = vec![head.u16(2, children.len() as u16)];
    out.extend(children);
    out
}

/// The header of a section.
fn section(kind: u8, items: u16, records: usize) -> R {
    R::new(kind).u32(4, records as u32).u16(8, items)
}

/// What a file holds.
pub struct File {
    pub version: (u8, u8),
    pub layers: Vec<R>,
    pub sets: Vec<Vec<R>>,
    pub symbols: Vec<Vec<R>>,
    pub packages: Vec<Vec<R>>,
    /// True when the packages section has a long description.
    pub description: bool,
    /// The long strings, in the order in which the fields take them.
    pub strings: Vec<&'static [u8]>,
}

impl File {
    pub fn new(version: (u8, u8)) -> File {
        File {
            version,
            layers: Vec::new(),
            sets: Vec::new(),
            symbols: Vec::new(),
            packages: Vec::new(),
            description: false,
            strings: Vec::new(),
        }
    }

    /// The records after the header.
    fn records(&self) -> Vec<R> {
        let flat = |list: &[Vec<R>]| list.iter().flatten().copied().collect::<Vec<R>>();
        let (sets, symbols, packages) =
            (flat(&self.sets), flat(&self.symbols), flat(&self.packages));
        let mut out = self.layers.clone();
        out.push(
            R::new(0x15)
                .u32(4, sets.len() as u32 + 1)
                .u32(8, symbols.len() as u32 + 1)
                .u32(12, packages.len() as u32 + 1)
                .text(16, b"lib"),
        );
        out.push(section(0x17, self.sets.len() as u16, sets.len()).text(16, b"lib"));
        out.extend(sets);
        out.push(section(0x18, self.symbols.len() as u16, symbols.len()).text(16, b"lib"));
        out.extend(symbols);
        let mut head = section(0x19, self.packages.len() as u16, packages.len()).text(16, b"lib");
        if self.description {
            head = head.long(10);
        }
        out.push(head);
        out.extend(packages);
        out
    }

    pub fn bytes(&self) -> Vec<u8> {
        let records = self.records();
        let header = R::new(0x10)
            .u16(2, self.layers.len() as u16)
            .u32(4, records.len() as u32 + 1)
            .b(8, self.version.0)
            .b(9, self.version.1);
        let mut out = header.0.to_vec();
        for r in records {
            out.extend_from_slice(&r.0);
        }
        if !self.strings.is_empty() {
            let mut blob = Vec::new();
            for s in &self.strings {
                blob.extend_from_slice(s);
                blob.push(0);
            }
            blob.push(0);
            out.extend_from_slice(&[0x13, 0x12, 0x99, 0x19]);
            out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
            out.extend(blob);
            out.extend_from_slice(&[0; 12]);
        }
        out
    }
}

pub fn layer(number: u8, name: &[u8]) -> R {
    R::new(0x13)
        .b(2, 0x0E)
        .b(3, number)
        .b(5, 1)
        .b(6, 4)
        .text(15, name)
}

pub fn gate(symbol: u16, addlevel: u8, name: &[u8]) -> R {
    R::new(0x2D)
        .i32(4, -127_000)
        .i32(8, 101_600)
        .b(12, addlevel)
        .u16(14, symbol)
        .text(16, name)
}

pub fn pin(x: i32, kind: u8, flags: u8, name: &[u8]) -> R {
    R::new(0x2C)
        .b(2, flags)
        .i32(4, x)
        .b(12, kind)
        .text(14, name)
}

pub fn smd(x: i32, name: &[u8]) -> R {
    R::new(0x2B)
        .b(2, 25)
        .b(3, 1)
        .i32(4, x)
        .u16(12, 1500)
        .u16(14, 7500)
        .text(19, name)
}

/// A small library of version 4: a resistor with two devices, a symbol and two footprints that
/// together hold every kind of drawing record.
pub fn sample() -> File {
    let mut f = File::new((4, 9));
    f.layers = vec![
        R::new(0x11),
        layer(1, b"Top"),
        layer(21, b"tPlace"),
        layer(94, b"Symbols"),
    ];
    // device set: one gate, two devices; the second device has no footprint
    let set = R::new(0x37)
        .u16(2, 1)
        .u16(4, 3)
        .b(6, 1)
        .text(8, b"R")
        .long(13)
        .text(18, b"RES");
    let first = R::new(0x36)
        .u16(2, 1)
        .u16(4, 1)
        .text(6, b"\x80H\xC3")
        .text(19, b"0603");
    // pad 1 -> pin 2, pad 2 -> pin 1
    let connections = R::new(0x3C).b(2, 0x12).b(3, 0x11);
    let second = R::new(0x36).text(19, b"''");
    f.sets = vec![vec![set, first, connections, second, gate(1, 2, b"G$1")]];
    f.symbols = vec![drawing(
        R::new(0x1D).text(16, b"R"),
        vec![
            R::new(0x22)
                .b(3, 94)
                .ends(-12_700, 5_080, 12_700, 5_080)
                .u16(20, 1270),
            // short, passive, no turn; visible off
            pin(-50_800, 0x16, 0x00, b"1"),
            // short, passive, turned by 180 degrees, with a dot
            pin(50_800, 0x96, 0xC1, b"2"),
            R::new(0x31)
                .b(2, 1)
                .b(3, 95)
                .i32(8, 10_000)
                .u16(12, 8890)
                .b(14, 0x28)
                .text(18, b">NAME"),
        ],
    )];
    let chip = drawing(
        R::new(0x1E).long(13).text(18, b"R0603"),
        vec![
            smd(-8_000, b"1"),
            smd(8_000, b"2"),
            // quarter circle, counterclockwise, flat cap
            R::new(0x22)
                .b(3, 21)
                .ends(10_000, 0, 0, 10_000)
                .u16(20, 635)
                .b(22, 0x30)
                .b(23, 0x7B),
            // general arc with the y of its centre stored
            R::new(0x22)
                .b(3, 21)
                .ends(5_000, 0, -3_000, 4_000)
                .centre(0)
                .u16(20, 635)
                .b(22, 0x20)
                .b(23, 0x81),
            // general arc with the x of its centre stored, clockwise, dashed
            R::new(0x24)
                .b(3, 21)
                .ends(2_900, 4_000, 2_900, -4_000)
                .centre(-100)
                .u16(20, 635)
                .b(22, 0x02),
            R::new(0x25)
                .b(3, 21)
                .i32(4, 20_000)
                .u32(12, 3_175)
                .u32(16, 3_175)
                .u32(20, 1_270),
            R::new(0x26)
                .b(3, 51)
                .ends(11_270, 25_654, 8_730, 16_256)
                .u16(20, 1024),
            R::new(0x28).i32(4, 1_000).i32(8, -1_000).u32(12, 5_000),
            R::new(0x31)
                .b(3, 25)
                .u16(12, 5000)
                .b(14, 0x20)
                .u16(16, 0x1400)
                .long(18),
        ],
    );
    // a through-hole footprint with a polygon and a frame
    let mut polygon = vec![R::new(0x21)
        .u16(2, 3)
        .u16(12, 508)
        .u16(14, 2540)
        .b(18, 1)
        .b(19, 0x8E)];
    for (x, y, code) in [(0, 0, 0), (10_000, 0, 0x7B), (0, 10_000, 0)] {
        polygon.push(
            R::new(0x22)
                .b(3, 1)
                .ends(x, y, y, x)
                .b(22, 0x20)
                .b(23, code),
        );
    }
    let mut header_items = vec![
        R::new(0x2A)
            .b(2, 0)
            .u16(12, 4064)
            .u16(14, 7000)
            .b(18, 0x08)
            .text(19, b"1"),
        R::new(0x2A).b(2, 4).i32(4, 25_400).u16(12, 4064).long(19),
    ];
    header_items.extend(polygon);
    header_items.push(
        R::new(0x43)
            .b(3, 21)
            .ends(0, 0, 100_000, 80_000)
            .b(20, 4)
            .b(21, 3),
    );
    let header = drawing(R::new(0x1E).text(18, b"HDR"), header_items);
    f.packages = vec![chip, header];
    f.description = true;
    f.strings = vec![
        // the description of the device set
        b"<b>Resistor</b>",
        // the description of the library, in the header of the footprints
        b"Test library\n<p>",
        // the description of the first footprint, its long text and the long name of a pad
        b"Chip resistor",
        b">VALUE of the part",
        b"PAD_TWO",
    ];
    f
}

/// A library of version 3: a device with two gates, and a text that goes on in the records
/// that follow it.
pub fn sample_3() -> File {
    let mut f = File::new((3, 55));
    f.layers = vec![layer(1, b"Top")];
    let head = R::new(0x36)
        .u16(8, 1)
        .b(10, 1)
        .text(11, b"IC")
        .text(14, b"TWO-GATES");
    // gate 1: pin 1 -> pad 3, pin 2 -> not connected; gate 2: pin 1 -> pad 1
    let first = R::new(0x3C).u16(4, 3);
    let second = R::new(0x3C).u16(4, 1);
    f.sets = vec![drawing(
        head,
        vec![gate(1, 4, b"A"), gate(2, 0, b"P"), first, second],
    )];
    f.symbols = vec![
        drawing(
            R::new(0x1D).text(16, b"TWO"),
            vec![pin(0, 0x31, 0xC0, b"IN"), pin(25_400, 0x32, 0xC0, b"OUT")],
        ),
        drawing(
            R::new(0x1D).text(16, b"PWR"),
            vec![pin(0, 0x35, 0xC0, b"VCC")],
        ),
    ];
    f.packages = vec![drawing(
        R::new(0x1E).text(18, b"DIL"),
        vec![
            smd(-25_400, b"1"),
            smd(0, b"2"),
            smd(25_400, b"3"),
            R::new(0x31).b(2, 2).b(3, 21).u16(12, 5000).b(14, 0x20),
            R::new(0x32).text(2, b"a text that goes on be"),
            R::new(0x32).text(2, b"yond one record"),
        ],
    )];
    f
}
