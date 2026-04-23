//! Shovel Knight spreadsheet and localization table formats.

pub mod stb;
pub mod stl;

mod strings;

pub use stb::error::{StbError, StbTablesValidation, TablesMismatchKind};
pub use stb::groups;
pub use stb::hash;
pub use stb::hash::stb_hash;
pub use stb::stb_inner_cells::StbInnerCells;
pub use stb::stb_line::StbLine;
pub use stb::table_line::{LineReplaceMode, TableLine};
pub use stl::Stl;

use stb::groups::{Group, bucket_index_for_hash};

/// A validated STB spreadsheet: a header row plus data rows, all rectangular.
///
/// Cell hashes and row/column group buckets are **private** and kept consistent
/// with cell strings:
/// - [`Self::set_inner_cell`] — inner cells only; group tables unchanged.
/// - [`Self::set_row_key`] — key cell for a data row (first column); recomputes row groups.
/// - [`Self::set_column_key`] — key cell for a column (header row); recomputes column
///   groups, and row groups as well when `col == 0`.
///
/// Table indices are **always** zero-based: `row == 0` is the header, `col == 0`
/// is the first column — same for [`Self::cell`], [`Self::cell_hash`], and edits.
/// Key-based scans: [`Self::row_for_column`], [`Self::column_for_row`], [`Self::row_for_named_column`]
/// (header row and column `0` use the on-disk row/column group buckets; see [`Self::col_group_bucket_for_key`],
/// [`Self::row_group_bucket_for_key`]).
/// Scoped sessions: [`StbInnerCells`] (owned [`Stb`], inner cells only; cheap [`StbInnerCells::finish`])
/// vs [`Self::line_mut`] / [`StbLine`] (borrowed [`Stb`], one row/column; [`StbLine::finish`] refreshes that line).
/// Line modes: [`LineReplaceMode::Inner`] (no index `0` on that line) vs [`LineReplaceMode::Full`] (includes keys).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stb {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    cell_hashes: Vec<Vec<u32>>,
    row_groups: Vec<Group>,
    col_groups: Vec<Group>,
}

impl Stb {
    /// Build a table from a header row and data rows.
    ///
    /// Returns an error if any data row does not have the same length as
    /// `columns`. Hashes and group buckets are computed from the strings.
    pub fn from_rows(columns: Vec<String>, rows: Vec<Vec<String>>) -> Result<Self, StbError> {
        validate_rectangular(&columns, &rows)?;
        let (cell_hashes, row_groups, col_groups) = build_tables(&columns, &rows);
        Ok(Self {
            columns,
            rows,
            cell_hashes,
            row_groups,
            col_groups,
        })
    }

    /// Used by the STB binary reader ([`crate::stb::io`]) after file contents are verified.
    ///
    /// `validation` controls whether hashes and group buckets are checked
    /// against values derived from cell strings ([`StbTablesValidation::Full`]),
    /// or only dimensions ([`StbTablesValidation::DimensionsOnly`]).
    pub(crate) fn from_tables(
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        cell_hashes: Vec<Vec<u32>>,
        row_groups: Vec<Group>,
        col_groups: Vec<Group>,
        validation: StbTablesValidation,
    ) -> Result<Self, StbError> {
        validate_rectangular(&columns, &rows)?;
        let nr = rows.len() + 1;
        let nc = columns.len();
        if cell_hashes.len() != nr {
            return Err(StbError::InternalInvariant(
                "cell_hashes height does not match row count",
            ));
        }
        for row in &cell_hashes {
            if row.len() != nc {
                return Err(StbError::InternalInvariant(
                    "cell_hashes row width does not match column count",
                ));
            }
        }

        if validation == StbTablesValidation::Full {
            let (expected_hashes, expected_row_groups, expected_col_groups) =
                build_tables(&columns, &rows);
            if expected_hashes != cell_hashes {
                return Err(StbError::TablesMismatch(TablesMismatchKind::CellHashes));
            }
            if expected_row_groups != row_groups {
                return Err(StbError::TablesMismatch(TablesMismatchKind::RowGroups));
            }
            if expected_col_groups != col_groups {
                return Err(StbError::TablesMismatch(TablesMismatchKind::ColGroups));
            }
        }

        Ok(Self {
            columns,
            rows,
            cell_hashes,
            row_groups,
            col_groups,
        })
    }

