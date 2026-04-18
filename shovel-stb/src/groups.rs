use binrw::BinRead;

/// A single entry inside a row/column group: `(index, hash)`.
#[derive(Debug, Clone, BinRead)]
#[br(little)]
pub struct GroupEntry {
    pub index: u32,
    pub hash: u32,
}

/// One bucket in a row-group or column-group index.
#[derive(Debug, Clone)]
pub struct Group {
    pub entries: Vec<GroupEntry>,
}

/// Build group buckets from (index, hash) pairs.
///
/// Bucket count is `count / 3` (integer division, minimum 1).
/// Assignment is `hash % bucket_count`.
pub(crate) fn build_groups(entries: Vec<(u32, u32)>, count: usize) -> Vec<Group> {
    let bucket_count = (count / 3).max(1);
    let mut buckets: Vec<Vec<GroupEntry>> = vec![Vec::new(); bucket_count];

    for (index, hash) in entries {
        let bucket = (hash as usize) % bucket_count;
        buckets[bucket].push(GroupEntry { index, hash });
    }

    buckets
        .into_iter()
        .map(|entries| Group { entries })
        .collect()
}

/// Encode a group table into bytes, given the absolute file offset where
/// this table starts.
pub(crate) fn encode_group_table(groups: &[Group], table_start: u64) -> Vec<u8> {
    let num_groups = groups.len();
    let offsets_size = num_groups as u64 * 8;

    let mut group_data_chunks: Vec<Vec<u8>> = Vec::with_capacity(num_groups);
    for group in groups {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(group.entries.len() as u64).to_le_bytes());
        chunk.extend_from_slice(&0x10u64.to_le_bytes());
        for entry in &group.entries {
            chunk.extend_from_slice(&entry.index.to_le_bytes());
            chunk.extend_from_slice(&entry.hash.to_le_bytes());
        }
        group_data_chunks.push(chunk);
    }

    let mut group_offsets: Vec<u64> = Vec::with_capacity(num_groups);
    let mut running_offset = table_start + offsets_size;
    for chunk in &group_data_chunks {
        group_offsets.push(running_offset);
        running_offset += chunk.len() as u64;
    }

    let mut out = Vec::new();
    for &off in &group_offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    for chunk in &group_data_chunks {
        out.extend_from_slice(chunk);
    }
    out
}
