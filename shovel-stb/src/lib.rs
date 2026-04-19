//! Shovel Knight spreadsheet and localization table formats.

pub mod stb;
pub mod stl;

mod strings;

pub use stb::error::{StbError, StbTablesValidation, TablesMismatchKind};
pub use stb::groups;
pub use stb::hash;
pub use stb::hash::stb_hash;
pub use stb::inner_cell_editor::InnerCellEditor;
pub use stl::Stl;

use stb::groups::Group;

/// A validated STB spreadsheet: a header row plus data rows, all rectangular.
///
/// Cell hashes and row/column group buckets are **private** and kept consistent
/// with cell strings. Use [`Self::set_inner_cell`] for edits that do not touch
/// the header row or first column; other changes will use additional APIs later.
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

    /// Used by [`crate::stb::read`](stb::read) after file contents are verified.
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
        let ntr = rows.len() + 1;
        let nc = columns.len();
        if cell_hashes.len() != ntr {
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

    /// Column headers (row 0 of the logical table).
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Data rows (everything after the header). Each row has `columns().len()`
    /// cells.
    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }

    /// Number of logical rows including the header (`1 + data rows`).
    pub fn num_rows(&self) -> usize {
        self.rows.len() + 1
    }

    /// Hash for one cell: `row == 0` is the header row; `1` is the first
    /// data row.
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

    /// Number of data rows (excludes the header row).
    pub fn num_data_rows(&self) -> usize {
        self.rows.len()
    }

    /// Number of columns.
    pub fn num_cols(&self) -> usize {
        self.columns.len()
    }

    /// Look up a cell by (row, col). Row 0 is the first *data* row.
    pub fn get(&self, row: usize, col: usize) -> Option<&str> {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(|s| s.as_str())
    }

    /// Find the column index for a given header name.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|h| h == name)
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
