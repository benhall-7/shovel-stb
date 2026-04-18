use std::io::Cursor;

use crate::{Stb, stb_hash};

fn fixture(name: &str) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    p
}

// -- Parsing --

#[test]
fn parse_character_attributes() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    assert_eq!(stb.num_cols(), 17);
    assert_eq!(stb.num_rows(), 20);

    assert_eq!(stb.columns[0], "ID");
    assert_eq!(stb.columns[1], "Name");
    assert_eq!(stb.columns[3], "Health");

    assert_eq!(stb.get(0, 1), Some("Shovel Knight"));
    assert_eq!(stb.get(0, 3), Some("8"));
    assert_eq!(stb.get(0, 4), Some("9.75"));
    assert_eq!(stb.get(1, 1), Some("Plague Knight"));
    assert_eq!(stb.get(2, 1), Some("Specter Knight"));
    assert_eq!(stb.cell_hashes[0][0], 0x9911DB53);
}

#[test]
fn parse_character_entry() {
    let stb = Stb::open(fixture("battle/characterEntry.stb")).unwrap();

    assert_eq!(stb.num_cols(), 44);
    assert_eq!(stb.num_rows(), 31);
    assert_eq!(stb.columns[0], "ID");
    assert_eq!(stb.columns[1], "Speaker");
    assert_eq!(stb.get(0, 1), Some("Player"));
}

#[test]
fn parse_speakers() {
    let stb = Stb::open(fixture("dialogue/speakers.stb")).unwrap();

    assert_eq!(stb.num_cols(), 23);
    assert_eq!(stb.num_rows(), 225);
    assert_eq!(stb.columns[0], "ID");
    assert_eq!(stb.columns[9], "English");
}

#[test]
fn parse_passerby() {
    let stb = Stb::open(fixture("dialogue/passerby.stb")).unwrap();

    assert_eq!(stb.num_cols(), 8);
    assert_eq!(stb.num_rows(), 150);
    assert_eq!(stb.columns[0], "ID");
    assert_eq!(stb.get(0, 0), Some("A_ID1"));
}

#[test]
fn parse_credits() {
    let stb = Stb::open(fixture("menus/credits.stb")).unwrap();

    assert_eq!(stb.num_cols(), 20);
    assert_eq!(stb.num_rows(), 361);
    assert_eq!(stb.columns[0], "Format");
    assert_eq!(stb.columns[7], "English");
}

// -- STM parsing --

#[test]
fn parse_stm_dialogue() {
    let stb = Stb::open(fixture("loctext/dialogue.stm")).unwrap();

    assert_eq!(stb.num_cols(), 8);
    assert_eq!(stb.columns[0], "");
    assert_eq!(stb.columns[1], "ID");
    assert_eq!(stb.columns[2], "Speaker");
    assert_eq!(stb.columns[3], "Trigger");
}

#[test]
fn parse_stm_menus() {
    let stb = Stb::open(fixture("loctext/menus.stm")).unwrap();

    assert_eq!(stb.num_cols(), 1);
    assert_eq!(stb.columns[0], "ID");
    assert_eq!(stb.get(0, 0), Some("yes"));
}

// -- Accessors --

#[test]
fn column_index_lookup() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    assert_eq!(stb.column_index("Name"), Some(1));
    assert_eq!(stb.column_index("Health"), Some(3));
    assert_eq!(stb.column_index("nonexistent"), None);
}

// -- Hash --

#[test]
fn hash_known_values() {
    assert_eq!(stb_hash("ID"), 0x9911DB53);
    assert_eq!(stb_hash("Name"), 0xE8DF52FD);
    assert_eq!(stb_hash("0"), 0x3180D09D);
    assert_eq!(stb_hash("1"), 0xD0F4B96D);
    assert_eq!(stb_hash("Health"), 0xCFEB08E0);
    assert_eq!(stb_hash("8"), 0xE55C1F50);
    assert_eq!(stb_hash("Shovel Knight"), 0x33D927A0);
}

