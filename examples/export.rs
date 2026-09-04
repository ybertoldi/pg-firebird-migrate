// Export a table from Firebird into an NDJSON file (one JSON row per line).
//
// Plain `SELECT * FROM table`, fetched all at once into a Vec, then written
// out and dropped row by row. `YourFirebirdConnection` and `fb_conn.query(...)`
// are placeholders — swap them for your actual Firebird crate's connection
// type and query call.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Serialize, Deserialize)]
enum SqlType {
    Text(String),
    Integer(i64),
    Floating(f64),
    Boolean(bool),
    Binary(Vec<u8>),
    Null,
}

fn export_from_firebird(
    fb_conn: &mut YourFirebirdConnection, // placeholder: your real connection type
    table: &str,
    out_path: &str,
) -> anyhow::Result<()> {
    let sql = format!("SELECT * FROM {table}");
    let rows: Vec<Vec<SqlType>> = fb_conn.query(&sql, ())?; // placeholder: your real query call

    let file = File::create(out_path)?;
    let mut writer = BufWriter::new(file);

    for row in rows {
        serde_json::to_writer(&mut writer, &row)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut fb_conn = /* connect to firebird using your crate */ todo!();
    export_from_firebird(&mut fb_conn, "my_table", "export.jsonl")?;
    Ok(())
}
