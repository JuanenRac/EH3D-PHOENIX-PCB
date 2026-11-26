// SPDX-License-Identifier: GPL-3.0-or-later
//! Binary boards made byte by byte for the tests. They are invented here and come from no file
//! of any program.

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
}

/// A head record followed by its own records, counted in its bytes 2 and 3.
fn group(head: R, children: Vec<R>) -> Vec<R> {
    let mut out = vec![head.u16(2, children.len() as u16)];
    out.extend(children);
    out
}

/// A straight line from one point to another, in tenths of a micrometre.
pub fn line(layer: u8, from: (i32, i32), to: (i32, i32), width: u16) -> R {
    R::new(0x22)
        .b(3, layer)
        .i32(4, from.0 & 0x00FF_FFFF)
        .i32(8, from.1 & 0x00FF_FFFF)
        .i32(12, to.0 & 0x00FF_FFFF)
        .i32(16, to.1 & 0x00FF_FFFF)
        .u16(20, width / 2)
}

/// A text or the place of an attribute: layer, position and a height of 1.27 mm.
fn placed(kind: u8, layer: u8, x: i32, y: i32) -> R {
    R::new(kind)
        .b(2, 1)
        .b(3, layer)
        .i32(4, x)
        .i32(8, y)
        .u16(12, 6350)
        .b(14, 8 << 2)
}

fn layer(number: u8, name: &[u8]) -> R {
    R::new(0x13)
        .b(2, 0x0E)
        .b(3, number)
        .b(5, 1)
        .b(6, 4)
        .text(15, name)
}

fn smd(version: u8, x: i32, name: &[u8]) -> R {
    let name_at = if version < 4 { 16 } else { 19 };
    R::new(0x2B)
        .b(3, 1)
        .i32(4, x)
        .u16(12, 2500)
        .u16(14, 4000)
        .text(name_at, name)
}

/// What a board holds.
pub struct BoardFile {
    pub version: (u8, u8),
    pub root: u8,
}

impl BoardFile {
    pub fn new(version: (u8, u8)) -> BoardFile {
        BoardFile {
            version,
            root: 0x1B,
        }
    }

