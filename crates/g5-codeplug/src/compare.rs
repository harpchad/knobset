//! Logical comparison of two SQLite 2 databases: same tables, same columns,
//! same rows in the same rowid order, values compared as exact text.

use sqlite2::Connection;

use crate::row::load_table;
use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableStatus {
    Same,
    OnlyLeft,
    OnlyRight,
    ColumnsDiffer,
    RowsDiffer { first_index: usize, left: Option<Vec<String>>, right: Option<Vec<String>> },
}

#[derive(Debug, Clone)]
pub struct TableDiff {
    pub table: String,
    pub left_rows: usize,
    pub right_rows: usize,
    pub status: TableStatus,
}

pub fn compare(left: &Connection, right: &Connection) -> Result<Vec<TableDiff>, Error> {
    let lt = left.table_names()?;
    let rt = right.table_names()?;
    let mut names: Vec<String> = lt.iter().chain(rt.iter()).cloned().collect();
    names.sort();
    names.dedup();
    let mut out = Vec::new();
    for t in names {
        let (in_l, in_r) = (lt.contains(&t), rt.contains(&t));
        if !in_l || !in_r {
            let n = if in_l { rows_of(left, &t).len() } else { rows_of(right, &t).len() };
            out.push(TableDiff {
                table: t,
                left_rows: if in_l { n } else { 0 },
                right_rows: if in_r { n } else { 0 },
                status: if in_l { TableStatus::OnlyLeft } else { TableStatus::OnlyRight },
            });
            continue;
        }
        if left.table_columns(&t)? != right.table_columns(&t)? {
            out.push(TableDiff { table: t, left_rows: 0, right_rows: 0, status: TableStatus::ColumnsDiffer });
            continue;
        }
        let (a, b) = (rows_of(left, &t), rows_of(right, &t));
        let status = match a.iter().zip(&b).position(|(x, y)| x != y) {
            Some(i) => TableStatus::RowsDiffer { first_index: i, left: Some(a[i].clone()), right: Some(b[i].clone()) },
            None if a.len() != b.len() => {
                let i = a.len().min(b.len());
                TableStatus::RowsDiffer { first_index: i, left: a.get(i).cloned(), right: b.get(i).cloned() }
            }
            None => TableStatus::Same,
        };
        out.push(TableDiff { table: t, left_rows: a.len(), right_rows: b.len(), status });
    }
    Ok(out)
}

fn rows_of(db: &Connection, t: &str) -> Vec<Vec<String>> {
    let cols = db.table_columns(t).unwrap_or_default();
    load_table(db, t)
        .iter()
        .map(|r| cols.iter().map(|c| r.get(c).map_or_else(|| "<NULL>".to_string(), str::to_string)).collect())
        .collect()
}
