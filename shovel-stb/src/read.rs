use std::io::{Read, Seek, SeekFrom};

use binrw::{BinRead, BinResult};

use crate::Stb;
use crate::groups::{Group, GroupEntry};

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

fn read_null_string<R: Read>(reader: &mut R) -> BinResult<String> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        reader.read_exact(&mut byte)?;
        if byte[0] == 0 {
            break;
        }
        buf.push(byte[0]);
    }
    String::from_utf8(buf).map_err(|e| binrw::Error::Custom {
        pos: 0,
        err: Box::new(e),
    })
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

        let mut cell_hashes: Vec<Vec<u32>> = Vec::with_capacity(header.num_rows as usize);
        for r in 0..header.num_rows as usize {
            let start = r * cols;
            cell_hashes.push(hashes_flat[start..start + cols].to_vec());
        }

        let row_groups = read_groups(reader, header.row_groups_offset, header.row_group_count)?;
        let col_groups = read_groups(reader, header.col_groups_offset, header.col_group_count)?;

        Ok(Self {
            columns,
            rows,
            cell_hashes,
            row_groups,
            col_groups,
        })
    }

    /// Convenience: open a file by path and parse it.
    pub fn open(path: impl AsRef<std::path::Path>) -> BinResult<Self> {
        let mut file = std::fs::File::open(path.as_ref())
            .map(std::io::BufReader::new)
            .map_err(binrw::Error::Io)?;
        Self::read(&mut file)
    }
}