#[test]
fn hash_matches_parsed_cells() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    for (col, header) in stb.columns.iter().enumerate() {
        assert_eq!(
            stb.cell_hashes[0][col],
            stb_hash(header),
            "header hash mismatch for column {col} (\"{header}\")"
        );
    }

    for (r, row) in stb.rows.iter().enumerate() {
        for (c, val) in row.iter().enumerate() {
            assert_eq!(
                stb.cell_hashes[r + 1][c],
                stb_hash(val),
                "hash mismatch at row {r} col {c} (\"{val}\")"
            );
        }
    }
}

// -- Groups --

#[test]
fn row_and_col_groups() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    assert_eq!(stb.row_groups.len(), 7);
    assert_eq!(stb.col_groups.len(), 5);

    let has_id_entry = stb.col_groups.iter().any(|g| {
        g.entries
            .iter()
            .any(|e| e.index == 0 && e.hash == 0x9911DB53)
    });
    assert!(has_id_entry);
}

// -- CSV round-trip --

#[test]
fn csv_round_trip() {
    let original = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    let mut csv_buf = Vec::new();
    original.write_csv(&mut csv_buf, true).unwrap();

    assert_eq!(&csv_buf[..3], b"\xEF\xBB\xBF", "BOM should be present");

    let restored = Stb::read_csv(Cursor::new(&csv_buf)).unwrap();

    assert_eq!(original.columns, restored.columns);
    assert_eq!(original.rows, restored.rows);
    assert_eq!(original.cell_hashes, restored.cell_hashes);

    let mut no_bom_buf = Vec::new();
    original.write_csv(&mut no_bom_buf, false).unwrap();
    assert_ne!(&no_bom_buf[..3], b"\xEF\xBB\xBF", "BOM should be absent");

    let restored2 = Stb::read_csv(Cursor::new(&no_bom_buf)).unwrap();
    assert_eq!(original.columns, restored2.columns);
}

// -- Binary round-trip --

#[test]
fn stb_binary_round_trip_character_attributes() {
    binary_round_trip("battle/characterAttributes.stb");
}

#[test]
fn stb_binary_round_trip_character_entry() {
    binary_round_trip("battle/characterEntry.stb");
}

#[test]
fn stb_binary_round_trip_speakers() {
    binary_round_trip("dialogue/speakers.stb");
}

#[test]
fn stb_binary_round_trip_passerby() {
    binary_round_trip("dialogue/passerby.stb");
}

#[test]
fn stb_binary_round_trip_credits() {
    binary_round_trip("menus/credits.stb");
}

#[test]
fn stm_binary_round_trip_dialogue() {
    binary_round_trip_path(&fixture("loctext/dialogue.stm"));
}

#[test]
fn stm_binary_round_trip_menus() {
    binary_round_trip_path(&fixture("loctext/menus.stm"));
}

fn binary_round_trip(name: &str) {
    binary_round_trip_path(&fixture(name));
}

fn binary_round_trip_path(path: &std::path::Path) {
    let original_bytes = std::fs::read(path).unwrap();
    let stb = Stb::open(path).unwrap();

    let mut written = Cursor::new(Vec::new());
    stb.write_stb(&mut written).unwrap();
    let written_bytes = written.into_inner();

    assert_eq!(
        original_bytes,
        written_bytes,
        "byte mismatch for {}",
        path.display()
    );
}

// -- Full pipeline --

#[test]
fn full_pipeline_stb_csv_stb() {
    let original = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    let mut csv_buf = Vec::new();
    original.write_csv(&mut csv_buf, true).unwrap();

    let from_csv = Stb::read_csv(Cursor::new(&csv_buf)).unwrap();

    let mut stb_buf = Cursor::new(Vec::new());
    from_csv.write_stb(&mut stb_buf).unwrap();

    let reparsed = Stb::read(&mut Cursor::new(stb_buf.into_inner())).unwrap();

    assert_eq!(original.columns, reparsed.columns);
    assert_eq!(original.rows, reparsed.rows);
    assert_eq!(original.cell_hashes, reparsed.cell_hashes);
}
