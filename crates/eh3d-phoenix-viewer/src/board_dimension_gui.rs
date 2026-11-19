// SPDX-License-Identifier: GPL-3.0-or-later
//! The dimension tool of the board editor in the window (built with the `gui` feature): the
//! dimensions already on the board with the points that can be taken, and the dimension in
//! progress or being edited as the pointer would leave it. Every decision is taken in
//! `eh3d-phoenix-boardedit` (`dimension_edit`), in code that is tested; this file has not been
//! seen on a screen and has not been used with a mouse.

use eframe::egui::{self, Color32, Pos2, Stroke};
use eh3d_phoenix_board::{Dimension, SilkItem};
use eh3d_phoenix_boardedit::dimension_edit::handles;
use eh3d_phoenix_boardedit::{EditState, Tool};
use eh3d_phoenix_model::Point;

use crate::gui::{ui_accent, ui_dim};

/// Draws the lines of a dimension and its measure as text next to the line.
fn draw_dimension(
    painter: &egui::Painter,
    d: &Dimension,
    colour: Color32,
    to_screen: &dyn Fn(Point) -> Pos2,
) {
    let Ok(items) = d.items() else {
        return;
    };
    let mut labelled = false;
    for item in &items {
        match item {
            SilkItem::Stroke { path, .. } => {
                for piece in path {
                    painter.line_segment(
                        [to_screen(piece.start()), to_screen(piece.end())],
                        Stroke::new(1.5, colour),
                    );
                }
            }
            SilkItem::Text { at, .. } => {
                if !labelled {
                    labelled = true;
                    painter.text(
                        to_screen(*at),
                        egui::Align2::LEFT_BOTTOM,
                        d.label_text(),
                        egui::FontId::proportional(12.0),
                        colour,
                    );
                }
            }
        }
    }
}

/// Draws, with the dimension tool chosen, the dimensions of the board with the points that can be
/// taken (shift and a click), the one whose point is taken in the accent colour, and the preview
/// of the dimension in progress or being edited with the pointer as its next point.
pub fn draw(
    painter: &egui::Painter,
    state: &EditState,
    pointer: Option<Point>,
    to_screen: &dyn Fn(Point) -> Pos2,
) {
    if state.tool != Tool::Dimension {
        return;
    }
    for (id, d) in &state.board().dimensions {
        let grabbed = state.dimension_grab.filter(|g| g.dimension == *id);
        draw_dimension(painter, d, ui_dim(), to_screen);
        for (handle, at) in handles(d) {
            let at = to_screen(at);
            if grabbed.is_some_and(|g| g.handle == handle) {
                painter.circle_filled(at, 4.0, ui_accent());
            } else {
                painter.circle_stroke(at, 3.0, Stroke::new(1.0, ui_dim()));
            }
        }
    }
    if let Some(shown) = pointer.and_then(|p| state.dimension_preview(p)) {
        draw_dimension(painter, &shown, ui_accent(), to_screen);
    }
}
