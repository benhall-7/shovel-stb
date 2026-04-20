/// Identifies a full **row** or **column** in table coordinates (see [`crate::Stb`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableLine {
    /// Table row index `0 .. num_rows` (row `0` is the header).
    Row(usize),
    /// Column index `0 .. num_cols` (column `0` is the first column).
    Column(usize),
}

/// Whether a line replace touches **key** cells at index `0` on the line’s axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineReplaceMode {
    /// Only **inner** cells: on a row line, columns `1..`; on a column line, rows `1..`.
    /// Cheaper: group buckets are unchanged (same as repeated [`crate::Stb::set_inner_cell`]).
    Inner,
    /// The **full** line, including `(row, 0)` on a data row or `(0, col)` on any column.
    /// May rebuild row and/or column group tables when keys change.
    Full,
}
