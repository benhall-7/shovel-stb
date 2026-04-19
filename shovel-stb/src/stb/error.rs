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
    Io(std::io::Error),
    /// [`crate::Stb::set_inner_cell`] only allows `row >= 1` and `col >= 1` (not the header row or first column).
    NotInnerCell {
        row: usize,
        col: usize,
    },
    /// Table row or column index out of range for the current dimensions.
    CellOutOfBounds {
        row: usize,
        col: usize,
    },
    /// Loaded tables do not match cell strings (internal invariant).
    InternalInvariant(&'static str),
    /// Parsed cell hashes or group buckets disagree with strings (see [`StbTablesValidation::Full`]).
    TablesMismatch(TablesMismatchKind),
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
            StbError::Io(e) => write!(f, "{e}"),
            StbError::NotInnerCell { row, col } => write!(
                f,
                "inner-cell edits cannot target header row or first column (row={row}, col={col})"
            ),
            StbError::CellOutOfBounds { row, col } => {
                write!(f, "cell index out of bounds (row={row}, col={col})")
            }
            StbError::InternalInvariant(msg) => write!(f, "{msg}"),
            StbError::TablesMismatch(kind) => match kind {
                TablesMismatchKind::CellHashes => {
                    write!(f, "cell_hashes do not match strings (recomputed hashes differ)")
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
