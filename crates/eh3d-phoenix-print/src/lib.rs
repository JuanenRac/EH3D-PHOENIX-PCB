// SPDX-License-Identifier: GPL-3.0-or-later
//! Printing of schematics, boards and panels (a prototype).
//!
//! The crate knows no document: it takes drawings made of lines, polygons, circles and text in
//! millimetres on layers of a kind, and puts them on paper.
//!
//! * [`draw`]: the shapes, the colours with transparency, the kinds of layer and the page;
//! * [`page`]: the content of one page put on the paper at a scale, with a border, a title block
//!   (project, title, sheet, revision, author, scale and paper, in the language chosen), a scale
//!   bar and optional dimension lines, and the choice of layers;
//! * [`svg`]: the writer of SVG, the same every time;
//! * [`pdf`] and [`pdfcheck`]: a minimal PDF made by hand, with the tables and the lengths worked
//!   out while writing, the standard fonts and vectors, and a small reader that checks them;
//! * [`render`] and [`output`]: the pages of a document turned into files, and the files written
//!   to a place that is new;
//! * [`config`], [`paper`], [`font`], [`text`] and [`codes`]: the settings file, the sizes of
//!   paper, the widths of the fonts, the words in the eight languages and the codes
//!   `PHOENIX-PRINT-<NNN>`.
//!
//! The viewer (`eh3d-phoenix-viewer`) makes the content of a sheet of a schematic, of a side of a
//! board and of a panel; this crate is a prototype whose PDF has not been opened in real readers
//! of PDF.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod config;
pub mod draw;
pub mod font;
pub mod output;
pub mod page;
pub mod paper;
pub mod pdf;
pub mod pdfcheck;
pub mod render;
pub mod svg;
pub mod text;

pub use codes::{PrintError, REGISTRY};
pub use config::PrintConfig;
pub use draw::{Anchor, Drawing, Item, Layer, LayerKind, Pt, Rgba, Shape};
pub use page::{compose, Content};
pub use paper::Paper;
pub use render::{render, Format, Rendered};
pub use text::PrintText;
