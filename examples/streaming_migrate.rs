// Example pattern: stream rows from Firebird -> NDJSON file -> batched inserts
// into Postgres, without ever materializing the whole result set in memory.
//
// This is illustrative, not a drop-in binary: `YourFirebirdConnection`,
// `fb_conn.query_iter(...)`, and the Firebird connection setup in `main`
// are placeholders — swap them for your actual Firebird crate's API.

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

#[derive(Serialize, Deserialize)]
enum SqlType {
    Text(String),
    Integer(i64),
    Floating(f64),
    Boolean(bool),
    Binary(Vec<u8>),
    Null,
}

// --- literal formatting, for building the plain (non-prepared) INSERT string ---

trait SqlLiteral {
    fn sql_literal(&self) -> String;
}

impl SqlLiteral for str {
    fn sql_literal(&self) -> String {
        format!("'{}'", self.replace('\'', "''"))
    }
}

impl SqlLiteral for i64 {
    fn sql_literal(&self) -> String { self.to_string() }
}

impl SqlLiteral for f64 {
    fn sql_literal(&self) -> String { self.to_string() }
}

impl SqlLiteral for bool {
    fn sql_literal(&self) -> String {
        if *self { "TRUE".into() } else { "FALSE".into() }
    }
}

impl SqlLiteral for [u8] {
    fn sql_literal(&self) -> String {
        let hex: String = self.iter().map(|b| format!("{b:02x}")).collect();
        format!("'\\x{hex}'")
    }
}

fn prepare<T: SqlLiteral + ?Sized>(v: &T) -> String {
    v.sql_literal()
}

fn sqltype_to_literal(v: &SqlType) -> String {
    match v {
        SqlType::Text(s) => prepare(s.as_str()),
        SqlType::Integer(i) => prepare(i),
        SqlType::Floating(f) => prepare(f),
        SqlType::Boolean(b) => prepare(b),
        SqlType::Binary(b) => prepare(b.as_slice()),
        SqlType::Null => "NULL".to_string(),
    }
}

// --- 1. stream SELECT results from Firebird straight to an NDJSON file ---
//
// Never collects a Vec<Vec<SqlType>> of the whole result set — each row is
// written and dropped before the next one is fetched, so peak memory stays
// at roughly one row's size instead of the whole table's.

fn export_from_firebird(
    fb_conn: &mut YourFirebirdConnection, // placeholder: your real connection type
    sql: &str,
    out_path: &str,
) -> anyhow::Result<()> {
    let file = File::create(out_path)?;
    let mut writer = BufWriter::new(file);

    // placeholder: swap for your crate's actual streaming/cursor query call
    let rows_iter = fb_conn.query_iter::<Vec<SqlType>>(sql, ())?;

    for row in rows_iter {
        let row = row?;
        serde_json::to_writer(&mut writer, &row)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(())
}

// --- 2. read the NDJSON file back in batches and insert into Postgres ---

const BATCH_SIZE: usize = 500;

fn migrate(client: &mut postgres::Client, path: &str, table: &str) -> anyhow::Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let stream = Deserializer::from_reader(reader).into_iter::<Vec<SqlType>>();

    let mut batch: Vec<Vec<SqlType>> = Vec::with_capacity(BATCH_SIZE);

    for row in stream {
        batch.push(row?);
        if batch.len() == BATCH_SIZE {
            insert_batch(client, table, &batch)?;
            batch.clear();
        }
    }

    if !batch.is_empty() {
        insert_batch(client, table, &batch)?;
    }

    Ok(())
}

fn insert_batch(client: &mut postgres::Client, table: &str, rows: &[Vec<SqlType>]) -> Result<u64, postgres::Error> {
    let tuples: Vec<String> = rows
        .iter()
        .map(|row| {
            let values: Vec<String> = row.iter().map(sqltype_to_literal).collect();
            format!("({})", values.join(", "))
        })
        .collect();

    let query = format!("INSERT INTO {table} VALUES {}", tuples.join(", "));
    client.execute(&query, &[])
}

// --- 3. wire it together ---

fn main() -> anyhow::Result<()> {
    let mut fb_conn = /* connect to firebird using your crate */ todo!();
    export_from_firebird(&mut fb_conn, "SELECT * FROM my_table", "export.jsonl")?;

    let mut pg_client = postgres::Client::connect(
        "host=localhost user=postgres password=postgres dbname=mydb",
        postgres::NoTls,
    )?;
    migrate(&mut pg_client, "export.jsonl", "my_table")?;

    Ok(())
}
