use std::io::Cursor;

use crate::Stb;

use super::common::fixture;

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

#[test]
fn full_pipeline_stb_csv_stb() {
    let original = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    let mut csv_buf = Vec::new();
    original.write_csv(&mut csv_buf, true).unwrap();

    let from_csv = Stb::read_csv(Cursor::new(&csv_buf)).unwrap();

    let mut stb_buf = Cursor::new(Vec::new());
    from_csv.write_stb(&mut stb_buf).unwrap();

    let reparsed = Stb::read(&mut Cursor::new(stb_buf.into_inner())).unwrap();

    assert_eq!(original, reparsed);
}
