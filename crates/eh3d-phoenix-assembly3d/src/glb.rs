// SPDX-License-Identifier: GPL-3.0-or-later
//! Prisms written as a binary glTF 2.0 file (GLB) with a profile of our own, `phoenix-board-glb/1`,
//! written from the glTF 2.0 specification and checked by the conformance tests of this module.
//!
//! The profile: one scene; one node, one mesh and one primitive of triangles for each prism, in the
//! order given; the name of the prism is the name of the node and its stable identifier (`extras.id`);
//! positions in metres as 32-bit floats with the axes of glTF (X to the right, Y up, so the height of
//! the prism is Y and the board plane is X and -Z); a flat normal for each face; 32-bit indices; the
//! bounds of every position accessor; one material for each role told by the name of the prism
//! (`copper`, `mask`, `silk`, `dielectric`, `board`, anything else `part`), whose colours come from
//! `config/glb_profile.cfg`. The file has no date, no host and no program version, so the same prisms
//! give the same bytes. The faces of the top and the bottom are cut into triangles by ear clipping, so
//! a prism may be concave; it cannot have holes (the drilled holes are not in the file), and a prism
//! that crosses itself is refused.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use crate::geom::{counter_clockwise, P};
use crate::prism::Prism;

/// The text of the colours that comes with the program.
pub const PROFILE_TEXT: &str = include_str!("../../../config/glb_profile.cfg");

/// The name of the profile, written in the file.
pub const PROFILE: &str = "phoenix-board-glb/1";

/// Why a file could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlbError {
    /// The colours file is not valid (the line, what is wrong).
    Profile(usize, String),
    /// Two prisms have the same name, which would give two nodes the same identifier.
    DuplicateName(String),
    /// A prism has fewer than three corners, no height, or an area of zero.
    Degenerate(String),
    /// The triangles of a face could not be made (the polygon crosses itself).
    NotSimple(String),
    /// A number does not fit a 32-bit float or the file would be over 4 GiB.
    TooLarge,
}

impl std::fmt::Display for GlbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlbError::Profile(line, why) => write!(f, "colours, line {line}: {why}"),
            GlbError::DuplicateName(n) => write!(f, "the name {n} is used by two prisms"),
            GlbError::Degenerate(n) => {
                write!(f, "{n} has fewer than three corners, no height or no area")
            }
            GlbError::NotSimple(n) => write!(f, "{n} crosses itself"),
            GlbError::TooLarge => write!(f, "the file would be too large"),
        }
    }
}

impl std::error::Error for GlbError {}

/// The colours of the roles: red, green, blue and alpha, each from 0 to 1.
pub fn parse_profile(text: &str) -> Result<BTreeMap<String, [f64; 4]>, GlbError> {
    let mut out = BTreeMap::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = index + 1;
        let Some((key, value)) = line.split_once('=') else {
            return Err(GlbError::Profile(at, "expected key = value".to_owned()));
        };
        let Some(role) = key.trim().strip_prefix("material.") else {
            return Err(GlbError::Profile(
                at,
                "a key must start with material.".to_owned(),
            ));
        };
        let numbers: Vec<f64> = value
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| GlbError::Profile(at, "four numbers are needed".to_owned()))?;
        let [r, g, b, a] = numbers[..] else {
            return Err(GlbError::Profile(at, "four numbers are needed".to_owned()));
        };
        if ![r, g, b, a].iter().all(|v| (0.0..=1.0).contains(v)) {
            return Err(GlbError::Profile(
                at,
                "a colour goes from 0 to 1".to_owned(),
            ));
        }
        if out.insert(role.to_owned(), [r, g, b, a]).is_some() {
            return Err(GlbError::Profile(at, format!("{role} is given twice")));
        }
    }
    if !out.contains_key("part") {
        return Err(GlbError::Profile(0, "the role part is needed".to_owned()));
    }
    Ok(out)
}

/// The role of a prism, told by the start of its name.
pub fn role_of(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    for role in ["copper", "mask", "silk", "dielectric", "board"] {
        if lower.starts_with(role) {
            return role;
        }
    }
    "part"
}

