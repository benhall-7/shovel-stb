use std::fmt;

/// How strictly `Stb::from_tables` validates loaded hashes and group buckets
/// against cell strings (crate-internal construction).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StbTablesValidation {
    /// Only check that `cell_hashes` has the correct height and row widths.
    DimensionsOnly,
    /// Also require that hashes and row/column groups match values derived from
    /// the header and data strings (recommended for untrusted input).
    #[default]
    Full,
}

/// Which derived table did not match when using [`StbTablesValidation::Full`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TablesMismatchKind {
    CellHashes,
    RowGroups,
    ColGroups,
}

/// Failure to build or load a table while preserving invariants.
#[derive(Debug)]
pub enum StbError {
    /// A data row has a different width than the header.
    Rectangular {
        data_row: usize,
        expected: usize,
        found: usize,
    },
    Csv(csv::Error),
    CsvRead(String),
    Io(std::io::Error),
    /// [`crate::Stb::set_inner_cell`] only allows table `row >= 1` and `col >= 1`
    /// (not row `0` or column `0`).
    NotInnerCell {
        row: usize,
        col: usize,
    },
    /// Table row or column index out of range for the current dimensions.
    CellOutOfBounds {
        row: usize,
        col: usize,
    },
    /// [`crate::Stb::set_row_key`] requires a table row index ≥ 1 (not the header row `0`).
    RowKeyRequiresDataRow {
        row: usize,
    },
    /// Loaded tables do not match cell strings (internal invariant).
    InternalInvariant(&'static str),
    /// Parsed cell hashes or group buckets disagree with strings (see [`StbTablesValidation::Full`]).
    TablesMismatch(TablesMismatchKind),
    /// [`crate::Stb::replace_line`] could not apply the requested slice (wrong length, invalid line for mode).
    LineReplaceInvalid(&'static str),
    /// Wrong number of cells passed to [`crate::Stb::replace_line`] or [`crate::StbLine::set_line`].
    LineReplaceBadLen {
        expected: usize,
        found: usize,
    },
    /// Index along a row/column line is out of range for [`crate::StbLine`].
    LineIndexOutOfBounds {
        len: usize,
        index: usize,
    },
    /// No column whose header matches this string ([`crate::StbLine::get_by_cross_axis_key`] on a row line).
    ColumnHeaderNotFound(String),
    /// No table row whose first-column key matches this string ([`crate::StbLine::get_by_cross_axis_key`] on a column line).
    RowKeyNotFound(String),
    /// Key resolves only to table index `0` on the cross axis, but inner line mode omits that index.
    LineCrossAxisKeyOutsideLine(String),
}

impl fmt::Display for StbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StbError::Rectangular {
                data_row,
                expected,
                found,
            } => write!(
                f,
                "row {data_row} has {found} cells, expected {expected} (header width)"
            ),
            StbError::Csv(e) => write!(f, "{e}"),
            StbError::CsvRead(msg) => write!(f, "{msg}"),
            StbError::Io(e) => write!(f, "{e}"),
            StbError::NotInnerCell { row, col } => write!(
                f,
                "inner-cell edits cannot target header row or first column (row={row}, col={col})"
            ),
            StbError::CellOutOfBounds { row, col } => {
                write!(f, "cell index out of bounds (row={row}, col={col})")
            }
            StbError::RowKeyRequiresDataRow { row } => write!(
                f,
                "set_row_key requires table row >= 1 (data row), got {row}"
            ),
            StbError::InternalInvariant(msg) => write!(f, "{msg}"),
            StbError::LineReplaceInvalid(msg) => write!(f, "{msg}"),
            StbError::LineReplaceBadLen { expected, found } => {
                write!(f, "line edit expected {expected} cell(s), got {found}")
            }
            StbError::LineIndexOutOfBounds { len, index } => {
                write!(f, "line index {index} out of bounds (len {len})")
            }
            StbError::ColumnHeaderNotFound(key) => {
                write!(f, "no column with header `{key}`")
            }
            StbError::RowKeyNotFound(key) => {
                write!(f, "no data/header row with first-column key `{key}`")
            }
            StbError::LineCrossAxisKeyOutsideLine(key) => write!(
                f,
                "cross-axis key `{key}` matches only index 0 on this axis, which is not part of an inner line"
            ),
            StbError::TablesMismatch(kind) => match kind {
                TablesMismatchKind::CellHashes => {
                    write!(
                        f,
                        "cell_hashes do not match strings (recomputed hashes differ)"
                    )
                }
                TablesMismatchKind::RowGroups => {
                    write!(f, "row_groups do not match cell_hashes")
                }
                TablesMismatchKind::ColGroups => {
                    write!(f, "col_groups do not match cell_hashes")
                }
            },
        }
    }
}

impl std::error::Error for StbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StbError::Csv(e) => Some(e),
            StbError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<csv::Error> for StbError {
    fn from(e: csv::Error) -> Self {
        StbError::Csv(e)
    }
}

impl From<std::io::Error> for StbError {
    fn from(e: std::io::Error) -> Self {
        StbError::Io(e)
    }
}
