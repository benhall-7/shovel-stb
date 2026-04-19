use std::io::{Read, Write};

use crate::Stb;
use crate::StbError;

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
    pub fn read_csv<R: Read>(reader: R) -> Result<Self, StbError> {
        let mut rdr = csv::Reader::from_reader(reader);
        let columns: Vec<String> = rdr.headers()?.iter().map(|s| s.to_owned()).collect();

        let mut rows = Vec::new();
        for result in rdr.records() {
            let record = result?;
            rows.push(record.iter().map(|s| s.to_owned()).collect());
        }

        Stb::from_rows(columns, rows)
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
