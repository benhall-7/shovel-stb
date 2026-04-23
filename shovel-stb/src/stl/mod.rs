use std::io::{Read, Seek, SeekFrom, Write};

use binrw::{BinRead, BinResult};
use lazycsv::{Csv, CsvIterItem};

use crate::strings::{StringPool, read_null_string};

/// A parsed STL localization string table.
///
/// STL files contain a flat list of localized strings — one per row in the
/// corresponding STM master table. There are no column headers, cell hashes,
/// or group indices.
#[derive(Debug, Clone)]
pub struct Stl {
    /// The string entries, one per row.
    pub entries: Vec<String>,
}

#[derive(Debug, BinRead)]
#[br(little)]
struct RawHeader {
    _version: u64,
    num_entries: u32,
    _num_columns: u32,
    offsets_start: u64,
}

impl Stl {
    /// Create an `Stl` from a list of strings.
    pub fn from_entries(entries: Vec<String>) -> Self {
        Self { entries }
    }

    pub fn num_entries(&self) -> usize {
        self.entries.len()
    }

    // -- Binary read --

    /// Parse an STL file from any seekable reader.
    pub fn read<R: Read + Seek>(reader: &mut R) -> BinResult<Self> {
        let header = RawHeader::read(reader)?;
        let num = header.num_entries as usize;

        reader.seek(SeekFrom::Start(header.offsets_start))?;
        let mut string_offsets = Vec::with_capacity(num);
        for _ in 0..num {
            string_offsets.push(u64::read_le(reader)?);
        }

        let mut entries = Vec::with_capacity(num);
        for &off in &string_offsets {
            reader.seek(SeekFrom::Start(off))?;
            entries.push(read_null_string(reader)?);
        }

        Ok(Self { entries })
    }

    /// Convenience: open a file by path and parse it.
    pub fn open(path: impl AsRef<std::path::Path>) -> BinResult<Self> {
        let mut file = std::fs::File::open(path.as_ref())
            .map(std::io::BufReader::new)
            .map_err(binrw::Error::Io)?;
        Self::read(&mut file)
    }

    // -- Binary write --

    /// Serialize this string table to the binary STL format.
    pub fn write_stl<W: Write + Seek>(&self, writer: &mut W) -> std::io::Result<()> {
        let num_entries = self.entries.len() as u32;
        let offsets_start: u64 = 0x18;
        let offsets_size = num_entries as u64 * 8;
        let pool_start = offsets_start + offsets_size;

        let mut pool = StringPool::new(pool_start);
        for s in &self.entries {
            pool.push(s);
        }

        // Header (24 bytes)
        writer.write_all(&0u64.to_le_bytes())?; // version
        writer.write_all(&num_entries.to_le_bytes())?;
        writer.write_all(&1u32.to_le_bytes())?; // num_columns (always 1)
        writer.write_all(&offsets_start.to_le_bytes())?;

        // String offsets
        debug_assert_eq!(writer.stream_position()?, offsets_start);
        for off in &pool.offsets {
            writer.write_all(&off.to_le_bytes())?;
        }

        // String pool
        debug_assert_eq!(writer.stream_position()?, pool_start);
        writer.write_all(&pool.data)?;

        Ok(())
    }

    /// Convenience: save to an STL file path.
    pub fn save_stl(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let file = std::fs::File::create(path.as_ref())?;
        let mut writer = std::io::BufWriter::new(file);
        self.write_stl(&mut writer)
    }

    // -- CSV --

    /// Write the string table as a single-column CSV.
    pub fn write_csv<W: Write>(&self, mut writer: W, bom: bool) -> Result<(), csv::Error> {
        if bom {
            writer.write_all(b"\xEF\xBB\xBF")?;
        }
        let mut wtr = csv::Writer::from_writer(writer);
        wtr.write_record(["Text"])?;
        for entry in &self.entries {
            wtr.write_record([entry])?;
        }
        wtr.flush()?;
        Ok(())
    }

    /// Read a string table from a single-column CSV.
    ///
    /// Returns an error if any data row has more than one column. Empty rows are preserved
    pub fn read_csv<R: Read>(mut reader: R) -> Result<Self, Box<dyn std::error::Error>> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let input = strip_utf8_bom(&bytes);

        let mut records = parse_csv_records(input)?;
        if records.is_empty() {
            return Err("STL CSV is empty (missing header row)".into());
        }

        let header = records.remove(0);
        if header.len() != 1 {
            return Err(format!(
                "STL CSV must have exactly 1 column, found {} in header row",
                header.len()
            )
            .into());
        }

        let mut entries = Vec::with_capacity(records.len());
        for (i, record) in records.into_iter().enumerate() {
            if record.len() != 1 {
                return Err(format!(
                    "STL CSV row {} has {} columns, expected 1",
                    i + 2,
                    record.len()
                )
                .into());
            }
            entries.push(record.into_iter().next().unwrap());
        }

        Ok(Self::from_entries(entries))
    }

    /// Convenience: save CSV to a file path.
    pub fn save_csv(&self, path: impl AsRef<std::path::Path>, bom: bool) -> Result<(), csv::Error> {
        let file = std::fs::File::create(path.as_ref())?;
        self.write_csv(std::io::BufWriter::new(file), bom)
    }

    /// Convenience: load from a CSV file path.
    pub fn open_csv(path: impl AsRef<std::path::Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path.as_ref())?;
        Self::read_csv(std::io::BufReader::new(file))
    }
}

/// Strip a leading UTF-8 BOM (`EF BB BF`) if present
pub(crate) fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes)
}

/// Parse bytes as CSV records.
///
/// Empty lines produce records with a single empty field, so the row count of the output matches
/// the row count a spreadsheet user would see in their editor.
pub(crate) fn parse_csv_records(bytes: &[u8]) -> Result<Vec<Vec<String>>, std::str::Utf8Error> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for item in Csv::new(bytes) {
        match item {
            CsvIterItem::Cell(cell) => current.push(cell.try_as_str()?.into_owned()),
            CsvIterItem::LineEnd => records.push(std::mem::take(&mut current)),
        }
    }
    // Handle trailing record without a final line terminator.
    if !current.is_empty() {
        records.push(current);
    }
    Ok(records)
}

impl std::fmt::Display for Stl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for entry in &self.entries {
            writeln!(f, "{entry}")?;
        }
        Ok(())
    }
}
