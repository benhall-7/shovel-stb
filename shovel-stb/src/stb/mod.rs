//! Spreadsheet table binaries (`.stb` / `.stm`) and related types.
//!
//! **Symmetric rows and columns.** The STB binary layout treats the header row and the first
//! column as parallel key axes; public APIs should mirror that (same indexing rules, inner vs.
//! full line edits, and group rebuild behavior) whether the consumer is addressing a row or a column.

pub mod error;
pub mod groups;
pub mod hash;
pub mod inner_cell_editor;
pub mod stb_line;
pub mod table_line;

mod csv;
mod read;
mod write;
