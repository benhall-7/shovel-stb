use crate::Stb;
use crate::StbError;

/// Scoped edit session: batch **inner-cell** changes, then [`Self::finish`] to recover the [`Stb`].
///
/// Only [`Stb::set_inner_cell`] is exposed; row/column group tables stay valid after each call, so
/// [`Self::finish`] does not rebuild them—it only returns ownership. Use [`crate::StbLine`] when
/// editing a whole row or column (including keys) and hashes must be refreshed for that line.
pub struct StbInnerCells {
    stb: Stb,
}

impl StbInnerCells {
    pub fn new(stb: Stb) -> Self {
        Self { stb }
    }

    pub fn set_inner_cell(
        &mut self,
        row: usize,
        col: usize,
        value: String,
    ) -> Result<(), StbError> {
        self.stb.set_inner_cell(row, col, value)
    }

    /// Read the table while editing (same idea as [`crate::StbLine::stb`]).
    pub fn stb(&self) -> &Stb {
        &self.stb
    }

    /// End the session and return the [`Stb`].
    pub fn finish(self) -> Stb {
        self.stb
    }
}
