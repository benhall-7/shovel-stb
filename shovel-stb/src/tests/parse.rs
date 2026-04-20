use crate::Stb;

use super::common::fixture;

#[test]
fn parse_character_attributes() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    assert_eq!(stb.num_cols(), 17);
    assert_eq!(stb.num_body_rows(), 20);

    assert_eq!(stb.columns()[0], "ID");
    assert_eq!(stb.columns()[1], "Name");
    assert_eq!(stb.columns()[3], "Health");

    assert_eq!(stb.cell(1, 1), Some("Shovel Knight"));
    assert_eq!(stb.cell(1, 3), Some("8"));
    assert_eq!(stb.cell(1, 4), Some("9.75"));
    assert_eq!(stb.cell(2, 1), Some("Plague Knight"));
    assert_eq!(stb.cell(3, 1), Some("Specter Knight"));
    assert_eq!(stb.cell_hash(0, 0), Some(0x9911DB53));
}

#[test]
fn parse_character_entry() {
    let stb = Stb::open(fixture("battle/characterEntry.stb")).unwrap();

    assert_eq!(stb.num_cols(), 44);
    assert_eq!(stb.num_body_rows(), 31);
    assert_eq!(stb.columns()[0], "ID");
    assert_eq!(stb.columns()[1], "Speaker");
    assert_eq!(stb.cell(1, 1), Some("Player"));
}

#[test]
fn parse_speakers() {
    let stb = Stb::open(fixture("dialogue/speakers.stb")).unwrap();

    assert_eq!(stb.num_cols(), 23);
    assert_eq!(stb.num_body_rows(), 225);
    assert_eq!(stb.columns()[0], "ID");
    assert_eq!(stb.columns()[9], "English");
}

#[test]
fn parse_passerby() {
    let stb = Stb::open(fixture("dialogue/passerby.stb")).unwrap();

    assert_eq!(stb.num_cols(), 8);
    assert_eq!(stb.num_body_rows(), 150);
    assert_eq!(stb.columns()[0], "ID");
    assert_eq!(stb.cell(1, 0), Some("A_ID1"));
}

#[test]
fn parse_credits() {
    let stb = Stb::open(fixture("menus/credits.stb")).unwrap();

    assert_eq!(stb.num_cols(), 20);
    assert_eq!(stb.num_body_rows(), 361);
    assert_eq!(stb.columns()[0], "Format");
    assert_eq!(stb.columns()[7], "English");
}

#[test]
fn parse_stm_dialogue() {
    let stb = Stb::open(fixture("loctext/dialogue.stm")).unwrap();

    assert_eq!(stb.num_cols(), 8);
    assert_eq!(stb.columns()[0], "");
    assert_eq!(stb.columns()[1], "ID");
    assert_eq!(stb.columns()[2], "Speaker");
    assert_eq!(stb.columns()[3], "Trigger");
}

#[test]
fn parse_stm_menus() {
    let stb = Stb::open(fixture("loctext/menus.stm")).unwrap();

    assert_eq!(stb.num_cols(), 1);
    assert_eq!(stb.columns()[0], "ID");
    assert_eq!(stb.cell(1, 0), Some("yes"));
}
