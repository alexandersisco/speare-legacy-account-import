// Generate the SQLite column catalog from the checked-in SQL Server reference. Fail the build
// for an unknown type rather than silently staging a newly discovered column incorrectly.
use std::{collections::HashSet, env, fmt::Write, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=legacy-sql-server-schema.sql");
    let input =
        fs::read_to_string("legacy-sql-server-schema.sql").expect("legacy schema reference");
    let mut output = String::from("pub(crate) static TABLES: &[Table] = &[\n");
    let mut table: Option<(String, String)> = None;
    let mut columns = Vec::new();
    let mut tables = 0;
    let mut names = HashSet::new();
    for line in input.lines() {
        let line = line.trim();
        if let Some(start) = line.strip_prefix("CREATE TABLE ") {
            assert!(table.is_none(), "nested table definition");
            let start = start.strip_prefix('[').expect("SQL Server schema name");
            let (schema, name) = start.split_once("].[").expect("SQL Server table header");
            table = Some((
                schema.to_owned(),
                name.split_once(']').expect("table name").0.to_owned(),
            ));
        } else if line == ");" && table.is_some() {
            let (schema, name) = table.take().unwrap();
            assert!(
                names.insert(name.clone()),
                "duplicate table name {name} across SQL schemas; qualify dataset names first"
            );
            assert!(
                columns
                    .iter()
                    .any(|s: &String| s.starts_with("Column { name: \"Id\"")),
                "{name} needs Id"
            );
            writeln!(
                output,
                "    Table {{ name: {name:?}, source_table: {:?}, columns: &[{}] }},",
                format!("{schema}.{name}"),
                columns.join(", ")
            )
            .unwrap();
            columns.clear();
            tables += 1;
        } else if table.is_some()
            && let Some(start) = line.strip_prefix('[')
        {
            let (name, rest) = start.split_once(']').expect("column name");
            let rest = rest.trim_start();
            let kind = if rest.starts_with("INT ") {
                "Integer"
            } else if rest.starts_with("BIT ") {
                "Boolean"
            } else if rest.starts_with("UNIQUEIDENTIFIER ")
                || rest.starts_with("VARCHAR ")
                || rest.starts_with("NVARCHAR ")
                || rest.starts_with("NCHAR ")
                || rest.starts_with("DATETIME ")
                || rest.starts_with("DATETIME2 ")
            {
                "Text"
            } else {
                panic!(
                    "unsupported SQL type in {}.{}: {rest}",
                    table.as_ref().unwrap().1,
                    name
                )
            };
            let nullable = !rest.contains("NOT NULL");
            columns.push(format!(
                "Column {{ name: {name:?}, source_type: {:?}, kind: Kind::{kind}, nullable: {nullable} }}",
                rest.split_whitespace().next().unwrap()
            ));
        }
    }
    assert!(table.is_none() && tables > 0, "incomplete legacy schema");
    output.push_str("];\n");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("legacy_tables.rs"),
        output,
    )
    .expect("write generated schema");
}