    /// Edit a single **inner** cell (`row >= 1`, `col >= 1`).
    ///
    /// Does not change row/column group buckets (they depend only on the first
    /// column per row and the header row).
    pub fn set_inner_cell(
        &mut self,
        row: usize,
        col: usize,
        value: String,
    ) -> Result<(), StbError> {
        if row == 0 || col == 0 {
            return Err(StbError::NotInnerCell { row, col });
        }
        let num_tr = self.num_rows();
        let num_c = self.num_cols();
        if row >= num_tr || col >= num_c {
            return Err(StbError::CellOutOfBounds { row, col });
        }
        let data_row = row - 1;
        self.rows[data_row][col] = value;
        let h = stb_hash(&self.rows[data_row][col]);
        self.cell_hashes[row][col] = h;
        Ok(())
    }

    /// Set the **row key** (first column of a data row, `row >= 1`). Rebuilds [`Self::row_groups`] only.
    pub fn set_row_key(&mut self, row: usize, value: String) -> Result<(), StbError> {
        if row < 1 {
            return Err(StbError::RowKeyRequiresDataRow { row });
        }
        let num_tr = self.num_rows();
        if row >= num_tr {
            return Err(StbError::CellOutOfBounds { row, col: 0 });
        }
        let data_row = row - 1;
        self.rows[data_row][0] = value;
        self.cell_hashes[row][0] = stb_hash(&self.rows[data_row][0]);
        self.row_groups = rebuild_row_groups(&self.cell_hashes);
        Ok(())
    }

    /// Set the **column key** (header row, `row == 0`). Rebuilds [`Self::col_groups`];
    /// if `col == 0`, also rebuilds [`Self::row_groups`].
    pub fn set_column_key(&mut self, col: usize, value: String) -> Result<(), StbError> {
        let num_c = self.num_cols();
        if col >= num_c {
            return Err(StbError::CellOutOfBounds { row: 0, col });
        }
        self.columns[col] = value;
        self.cell_hashes[0][col] = stb_hash(&self.columns[col]);
        self.col_groups = rebuild_col_groups(&self.cell_hashes, num_c);
        if col == 0 {
            self.row_groups = rebuild_row_groups(&self.cell_hashes);
        }
        Ok(())
    }

