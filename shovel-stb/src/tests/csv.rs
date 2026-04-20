use std::io::Cursor;

use crate::Stb;

use super::common::fixture;

#[test]
fn csv_round_trip() {
    let original = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    let mut csv_buf = Vec::new();
    original.write_csv(&mut csv_buf, true).unwrap();

    assert_eq!(&csv_buf[..3], b"\xEF\xBB\xBF", "BOM should be present");

    let restored = Stb::read_csv(Cursor::new(&csv_buf)).unwrap();

    assert_eq!(original, restored);

    let mut no_bom_buf = Vec::new();
    original.write_csv(&mut no_bom_buf, false).unwrap();
    assert_ne!(&no_bom_buf[..3], b"\xEF\xBB\xBF", "BOM should be absent");

    let restored2 = Stb::read_csv(Cursor::new(&no_bom_buf)).unwrap();
    assert_eq!(original, restored2);
}
