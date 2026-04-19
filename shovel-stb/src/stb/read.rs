use std::io::{Read, Seek, SeekFrom};

use binrw::{BinRead, BinResult};

use crate::Stb;
use crate::stb::groups::{self, Group, GroupEntry};
use crate::stb::hash::stb_hash;
use crate::strings::read_null_string;

#[derive(Debug, BinRead)]
#[br(little)]
struct RawHeader {
    _version: u64,
    num_rows: u32,
    num_cols: u32,
    cells_offset: u64,
    string_offsets_offset: u64,
    _pad1: u32,
    row_group_count: u32,
    row_groups_offset: u64,
    _pad2: u32,
    col_group_count: u32,
    col_groups_offset: u64,
}

fn read_groups<R: Read + Seek>(
    reader: &mut R,
    table_offset: u64,
    count: u32,
) -> BinResult<Vec<Group>> {
    let count = count as usize;

    reader.seek(SeekFrom::Start(table_offset))?;
    let mut group_offsets = Vec::with_capacity(count);
    for _ in 0..count {
        group_offsets.push(u64::read_le(reader)?);
    }

    let mut groups = Vec::with_capacity(count);
    for &off in &group_offsets {
        reader.seek(SeekFrom::Start(off))?;
        let entry_count = u64::read_le(reader)? as usize;
        let _entry_size = u64::read_le(reader)?;

        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            entries.push(GroupEntry::read_le(reader)?);
        }
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
        let mut hashes_flat: Vec<u32> = Vec::with_capacity(total_cells);
        for _ in 0..total_cells {
            hashes_flat.push(u32::read_le(reader)?);
        }

        reader.seek(SeekFrom::Start(header.string_offsets_offset))?;
        let mut string_offsets: Vec<u64> = Vec::with_capacity(total_cells);
        for _ in 0..total_cells {
            string_offsets.push(u64::read_le(reader)?);
        }

        let mut strings: Vec<String> = Vec::with_capacity(total_cells);
        for &off in &string_offsets {
            reader.seek(SeekFrom::Start(off))?;
            strings.push(read_null_string(reader)?);
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
