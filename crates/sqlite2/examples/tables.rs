//! Print table names and row counts: `cargo run -p sqlite2 --example tables -- file.db`
fn main() {
    let path = std::env::args().nth(1).expect("usage: tables <file.db>");
    let db = sqlite2::Connection::open(std::path::Path::new(&path)).unwrap();
    for t in db.table_names().unwrap() {
        let n = db.query(&format!("SELECT count(*) FROM '{t}'"), &[]).unwrap();
        println!("{:>6}  {t}", n.rows[0][0].as_deref().unwrap_or("?"));
    }
}
