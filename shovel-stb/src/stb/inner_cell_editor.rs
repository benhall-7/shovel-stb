use crate::Stb;
use crate::StbError;

/// Batch **inner-cell** edits while keeping the existing row/column group tables.
///
/// Only [`Stb::set_inner_cell`] is allowed through this wrapper; finish to get
/// an owned [`Stb`].
pub struct InnerCellEditor {
    stb: Stb,
}

impl InnerCellEditor {
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

    pub fn stb(&self) -> &Stb {
        &self.stb
    }

    pub fn finish(self) -> Stb {
        self.stb
    }
}
