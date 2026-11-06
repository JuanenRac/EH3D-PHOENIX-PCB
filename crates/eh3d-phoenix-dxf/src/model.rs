// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing that the reader makes and the writer takes: entities on layers, in nanometres.
//! A path is a run of lines and arcs; every arc is a piece with a sweep, however the file wrote
//! it (an arc, a polyline with a bulge, a spline that was approximated). Nothing here is a
//! picture: no colour, no line type, no width.

use crate::geom::{Seg, P};

/// The shape of an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    /// A run of pieces, each starting where the one before ends. A closed path ends where it
    /// starts.
    Path {
        /// The pieces.
        segs: Vec<Seg>,
        /// Whether the path is closed.
        closed: bool,
    },
    /// A circle.
    Circle {
        /// The centre.
        centre: P,
        /// The radius.
        radius: i64,
    },
    /// A point.
    Point {
        /// Where it is.
        at: P,
    },
    /// A text on one line.
    Text {
        /// The lower left corner.
        at: P,
        /// The height of the letters.
        height: i64,
        /// The text.
        text: String,
    },
}

/// An entity: a shape on a layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    /// The name of the layer.
    pub layer: String,
    /// The shape.
    pub shape: Shape,
}

/// A drawing: the entities in the order they were made.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Drawing {
    /// The entities.
    pub entities: Vec<Entity>,
}

impl Drawing {
    /// An empty drawing.
    pub fn new() -> Drawing {
        Drawing::default()
    }

    /// Adds a path of pieces.
    pub fn path(&mut self, layer: &str, segs: Vec<Seg>, closed: bool) {
        if segs.is_empty() {
            return;
        }
        self.entities.push(Entity {
            layer: layer.to_owned(),
            shape: Shape::Path { segs, closed },
        });
    }

    /// Adds a closed polygon of straight pieces (the last point joins the first).
    pub fn polygon(&mut self, layer: &str, points: &[P]) {
        if points.len() < 3 {
            return;
        }
        let segs: Vec<Seg> = points
            .iter()
            .enumerate()
            .map(|(i, &a)| Seg::line(a, points[(i + 1) % points.len()]))
            .collect();
        self.path(layer, segs, true);
    }

    /// Adds an open run of straight pieces.
    pub fn polyline(&mut self, layer: &str, points: &[P]) {
        let segs: Vec<Seg> = points.windows(2).map(|w| Seg::line(w[0], w[1])).collect();
        self.path(layer, segs, false);
    }

    /// Adds one straight line.
    pub fn line(&mut self, layer: &str, a: P, b: P) {
        self.path(layer, vec![Seg::line(a, b)], false);
    }

    /// Adds a circle.
    pub fn circle(&mut self, layer: &str, centre: P, radius: i64) {
        self.entities.push(Entity {
            layer: layer.to_owned(),
            shape: Shape::Circle { centre, radius },
        });
    }

    /// Adds a point.
    pub fn point(&mut self, layer: &str, at: P) {
        self.entities.push(Entity {
            layer: layer.to_owned(),
            shape: Shape::Point { at },
        });
    }

    /// Adds a cross of two lines about a point, with arms of `arm` nanometres.
    pub fn cross(&mut self, layer: &str, at: P, arm: i64) {
        self.line(layer, (at.0 - arm, at.1), (at.0 + arm, at.1));
        self.line(layer, (at.0, at.1 - arm), (at.0, at.1 + arm));
    }

    /// Adds a text.
    pub fn text(&mut self, layer: &str, at: P, height: i64, text: &str) {
        self.entities.push(Entity {
            layer: layer.to_owned(),
            shape: Shape::Text {
                at,
                height,
                text: text.to_owned(),
            },
        });
    }

    /// The names of the layers that hold an entity, sorted and without repeats.
    pub fn layers(&self) -> Vec<String> {
        let mut names: Vec<String> = self.entities.iter().map(|e| e.layer.clone()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// The entities of a layer (without regard to case).
    pub fn on_layer<'a>(&'a self, layer: &'a str) -> impl Iterator<Item = &'a Entity> {
        self.entities
            .iter()
            .filter(move |e| e.layer.eq_ignore_ascii_case(layer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_polygon_is_closed_and_a_cross_is_two_lines() {
        let mut d = Drawing::new();
        d.polygon("A", &[(0, 0), (10, 0), (10, 10)]);
        match &d.entities[0].shape {
            Shape::Path { segs, closed } => {
                assert!(*closed);
                assert_eq!(segs.len(), 3);
                assert_eq!(segs[2].a, (10, 10));
                assert_eq!(segs[2].b, (0, 0));
            }
            other => panic!("{other:?}"),
        }
        d.cross("B", (5, 5), 2);
        assert_eq!(d.entities.len(), 3);
        d.polygon("A", &[(0, 0), (1, 1)]);
        d.polyline("A", &[(0, 0)]);
        assert_eq!(d.entities.len(), 3);
        assert_eq!(d.layers(), vec!["A".to_owned(), "B".to_owned()]);
        assert_eq!(d.on_layer("b").count(), 2);
    }
}
