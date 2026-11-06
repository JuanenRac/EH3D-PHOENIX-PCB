// SPDX-License-Identifier: GPL-3.0-or-later
//! From the pages of a document to the bytes of its files: every page is composed on the paper,
//! and written as an SVG file of its own or as a page of a single PDF, which is checked by the
//! reader of this crate before it is given back.

use eh3d_phoenix_rules::Lang;

use crate::codes::{PrintError, NOTHING_TO_PRINT, PDF_CHECK_FAILED};
use crate::config::PrintConfig;
use crate::page::{compose, Content};
use crate::pdf::to_pdf;
use crate::pdfcheck::check_pdf;
use crate::svg::to_svg;

/// The kind of file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// One SVG file for each page.
    Svg,
    /// One PDF file with all the pages.
    Pdf,
}

impl Format {
    /// Reads `svg` or `pdf`, in any case.
    pub fn parse(text: &str) -> Option<Format> {
        match text.trim().to_ascii_lowercase().as_str() {
            "svg" => Some(Format::Svg),
            "pdf" => Some(Format::Pdf),
            _ => None,
        }
    }

    /// The extension of the files, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Svg => "svg",
            Format::Pdf => "pdf",
        }
    }
}

/// A file ready to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFile {
    /// The name of the file, with its extension.
    pub name: String,
    /// The content.
    pub bytes: Vec<u8>,
    /// The pages that it holds.
    pub pages: usize,
}

/// What was worked out for a page.
#[derive(Debug, Clone, PartialEq)]
pub struct PageInfo {
    /// The title.
    pub title: String,
    /// The width of the paper in millimetres.
    pub width: f64,
    /// The height of the paper in millimetres.
    pub height: f64,
    /// The scale: paper over design.
    pub scale: f64,
}

/// The files of a document and what is worth saying about them.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    /// The format.
    pub format: Format,
    /// The files.
    pub files: Vec<RenderedFile>,
    /// One entry for each page.
    pub pages: Vec<PageInfo>,
    /// How many characters of a PDF were drawn as boxes because the fonts lack them.
    pub boxed_chars: usize,
}

/// Composes the pages and writes them. `name` is the name of the PDF file, without the
/// extension; an SVG file is named after its page.
pub fn render(
    name: &str,
    contents: &[Content],
    format: Format,
    cfg: &PrintConfig,
    lang: Lang,
) -> Result<Rendered, PrintError> {
    if contents.is_empty() {
        return Err(PrintError::new(NOTHING_TO_PRINT, name));
    }
    let mut drawings = Vec::new();
    for content in contents {
        drawings.push(compose(content, cfg, lang)?);
    }
    let pages: Vec<PageInfo> = drawings
        .iter()
        .map(|d| PageInfo {
            title: d.title.clone(),
            width: d.width,
            height: d.height,
            scale: d.scale,
        })
        .collect();
    match format {
        Format::Svg => {
            let files = contents
                .iter()
                .zip(&drawings)
                .map(|(content, drawing)| RenderedFile {
                    name: format!("{}.svg", content.name),
                    bytes: to_svg(drawing).into_bytes(),
                    pages: 1,
                })
                .collect();
            Ok(Rendered {
                format,
                files,
                pages,
                boxed_chars: 0,
            })
        }
        Format::Pdf => {
            let out = to_pdf(&drawings);
            let info =
                check_pdf(&out.bytes).map_err(|why| PrintError::new(PDF_CHECK_FAILED, why))?;
            if info.pages != drawings.len() {
                return Err(PrintError::new(
                    PDF_CHECK_FAILED,
                    format!("{} pages written, {} found", drawings.len(), info.pages),
                ));
            }
            Ok(Rendered {
                format,
                files: vec![RenderedFile {
                    name: format!("{name}.pdf"),
                    bytes: out.bytes,
                    pages: drawings.len(),
                }],
                pages,
                boxed_chars: out.boxed_chars,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{Item, Layer, LayerKind, Rgba};

    fn content(name: &str, no: usize) -> Content {
        Content {
            name: name.to_owned(),
            project: "P".to_owned(),
            title: format!("T{no}"),
            sheet_no: no,
            sheet_count: 2,
            layers: vec![Layer {
                kind: LayerKind::Outline,
                items: vec![Item::rect(
                    (0.0, 0.0),
                    (80.0, 40.0),
                    Rgba::new(0, 0, 0, 255),
                    0.1,
                    false,
                )],
            }],
            dimension_box: None,
        }
    }

    #[test]
    fn svg_gives_a_file_for_each_page_and_pdf_one_for_all() {
        let pages = [content("sheet-01", 1), content("sheet-02", 2)];
        let cfg = PrintConfig::default();
        let svg = render("doc", &pages, Format::Svg, &cfg, Lang::En).unwrap();
        let names: Vec<&str> = svg.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["sheet-01.svg", "sheet-02.svg"]);
        assert_eq!(svg.pages.len(), 2);
        let pdf = render("doc", &pages, Format::Pdf, &cfg, Lang::En).unwrap();
        assert_eq!(pdf.files.len(), 1);
        assert_eq!(pdf.files[0].name, "doc.pdf");
        assert_eq!(pdf.files[0].pages, 2);
        assert_eq!(pdf.pages[1].title, "T2");
        // the same input, the same bytes
        let again = render("doc", &pages, Format::Pdf, &cfg, Lang::En).unwrap();
        assert_eq!(pdf, again);
    }

    #[test]
    fn nothing_to_print_and_an_error_of_a_page_are_reported() {
        let cfg = PrintConfig::default();
        let error = render("doc", &[], Format::Pdf, &cfg, Lang::En).unwrap_err();
        assert_eq!(error.code, NOTHING_TO_PRINT);
        let mut empty = content("a", 1);
        empty.layers.clear();
        let error = render("doc", &[empty], Format::Svg, &cfg, Lang::En).unwrap_err();
        assert_eq!(error.code, NOTHING_TO_PRINT);
    }

    #[test]
    fn the_formats_are_read() {
        assert_eq!(Format::parse(" SVG "), Some(Format::Svg));
        assert_eq!(Format::parse("pdf"), Some(Format::Pdf));
        assert_eq!(Format::parse("png"), None);
        assert_eq!(Format::Pdf.extension(), "pdf");
    }
}
