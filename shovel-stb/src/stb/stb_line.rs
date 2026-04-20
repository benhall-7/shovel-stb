use crate::{LineReplaceMode, Stb, StbError, TableLine};

/// Scoped edit session: one **row** or **column** of the table, then [`Self::finish`] to apply
/// pending cell hashes and (in [`LineReplaceMode::Full`]) group-table rebuilds for that line.
///
/// Holds `&mut [`Stb`]` (unlike [`crate::StbInnerCells`], which owns the [`Stb`]). Use
/// [`Self::stb`] to read the rest of the sheet while the borrow is active.
///
/// Indices along the line are **symmetric**:
/// - [`TableLine::Row`]`(r)`: index `i` is column `i` ([`LineReplaceMode::Full`]) or `i + 1`
///   ([`LineReplaceMode::Inner`], data rows only).
/// - [`TableLine::Column`]`(c)`: index `i` is table row `i` ([`LineReplaceMode::Full`]) or `i + 1`
///   ([`LineReplaceMode::Inner`]).
///
/// After changing cells through [`Self::get_mut`] or [`Self::set_line`], call [`Self::finish`] so
/// cell hashes and (in [`LineReplaceMode::Full`]) group tables match the strings.
///
/// **Cross-axis keys:** [`Self::get_by_cross_axis_key`] resolves the same way for a row or column line:
/// on a **row** line, `key` matches a **column header** (row `0`); on a **column** line, `key` matches a
/// **row key** (column `0`). [`LineReplaceMode::Inner`] uses the same span as numeric indices (no column `0`
/// on inner row lines; no header row on inner column lines).
pub struct StbLine<'a> {
    stb: &'a mut Stb,
    line: TableLine,
    mode: LineReplaceMode,
}

impl<'a> StbLine<'a> {
    pub(crate) fn new(
        stb: &'a mut Stb,
        line: TableLine,
        mode: LineReplaceMode,
    ) -> Result<Self, StbError> {
        stb.validate_line_access(line, mode)?;
        Ok(Self { stb, line, mode })
    }

    /// Read the underlying table while this line is borrowed (same idea as [`crate::StbInnerCells::stb`]).
    pub fn stb(&self) -> &Stb {
        &*self.stb
    }

    /// Number of cells along this line (same rules as [`Stb::replace_line`] lengths).
    pub fn len(&self) -> usize {
        self.stb.line_len(self.line, self.mode)
    }

    /// Read one cell along the line (`0 .. len()`).
    pub fn get(&self, i: usize) -> Result<&str, StbError> {
        let (row, col) = self.stb.line_coord(self.line, self.mode, i)?;
        Ok(self.stb.cell(row, col).expect("line_coord in bounds"))
    }

    /// Mutate one cell along the line (`0 .. len()`). Call [`Self::finish`] afterward.
    pub fn get_mut(&mut self, i: usize) -> Result<&mut String, StbError> {
        let (row, col) = self.stb.line_coord(self.line, self.mode, i)?;
        self.stb.cell_string_mut(row, col)
    }

    /// Copy the current line into an owned [`Vec`]. Edit it, then [`Self::set_line`], then [`Self::finish`].
    pub fn get_line(&self) -> Vec<String> {
        (0..self.len())
            .map(|i| self.get(i).expect("in range").to_string())
            .collect()
    }

    /// Replace every string along the line. Does not refresh hashes or group tables until [`Self::finish`].
    pub fn set_line(&mut self, cells: Vec<String>) -> Result<(), StbError> {
        self.stb.write_line_strings(self.line, self.mode, cells)
    }

    /// Apply hashes (and in [`LineReplaceMode::Full`], group rebuilds) for this line. Unlike
    /// [`crate::StbInnerCells::finish`], this does real work; call it after [`Self::get_mut`],
    /// [`Self::set_line`], or [`Self::get_mut_by_cross_axis_key`].
    pub fn finish(self) {
        self.stb.finish_line_edit(self.line, self.mode);
    }

    /// Read one cell by **cross-axis key** (header name on a row line, first-column key on a column line).
    pub fn get_by_cross_axis_key(&self, key: &str) -> Result<&str, StbError> {
        let stb: &Stb = self.stb;
        let (row, col) = resolve_cross_axis_cell(stb, self.line, self.mode, key)?;
        stb.cell(row, col).ok_or(StbError::CellOutOfBounds { row, col })
    }

    /// Like [`Self::get_by_cross_axis_key`], but mutable. Call [`Self::finish`] afterward.
    pub fn get_mut_by_cross_axis_key(&mut self, key: &str) -> Result<&mut String, StbError> {
        let line = self.line;
        let mode = self.mode;
        let (row, col) = resolve_cross_axis_cell(self.stb, line, mode, key)?;
        self.stb.cell_string_mut(row, col)
    }
}

fn resolve_cross_axis_cell(
    stb: &Stb,
    line: TableLine,
    mode: LineReplaceMode,
    key: &str,
) -> Result<(usize, usize), StbError> {
    match line {
        TableLine::Row(r) => {
            let col = resolve_col_for_row_line(stb, mode, key)?;
            Ok((r, col))
        }
        TableLine::Column(c) => {
            let r = resolve_row_for_column_line(stb, mode, key)?;
            Ok((r, c))
        }
    }
}

/// Smallest table index on the cross axis included for this mode (`Inner` omits key index `0`).
fn min_cross_axis_index(mode: LineReplaceMode) -> usize {
    match mode {
        LineReplaceMode::Full => 0,
        LineReplaceMode::Inner => 1,
    }
}

#[derive(Clone, Copy)]
enum CrossAxisTableIndex {
    /// Column index from the header row ([`crate::Stb::column_index_for_key`]).
    Column,
    /// Row index from the first column ([`crate::Stb::row_index_for_key`]).
    Row,
}

fn resolve_cross_axis_line_index(
    stb: &Stb,
    mode: LineReplaceMode,
    key: &str,
    which: CrossAxisTableIndex,
) -> Result<usize, StbError> {
    let min = min_cross_axis_index(mode);
    let at = |m: usize| match which {
        CrossAxisTableIndex::Column => stb.column_index_for_key(key, m),
        CrossAxisTableIndex::Row => stb.row_index_for_key(key, m),
    };
    match at(min) {
        Some(i) => Ok(i),
        None if min > 0 && at(0).is_some_and(|i| i < min) => {
            Err(StbError::LineCrossAxisKeyOutsideLine(key.to_string()))
        }
        None => Err(match which {
            CrossAxisTableIndex::Column => StbError::ColumnHeaderNotFound(key.to_string()),
            CrossAxisTableIndex::Row => StbError::RowKeyNotFound(key.to_string()),
        }),
    }
}

fn resolve_col_for_row_line(
    stb: &Stb,
    mode: LineReplaceMode,
    key: &str,
) -> Result<usize, StbError> {
    resolve_cross_axis_line_index(stb, mode, key, CrossAxisTableIndex::Column)
}

fn resolve_row_for_column_line(
    stb: &Stb,
    mode: LineReplaceMode,
    key: &str,
) -> Result<usize, StbError> {
    resolve_cross_axis_line_index(stb, mode, key, CrossAxisTableIndex::Row)
}