    /// Two resistors on one library, an outline of 20 by 10 mm, one signal with a track, a via,
    /// a connection that is not routed and a polygon, the design rules and two net classes.
    pub fn bytes(&self) -> Vec<u8> {
        let major = self.version.0;
        let layers = vec![
            layer(1, b"Top"),
            layer(16, b"Bottom"),
            layer(19, b"Unrouted"),
            layer(20, b"Dimension"),
            layer(21, b"tPlace"),
        ];
        let drawing = vec![
            smd(major, -8_000, b"1"),
            smd(major, 8_000, b"2"),
            line(21, (-5_000, 4_000), (5_000, 4_000), 1_270),
        ];
        // before version 4 the name of a footprint starts two bytes earlier
        let name_at = if major < 4 { 16 } else { 18 };
        let package = group(R::new(0x1E).text(name_at, b"R0805"), drawing);
        let mut libraries = vec![R::new(0x19)
            .u32(4, package.len() as u32)
            .u16(8, 1)
            .text(16, b"rcl")];
        libraries.extend(package);
        // a library with nothing in it, as the program leaves them
        libraries.push(R::new(0x19).text(16, b"empty"));

        let outline = [(0, 0), (200_000, 0), (200_000, 100_000), (0, 100_000)];
        let mut plain: Vec<R> = (0..4)
            .map(|k| line(20, outline[k], outline[(k + 1) % 4], 0))
            .collect();
        plain.push(placed(0x31, 21, 10_000, 90_000).text(18, b"demo"));
        plain.push(R::new(0x28).i32(4, 10_000).i32(8, 10_000).u32(12, 16_000));
        plain.push(R::new(0x42).text(7, b"AUTHOR!test"));

        let element = |x: i32, y: i32| R::new(0x2E).i32(4, x).i32(8, y).u16(12, 1).u16(14, 1);
        let mut elements = group(
            element(50_000, 50_000).b(18, 1),
            vec![
                R::new(0x2F).text(2, b"R1").text(10, b"10k"),
                placed(0x34, 25, 50_000, 60_000),
                placed(0x35, 27, 50_000, 40_000),
            ],
        );
        // a quarter of a turn, a long value and an attribute of its own
        elements.extend(group(
            element(150_000, 50_000).u16(16, 1_024),
            vec![
                R::new(0x2F).text(2, b"R2").long(10),
                placed(0x41, 27, 150_000, 40_000).text(18, b"MPN!X1"),
            ],
        ));

        let contact = |element: u16, pad: u16| R::new(0x3E).u16(4, element).u16(6, pad);
        let signal = group(
            R::new(0x1C).b(13, 1).long(16),
            vec![
                contact(1, 2),
                contact(2, 1),
                line(1, (58_000, 50_000), (142_000, 50_000), 4_000),
                R::new(0x29)
                    .b(2, 1)
                    .i32(4, 100_000)
                    .i32(8, 50_000)
                    .u16(12, 1_750)
                    .u16(14, 4_000)
                    .b(16, 0xF0),
                line(19, (58_000, 50_000), (142_000, 50_000), 0).b(23, 1),
                // a hatched polygon of rank 2 on the bottom layer, with an isolation
                R::new(0x21)
                    .u16(2, 4)
                    .u16(12, 1_016)
                    .u16(14, 3_048)
                    .u16(16, 1_270)
                    .b(18, 16)
                    .b(19, 0x85),
                line(16, outline[0], outline[1], 0),
                line(16, outline[1], outline[2], 0),
                line(16, outline[2], outline[3], 0),
                line(16, outline[3], outline[0], 0),
            ],
        );

        let mut records = layers.clone();
        records.push(
            R::new(self.root)
                .u16(2, plain.len() as u16)
                .u32(12, libraries.len() as u32)
                .u32(16, elements.len() as u32)
                .u32(20, signal.len() as u32),
        );
        records.extend(libraries);
        records.extend(plain);
        records.extend(elements);
        records.extend(signal);

        let header = R::new(0x10)
            .u16(2, layers.len() as u16)
            .u32(4, records.len() as u32 + 1)
            .b(8, self.version.0)
            .b(9, self.version.1);
        let mut out = header.0.to_vec();
        for r in records {
            out.extend_from_slice(&r.0);
        }
        // the long strings: the value of R2 and then the name of the signal
        let mut blob = Vec::new();
        for s in [&b"4k7 precision resistor"[..], b"SUPPLY_5V"] {
            blob.extend_from_slice(s);
            blob.push(0);
        }
        blob.push(0);
        out.extend_from_slice(&[0x13, 0x12, 0x99, 0x19]);
        out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        out.extend(blob);
        out.extend_from_slice(&[0; 12]);
        out.extend(tail());
        out
    }
}

fn block(mark: [u8; 4], content: &[u8]) -> Vec<u8> {
    let mut out = mark.to_vec();
    out.extend_from_slice(&(content.len() as u32).to_le_bytes());
    out.extend_from_slice(content);
    out
}

fn class(name: &[u8], number: u32, values: [i32; 3]) -> Vec<u8> {
    let mut content = name.to_vec();
    content.push(0);
    content.extend_from_slice(&number.to_le_bytes());
    content.extend_from_slice(&[0x21, 0x43, 0x65, 0x87]);
    for v in values {
        content.extend_from_slice(&v.to_le_bytes());
    }
    content.extend_from_slice(&[0xEF, 0xCD, 0xAB, 0x89]);
    block([0x25, 0x04, 0x00, 0x20], &content)
}

/// The design rules (every distance 0.2032 mm, the smallest width 0.254 mm) and the classes.
pub fn tail() -> Vec<u8> {
    let mut content = b"mine\0some rules\0(1*16)\0".to_vec();
    content.extend_from_slice(&[0x78, 0x56, 0x34, 0x12]);
    let mut ints = [2_032i32; 19];
    ints[10] = 2;
    ints[16] = 2_540;
    ints[17] = 6_096;
    for v in ints {
        content.extend_from_slice(&v.to_le_bytes());
    }
    for _ in 0..8 {
        content.extend_from_slice(&0.25f64.to_le_bytes());
    }
    for _ in 0..14 {
        content.extend_from_slice(&2_540i32.to_le_bytes());
    }
    let mut out = block([0x10, 0x04, 0x00, 0x20], &content);
    out.extend(class(b"default", 0, [0, 0, 0]));
    out.extend(class(b"power", 1, [10_160, 10_160, 5_080]));
    out.extend(class(b"", 2, [0, 0, 0]));
    out
}
