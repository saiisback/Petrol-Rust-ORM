use crate::schema::{
    DefaultValue, Field, FieldAttribute, FieldType, Model, ModelAttribute, ScalarType, Schema,
};

#[derive(Debug, Clone)]
pub struct SqlTable {
    pub name: String,
    pub columns: Vec<SqlColumn>,
    pub primary_key: Vec<String>,
    pub uniques: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct SqlColumn {
    pub name: String,
    pub sql_type: SqlType,
    pub nullable: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SqlType {
    Serial,
    BigSerial,
    Integer,
    BigInt,
    Float,
    Decimal,
    Text,
    Boolean,
    Timestamp,
    Date,
    Uuid,
    Jsonb,
    Bytes,
}

impl SqlType {
    pub fn render(&self) -> &'static str {
        match self {
            SqlType::Serial => "SERIAL",
            SqlType::BigSerial => "BIGSERIAL",
            SqlType::Integer => "INTEGER",
            SqlType::BigInt => "BIGINT",
            SqlType::Float => "DOUBLE PRECISION",
            SqlType::Decimal => "DECIMAL",
            SqlType::Text => "TEXT",
            SqlType::Boolean => "BOOLEAN",
            SqlType::Timestamp => "TIMESTAMPTZ",
            SqlType::Date => "DATE",
            SqlType::Uuid => "UUID",
            SqlType::Jsonb => "JSONB",
            SqlType::Bytes => "BYTEA",
        }
    }
}

impl SqlTable {
    pub fn from_model(model: &Model) -> Self {
        let mut columns = Vec::new();
        let mut primary = Vec::new();
        let mut uniques = Vec::new();

        for field in &model.fields {
            if let Some(column) = SqlColumn::from_field(field) {
                columns.push(column);
            }

            if field
                .attributes
                .iter()
                .any(|attr| matches!(attr, FieldAttribute::Id))
            {
                primary.push(field.column_name());
            }
            if field
                .attributes
                .iter()
                .any(|attr| matches!(attr, FieldAttribute::Unique))
            {
                uniques.push(vec![field.column_name()]);
            }
        }

        for attr in &model.attributes {
            if let ModelAttribute::Unique(fields) = attr {
                uniques.push(fields.clone());
            }
        }

        Self {
            name: model.table_name(),
            columns,
            primary_key: primary,
            uniques,
        }
    }

    pub fn to_sql(&self) -> String {
        let mut buffer = String::new();
        buffer.push_str(&format!("CREATE TABLE IF NOT EXISTS \"{}\" (\n", self.name));

        let mut column_fragments: Vec<String> =
            self.columns.iter().map(|column| column.to_sql()).collect();

        if !self.primary_key.is_empty() {
            column_fragments.push(format!(
                "PRIMARY KEY ({})",
                self.primary_key
                    .iter()
                    .map(|name| format!("\"{}\"", name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        for unique in &self.uniques {
            column_fragments.push(format!(
                "UNIQUE ({})",
                unique
                    .iter()
                    .map(|name| format!("\"{}\"", name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        buffer.push_str(&column_fragments.join(",\n"));
        buffer.push_str("\n);\n");
        buffer
    }
}

impl SqlColumn {
    pub fn from_field(field: &Field) -> Option<Self> {
        match &field.r#type {
            FieldType::Scalar(scalar, modifiers) => {
                let sql_type = scalar_to_sql_type(scalar, field);
                let nullable = modifiers.optional;
                let default = default_clause(field);

                Some(Self {
                    name: field.column_name(),
                    sql_type,
                    nullable,
                    default,
                })
            }
            FieldType::Relation(_) => None,
        }
    }

    fn to_sql(&self) -> String {
        let mut fragment = format!("  \"{}\" {}", self.name, self.sql_type.render());
        if !self.nullable {
            fragment.push_str(" NOT NULL");
        }
        if let Some(default) = &self.default {
            fragment.push_str(&format!(" DEFAULT {}", default));
        }
        fragment
    }
}

pub fn schema_to_tables(schema: &Schema) -> Vec<SqlTable> {
    schema.models.iter().map(SqlTable::from_model).collect()
}

fn scalar_to_sql_type(scalar: &ScalarType, field: &Field) -> SqlType {
    match scalar {
        ScalarType::Int => {
            if has_autoincrement(field) {
                SqlType::Serial
            } else {
                SqlType::Integer
            }
        }
        ScalarType::BigInt => {
            if has_autoincrement(field) {
                SqlType::BigSerial
            } else {
                SqlType::BigInt
            }
        }
        ScalarType::Float => SqlType::Float,
        ScalarType::Decimal => SqlType::Decimal,
        ScalarType::String => SqlType::Text,
        ScalarType::Boolean => SqlType::Boolean,
        ScalarType::DateTime => SqlType::Timestamp,
        ScalarType::Date => SqlType::Date,
        ScalarType::Uuid => SqlType::Uuid,
        ScalarType::Json => SqlType::Jsonb,
        ScalarType::Bytes => SqlType::Bytes,
    }
}

fn has_autoincrement(field: &Field) -> bool {
    field
        .attributes
        .iter()
        .any(|attr| matches!(attr, FieldAttribute::Default(DefaultValue::AutoIncrement)))
}

fn default_clause(field: &Field) -> Option<String> {
    for attr in &field.attributes {
        if let FieldAttribute::Default(value) = attr {
            return match value {
                DefaultValue::AutoIncrement => None,
                DefaultValue::Uuid => Some("uuid_generate_v4()".to_string()),
                DefaultValue::Now => Some("now()".to_string()),
                DefaultValue::Boolean(v) => Some(v.to_string()),
                DefaultValue::Int(v) => Some(v.to_string()),
                DefaultValue::Float(v) => Some(v.to_string()),
                DefaultValue::String(v) => Some(format!("'{}'", v.replace("'", "''"))),
            };
        }
    }
    None
}
