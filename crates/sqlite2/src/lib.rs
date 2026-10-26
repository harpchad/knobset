//! Minimal safe wrapper over the vendored SQLite 2.8.17 C API.
//!
//! SQLite 2 is typeless: every value is text or NULL, so rows are
//! `Vec<Option<String>>`. Only what the pager tooling needs is exposed.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;
use std::ptr;

mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    #[repr(C)]
    pub struct sqlite {
        _private: [u8; 0],
    }
    #[repr(C)]
    pub struct sqlite_vm {
        _private: [u8; 0],
    }

    pub const SQLITE_OK: c_int = 0;
    pub const SQLITE_ROW: c_int = 100;
    pub const SQLITE_DONE: c_int = 101;

    unsafe extern "C" {
        pub fn sqlite_open(filename: *const c_char, mode: c_int, errmsg: *mut *mut c_char) -> *mut sqlite;
        pub fn sqlite_close(db: *mut sqlite);
        pub fn sqlite_exec(
            db: *mut sqlite,
            sql: *const c_char,
            cb: Option<unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_char, *mut *mut c_char) -> c_int>,
            arg: *mut c_void,
            errmsg: *mut *mut c_char,
        ) -> c_int;
        pub fn sqlite_compile(
            db: *mut sqlite,
            sql: *const c_char,
            tail: *mut *const c_char,
            vm: *mut *mut sqlite_vm,
            errmsg: *mut *mut c_char,
        ) -> c_int;
        pub fn sqlite_step(
            vm: *mut sqlite_vm,
            n: *mut c_int,
            values: *mut *const *const c_char,
            colnames: *mut *const *const c_char,
        ) -> c_int;
        pub fn sqlite_finalize(vm: *mut sqlite_vm, errmsg: *mut *mut c_char) -> c_int;
        pub fn sqlite_bind(vm: *mut sqlite_vm, idx: c_int, value: *const c_char, len: c_int, copy: c_int) -> c_int;
        pub fn sqlite_changes(db: *mut sqlite) -> c_int;
        pub fn sqlite_error_string(code: c_int) -> *const c_char;
        pub fn sqlite_freemem(p: *mut c_void);
        pub fn sqlite_libversion() -> *const c_char;
        pub fn sqlite_libencoding() -> *const c_char;
        pub static mut sqlite_temp_directory: *const c_char;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlite2 error {code}: {message}")]
    Sqlite { code: i32, message: String },
    #[error("string contains NUL byte")]
    Nul(#[from] std::ffi::NulError),
    #[error("path is not valid UTF-8")]
    Path,
    #[error("expected a single SQL statement")]
    MultipleStatements,
}

pub type Result<T> = std::result::Result<T, Error>;

/// Header every SQLite 2.1 file starts with.
pub const FILE_HEADER: &[u8] = b"** This file contains an SQLite 2.1 database **";

pub fn version() -> &'static str {
    unsafe { CStr::from_ptr(ffi::sqlite_libversion()) }.to_str().unwrap_or("?")
}

pub fn encoding() -> &'static str {
    unsafe { CStr::from_ptr(ffi::sqlite_libencoding()) }.to_str().unwrap_or("?")
}

/// True if the file starts with the SQLite 2.1 header.
pub fn is_sqlite2(path: &Path) -> bool {
    use std::io::Read;
    let mut buf = [0u8; 47];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut buf)).is_ok() && buf == FILE_HEADER
}

/// Point SQLite 2's temp files at the OS temp dir. Its fallback list is
/// /var/tmp, /usr/tmp, /tmp, then ".", none of which an iOS app may write.
fn set_temp_directory() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(dir) = std::env::temp_dir().to_str().and_then(|s| CString::new(s).ok()) else { return };
        // Leaked on purpose: SQLite keeps the pointer for the life of the process.
        unsafe { ffi::sqlite_temp_directory = dir.into_raw() };
    });
}

/// Query result: column names plus rows of nullable text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

pub struct Connection {
    db: *mut ffi::sqlite,
}

impl Connection {
    /// Open (or create) a database file.
    pub fn open(path: &Path) -> Result<Self> {
        set_temp_directory();
        let p = CString::new(path.to_str().ok_or(Error::Path)?)?;
        let mut err: *mut c_char = ptr::null_mut();
        let db = unsafe { ffi::sqlite_open(p.as_ptr(), 0, &mut err) };
        if db.is_null() {
            return Err(Error::Sqlite { code: 1, message: take_err(err).unwrap_or_else(|| "open failed".into()) });
        }
        Ok(Self { db })
    }