fn cross(o: P, a: P, b: P) -> i128 {
    (a.0 - o.0) as i128 * (b.1 - o.1) as i128 - (a.1 - o.1) as i128 * (b.0 - o.0) as i128
}

fn inside(a: P, b: P, c: P, p: P) -> bool {
    cross(a, b, p) >= 0 && cross(b, c, p) >= 0 && cross(c, a, p) >= 0
}

/// True when two edges of the polygon that are not neighbours cross each other. The work is
/// quadratic, so a polygon of more than 4096 corners is left to the ear clipping alone.
fn crosses_itself(poly: &[P]) -> bool {
    let n = poly.len();
    if n > 4096 {
        return false;
    }
    let side = |o: P, a: P, b: P| cross(o, a, b).signum();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let (c, d) = (poly[j], poly[(j + 1) % n]);
            if side(a, b, c) * side(a, b, d) < 0 && side(c, d, a) * side(c, d, b) < 0 {
                return true;
            }
        }
    }
    false
}

/// The triangles (indices into `poly`, counter-clockwise) of a simple polygon that runs
/// counter-clockwise, by ear clipping. `None` when no ear is left before the polygon is used up.
fn ear_clip(poly: &[P]) -> Option<Vec<[usize; 3]>> {
    let mut left: Vec<usize> = (0..poly.len()).collect();
    let mut out = Vec::with_capacity(poly.len().saturating_sub(2));
    while left.len() > 3 {
        let n = left.len();
        let mut clipped = false;
        for k in 0..n {
            let (i, j, l) = (left[(k + n - 1) % n], left[k], left[(k + 1) % n]);
            if cross(poly[i], poly[j], poly[l]) <= 0 {
                continue;
            }
            let blocked = left.iter().any(|&m| {
                m != i
                    && m != j
                    && m != l
                    && inside(poly[i], poly[j], poly[l], poly[m])
                    && poly[m] != poly[i]
                    && poly[m] != poly[j]
                    && poly[m] != poly[l]
            });
            if !blocked {
                out.push([i, j, l]);
                left.remove(k);
                clipped = true;
                break;
            }
        }
        if !clipped {
            // a corner on a straight line has no area: drop it and go on
            let n = left.len();
            let flat = (0..n).find(|&k| {
                cross(
                    poly[left[(k + n - 1) % n]],
                    poly[left[k]],
                    poly[left[(k + 1) % n]],
                ) == 0
            })?;
            left.remove(flat);
        }
    }
    if left.len() == 3 && cross(poly[left[0]], poly[left[1]], poly[left[2]]) > 0 {
        out.push([left[0], left[1], left[2]]);
    }
    Some(out)
}

struct Mesh {
    positions: Vec<f32>,
    normals: Vec<f32>,
    indices: Vec<u32>,
}

fn metres(nm: i64) -> f32 {
    (nm as f64 / 1.0e9) as f32
}

/// The mesh of a prism: the bottom, the top and a quad for each side, each face with its own
/// vertices and its flat normal.
fn mesh_of(prism: &Prism) -> Result<Mesh, GlbError> {
    if prism.base.len() < 3 || prism.z_high <= prism.z_low {
        return Err(GlbError::Degenerate(prism.name.clone()));
    }
    let base = counter_clockwise(&prism.base);
    if crosses_itself(&base) {
        return Err(GlbError::NotSimple(prism.name.clone()));
    }
    let triangles = ear_clip(&base).ok_or_else(|| GlbError::NotSimple(prism.name.clone()))?;
    if triangles.is_empty() {
        return Err(GlbError::Degenerate(prism.name.clone()));
    }
    let mut m = Mesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
    };
    // glTF: x = x, y = height, z = -y of the board
    let vertex = |m: &mut Mesh, p: P, z: i64, normal: [f32; 3]| -> u32 {
        let index = (m.positions.len() / 3) as u32;
        m.positions.extend([metres(p.0), metres(z), -metres(p.1)]);
        m.normals.extend(normal);
        index
    };
    let top = prism.z_high;
    let bottom = prism.z_low;
    let top_first = (m.positions.len() / 3) as u32;
    for p in &base {
        vertex(&mut m, *p, top, [0.0, 1.0, 0.0]);
    }
    for t in &triangles {
        m.indices.extend([
            top_first + t[0] as u32,
            top_first + t[1] as u32,
            top_first + t[2] as u32,
        ]);
    }
    let bottom_first = (m.positions.len() / 3) as u32;
    for p in &base {
        vertex(&mut m, *p, bottom, [0.0, -1.0, 0.0]);
    }
    for t in &triangles {
        // the bottom faces down: the order is reversed
        m.indices.extend([
            bottom_first + t[0] as u32,
            bottom_first + t[2] as u32,
            bottom_first + t[1] as u32,
        ]);
    }
    let n = base.len();
    for k in 0..n {
        let (a, b) = (base[k], base[(k + 1) % n]);
        let (dx, dy) = ((b.0 - a.0) as f64, (b.1 - a.1) as f64);
        let length = dx.hypot(dy);
        if length == 0.0 {
            continue;
        }
        // the outward normal of a counter-clockwise edge is the edge turned to the right
        let normal = [(dy / length) as f32, 0.0, (dx / length) as f32];
        let v0 = vertex(&mut m, a, bottom, normal);
        let v1 = vertex(&mut m, b, bottom, normal);
        let v2 = vertex(&mut m, b, top, normal);
        let v3 = vertex(&mut m, a, top, normal);
        m.indices.extend([v0, v1, v2, v0, v2, v3]);
    }
    Ok(m)
}

