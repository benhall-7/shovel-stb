use crate::{LineReplaceMode, Stb, StbError, TableLine};

use super::common::fixture;

#[test]
fn column_index_lookup() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    assert_eq!(stb.column_index("Name"), Some(1));
    assert_eq!(stb.column_index("Health"), Some(3));
    assert_eq!(stb.column_index("nonexistent"), None);
}

#[test]
fn key_lookup_by_name() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    assert_eq!(stb.row_for_named_column("Name", "Specter Knight"), Some(3));
    assert_eq!(stb.row_for_named_column("Name", "NobodyHere"), None);
    assert_eq!(stb.row_for_named_column("no_such_column", "x"), None);
}

#[test]
fn key_lookup_by_column_index() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();

    assert_eq!(stb.row_for_column(1, "Plague Knight"), Some(2));
    assert_eq!(stb.row_for_column(99, "x"), None);
}

#[test]
fn column_lookup_by_row() {
    let stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    assert_eq!(stb.column_for_row(1, "8"), Some(3));
    assert_eq!(stb.column_for_row(0, "Health"), Some(3));
}

#[test]
fn replace_line_inner_row_matches_set_inner() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let nc = stb.num_cols();
    let r = 2usize;
    let mut inner: Vec<String> = (1..nc)
        .map(|c| stb.cell(r, c).unwrap().to_string())
        .collect();
    inner[0] = "patched".to_string();

    let mut stb2 = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    stb2.set_inner_cell(r, 1, "patched".to_string()).unwrap();

    stb.replace_line(TableLine::Row(r), LineReplaceMode::Inner, inner)
        .unwrap();
    assert_eq!(stb.cell(r, 1), Some("patched"));
    assert_eq!(stb.cell(r, 2), stb2.cell(r, 2));
}

#[test]
fn replace_line_inner_column_preserves_groups() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let rg = stb.row_groups().to_vec();
    let cg = stb.col_groups().to_vec();
    let col = 3usize;
    let nb = stb.num_body_rows();
    let col_cells: Vec<String> = (0..nb)
        .map(|br| stb.cell(br + 1, col).unwrap().to_string())
        .collect();

    stb.replace_line(TableLine::Column(col), LineReplaceMode::Inner, col_cells)
        .unwrap();
    assert_eq!(stb.row_groups(), rg.as_slice());
    assert_eq!(stb.col_groups(), cg.as_slice());
}

#[test]
fn replace_line_full_row_rebuilds_row_groups() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let rg_before = stb.row_groups().to_vec();
    let nc = stb.num_cols();
    let row: Vec<String> = (0..nc)
        .map(|c| stb.cell(1, c).unwrap().to_string())
        .collect();

    stb.replace_line(TableLine::Row(1), LineReplaceMode::Full, row)
        .unwrap();
    assert_eq!(stb.row_groups(), rg_before.as_slice());
}

#[test]
fn replace_line_inner_rejects_row0_or_col0() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    assert!(
        stb.replace_line(TableLine::Row(0), LineReplaceMode::Inner, vec![],)
            .is_err()
    );
    assert!(
        stb.replace_line(TableLine::Column(0), LineReplaceMode::Inner, vec![],)
            .is_err()
    );
}

#[test]
fn replace_line_bad_len() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let r = stb.replace_line(TableLine::Row(1), LineReplaceMode::Full, vec![]);
    assert!(matches!(r, Err(StbError::LineReplaceBadLen { .. })));
}

#[test]
fn stb_line_inner_row_get_mut_finish_matches_replace_line() {
    let mut a = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let mut b = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let r = 2usize;
    let nc = a.num_cols();

    let mut line = a
        .line_mut(TableLine::Row(r), LineReplaceMode::Inner)
        .unwrap();
    *line.get_mut(0).unwrap() = "patched".to_string();
    line.finish();

    let inner: Vec<String> = (1..nc).map(|c| b.cell(r, c).unwrap().to_string()).collect();
    let mut inner = inner;
    inner[0] = "patched".to_string();
    b.replace_line(TableLine::Row(r), LineReplaceMode::Inner, inner)
        .unwrap();

    assert_eq!(a.cell(r, 1), b.cell(r, 1));
    assert_eq!(a.cell_hash(r, 1), b.cell_hash(r, 1));
}

#[test]
fn stb_line_cross_axis_row_by_column_header() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let r = 2usize;
    let mut line = stb
        .line_mut(TableLine::Row(r), LineReplaceMode::Full)
        .unwrap();
    assert_eq!(line.get_by_cross_axis_key("Name").unwrap(), "Plague Knight");
    *line.get_mut_by_cross_axis_key("Name").unwrap() = "patched".to_string();
    line.finish();
    assert_eq!(stb.cell(r, 1), Some("patched"));
}

#[test]
fn stb_line_cross_axis_column_by_row_key() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let col = 3usize;
    let row = 2usize;
    let row_key = stb.cell(row, 0).unwrap().to_string();
    let expected = stb.cell(row, col).unwrap().to_string();
    let mut line = stb
        .line_mut(TableLine::Column(col), LineReplaceMode::Full)
        .unwrap();
    assert_eq!(
        line.get_by_cross_axis_key(&row_key).unwrap(),
        expected.as_str()
    );
    *line.get_mut_by_cross_axis_key(&row_key).unwrap() = "patched".to_string();
    line.finish();
    assert_eq!(stb.cell(row, col), Some("patched"));
}

#[test]
fn stb_line_cross_axis_inner_row_id_header_excluded() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let line = stb
        .line_mut(TableLine::Row(2), LineReplaceMode::Inner)
        .unwrap();
    let e = line.get_by_cross_axis_key("ID").unwrap_err();
    assert!(matches!(e, StbError::LineCrossAxisKeyOutsideLine(_)));
}

#[test]
fn stb_line_cross_axis_inner_column_row0_key_excluded() {
    let mut stb = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    assert_eq!(stb.cell(0, 0), Some("ID"));
    let line = stb
        .line_mut(TableLine::Column(1), LineReplaceMode::Inner)
        .unwrap();
    let e = line.get_by_cross_axis_key("ID").unwrap_err();
    assert!(matches!(e, StbError::LineCrossAxisKeyOutsideLine(_)));
}

#[test]
fn stb_line_get_line_set_line_finish_matches_noop_column() {
    let mut a = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let b = Stb::open(fixture("battle/characterAttributes.stb")).unwrap();
    let col = 3usize;

    let mut line = a
        .line_mut(TableLine::Column(col), LineReplaceMode::Inner)
        .unwrap();
    let v = line.get_line();
    line.set_line(v).unwrap();
    line.finish();

    assert_eq!(a.row_groups(), b.row_groups());
    assert_eq!(a.col_groups(), b.col_groups());
}