    /// Run one or more statements with no parameters and no results.
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let s = CString::new(sql)?;
        let mut err: *mut c_char = ptr::null_mut();
        let rc = unsafe { ffi::sqlite_exec(self.db, s.as_ptr(), None, ptr::null_mut(), &mut err) };
        check(rc, err)
    }

    /// Run one statement with `?` parameters; returns rows changed.
    pub fn execute(&self, sql: &str, params: &[Option<&str>]) -> Result<usize> {
        self.run(sql, params, |_, _| {})?;
        Ok(unsafe { ffi::sqlite_changes(self.db) } as usize)
    }

    /// Run one statement with `?` parameters and collect all rows.
    pub fn query(&self, sql: &str, params: &[Option<&str>]) -> Result<Rows> {
        let mut out = Rows::default();
        self.run(sql, params, |cols, row| {
            if out.columns.is_empty() {
                out.columns = cols.to_vec();
            }
            if let Some(r) = row {
                out.rows.push(r);
            }
        })?;
        Ok(out)
    }

    /// Column names of a table, in declaration order.
    pub fn table_columns(&self, table: &str) -> Result<Vec<String>> {
        let r = self.query(&format!("PRAGMA table_info('{}')", table.replace('\'', "''")), &[])?;
        let name = r.columns.iter().position(|c| c == "name").unwrap_or(1);
        Ok(r.rows.into_iter().filter_map(|mut row| row.get_mut(name).and_then(Option::take)).collect())
    }

    /// Names of all tables, sorted.
    pub fn table_names(&self) -> Result<Vec<String>> {
        let r = self.query("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name", &[])?;
        Ok(r.rows.into_iter().filter_map(|mut row| row[0].take()).collect())
    }

    fn run(
        &self,
        sql: &str,
        params: &[Option<&str>],
        mut on_row: impl FnMut(&[String], Option<Vec<Option<String>>>),
    ) -> Result<()> {
        let s = CString::new(sql)?;
        let mut tail: *const c_char = ptr::null();
        let mut vm: *mut ffi::sqlite_vm = ptr::null_mut();
        let mut err: *mut c_char = ptr::null_mut();
        let rc = unsafe { ffi::sqlite_compile(self.db, s.as_ptr(), &mut tail, &mut vm, &mut err) };
        check(rc, err)?;
        if vm.is_null() {
            return Ok(()); // empty statement
        }
        let rest = unsafe { CStr::from_ptr(tail) }.to_string_lossy();
        if !rest.trim().trim_start_matches(';').trim().is_empty() {
            unsafe { ffi::sqlite_finalize(vm, ptr::null_mut()) };
            return Err(Error::MultipleStatements);
        }
        // Bound strings must outlive sqlite_bind; copy=1 makes SQLite own a copy anyway.
        let owned: Vec<Option<CString>> = params.iter().map(|p| p.map(CString::new).transpose()).collect::<std::result::Result<_, _>>()?;
        for (i, p) in owned.iter().enumerate() {
            let v = p.as_ref().map_or(ptr::null(), |c| c.as_ptr());
            let rc = unsafe { ffi::sqlite_bind(vm, (i + 1) as c_int, v, -1, 1) };
            if rc != ffi::SQLITE_OK {
                unsafe { ffi::sqlite_finalize(vm, ptr::null_mut()) };
                return Err(Error::Sqlite { code: rc, message: code_string(rc) });
            }
        }

        let mut columns: Vec<String> = Vec::new();
        loop {
            let mut n: c_int = 0;
            let mut values: *const *const c_char = ptr::null();
            let mut names: *const *const c_char = ptr::null();
            let rc = unsafe { ffi::sqlite_step(vm, &mut n, &mut values, &mut names) };
            if columns.is_empty() && !names.is_null() {
                columns = (0..n as usize).map(|i| cstr_opt(unsafe { *names.add(i) }).unwrap_or_default()).collect();
            }
            match rc {
                ffi::SQLITE_ROW => {
                    let row = (0..n as usize).map(|i| cstr_opt(unsafe { *values.add(i) })).collect();
                    on_row(&columns, Some(row));
                }
                ffi::SQLITE_DONE => {
                    on_row(&columns, None);
                    break;
                }
                _ => break,
            }
        }
        let mut err: *mut c_char = ptr::null_mut();
        let rc = unsafe { ffi::sqlite_finalize(vm, &mut err) };
        check(rc, err)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe { ffi::sqlite_close(self.db) };
    }
}

fn cstr_opt(p: *const c_char) -> Option<String> {
    (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}

fn code_string(rc: c_int) -> String {
    cstr_opt(unsafe { ffi::sqlite_error_string(rc) }).unwrap_or_default()
}

fn take_err(err: *mut c_char) -> Option<String> {
    if err.is_null() {
        return None;
    }
    let s = cstr_opt(err);
    unsafe { ffi::sqlite_freemem(err as *mut c_void) };
    s
}

fn check(rc: c_int, err: *mut c_char) -> Result<()> {
    let msg = take_err(err);
    if rc == ffi::SQLITE_OK {
        return Ok(());
    }
    Err(Error::Sqlite { code: rc, message: msg.unwrap_or_else(|| code_string(rc)) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_is_utf8_2_8() {
        assert_eq!(version(), "2.8.17");
        assert_eq!(encoding(), "UTF-8");
    }

    #[test]
    fn create_insert_query_preserves_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch("CREATE TABLE t(a INTEGER, b VARCHAR(10));").unwrap();
            assert_eq!(db.execute("INSERT INTO t VALUES(?, ?)", &[Some("0000000000000000"), None]).unwrap(), 1);
            db.execute("INSERT INTO t VALUES('42','it''s é')", &[]).unwrap();
            let r = db.query("SELECT a, b FROM t ORDER BY rowid", &[]).unwrap();
            assert_eq!(r.columns, ["a", "b"]);
            assert_eq!(r.rows, vec![
                vec![Some("0000000000000000".into()), None],
                vec![Some("42".into()), Some("it's é".into())],
            ]);
            assert_eq!(db.table_columns("t").unwrap(), ["a", "b"]);
            assert_eq!(db.table_names().unwrap(), ["t"]);
        }
        assert!(is_sqlite2(&path));
    }

    #[test]
    fn errors_and_single_statement_rule() {
        let dir = tempfile::tempdir().unwrap();
        let db = Connection::open(&dir.path().join("e.db")).unwrap();
        assert!(matches!(db.query("SELECT * FROM missing", &[]), Err(Error::Sqlite { .. })));
        assert!(matches!(db.query("SELECT 1; SELECT 2", &[]), Err(Error::MultipleStatements)));
    }
}
