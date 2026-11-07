// SPDX-License-Identifier: GPL-3.0-or-later
//! Bill of materials with costs.
//!
//! * [`bom`]: the parts of a board (and the parts of a schematic that are not placed) grouped by
//!   value, footprint, maker and part number, the quantity and the references of each line, the
//!   cheapest supplier for the quantity of the batch with the waste, and the cost per board and
//!   per batch;
//! * [`assembly`]: the bills of the top face, the bottom face and the whole board, with the
//!   quantities of a board and of a panel and the parts that are not fitted listed apart;
//! * [`prices`] and [`parts`]: the local files that hold the prices by quantity break and the
//!   makers, part numbers and parts that are not fitted (nothing is ever fetched from a network);
//! * [`pcb`]: the estimate of the bare boards from the area, the layers, the thickness and the
//!   holes, with the tariffs of the settings;
//! * [`settings`]: every rate, percentage and limit, read from `config/costs.cfg`;
//! * [`report`]: the CSV, the JSON, the HTML and the text, the same every time;
//! * [`money`] and [`text`]: money in whole millionths, and the reading and the quoting of CSV
//!   and the escaping of HTML;
//! * [`import`]: the bill of another program read with a map of columns, written as part data or
//!   compared with a board;
//! * [`formats`]: the formats of that bill (a generic CSV, the partlist of EAGLE, and the CSV and
//!   the XML bills of KiCad), told apart by [`detect`] and read by [`import_as`];
//! * [`range`]: the ranges of references (`R1-R5`) of a cell of a bill, expanded to single parts;
//! * [`codes`]: the codes `PHOENIX-BOM-001` to `PHOENIX-BOM-024` with their titles in the eight
//!   languages.
//!
//! **State: prototype.** It has been tried on small boards made by the tests only. The tariffs of
//! the settings are an example and not the prices of any maker, so the cost of the boards is an
//! estimate of the shape of a quote and never a quote; the prices are the ones of the file the
//! user gives, with no check that they are current; the lines are not merged by equivalent parts.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod assembly;
pub mod bom;
pub mod codes;
pub mod formats;
pub mod import;
pub mod money;
pub mod parts;
pub mod pcb;
pub mod prices;
pub mod range;
pub mod report;
pub mod settings;
pub mod text;

pub use bom::{build_bom, natural_key, Bom, BomInputs, BomLine, Purchase};
pub use codes::{title, BomError, Finding, Severity};
pub use formats::{detect, import_as, ImportFormat};
pub use import::{bill_from_table, import_bill, ImportMap, ImportedBill, ImportedLine};
pub use parts::{PartData, PartsFile};
pub use pcb::{pcb_cost, polygon_area, PcbCost};
pub use prices::{PriceBook, PriceRow};
pub use report::{bom_csv, bom_csv_with, bom_html, bom_json, bom_text, line_status, COLUMNS};
pub use settings::{CostSettings, GroupBy};