    /// Borrow one row or column for editing; finish with [`StbLine::finish`] after [`StbLine::get_mut`] or [`StbLine::set_line`].
    pub fn line_mut(
        &mut self,
        line: TableLine,
        mode: LineReplaceMode,
    ) -> Result<StbLine<'_>, StbError> {
        StbLine::new(self, line, mode)
    }

    /// Replace every cell along one **row** ([`TableLine::Row`]) or **column** ([`TableLine::Column`]).
    ///
    /// * [`LineReplaceMode::Inner`] — only columns `1..` on a data row, or only rows `1..` on a column;
    ///   cannot target row `0` or column `0`. Same cost as repeated [`Self::set_inner_cell`].
    /// * [`LineReplaceMode::Full`] — entire line, including `(row, 0)` and `(0, col)` when present;
    ///   may rebuild group tables when keys change.
    ///
    /// Cell counts: **Row** — `num_cols()` ([`LineReplaceMode::Full`]) or `num_cols() - 1` ([`LineReplaceMode::Inner`],
    /// data row only). **Column** — `num_rows()` ([`LineReplaceMode::Full`]) or `num_body_rows()` ([`LineReplaceMode::Inner`]).
    ///
    /// Equivalent to [`Self::line_mut`], then [`StbLine::set_line`], then [`StbLine::finish`].
    pub fn replace_line(
        &mut self,
        line: TableLine,
        mode: LineReplaceMode,
        cells: Vec<String>,
    ) -> Result<(), StbError> {
        self.validate_line_access(line, mode)?;
        let expected = self.line_len(line, mode);
        if cells.len() != expected {
            return Err(StbError::LineReplaceBadLen {
                expected,
                found: cells.len(),
            });
        }
        self.write_line_strings(line, mode, cells)?;
        self.finish_line_edit(line, mode);
        Ok(())
    }

    /// Column headers (row 0 of the logical table).
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Data rows (everything after the header). Each row has `columns().len()`
    /// cells.
    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }

    /// Number of table rows (including the header row).
    pub fn num_rows(&self) -> usize {
        self.rows.len() + 1
    }

    /// Number of body rows (excluding the header); `num_rows() == num_body_rows() + 1`.
    pub fn num_body_rows(&self) -> usize {
        self.rows.len()
    }

    /// Number of columns.
    pub fn num_cols(&self) -> usize {
        self.columns.len()
    }

    /// Read one cell. `row` and `col` are table indices (`0 .. num_rows` and
    /// `0 .. num_cols`): row `0` is the header, column `0` is the first column.
    pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
        if col >= self.num_cols() {
            return None;
        }
        if row == 0 {
            return self.columns.get(col).map(|s| s.as_str());
        }
        let dr = row.checked_sub(1)?;
        self.rows.get(dr)?.get(col).map(|s| s.as_str())
    }

    /// Hash for one cell; indices follow [`Self::cell`].
    pub fn cell_hash(&self, row: usize, col: usize) -> Option<u32> {
        self.cell_hashes.get(row)?.get(col).copied()
    }

    /// Row-group index buckets.
    pub fn row_groups(&self) -> &[Group] {
        &self.row_groups
    }

    /// Column-group index buckets.
    pub fn col_groups(&self) -> &[Group] {
        &self.col_groups
    }

    /// Column-group bucket for a header string (`stb_hash` + same bucketing as the STB file). Inspect
    /// [`Group::entries`] to search without scanning every column.
    pub fn col_group_bucket_for_key(&self, key: &str) -> &Group {
        let nc = self.num_cols();
        let idx = bucket_index_for_hash(stb_hash(key), nc);
        &self.col_groups[idx]
    }

    /// Row-group bucket for a first-column key (`stb_hash` + same bucketing as the STB file).
    pub fn row_group_bucket_for_key(&self, key: &str) -> &Group {
        let nr = self.num_rows();
        let idx = bucket_index_for_hash(stb_hash(key), nr);
        &self.row_groups[idx]
    }

    /// First **table row** index `r` such that `cell(r, col) == value`.
    ///
    /// When `col == 0`, uses [`Self::row_group_bucket_for_key`] (same as the binary’s row groups).
    pub fn row_for_column(&self, col: usize, value: &str) -> Option<usize> {
        if col >= self.num_cols() {
            return None;
        }
        if col == 0 {
            self.row_index_for_key(value, 0)
        } else {
            (0..self.num_rows()).find(|&r| self.cell(r, col) == Some(value))
        }
    }

    /// First column index `c` such that `cell(row, c) == value`.
    ///
    /// When `row == 0`, uses [`Self::col_group_bucket_for_key`] (same as the binary’s column groups).
    pub fn column_for_row(&self, row: usize, value: &str) -> Option<usize> {
        if row >= self.num_rows() {
            return None;
        }
        if row == 0 {
            self.column_index_for_key(value, 0)
        } else {
            (0..self.num_cols()).find(|&c| self.cell(row, c) == Some(value))
        }
    }

    /// Like [`Self::row_for_column`], but resolves `col` by matching a header name in row `0`.
    pub fn row_for_named_column(&self, column_name: &str, value: &str) -> Option<usize> {
        let col = self.column_index(column_name)?;
        self.row_for_column(col, value)
    }

    /// Column index whose header cell (row `0`) equals `name` (first match).
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.column_for_row(0, name)
    }

    /// Lowest table row index `r >= min_row` with `cell(r, 0) == key` (uses [`Self::row_group_bucket_for_key`]).
    pub(crate) fn row_index_for_key(&self, key: &str, min_row: usize) -> Option<usize> {
        let nr = self.num_rows();
        if nr == 0 || min_row >= nr {
            return None;
        }
        let h = stb_hash(key);
        let b = bucket_index_for_hash(h, nr);
        let bucket = &self.row_groups[b];
        let mut best: Option<usize> = None;
        for e in &bucket.entries {
            if e.hash != h {
                continue;
            }
            let r = e.index as usize;
            if r < min_row || r >= nr {
                continue;
            }
            if self.cell(r, 0) == Some(key) {
                best = Some(best.map_or(r, |x| x.min(r)));
            }
        }
        best
    }

    /// Lowest table column index `c >= min_col` with `cell(0, c) == key` (uses [`Self::col_group_bucket_for_key`]).
    pub(crate) fn column_index_for_key(&self, key: &str, min_col: usize) -> Option<usize> {
        let nc = self.num_cols();
        if nc == 0 || min_col >= nc {
            return None;
        }
        let h = stb_hash(key);
        let b = bucket_index_for_hash(h, nc);
        let bucket = &self.col_groups[b];
        let mut best: Option<usize> = None;
        for e in &bucket.entries {
            if e.hash != h {
                continue;
            }
            let c = e.index as usize;
            if c < min_col || c >= nc {
                continue;
            }
            if self.cell(0, c) == Some(key) {
                best = Some(best.map_or(c, |x| x.min(c)));
            }
        }
        best
    }

    pub(crate) fn validate_line_access(
        &self,
        line: TableLine,
        mode: LineReplaceMode,
    ) -> Result<(), StbError> {
        let nr = self.num_rows();
        let nc = self.num_cols();
        match (line, mode) {
            (TableLine::Row(r), LineReplaceMode::Inner) => {
                if r == 0 {
                    return Err(StbError::LineReplaceInvalid(
                        "LineReplaceMode::Inner cannot replace header row (row 0)",
                    ));
                }
                if r >= nr {
                    return Err(StbError::CellOutOfBounds { row: r, col: 0 });
                }
                Ok(())
            }
            (TableLine::Row(r), LineReplaceMode::Full) => {
                if r >= nr {
                    return Err(StbError::CellOutOfBounds { row: r, col: 0 });
                }
                Ok(())
            }
            (TableLine::Column(c), LineReplaceMode::Inner) => {
                if c == 0 {
                    return Err(StbError::LineReplaceInvalid(
                        "LineReplaceMode::Inner cannot replace first column (column 0)",
                    ));
                }
                if c >= nc {
                    return Err(StbError::CellOutOfBounds { row: 0, col: c });
                }
                Ok(())
            }
            (TableLine::Column(c), LineReplaceMode::Full) => {
                if c >= nc {
                    return Err(StbError::CellOutOfBounds { row: 0, col: c });
                }
                Ok(())
            }
        }
    }

    pub(crate) fn line_len(&self, line: TableLine, mode: LineReplaceMode) -> usize {
        let nc = self.num_cols();
        let nr = self.num_rows();
        let nb = self.num_body_rows();
        match (line, mode) {
            (TableLine::Row(_), LineReplaceMode::Inner) => nc.saturating_sub(1),
            (TableLine::Row(_), LineReplaceMode::Full) => nc,
            (TableLine::Column(_), LineReplaceMode::Inner) => nb,
            (TableLine::Column(_), LineReplaceMode::Full) => nr,
        }
    }

    pub(crate) fn line_coord(
        &self,
        line: TableLine,
        mode: LineReplaceMode,
        i: usize,
    ) -> Result<(usize, usize), StbError> {
        let len = self.line_len(line, mode);
        if i >= len {
            return Err(StbError::LineIndexOutOfBounds { len, index: i });
        }
        let coord = match (line, mode) {
            (TableLine::Row(r), LineReplaceMode::Full) => (r, i),
            (TableLine::Row(r), LineReplaceMode::Inner) => (r, i + 1),
            (TableLine::Column(c), LineReplaceMode::Full) => (i, c),
            (TableLine::Column(c), LineReplaceMode::Inner) => (i + 1, c),
        };
        Ok(coord)
    }

    pub(crate) fn cell_string_mut(
        &mut self,
        row: usize,
        col: usize,
    ) -> Result<&mut String, StbError> {
        let num_tr = self.num_rows();
        let num_c = self.num_cols();
        if row >= num_tr || col >= num_c {
            return Err(StbError::CellOutOfBounds { row, col });
        }
        if row == 0 {
            Ok(&mut self.columns[col])
        } else {
            Ok(&mut self.rows[row - 1][col])
        }
    }

    pub(crate) fn write_line_strings(
        &mut self,
        line: TableLine,
        mode: LineReplaceMode,
        cells: Vec<String>,
    ) -> Result<(), StbError> {
        let expected = self.line_len(line, mode);
        if cells.len() != expected {
            return Err(StbError::LineReplaceBadLen {
                expected,
                found: cells.len(),
            });
        }

        match (line, mode) {
            (TableLine::Row(r), LineReplaceMode::Inner) => {
                let dr = r - 1;
                for (i, val) in cells.into_iter().enumerate() {
                    self.rows[dr][i + 1] = val;
                }
            }
            (TableLine::Row(r), LineReplaceMode::Full) => {
                if r == 0 {
                    for (col, val) in cells.into_iter().enumerate() {
                        self.columns[col] = val;
                    }
                } else {
                    let dr = r - 1;
                    for (col, val) in cells.into_iter().enumerate() {
                        self.rows[dr][col] = val;
                    }
                }
            }
            (TableLine::Column(c), LineReplaceMode::Inner) => {
                for (br, val) in cells.into_iter().enumerate() {
                    self.rows[br][c] = val;
                }
            }
            (TableLine::Column(c), LineReplaceMode::Full) => {
                for (table_row, val) in cells.into_iter().enumerate() {
                    if table_row == 0 {
                        self.columns[c] = val;
                    } else {
                        let dr = table_row - 1;
                        self.rows[dr][c] = val;
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn finish_line_edit(&mut self, line: TableLine, mode: LineReplaceMode) {
        let len = self.line_len(line, mode);
        for i in 0..len {
            let (row, col) = self.line_coord(line, mode, i).expect("i < len");
            let h = if row == 0 {
                stb_hash(&self.columns[col])
            } else {
                stb_hash(&self.rows[row - 1][col])
            };
            self.cell_hashes[row][col] = h;
        }

        if mode == LineReplaceMode::Inner {
            return;
        }

        let nc = self.num_cols();
        match line {
            TableLine::Row(r) => {
                if r == 0 {
                    self.col_groups = rebuild_col_groups(&self.cell_hashes, nc);
                    self.row_groups = rebuild_row_groups(&self.cell_hashes);
                } else {
                    self.row_groups = rebuild_row_groups(&self.cell_hashes);
                }
            }
            TableLine::Column(c) => {
                self.col_groups = rebuild_col_groups(&self.cell_hashes, nc);
                if c == 0 {
                    self.row_groups = rebuild_row_groups(&self.cell_hashes);
                }
            }
        }
    }
}

fn validate_rectangular(columns: &[String], rows: &[Vec<String>]) -> Result<(), StbError> {
    let expected = columns.len();
    for (data_row, row) in rows.iter().enumerate() {
        if row.len() != expected {
            return Err(StbError::Rectangular {
                data_row,
                expected,
                found: row.len(),
            });
        }
    }
    Ok(())
}

fn build_tables(
    columns: &[String],
    rows: &[Vec<String>],
) -> (Vec<Vec<u32>>, Vec<Group>, Vec<Group>) {
    let mut cell_hashes: Vec<Vec<u32>> = Vec::with_capacity(rows.len() + 1);
    cell_hashes.push(columns.iter().map(|s| stb_hash(s)).collect());
    for row in rows {
        cell_hashes.push(row.iter().map(|s| stb_hash(s)).collect());
    }

    let num_rows_total = cell_hashes.len();
    let row_groups = groups::build_groups(
        (0..num_rows_total as u32)
            .map(|i| (i, cell_hashes[i as usize][0]))
            .collect(),
        num_rows_total,
    );
    let num_cols = columns.len();
    let col_groups = groups::build_groups(
        (0..num_cols as u32)
            .map(|i| (i, cell_hashes[0][i as usize]))
            .collect(),
        num_cols,
    );

    (cell_hashes, row_groups, col_groups)
}

fn rebuild_row_groups(cell_hashes: &[Vec<u32>]) -> Vec<Group> {
    let num_rows_total = cell_hashes.len();
    groups::build_groups(
        (0..num_rows_total as u32)
            .map(|i| (i, cell_hashes[i as usize][0]))
            .collect(),
        num_rows_total,
    )
}

fn rebuild_col_groups(cell_hashes: &[Vec<u32>], num_cols: usize) -> Vec<Group> {
    groups::build_groups(
        (0..num_cols as u32)
            .map(|i| (i, cell_hashes[0][i as usize]))
            .collect(),
        num_cols,
    )
}

impl std::fmt::Display for Stb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, col) in self.columns.iter().enumerate() {
            if i > 0 {
                write!(f, "\t")?;
            }
            write!(f, "{col}")?;
        }
        writeln!(f)?;

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i > 0 {
                    write!(f, "\t")?;
                }
                write!(f, "{cell}")?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
