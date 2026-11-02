//! Logical comparison of two SQLite 2 databases: same tables, same columns,
//! same rows in the same rowid order, values compared as exact text (NULL
//! apart from any text), and the same schema entries: every table, index,
//! trigger and view with its SQL.

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
        out.push(TableDiff { table: t, left_rows: a.len(), right_rows: b.len(), status: row_status(&a, &b) });
    }
    // Listed only when it differs, so identical databases report just their tables.
    let (a, b) = (schema_of(left)?, schema_of(right)?);
    if a != b {
        out.push(TableDiff { table: "sqlite_master".into(), left_rows: a.len(), right_rows: b.len(), status: row_status(&a, &b) });
    }
    Ok(out)
}

type Values = Vec<Option<String>>;

fn row_status(a: &[Values], b: &[Values]) -> TableStatus {
    let show = |r: Option<&Values>| r.map(|r| r.iter().map(|v| v.clone().unwrap_or_else(|| "<NULL>".into())).collect());
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(i) => TableStatus::RowsDiffer { first_index: i, left: show(a.get(i)), right: show(b.get(i)) },
        None if a.len() != b.len() => {
            let i = a.len().min(b.len());
            TableStatus::RowsDiffer { first_index: i, left: show(a.get(i)), right: show(b.get(i)) }
        }
        None => TableStatus::Same,
    }
}

/// Every schema entry with its SQL. `rootpage` is left out: it follows the file's layout.
fn schema_of(db: &Connection) -> Result<Vec<Values>, Error> {
    Ok(db.query("SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY type, name", &[])?.rows)
}

fn rows_of(db: &Connection, t: &str) -> Vec<Values> {
    let cols = db.table_columns(t).unwrap_or_default();
    load_table(db, t).iter().map(|r| cols.iter().map(|c| r.get(c).map(str::to_string)).collect()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(dir: &tempfile::TempDir, name: &str, sql: &str) -> Connection {
        let c = Connection::open(&dir.path().join(name)).unwrap();
        c.execute_batch(sql).unwrap();
        c
    }

    fn differing(a: &Connection, b: &Connection) -> Vec<String> {
        compare(a, b).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect()
    }

    #[test]
    fn null_differs_from_any_text() {
        let dir = tempfile::tempdir().unwrap();
        let a = db(&dir, "a.db", "CREATE TABLE t(a); INSERT INTO t VALUES(NULL);");
        let b = db(&dir, "b.db", "CREATE TABLE t(a); INSERT INTO t VALUES('<NULL>');");
        let c = db(&dir, "c.db", "CREATE TABLE t(a); INSERT INTO t VALUES('');");
        assert_eq!(differing(&a, &b), ["t"]);
        assert_eq!(differing(&a, &c), ["t"]);
    }

    #[test]
    fn schema_entries_count_and_identical_databases_list_only_tables() {
        let dir = tempfile::tempdir().unwrap();
        let a = db(&dir, "a.db", "CREATE TABLE t(a); INSERT INTO t VALUES('1');");
        let b = db(&dir, "b.db", "CREATE TABLE t(a); INSERT INTO t VALUES('1'); CREATE INDEX i ON t(a);");
        let c = db(&dir, "c.db", "CREATE TABLE t(a); INSERT INTO t VALUES('1');");
        assert_eq!(differing(&a, &b), ["sqlite_master"]);
        let same = compare(&a, &c).unwrap();
        assert_eq!(same.iter().map(|d| d.table.as_str()).collect::<Vec<_>>(), ["t"]);
        assert!(same.iter().all(|d| d.status == TableStatus::Same));
    }
}
