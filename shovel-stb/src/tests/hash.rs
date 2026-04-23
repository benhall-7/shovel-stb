use crate::{Stb, stb_hash};

use super::common::fixture;

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

    for (col, header) in stb.columns().iter().enumerate() {
        assert_eq!(
            stb.cell_hash(0, col),
            Some(stb_hash(header)),
            "header hash mismatch for column {col} (\"{header}\")"
        );
    }

    for (r, row) in stb.rows().iter().enumerate() {
        for (c, val) in row.iter().enumerate() {
            assert_eq!(
                stb.cell_hash(r + 1, c),
                Some(stb_hash(val)),
                "hash mismatch at row {r} col {c} (\"{val}\")"
            );
        }
    }
}
