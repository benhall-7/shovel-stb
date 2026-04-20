use std::io::{Read, Seek, SeekFrom};

use binrw::{BinRead, BinResult};

use crate::Stb;
use crate::stb::groups::{self, Group};
use crate::stb::hash::stb_hash;
use crate::stb::io::layout::{
    RawCellHashBlock, RawDiskGroup, RawGroupOffsetTable, RawHeader, RawStringOffsetBlock,
};
use crate::strings::Utf8NullString;

fn read_groups<R: Read + Seek>(
    reader: &mut R,
    table_offset: u64,
    count: u32,
) -> BinResult<Vec<Group>> {
    let group_count = count as usize;

    reader.seek(SeekFrom::Start(table_offset))?;
    let RawGroupOffsetTable { offsets: group_offsets } =
        RawGroupOffsetTable::read_le_args(reader, (group_count,))?;

    let mut groups = Vec::with_capacity(group_count);
    for &off in &group_offsets {
        reader.seek(SeekFrom::Start(off))?;
        let RawDiskGroup { entries, .. } = RawDiskGroup::read_le(reader)?;
        groups.push(Group { entries });
    }

    Ok(groups)
}

impl Stb {
    /// Parse an STB file from any seekable reader.
    pub fn read<R: Read + Seek>(reader: &mut R) -> BinResult<Self> {
        let header = RawHeader::read(reader)?;
        let total_cells = header.num_rows as usize * header.num_cols as usize;
        let cols = header.num_cols as usize;

        reader.seek(SeekFrom::Start(header.cells_offset))?;
        let RawCellHashBlock { hashes: hashes_flat } =
            RawCellHashBlock::read_le_args(reader, (total_cells,))?;

        reader.seek(SeekFrom::Start(header.string_offsets_offset))?;
        let RawStringOffsetBlock {
            offsets: string_offsets,
        } = RawStringOffsetBlock::read_le_args(reader, (total_cells,))?;

        let mut strings: Vec<String> = Vec::with_capacity(total_cells);
        for &off in &string_offsets {
            reader.seek(SeekFrom::Start(off))?;
            let Utf8NullString(s) = Utf8NullString::read_le(reader)?;
            strings.push(s);
        }

        let columns: Vec<String> = strings[..cols].to_vec();

        let mut rows: Vec<Vec<String>> = Vec::with_capacity(header.num_rows as usize - 1);
        for r in 1..header.num_rows as usize {
            let start = r * cols;
            rows.push(strings[start..start + cols].to_vec());
        }

        for (i, (file_hash, s)) in hashes_flat.iter().zip(strings.iter()).enumerate() {
            let expected = stb_hash(s);
            if *file_hash != expected {
                let pos = reader.stream_position().unwrap_or(0);
                return Err(binrw::Error::AssertFail {
                    pos,
                    message: format!(
                        "cell hash mismatch at flat index {i}: file has 0x{file_hash:08x}, expected 0x{expected:08x} from string data"
                    ),
                });
            }
        }

        let mut cell_hashes: Vec<Vec<u32>> = Vec::with_capacity(header.num_rows as usize);
        for r in 0..header.num_rows as usize {
            let start = r * cols;
            cell_hashes.push(hashes_flat[start..start + cols].to_vec());
        }

        let row_groups_file =
            read_groups(reader, header.row_groups_offset, header.row_group_count)?;
        let col_groups_file =
            read_groups(reader, header.col_groups_offset, header.col_group_count)?;

        let num_rows_total = cell_hashes.len();
        let expected_row_groups = groups::build_groups(
            (0..num_rows_total as u32)
                .map(|i| (i, cell_hashes[i as usize][0]))
                .collect(),
            num_rows_total,
        );
        let expected_col_groups = groups::build_groups(
            (0..cols as u32)
                .map(|i| (i, cell_hashes[0][i as usize]))
                .collect(),
            cols,
        );

        if row_groups_file != expected_row_groups {
            let pos = reader.stream_position().unwrap_or(0);
            return Err(binrw::Error::AssertFail {
                pos,
                message: "row group table does not match cell hashes (file may be corrupt)".into(),
            });
        }
        if col_groups_file != expected_col_groups {
            let pos = reader.stream_position().unwrap_or(0);
            return Err(binrw::Error::AssertFail {
                pos,
                message: "column group table does not match cell hashes (file may be corrupt)"
                    .into(),
            });
        }

        Self::from_tables(
            columns,
            rows,
            cell_hashes,
            row_groups_file,
            col_groups_file,
            crate::StbTablesValidation::default(),
        )
        .map_err(
            |e| {
                let pos = reader.stream_position().unwrap_or(0);
                binrw::Error::AssertFail {
                    pos,
                    message: e.to_string(),
                }
            },
        )
    }

    /// Convenience: open a file by path and parse it.
    pub fn open(path: impl AsRef<std::path::Path>) -> BinResult<Self> {
        let mut file = std::fs::File::open(path.as_ref())
            .map(std::io::BufReader::new)
            .map_err(binrw::Error::Io)?;
        Self::read(&mut file)
    }
}
