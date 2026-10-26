//! Untyped rows with case-insensitive column access, mirroring how PPS reads
//! `DataRow` values (`ToString()` / `Convert.ToInt32`).

use std::cell::RefCell;
use std::rc::Rc;

use sqlite2::Connection;

thread_local! {
    /// Conversion errors (PPS would throw and abort). Collected so the
    /// converter code can stay close to the original without `?` on every read.
    static ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn record_error(msg: String) {
    ERRORS.with(|e| e.borrow_mut().push(msg));
}

pub(crate) fn take_errors() -> Vec<String> {
    ERRORS.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

#[derive(Debug, Clone)]
pub struct Row {
    cols: Rc<[String]>,
    vals: Vec<Option<String>>,
}

impl Row {
    fn idx(&self, k: &str) -> Option<usize> {
        self.cols.iter().position(|c| c.eq_ignore_ascii_case(k))
    }

    /// Raw value: `None` for NULL or a missing column.
    pub fn get(&self, k: &str) -> Option<&str> {
        self.idx(k).and_then(|i| self.vals[i].as_deref())
    }

    /// `DataRow[k].ToString()`: NULL and missing become "".
    pub fn s(&self, k: &str) -> &str {
        self.get(k).unwrap_or("")
    }

    /// `Convert.ToInt32(DataRow[k])`: unparsable values are recorded as errors.
    pub fn i(&self, k: &str) -> i64 {
        parse_int(self.s(k), k)
    }

    pub fn cols(&self) -> &[String] {
        &self.cols
    }

    pub fn has(&self, k: &str) -> bool {
        self.idx(k).is_some()
    }

    pub fn is_null(&self, k: &str) -> bool {
        self.get(k).is_none()
    }

    pub fn set(&mut self, k: &str, v: impl Into<String>) {
        if let Some(i) = self.idx(k) {
            self.vals[i] = Some(v.into());
        }
    }
}

pub(crate) fn parse_int(s: &str, what: &str) -> i64 {
    match s.trim().parse::<i64>() {
        Ok(v) => v,
        Err(_) => {
            record_error(format!("not an integer: {what} = {s:?}"));
            0
        }
    }
}

/// All rows of a table in rowid order; empty if the table doesn't exist.
pub fn load_table(db: &Connection, table: &str) -> Vec<Row> {
    let Ok(r) = db.query(&format!("SELECT * FROM '{table}' ORDER BY rowid"), &[]) else {
        return Vec::new();
    };
    let cols: Rc<[String]> = if r.columns.is_empty() {
        db.table_columns(table).unwrap_or_default().into()
    } else {
        r.columns.into()
    };
    r.rows.into_iter().map(|vals| Row { cols: cols.clone(), vals }).collect()
}

/// Rows whose column `k` (as text) equals `v`.
pub fn sel<'a>(rows: &'a [Row], k: &str, v: &str) -> Vec<&'a Row> {
    rows.iter().filter(|r| r.s(k) == v).collect()
}
