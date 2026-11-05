// SPDX-License-Identifier: GPL-3.0-or-later
//! Libraries written by hand for the tests: small, generated here, never copied from anywhere.

/// A library file with the given footprints, symbols and device sets (their XML).
pub fn library_xml(packages: &[String], symbols: &[String], devicesets: &[String]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<eagle version=\"9.6.2\"><drawing><settings/><layers><layer number=\"1\" name=\"Top\"/></layers><library>\n<packages>\n{}\n</packages>\n<symbols>\n{}\n</symbols>\n<devicesets>\n{}\n</devicesets>\n</library></drawing></eagle>\n",
        packages.join("\n"),
        symbols.join("\n"),
        devicesets.join("\n")
    )
}

/// A footprint of surface-mount pads named 1, 2, ... at the given positions (millimetres), each
/// `w` by `h`, with a silkscreen box and an outline around them.
pub fn package_xml(name: &str, pads: &[(f64, f64)], w: f64, h: f64) -> String {
    let mut out = format!("<package name=\"{name}\"><description>test footprint</description>");
    for (i, (x, y)) in pads.iter().enumerate() {
        out.push_str(&format!(
            "<smd name=\"{}\" x=\"{x}\" y=\"{y}\" dx=\"{w}\" dy=\"{h}\" layer=\"1\"/>",
            i + 1
        ));
    }
    let (min_x, max_x) = pads.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.0), hi.max(p.0))
    });
    let (min_y, max_y) = pads.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.1), hi.max(p.1))
    });
    let (x0, x1, y0, y1) = (min_x - w, max_x + w, min_y - h, max_y + h);
    for (a, b, c, d) in [
        (x0, y0, x1, y0),
        (x1, y0, x1, y1),
        (x1, y1, x0, y1),
        (x0, y1, x0, y0),
    ] {
        out.push_str(&format!(
            "<wire x1=\"{a}\" y1=\"{b}\" x2=\"{c}\" y2=\"{d}\" width=\"0.1\" layer=\"21\"/>"
        ));
    }
    let (o0, o1, p0, p1) = (x0 - 0.5, x1 + 0.5, y0 - 0.5, y1 + 0.5);
    for (a, b, c, d) in [
        (o0, p0, o1, p0),
        (o1, p0, o1, p1),
        (o1, p1, o0, p1),
        (o0, p1, o0, p0),
    ] {
        out.push_str(&format!(
            "<wire x1=\"{a}\" y1=\"{b}\" x2=\"{c}\" y2=\"{d}\" width=\"0.05\" layer=\"39\"/>"
        ));
    }
    out.push_str("<text x=\"0\" y=\"2\" size=\"1\" layer=\"25\">&gt;NAME</text></package>");
    out
}

/// A symbol with the given pins (name, x, y, in millimetres) and a rectangle `body` wide.
pub fn symbol_xml(name: &str, pins: &[(&str, f64, f64)], body: f64) -> String {
    let mut out = format!("<symbol name=\"{name}\">");
    out.push_str(&format!(
        "<rectangle x1=\"0\" y1=\"-{body}\" x2=\"{body}\" y2=\"{body}\" layer=\"94\"/>"
    ));
    for (pin, x, y) in pins {
        out.push_str(&format!(
            "<pin name=\"{pin}\" x=\"{x}\" y=\"{y}\" length=\"short\"/>"
        ));
    }
    out.push_str("</symbol>");
    out
}

/// A device set with one gate and one device; `connects` are pin and pad names.
pub fn deviceset_xml(
    name: &str,
    prefix: &str,
    symbol: &str,
    package: &str,
    connects: &[(&str, &str)],
) -> String {
    let mut out = format!(
        "<deviceset name=\"{name}\" prefix=\"{prefix}\"><gates><gate name=\"G$1\" symbol=\"{symbol}\" x=\"0\" y=\"0\"/></gates><devices><device name=\"\" package=\"{package}\"><connects>"
    );
    for (pin, pad) in connects {
        out.push_str(&format!(
            "<connect gate=\"G$1\" pin=\"{pin}\" pad=\"{pad}\"/>"
        ));
    }
    out.push_str("</connects></device></devices></deviceset>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generated_library_is_read_by_the_eagle_reader() {
        let text = library_xml(
            &[package_xml("R", &[(-0.9, 0.0), (0.9, 0.0)], 1.2, 1.4)],
            &[symbol_xml("S", &[("1", 0.0, 0.0), ("2", 5.08, 0.0)], 1.0)],
            &[deviceset_xml("R", "R", "S", "R", &[("1", "1"), ("2", "2")])],
        );
        let r =
            eh3d_phoenix_eagle::read_bytes(text.as_bytes(), &eh3d_phoenix_eagle::Limits::default());
        assert!(r.is_clean(), "{:?}", r.diagnostics);
        let s = r.summary.unwrap();
        assert_eq!(
            (s.packages.len(), s.symbols.len(), s.device_sets.len()),
            (1, 1, 1)
        );
    }
}
