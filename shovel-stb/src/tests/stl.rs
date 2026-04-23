use std::io::Cursor;

use crate::Stl;

use super::common::fixture;

#[test]
fn parse_stl_dialogue() {
    let stl = Stl::open(fixture("loctext/dialogue_eng.stl")).unwrap();
    assert_eq!(stl.num_entries(), 2923);
    assert!(!stl.entries[0].is_empty() || stl.entries.iter().any(|e| !e.is_empty()));
}

#[test]
fn parse_stl_menus() {
    let stl = Stl::open(fixture("loctext/menus_eng.stl")).unwrap();
    assert_eq!(stl.num_entries(), 3162);
}

#[test]
fn stl_binary_round_trip_dialogue() {
    stl_binary_round_trip(&fixture("loctext/dialogue_eng.stl"));
}

#[test]
fn stl_binary_round_trip_menus() {
    stl_binary_round_trip(&fixture("loctext/menus_eng.stl"));
}

fn stl_binary_round_trip(path: &std::path::Path) {
    let original_bytes = std::fs::read(path).unwrap();
    let stl = Stl::open(path).unwrap();

    let mut written = Cursor::new(Vec::new());
    stl.write_stl(&mut written).unwrap();
    let written_bytes = written.into_inner();

    assert_eq!(
        original_bytes,
        written_bytes,
        "STL byte mismatch for {}",
        path.display()
    );
}

#[test]
fn stl_csv_round_trip() {
    let original = Stl::open(fixture("loctext/menus_eng.stl")).unwrap();

    let mut csv_buf = Vec::new();
    original.write_csv(&mut csv_buf, true).unwrap();

    let restored = Stl::read_csv(Cursor::new(&csv_buf)).unwrap();
    assert_eq!(original.entries, restored.entries);
}

#[test]
fn stl_csv_rejects_multi_column() {
    let csv_data = b"A,B\n1,2\n3,4\n";
    let result = Stl::read_csv(Cursor::new(csv_data));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("exactly 1 column"), "unexpected error: {err}");
}

#[test]
fn stl_csv_read_preserves_blank_rows() {
    let csv = b"Text\nA\n\nB\n";
    let stl = crate::Stl::read_csv(Cursor::new(csv.as_slice())).unwrap();
    assert_eq!(
        stl.entries,
        vec!["A".to_string(), String::new(), "B".to_string()]
    );
}

#[test]
fn stl_csv_read_excel_style() {
    let csv: &[u8] = b"\xEF\xBB\xBFText\r\nA\r\n\r\n\"line1\nline2 \"\"x\"\"\"\r\nB\r\n";
    let stl = crate::Stl::read_csv(Cursor::new(csv)).unwrap();
    assert_eq!(
        stl.entries,
        vec![
            "A".to_string(),
            String::new(),
            "line1\nline2 \"x\"".to_string(),
            "B".to_string(),
        ]
    );
}

#[test]
fn stl_csv_read_rejects_multi_column_data_row() {
    let csv = b"Text\nA\nfoo,bar\nB\n";
    let err = crate::Stl::read_csv(Cursor::new(csv.as_slice())).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("row 3") && msg.contains("2 columns"),
        "unexpected error: {msg}"
    );
}

#[test]
fn full_pipeline_stl_csv_stl() {
    let original = Stl::open(fixture("loctext/menus_eng.stl")).unwrap();

    let mut csv_buf = Vec::new();
    original.write_csv(&mut csv_buf, false).unwrap();

    let from_csv = Stl::read_csv(Cursor::new(&csv_buf)).unwrap();

    let mut stl_buf = Cursor::new(Vec::new());
    from_csv.write_stl(&mut stl_buf).unwrap();

    let reparsed = Stl::read(&mut Cursor::new(stl_buf.into_inner())).unwrap();

    assert_eq!(original.entries, reparsed.entries);
}
