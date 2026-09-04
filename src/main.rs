use postgres::{Client, NoTls};

// --- literal formatting ---

trait SqlLiteral {
    fn sql_literal(&self) -> String;
}

impl SqlLiteral for str {
    fn sql_literal(&self) -> String {
        format!("'{}'", self.replace('\'', "''"))
    }
}

impl SqlLiteral for String {
    fn sql_literal(&self) -> String {
        self.as_str().sql_literal()
    }
}

impl SqlLiteral for i64 {
    fn sql_literal(&self) -> String { self.to_string() }
}

impl SqlLiteral for i32 {
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

impl<T: SqlLiteral> SqlLiteral for Option<T> {
    fn sql_literal(&self) -> String {
        match self {
            Some(v) => v.sql_literal(),
            None => "NULL".into(),
        }
    }
}

fn prepare<T: SqlLiteral + ?Sized>(v: &T) -> String {
    v.sql_literal()
}

// --- firebird value type — adjust variants/types to match the real one ---

enum SqlType {
    Text(String),
    Integer(i64),
    Floating(f64),
    Boolean(bool),
    Binary(Vec<u8>),
    Null,
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

// --- build + run the insert ---

fn row_to_insert(table: &str, row: &[SqlType]) -> String {
    let values: Vec<String> = row.iter().map(sqltype_to_literal).collect();
    format!("INSERT INTO {table} VALUES ({})", values.join(", "))
}

fn main() -> Result<(), postgres::Error> {
    let mut client = Client::connect(
        "host=localhost user=postgres password=postgres dbname=mydb",
        NoTls,
    )?;

    // rows pulled from firebird
    let rows: Vec<Vec<SqlType>> = vec![
        vec![
            SqlType::Integer(1),
            SqlType::Text("teste".to_string()),
            SqlType::Boolean(true),
            SqlType::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF]),
            SqlType::Null,
        ],
    ];

    for row in &rows {
        let query = row_to_insert("my_table", row);
        client.execute(&query, &[])?;
    }

    Ok(())
}
