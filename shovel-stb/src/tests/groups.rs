use crate::{
    groups, Stb, StbError, StbInnerCells, StbTablesValidation, TablesMismatchKind, stb_hash,
};

use super::common::fixture;

#[test]
fn row_and_col_groups() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    assert_eq!(stb.row_groups().len(), 7);
    assert_eq!(stb.col_groups().len(), 5);
    assert_eq!(stb.row_groups().len(), groups::group_bucket_count(stb.num_rows()));
    assert_eq!(stb.col_groups().len(), groups::group_bucket_count(stb.num_cols()));

    let has_id_entry = stb.col_groups().iter().any(|g| {
        g.entries
            .iter()
            .any(|e| e.index == 0 && e.hash == 0x9911DB53)
    });
    assert!(has_id_entry);
}

#[test]
fn group_bucket_lookup_agrees_with_linear_scan() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let nc = stb.num_cols();
    let nr = stb.num_rows();

    for c in 0..nc {
        let header = stb.cell(0, c).unwrap();
        assert_eq!(stb.column_index(header), Some(c));
        let h = stb_hash(header);
        assert!(
            stb.col_group_bucket_for_key(header).entries.iter().any(|e| {
                e.index as usize == c && e.hash == h && stb.cell(0, e.index as usize) == Some(header)
            }),
            "col {c} header bucket"
        );
        assert_eq!(
            groups::bucket_index_for_hash(h, nc),
            stb
                .col_groups()
                .iter()
                .position(|g| g.entries.iter().any(|e| e.index as usize == c && e.hash == h))
                .expect("column entry in some bucket")
        );
    }

    for r in 0..nr {
        let key = stb.cell(r, 0).unwrap();
        assert_eq!(
            (0..nr).find(|&row| stb.cell(row, 0) == Some(key)),
            stb.row_for_column(0, key)
        );
        let h = stb_hash(key);
        assert!(
            stb.row_group_bucket_for_key(key).entries.iter().any(|e| {
                e.index as usize == r && e.hash == h && stb.cell(e.index as usize, 0) == Some(key)
            }),
            "row {r} key bucket"
        );
        assert_eq!(
            groups::bucket_index_for_hash(h, nr),
            stb
                .row_groups()
                .iter()
                .position(|g| g.entries.iter().any(|e| e.index as usize == r && e.hash == h))
                .expect("row entry in some bucket")
        );
    }
}

#[test]
fn set_inner_cell_updates_hash_preserves_groups() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let rg_before = stb.row_groups().to_vec();
    let cg_before = stb.col_groups().to_vec();

    stb.set_inner_cell(1, 3, "999".to_string()).unwrap();
    assert_eq!(stb.cell(1, 3), Some("999"));
    assert_eq!(stb.cell_hash(1, 3), Some(stb_hash("999")));
    assert_eq!(stb.row_groups(), rg_before.as_slice());
    assert_eq!(stb.col_groups(), cg_before.as_slice());
}

#[test]
fn from_tables_full_rejects_bad_hashes() {
    let columns = vec!["H".to_string()];
    let rows = vec![vec!["cell".to_string()]];
    let ok = Stb::from_rows(columns.clone(), rows.clone()).unwrap();
    let mut bad_hashes = vec![
        vec![stb_hash("H")],
        vec![stb_hash("cell")],
    ];
    bad_hashes[1][0] ^= 0xDEAD_BEEF;
    let r = Stb::from_tables(
        columns,
        rows,
        bad_hashes,
        ok.row_groups().to_vec(),
        ok.col_groups().to_vec(),
        StbTablesValidation::Full,
    );
    assert!(matches!(
        r,
        Err(StbError::TablesMismatch(TablesMismatchKind::CellHashes))
    ));
}

#[test]
fn from_tables_dimensions_only_accepts_bad_hashes() {
    let columns = vec!["H".to_string()];
    let rows = vec![vec!["cell".to_string()]];
    let ok = Stb::from_rows(columns.clone(), rows.clone()).unwrap();
    let mut bad_hashes = vec![
        vec![stb_hash("H")],
        vec![stb_hash("cell")],
    ];
    let corrupt = bad_hashes[1][0] ^ 0xDEAD_BEEF;
    bad_hashes[1][0] = corrupt;
    let stb = Stb::from_tables(
        columns,
        rows,
        bad_hashes,
        ok.row_groups().to_vec(),
        ok.col_groups().to_vec(),
        StbTablesValidation::DimensionsOnly,
    )
    .unwrap();
    assert_eq!(stb.cell_hash(1, 0), Some(corrupt));
}

#[test]
fn set_row_key_rebuilds_row_groups_only() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let cg_before = stb.col_groups().to_vec();
    let rg_before = stb.row_groups().to_vec();

    stb.set_row_key(1, "ROWKEY_TEST".to_string()).unwrap();
    assert_eq!(stb.cell(1, 0), Some("ROWKEY_TEST"));
    assert_eq!(stb.cell_hash(1, 0), Some(stb_hash("ROWKEY_TEST")));
    assert_eq!(stb.col_groups(), cg_before.as_slice());
    assert_ne!(stb.row_groups(), rg_before.as_slice());
}

#[test]
fn set_column_key_inner_col_rebuilds_col_groups_only() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let rg_before = stb.row_groups().to_vec();
    let cg_before = stb.col_groups().to_vec();

    stb.set_column_key(3, "RenamedCol".to_string()).unwrap();
    assert_eq!(stb.columns()[3], "RenamedCol");
    assert_eq!(stb.cell_hash(0, 3), Some(stb_hash("RenamedCol")));
    assert_eq!(stb.row_groups(), rg_before.as_slice());
    assert_ne!(stb.col_groups(), cg_before.as_slice());
}

#[test]
fn set_column_key_col0_rebuilds_row_and_col_groups() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let rg_before = stb.row_groups().to_vec();
    let cg_before = stb.col_groups().to_vec();

    stb.set_column_key(0, "ID_RENAME".to_string()).unwrap();
    assert_eq!(stb.columns()[0], "ID_RENAME");
    assert_ne!(stb.row_groups(), rg_before.as_slice());
    assert_ne!(stb.col_groups(), cg_before.as_slice());
}

#[test]
fn set_row_key_rejects_header_row() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let r = stb.set_row_key(0, "nope".to_string());
    assert!(matches!(
        r,
        Err(StbError::RowKeyRequiresDataRow { row: 0 })
    ));
}

#[test]
fn stb_inner_cells_round_trip() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let mut ed = StbInnerCells::new(stb);
    ed.set_inner_cell(2, 2, "edited".to_string()).unwrap();
    let stb = ed.finish();
    assert_eq!(stb.cell(2, 2), Some("edited"));
}