fn pad4(bytes: &mut Vec<u8>, with: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(with);
    }
}

fn bounds(positions: &[f32]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in positions.as_chunks::<3>().0 {
        for (axis, x) in v.iter().enumerate() {
            lo[axis] = lo[axis].min(*x);
            hi[axis] = hi[axis].max(*x);
        }
    }
    (lo, hi)
}

/// Write the prisms as a GLB file with the colours of `profile` (see [`parse_profile`]). `metadata`
/// is kept in `asset.extras.metadata` (text only, in the order of its keys): the caller decides what
/// goes there, and a date or a host is the caller's choice.
pub fn prisms_glb(
    prisms: &[Prism],
    profile: &BTreeMap<String, [f64; 4]>,
    metadata: &BTreeMap<String, String>,
) -> Result<Vec<u8>, GlbError> {
    let mut seen = std::collections::BTreeSet::new();
    for prism in prisms {
        if !seen.insert(crate::prism::safe_name(&prism.name)) {
            return Err(GlbError::DuplicateName(prism.name.clone()));
        }
    }
    let mut roles: Vec<&'static str> = prisms.iter().map(|p| role_of(&p.name)).collect();
    roles.sort_unstable();
    roles.dedup();
    let mut materials = Vec::new();
    for role in &roles {
        let c = profile
            .get(*role)
            .or_else(|| profile.get("part"))
            .ok_or_else(|| GlbError::Profile(0, "the role part is needed".to_owned()))?;
        materials.push(json!({
            "name": role,
            "pbrMetallicRoughness": {
                "baseColorFactor": [c[0], c[1], c[2], c[3]],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.8
            },
            "alphaMode": if c[3] < 1.0 { "BLEND" } else { "OPAQUE" },
            "doubleSided": false
        }));
    }
    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut node_ids = Vec::new();
    for (i, prism) in prisms.iter().enumerate() {
        let mesh = mesh_of(prism)?;
        let vertex_count = mesh.positions.len() / 3;
        let (lo, hi) = bounds(&mesh.positions);
        let mut add_view = |bin: &mut Vec<u8>, bytes: Vec<u8>, target: u32| -> usize {
            pad4(bin, 0);
            let offset = bin.len();
            let length = bytes.len();
            bin.extend(bytes);
            views.push(
                json!({"buffer": 0, "byteOffset": offset, "byteLength": length, "target": target}),
            );
            views.len() - 1
        };
        let pos_bytes: Vec<u8> = mesh
            .positions
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let nor_bytes: Vec<u8> = mesh.normals.iter().flat_map(|v| v.to_le_bytes()).collect();
        let idx_bytes: Vec<u8> = mesh.indices.iter().flat_map(|v| v.to_le_bytes()).collect();
        let pv = add_view(&mut bin, pos_bytes, 34962);
        let nv = add_view(&mut bin, nor_bytes, 34962);
        let iv = add_view(&mut bin, idx_bytes, 34963);
        let base = accessors.len();
        accessors.push(json!({"bufferView": pv, "componentType": 5126, "count": vertex_count, "type": "VEC3", "min": lo, "max": hi}));
        accessors.push(
            json!({"bufferView": nv, "componentType": 5126, "count": vertex_count, "type": "VEC3"}),
        );
        accessors.push(json!({"bufferView": iv, "componentType": 5125, "count": mesh.indices.len(), "type": "SCALAR"}));
        let role = role_of(&prism.name);
        let material = roles.iter().position(|r| *r == role).unwrap_or(0);
        let name = crate::prism::safe_name(&prism.name);
        meshes.push(json!({
            "name": name,
            "primitives": [{
                "attributes": {"POSITION": base, "NORMAL": base + 1},
                "indices": base + 2,
                "material": material,
                "mode": 4
            }]
        }));
        nodes.push(json!({
            "name": name,
            "mesh": i,
            "extras": {"id": format!("phoenix:{name}"), "role": role}
        }));
        node_ids.push(i);
    }
    pad4(&mut bin, 0);
    let mut extras = Map::new();
    extras.insert("profile".to_owned(), Value::from(PROFILE));
    extras.insert("units".to_owned(), Value::from("metres, Y up"));
    let meta: Map<String, Value> = metadata
        .iter()
        .map(|(k, v)| (k.clone(), Value::from(v.clone())))
        .collect();
    extras.insert("metadata".to_owned(), Value::Object(meta));
    let mut root = Map::new();
    root.insert(
        "asset".to_owned(),
        json!({"version": "2.0", "generator": "EH3D-PHOENIX-PCB", "extras": Value::Object(extras)}),
    );
    root.insert("scene".to_owned(), Value::from(0));
    root.insert(
        "scenes".to_owned(),
        json!([{"name": "board", "nodes": node_ids}]),
    );
    root.insert("nodes".to_owned(), Value::Array(nodes));
    root.insert("meshes".to_owned(), Value::Array(meshes));
    root.insert("materials".to_owned(), Value::Array(materials));
    root.insert("accessors".to_owned(), Value::Array(accessors));
    root.insert("bufferViews".to_owned(), Value::Array(views));
    root.insert("buffers".to_owned(), json!([{"byteLength": bin.len()}]));
    let mut json_bytes =
        serde_json::to_vec(&Value::Object(root)).map_err(|_| GlbError::TooLarge)?;
    pad4(&mut json_bytes, b' ');
    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    if total > u32::MAX as usize {
        return Err(GlbError::TooLarge);
    }
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((json_bytes.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(json_bytes);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(bin);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> P {
        (x, y)
    }

    fn square(name: &str, size: i64, z0: i64, z1: i64) -> Prism {
        Prism {
            name: name.to_owned(),
            base: vec![p(0, 0), p(size, 0), p(size, size), p(0, size)],
            z_low: z0,
            z_high: z1,
        }
    }

    fn profile() -> BTreeMap<String, [f64; 4]> {
        parse_profile(PROFILE_TEXT).unwrap()
    }

    /// The pieces of a GLB: the JSON and the binary chunk, after the checks of the container.
    fn split(bytes: &[u8]) -> (Value, Vec<u8>) {
        assert_eq!(&bytes[0..4], b"glTF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 2);
        let total = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        assert_eq!(total, bytes.len());
        let jl = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        assert_eq!(&bytes[16..20], b"JSON");
        assert!(jl.is_multiple_of(4));
        let json: Value = serde_json::from_slice(&bytes[20..20 + jl]).unwrap();
        let at = 20 + jl;
        let bl = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        assert_eq!(&bytes[at + 4..at + 8], b"BIN\0");
        assert!(bl.is_multiple_of(4));
        assert_eq!(at + 8 + bl, bytes.len());
        (json, bytes[at + 8..].to_vec())
    }

    fn floats(bin: &[u8], view: &Value) -> Vec<f32> {
        let o = view["byteOffset"].as_u64().unwrap() as usize;
        let l = view["byteLength"].as_u64().unwrap() as usize;
        bin[o..o + l]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect()
    }

    #[test]
    fn the_file_keeps_the_container_the_accessors_and_the_bounds_of_the_specification() {
        let prisms = vec![
            square("board", 100_000_000, 0, 1_600_000),
            square("copper-top", 50_000_000, 1_600_000, 1_635_000),
        ];
        let bytes = prisms_glb(&prisms, &profile(), &BTreeMap::new()).unwrap();
        let (json, bin) = split(&bytes);
        assert_eq!(json["asset"]["version"], "2.0");
        assert_eq!(json["asset"]["extras"]["profile"], PROFILE);
        assert_eq!(
            json["buffers"][0]["byteLength"].as_u64().unwrap() as usize,
            bin.len()
        );
        for view in json["bufferViews"].as_array().unwrap() {
            let (o, l) = (
                view["byteOffset"].as_u64().unwrap(),
                view["byteLength"].as_u64().unwrap(),
            );
            assert!(o.is_multiple_of(4));
            assert!((o + l) as usize <= bin.len());
        }
        let accessors = json["accessors"].as_array().unwrap();
        assert_eq!(accessors.len(), 6);
        for mesh in json["meshes"].as_array().unwrap() {
            let prim = &mesh["primitives"][0];
            assert_eq!(prim["mode"], 4);
            let pa = &accessors[prim["attributes"]["POSITION"].as_u64().unwrap() as usize];
            let na = &accessors[prim["attributes"]["NORMAL"].as_u64().unwrap() as usize];
            let ia = &accessors[prim["indices"].as_u64().unwrap() as usize];
            assert_eq!(pa["count"], na["count"]);
            let pos = floats(
                &bin,
                &json["bufferViews"][pa["bufferView"].as_u64().unwrap() as usize],
            );
            let nor = floats(
                &bin,
                &json["bufferViews"][na["bufferView"].as_u64().unwrap() as usize],
            );
            assert_eq!(pos.len() / 3, pa["count"].as_u64().unwrap() as usize);
            let (lo, hi) = bounds(&pos);
            for (axis, (l, h)) in lo.iter().zip(hi.iter()).enumerate() {
                assert_eq!(pa["min"][axis].as_f64().unwrap() as f32, *l);
                assert_eq!(pa["max"][axis].as_f64().unwrap() as f32, *h);
            }
            for n in nor.as_chunks::<3>().0 {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                assert!((len - 1.0).abs() < 1e-5);
            }
            let view = &json["bufferViews"][ia["bufferView"].as_u64().unwrap() as usize];
            let o = view["byteOffset"].as_u64().unwrap() as usize;
            let count = ia["count"].as_u64().unwrap() as usize;
            assert_eq!(count % 3, 0);
            let vertices = pa["count"].as_u64().unwrap() as u32;
            for c in bin[o..o + count * 4].as_chunks::<4>().0 {
                assert!(u32::from_le_bytes(*c) < vertices);
            }
            // a square: 2 triangles above, 2 below and 8 on the four sides
            assert_eq!(count / 3, 12);
        }
    }

    #[test]
    fn the_axes_are_those_of_gltf_and_the_names_are_the_stable_identifiers() {
        let prisms = vec![square("board", 100_000_000, 0, 1_600_000)];
        let bytes = prisms_glb(&prisms, &profile(), &BTreeMap::new()).unwrap();
        let (json, bin) = split(&bytes);
        let pos = floats(&bin, &json["bufferViews"][0]);
        let (lo, hi) = bounds(&pos);
        // 100 mm along X, 1.6 mm up, 100 mm along -Z
        assert_eq!((lo[0], hi[0]), (0.0, 0.1));
        assert_eq!((lo[1], hi[1]), (0.0, 0.0016));
        assert_eq!((lo[2], hi[2]), (-0.1, 0.0));
        assert_eq!(json["nodes"][0]["name"], "board");
        assert_eq!(json["nodes"][0]["extras"]["id"], "phoenix:board");
        assert_eq!(json["nodes"][0]["extras"]["role"], "board");
    }

    #[test]
    fn the_same_prisms_give_the_same_bytes_and_a_clockwise_prism_is_turned() {
        let a = square("board", 10_000_000, 0, 1_000_000);
        let mut b = a.clone();
        b.base.reverse();
        let first = prisms_glb(std::slice::from_ref(&a), &profile(), &BTreeMap::new()).unwrap();
        assert_eq!(
            first,
            prisms_glb(&[a], &profile(), &BTreeMap::new()).unwrap()
        );
        assert_eq!(
            first,
            prisms_glb(&[b], &profile(), &BTreeMap::new()).unwrap()
        );
    }

    #[test]
    fn a_concave_prism_is_cut_into_the_right_number_of_triangles() {
        // an L: 6 corners, 4 triangles on the top, 4 on the bottom and 12 on the six sides
        let l = Prism {
            name: "board".to_owned(),
            base: vec![p(0, 0), p(20, 0), p(20, 10), p(10, 10), p(10, 20), p(0, 20)]
                .into_iter()
                .map(|q| p(q.0 * 1_000_000, q.1 * 1_000_000))
                .collect(),
            z_low: 0,
            z_high: 1_000_000,
        };
        let bytes = prisms_glb(&[l], &profile(), &BTreeMap::new()).unwrap();
        let (json, _) = split(&bytes);
        assert_eq!(json["accessors"][2]["count"].as_u64().unwrap() / 3, 20);
    }

    #[test]
    fn the_metadata_and_the_roles_and_the_colours_are_written() {
        let prisms = vec![
            square("board", 10_000_000, 0, 1_000_000),
            square("copper-top", 10_000_000, 1_000_000, 1_035_000),
            square("U1", 1_000_000, 1_035_000, 2_000_000),
        ];
        let mut meta = BTreeMap::new();
        meta.insert("project".to_owned(), "demo".to_owned());
        let bytes = prisms_glb(&prisms, &profile(), &meta).unwrap();
        let (json, _) = split(&bytes);
        assert_eq!(json["asset"]["extras"]["metadata"]["project"], "demo");
        let names: Vec<&str> = json["materials"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["board", "copper", "part"]);
        assert_eq!(json["nodes"][2]["extras"]["role"], "part");
        assert_eq!(role_of("Mask-Bottom"), "mask");
        assert_eq!(role_of("dielectric-2"), "dielectric");
    }

    #[test]
    fn what_cannot_be_written_is_refused_with_a_reason() {
        let ok = square("a", 10, 0, 10);
        let twice = prisms_glb(&[ok.clone(), ok.clone()], &profile(), &BTreeMap::new());
        assert_eq!(twice, Err(GlbError::DuplicateName("a".to_owned())));
        let mut flat = ok.clone();
        flat.z_high = flat.z_low;
        assert!(matches!(
            prisms_glb(&[flat], &profile(), &BTreeMap::new()),
            Err(GlbError::Degenerate(_))
        ));
        let mut line = ok.clone();
        line.name = "b".to_owned();
        line.base = vec![p(0, 0), p(5, 0), p(10, 0)];
        assert!(matches!(
            prisms_glb(&[line], &profile(), &BTreeMap::new()),
            Err(GlbError::Degenerate(_))
        ));
        // a bow tie crosses itself
        let mut bow = ok;
        bow.name = "c".to_owned();
        bow.base = vec![p(0, 0), p(10, 10), p(10, 0), p(0, 10)];
        assert!(prisms_glb(&[bow], &profile(), &BTreeMap::new()).is_err());
    }

    #[test]
    fn the_colours_file_is_read_and_a_bad_one_is_refused_with_its_line() {
        let p = profile();
        assert!(p.contains_key("copper") && p.contains_key("part"));
        assert!(matches!(
            parse_profile("material.part = 1 1 1"),
            Err(GlbError::Profile(1, _))
        ));
        assert!(matches!(
            parse_profile("material.part = 1 1 1 2"),
            Err(GlbError::Profile(1, _))
        ));
        assert!(matches!(
            parse_profile("colour.part = 1 1 1 1"),
            Err(GlbError::Profile(1, _))
        ));
        assert!(matches!(
            parse_profile("material.part = 1 1 1 1\nmaterial.part = 0 0 0 1"),
            Err(GlbError::Profile(2, _))
        ));
        assert!(matches!(
            parse_profile("material.copper = 1 1 1 1"),
            Err(GlbError::Profile(0, _))
        ));
    }
}
