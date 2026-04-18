use std::io::{Seek, Write};

use crate::Stb;
use crate::groups::{build_groups, encode_group_table};
use crate::hash::stb_hash;

/// Non-deduplicating, 8-byte-aligned string pool.
///
/// Each call to `append` adds a new copy of the string.
/// Empty strings are recorded as offset 0 (pointing to the version field).
struct StringPool {
    data: Vec<u8>,
    offsets: Vec<u64>,
    next_offset: u64,
}

impl StringPool {
    fn new(base_offset: u64) -> Self {
        Self {
            data: Vec::new(),
            offsets: Vec::new(),
            next_offset: base_offset,
        }
    }

    fn push_empty(&mut self) {
        self.offsets.push(0);
    }

    fn append(&mut self, s: &str) {
        self.offsets.push(self.next_offset);

        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0);

        let unpadded_len = s.len() + 1;
        let padded_len = (unpadded_len + 7) & !7;
        self.data
            .resize(self.data.len() + padded_len - unpadded_len, 0);
        self.next_offset += padded_len as u64;
    }
}

impl Stb {
    /// Serialize this table to the binary STB format.
    pub fn write_stb<W: Write + Seek>(&self, writer: &mut W) -> std::io::Result<()> {
        let num_rows = (self.rows.len() + 1) as u32;
        let num_cols = self.columns.len() as u32;
        let total_cells = num_rows as usize * num_cols as usize;

        let mut all_strings: Vec<&str> = Vec::with_capacity(total_cells);
        for col in &self.columns {
            all_strings.push(col);
        }
        for row in &self.rows {
            for cell in row {
                all_strings.push(cell);
            }
        }

        let hashes_flat: Vec<u32> = all_strings.iter().map(|s| stb_hash(s)).collect();

        // Layout
        let header_size: u64 = 0x40;
        let cells_offset = header_size;
        let cells_size = total_cells as u64 * 4;
        let cells_end = cells_offset + cells_size;
        let string_offsets_offset = (cells_end + 7) & !7;
        let string_offsets_size = total_cells as u64 * 8;
        let string_offsets_end = string_offsets_offset + string_offsets_size;

        let mut pool = StringPool::new(string_offsets_end);
        for &s in &all_strings {
            if s.is_empty() {
                pool.push_empty();
            } else {
                pool.append(s);
            }
        }
        let string_pool_end = pool.next_offset;

        let row_group_count = (num_rows / 3).max(1);
        let col_group_count = (num_cols / 3).max(1);

        let row_groups = build_groups(
            (0..num_rows)
                .map(|i| (i, hashes_flat[i as usize * num_cols as usize]))
                .collect(),
            num_rows as usize,
        );
        let col_groups = build_groups(
            (0..num_cols)
                .map(|i| (i, hashes_flat[i as usize]))
                .collect(),
            num_cols as usize,
        );

        let row_groups_offset = string_pool_end;
        let row_groups_bytes = encode_group_table(&row_groups, row_groups_offset);
        let col_groups_offset = row_groups_offset + row_groups_bytes.len() as u64;
        let col_groups_bytes = encode_group_table(&col_groups, col_groups_offset);

        // Header
        writer.write_all(&0u64.to_le_bytes())?;
        writer.write_all(&num_rows.to_le_bytes())?;
        writer.write_all(&num_cols.to_le_bytes())?;
        writer.write_all(&cells_offset.to_le_bytes())?;
        writer.write_all(&string_offsets_offset.to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?;
        writer.write_all(&row_group_count.to_le_bytes())?;
        writer.write_all(&row_groups_offset.to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?;
        writer.write_all(&col_group_count.to_le_bytes())?;
        writer.write_all(&col_groups_offset.to_le_bytes())?;

        // Cell hashes
        debug_assert_eq!(writer.stream_position()?, cells_offset);
        for &h in &hashes_flat {
            writer.write_all(&h.to_le_bytes())?;
        }

        // Alignment padding
        let pos = writer.stream_position()?;
        for _ in pos..string_offsets_offset {
            writer.write_all(&[0u8])?;
        }

        // String offsets
        debug_assert_eq!(writer.stream_position()?, string_offsets_offset);
        for off in &pool.offsets {
            writer.write_all(&off.to_le_bytes())?;
        }

        // String pool
        debug_assert_eq!(writer.stream_position()?, string_offsets_end);
        writer.write_all(&pool.data)?;

        // Group tables
        debug_assert_eq!(writer.stream_position()?, row_groups_offset);
        writer.write_all(&row_groups_bytes)?;
        debug_assert_eq!(writer.stream_position()?, col_groups_offset);
        writer.write_all(&col_groups_bytes)?;

        Ok(())
    }

    /// Convenience: save to an STB file path.
    pub fn save_stb(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let file = std::fs::File::create(path.as_ref())?;
        let mut writer = std::io::BufWriter::new(file);
        self.write_stb(&mut writer)
    }
}
