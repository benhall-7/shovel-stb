//! Shovel Knight spreadsheet and localization table formats.

pub mod stb;
pub mod stl;

mod strings;

pub use stb::groups;
pub use stb::hash;
pub use stb::hash::stb_hash;
pub use stl::Stl;

/// A parsed STB spreadsheet.
#[derive(Debug, Clone)]
pub struct Stb {
    /// Column headers (from row 0 of the file).
    pub columns: Vec<String>,
    /// Data rows (everything after row 0). Each inner `Vec` has
    /// `columns.len()` elements.
    pub rows: Vec<Vec<String>>,
    /// The raw 32-bit hash stored alongside every cell value.
    /// Indexed as `cell_hashes[row_index][col_index]` where row 0 corresponds
    /// to the *header* row.
    pub cell_hashes: Vec<Vec<u32>>,
    /// Row-group index buckets.
    pub row_groups: Vec<groups::Group>,
    /// Column-group index buckets.
    pub col_groups: Vec<groups::Group>,
}

impl Stb {
    /// Build an `Stb` from column headers and data rows.
    ///
    /// Cell hashes and group tables are computed deterministically.
    pub fn from_rows(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let num_cols = columns.len();
        let num_rows_total = rows.len() + 1;

        let mut cell_hashes: Vec<Vec<u32>> = Vec::with_capacity(num_rows_total);
        cell_hashes.push(columns.iter().map(|s| stb_hash(s)).collect());
        for row in &rows {
            cell_hashes.push(row.iter().map(|s| stb_hash(s)).collect());
        }

        let row_groups = groups::build_groups(
            (0..num_rows_total as u32)
                .map(|i| (i, cell_hashes[i as usize][0]))
                .collect(),
            num_rows_total,
        );
        let col_groups = groups::build_groups(
            (0..num_cols as u32)
                .map(|i| (i, cell_hashes[0][i as usize]))
                .collect(),
            num_cols,
        );

        Self {
            columns,
            rows,
            cell_hashes,
            row_groups,
            col_groups,
        }
    }

    /// Number of data rows (excludes the header row).
    pub fn num_rows(&self) -> usize {
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
