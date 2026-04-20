//! Fixed-layout and counted STB record types shared by read and write code.

use binrw::{BinRead, BinWrite};

use crate::stb::groups::GroupEntry;

/// On-disk STB header (0x40 bytes).
#[derive(Debug, BinRead, BinWrite)]
#[brw(little)]
pub(crate) struct RawHeader {
    pub(crate) _version: u64,
    pub(crate) num_rows: u32,
    pub(crate) num_cols: u32,
    pub(crate) cells_offset: u64,
    pub(crate) string_offsets_offset: u64,
    pub(crate) _pad1: u32,
    pub(crate) row_group_count: u32,
    pub(crate) row_groups_offset: u64,
    pub(crate) _pad2: u32,
    pub(crate) col_group_count: u32,
    pub(crate) col_groups_offset: u64,
}

#[derive(Debug, BinRead)]
#[br(little, import(total_cells: usize))]
pub(crate) struct RawCellHashBlock {
    #[br(count = total_cells)]
    pub(crate) hashes: Vec<u32>,
}

#[derive(Debug, BinRead)]
#[br(little, import(total_cells: usize))]
pub(crate) struct RawStringOffsetBlock {
    #[br(count = total_cells)]
    pub(crate) offsets: Vec<u64>,
}

#[derive(Debug, BinRead)]
#[br(little, import(group_count: usize))]
pub(crate) struct RawGroupOffsetTable {
    #[br(count = group_count)]
    pub(crate) offsets: Vec<u64>,
}

#[derive(Debug, BinRead)]
#[br(little)]
pub(crate) struct RawDiskGroup {
    pub(crate) _entry_count: u64,
    pub(crate) _entry_size: u64,
    #[br(count = _entry_count)]
    pub(crate) entries: Vec<GroupEntry>,
}
