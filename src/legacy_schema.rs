use rusqlite::{Connection, types::Value as SqlValue};
use serde_json::{Map, Value};

use crate::{Error, Result};

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Integer,
    Boolean,
    Text,
}

pub(crate) struct Column {
    pub name: &'static str,
    pub source_type: &'static str,
    pub kind: Kind,
    pub nullable: bool,
}

pub(crate) struct Table {
    pub name: &'static str,
    pub source_table: &'static str,
    pub columns: &'static [Column],
}

include!(concat!(env!("OUT_DIR"), "/legacy_tables.rs"));

pub(crate) fn table(name: &str) -> Result<&'static Table> {
    TABLES.iter().find(|t| t.name == name)
        .ok_or_else(|| Error::Invalid(format!("unknown legacy dataset {name}; add it to the schema reference and review the required catalog")))
}

// All table and column identifiers originate from the checked-in schema, never the HTTP payload.
pub(crate) fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub(crate) fn create_tables(db: &Connection) -> Result<()> {
    for t in TABLES {
        let columns = t
            .columns
            .iter()
            .map(|c| {
                let sql_type = match c.kind {
                    Kind::Integer | Kind::Boolean => "INTEGER",
                    Kind::Text => "TEXT",
                };
                let constraint = if c.name == "Id" {
                    " PRIMARY KEY"
                } else if !c.nullable {
                    " NOT NULL"
                } else {
                    ""
                };
                let bit = if matches!(c.kind, Kind::Boolean) {
                    format!(" CHECK ({} IN (0, 1))", quoted(c.name))
                } else {
                    String::new()
                };
                format!("{} {sql_type}{constraint}{bit}", quoted(c.name))
            })
            .collect::<Vec<_>>()
            .join(", ");
        db.execute_batch(&format!(
            "CREATE TABLE {} ({columns}) STRICT;",
            quoted(t.name)
        ))?;
    }
    Ok(())
}

pub(crate) fn sql_values(t: &Table, data: &Value) -> Result<Vec<SqlValue>> {
    let object = data
        .as_object()
        .ok_or_else(|| Error::Invalid(format!("{} row is not an object", t.name)))?;
    if object.len() != t.columns.len() {
        return Err(Error::Invalid(format!(
            "{} row has missing or unexpected columns",
            t.name
        )));
    }
    t.columns
        .iter()
        .map(|c| {
            let value = object
                .get(c.name)
                .ok_or_else(|| Error::Invalid(format!("{}.{} is missing", t.name, c.name)))?;
            match (value, c.kind) {
                (Value::Null, _) if c.nullable => Ok(SqlValue::Null),
                (Value::Number(n), Kind::Integer)
                    if n.as_i64().is_some_and(|v| i32::try_from(v).is_ok()) =>
                {
                    Ok(SqlValue::Integer(n.as_i64().unwrap()))
                }
                (Value::Bool(b), Kind::Boolean) => Ok(SqlValue::Integer(i64::from(*b))),
                (Value::String(s), Kind::Text) => Ok(SqlValue::Text(s.clone())),
                _ => Err(Error::Invalid(format!(
                    "{}.{} has an invalid SQL type or null value",
                    t.name, c.name
                ))),
            }
        })
        .collect()
}

pub(crate) fn json_value(value: SqlValue, c: &Column) -> Result<Value> {
    match (value, c.kind) {
        (SqlValue::Null, _) if c.nullable => Ok(Value::Null),
        (SqlValue::Integer(n), Kind::Integer) => Ok(Value::from(n)),
        (SqlValue::Integer(n), Kind::Boolean) if n == 0 || n == 1 => Ok(Value::Bool(n == 1)),
        (SqlValue::Text(s), Kind::Text) => Ok(Value::String(s)),
        _ => Err(Error::Conflict(format!(
            "staged column {} has invalid storage type",
            c.name
        ))),
    }
}

pub(crate) fn json_row(t: &Table, values: Vec<SqlValue>) -> Result<Value> {
    let mut object = Map::new();
    for (column, value) in t.columns.iter().zip(values) {
        object.insert(column.name.into(), json_value(value, column)?);
    }
    Ok(Value::Object(object))
}
