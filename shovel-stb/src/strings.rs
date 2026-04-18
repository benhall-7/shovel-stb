use std::io::Read;

use binrw::BinResult;

/// Non-deduplicating, 8-byte-aligned string pool.
///
/// Each call to `append` adds a new copy of the string.
/// Empty strings are recorded as offset 0 (pointing to the version field).
pub(crate) struct StringPool {
    pub data: Vec<u8>,
    pub offsets: Vec<u64>,
    pub next_offset: u64,
}

impl StringPool {
    pub fn new(base_offset: u64) -> Self {
        Self {
            data: Vec::new(),
            offsets: Vec::new(),
            next_offset: base_offset,
        }
    }

    pub fn push_empty(&mut self) {
        self.offsets.push(0);
    }

    pub fn append(&mut self, s: &str) {
        self.offsets.push(self.next_offset);

        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0);

        let unpadded_len = s.len() + 1;
        let padded_len = (unpadded_len + 7) & !7;
        self.data
            .resize(self.data.len() + padded_len - unpadded_len, 0);
        self.next_offset += padded_len as u64;
    }

    /// Push a string, using offset 0 for empty strings.
    pub fn push(&mut self, s: &str) {
        if s.is_empty() {
            self.push_empty();
        } else {
            self.append(s);
        }
    }
}

/// Read a null-terminated string from the current position.
pub(crate) fn read_null_string<R: Read>(reader: &mut R) -> BinResult<String> {
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
