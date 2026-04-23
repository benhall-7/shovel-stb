use std::io::{Read, Write};

use crate::Stb;
use crate::StbError;
use crate::stl::{parse_csv_records, strip_utf8_bom};

impl Stb {
    /// Write the table as CSV to any writer. When `bom` is true a UTF-8 BOM is prepended,
    /// so that spreadsheet applications open the file with the correct encoding.
    pub fn write_csv<W: Write>(&self, mut writer: W, bom: bool) -> Result<(), csv::Error> {
        if bom {
            writer.write_all(b"\xEF\xBB\xBF")?;
        }
        let mut wtr = csv::Writer::from_writer(writer);
        wtr.write_record(&self.columns)?;
        for row in &self.rows {
            wtr.write_record(row)?;
        }
        wtr.flush()?;
        Ok(())
    }

    /// Read a table from CSV. Fails if a row has a different width than the header row.
    pub fn read_csv<R: Read>(mut reader: R) -> Result<Self, StbError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let input = strip_utf8_bom(&bytes);

        let mut records = parse_csv_records(input)
            .map_err(|e| StbError::CsvRead(format!("invalid UTF-8 in CSV cell: {e}")))?;

        if records.is_empty() {
            return Err(StbError::CsvRead(
                "STB CSV is empty (missing header row)".into(),
            ));
        }

        let columns = records.remove(0);
        Stb::from_rows(columns, records)
    }

    /// Convenience: save CSV to a file path (with BOM by default).
    pub fn save_csv(&self, path: impl AsRef<std::path::Path>, bom: bool) -> Result<(), csv::Error> {
        let file = std::fs::File::create(path.as_ref())?;
        self.write_csv(std::io::BufWriter::new(file), bom)
    }

    /// Convenience: load from a CSV file path.
    pub fn open_csv(path: impl AsRef<std::path::Path>) -> Result<Self, StbError> {
        let file = std::fs::File::open(path.as_ref())?;
        Self::read_csv(std::io::BufReader::new(file))
    }
}
