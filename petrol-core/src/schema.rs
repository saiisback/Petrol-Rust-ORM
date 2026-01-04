use crate::error::PetrolError;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub datasource: DatasourceBlock,
    pub generator: GeneratorBlock,
    pub models: Vec<Model>,
}

impl Schema {
    pub fn validate(&self) -> Result<(), PetrolError> {
        if self.models.is_empty() {
            return Err(PetrolError::validation(
                "schema must contain at least one model",
            ));
        }

        for model in &self.models {
            model.validate()?;
        }

        Ok(())
    }

    pub fn find_model(&self, name: &str) -> Option<&Model> {
        self.models.iter().find(|m| m.name == name)
    }

    pub fn datasource_url(&self) -> Option<String> {
        self.datasource
            .url
            .clone()
            .or_else(|| std::env::var("DATABASE_URL").ok())
    }
}

impl Display for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "datasource {} {{", self.datasource.name)?;
        writeln!(f, "  provider = \"{}\"", self.datasource.provider)?;
        if let Some(url) = &self.datasource.raw_url {
            writeln!(f, "  url      = {}", url)?;
        } else {
            writeln!(f, "  url      = env(\"DATABASE_URL\")")?;
        }
        if let Some(limit) = self.datasource.connection_limit {
            writeln!(f, "  connectionLimit = {}", limit)?;
        }
        if let Some(timeout) = self.datasource.pool_timeout_seconds {
            writeln!(f, "  poolTimeout     = {}", timeout)?;
        }
        writeln!(f, "}}\n")?;

        writeln!(f, "generator {} {{", self.generator.name)?;
        writeln!(f, "  provider = \"{}\"", self.generator.provider)?;
        if let Some(output) = &self.generator.output {
            writeln!(f, "  output   = \"{}\"", output)?;
        }
        writeln!(f, "}}\n")?;

        for model in &self.models {
            writeln!(f, "model {} {{", model.name)?;
            for field in &model.fields {
                writeln!(f, "  {}", field)?;
            }
            for attr in &model.attributes {
                writeln!(f, "  {}", attr)?;
            }
            writeln!(f, "}}\n")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasourceBlock {
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub raw_url: Option<String>,
    #[serde(default)]
    pub connection_limit: Option<u32>,
    #[serde(default)]
    pub pool_timeout_seconds: Option<u32>,
}

impl DatasourceBlock {
    pub fn new(provider: &str) -> Self {
        Self {
            name: "db".into(),
            provider: provider.into(),
            url: None,
            raw_url: None,
            connection_limit: None,
            pool_timeout_seconds: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorBlock {
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub output: Option<String>,
}

impl GeneratorBlock {
    pub fn new(provider: &str) -> Self {
        Self {
            name: "client".into(),
            provider: provider.into(),
            output: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub fields: Vec<Field>,
    #[serde(default)]
    pub attributes: Vec<ModelAttribute>,
}

impl Model {
    pub fn validate(&self) -> Result<(), PetrolError> {
        if self.fields.is_empty() {
            return Err(PetrolError::validation(format!(
                "model {} must contain at least one field",
                self.name
            )));
        }
        if !self
            .fields
            .iter()
            .any(|f| f.attributes.iter().any(|a| matches!(a, FieldAttribute::Id)))
        {
            return Err(PetrolError::validation(format!(
                "model {} must declare an @id field",
                self.name
            )));
        }
        Ok(())
    }

    pub fn table_name(&self) -> String {
        for attr in &self.attributes {
            if let ModelAttribute::Map(name) = attr {
                return name.clone();
            }
        }
        self.name.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub r#type: FieldType,
    #[serde(default)]
    pub attributes: Vec<FieldAttribute>,
}

impl Field {
    pub fn column_name(&self) -> String {
        for attr in &self.attributes {
            if let FieldAttribute::Map(name) = attr {
                return name.clone();
            }
        }
        self.name.clone()
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.r#type)?;
        for attr in &self.attributes {
            write!(f, " {}", attr)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    Scalar(ScalarType, TypeModifiers),
    Relation(RelationInfo),
}

impl FieldType {
    pub fn modifiers(&self) -> &TypeModifiers {
        match self {
            FieldType::Scalar(_, modifiers) => modifiers,
            FieldType::Relation(info) => &info.modifiers,
        }
    }
}

impl Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::Scalar(scalar, modifiers) => {
                write!(
                    f,
                    "{}{}{}",
                    scalar,
                    modifiers.optional_suffix(),
                    modifiers.list_suffix()
                )
            }
            FieldType::Relation(info) => {
                write!(
                    f,
                    "{}{}{}",
                    info.model,
                    info.modifiers.optional_suffix(),
                    info.modifiers.list_suffix()
                )
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeModifiers {
    pub optional: bool,
    pub list: bool,
}

impl TypeModifiers {
    pub fn optional_suffix(&self) -> &str {
        if self.optional {
            "?"
        } else {
            ""
        }
    }

    pub fn list_suffix(&self) -> &str {
        if self.list {
            "[]"
        } else {
            ""
        }
    }
}

impl Default for TypeModifiers {
    fn default() -> Self {
        Self {
            optional: false,
            list: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationInfo {
    pub model: String,
    pub modifiers: TypeModifiers,
    pub attributes: Vec<FieldAttribute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScalarType {
    Int,
    BigInt,
    Float,
    Decimal,
    String,
    Boolean,
    DateTime,
    Date,
    Uuid,
    Json,
    Bytes,
}

impl Display for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ident = match self {
            ScalarType::Int => "Int",
            ScalarType::BigInt => "BigInt",
            ScalarType::Float => "Float",
            ScalarType::Decimal => "Decimal",
            ScalarType::String => "String",
            ScalarType::Boolean => "Boolean",
            ScalarType::DateTime => "DateTime",
            ScalarType::Date => "Date",
            ScalarType::Uuid => "Uuid",
            ScalarType::Json => "Json",
            ScalarType::Bytes => "Bytes",
        };
        write!(f, "{}", ident)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldAttribute {
    Id,
    Unique,
    UpdatedAt,
    Map(String),
    Relation(RelationAttribute),
    Default(DefaultValue),
}

impl Display for FieldAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldAttribute::Id => write!(f, "@id"),
            FieldAttribute::Unique => write!(f, "@unique"),
            FieldAttribute::UpdatedAt => write!(f, "@updatedAt"),
            FieldAttribute::Map(name) => write!(f, "@map(\"{}\")", name),
            FieldAttribute::Relation(attr) => write!(
                f,
                "@relation(fields: [{}], references: [{}])",
                attr.fields.join(", "),
                attr.references.join(", ")
            ),
            FieldAttribute::Default(value) => write!(f, "@default({})", value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationAttribute {
    pub fields: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefaultValue {
    AutoIncrement,
    Uuid,
    Now,
    Boolean(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl Display for DefaultValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefaultValue::AutoIncrement => write!(f, "autoincrement()"),
            DefaultValue::Uuid => write!(f, "uuid()"),
            DefaultValue::Now => write!(f, "now()"),
            DefaultValue::Boolean(value) => write!(f, "{}", value),
            DefaultValue::Int(value) => write!(f, "{}", value),
            DefaultValue::Float(value) => write!(f, "{}", value),
            DefaultValue::String(value) => write!(f, "\"{}\"", value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelAttribute {
    Map(String),
    Unique(Vec<String>),
    Index(Vec<String>),
}

impl Display for ModelAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelAttribute::Map(name) => write!(f, "@@map(\"{}\")", name),
            ModelAttribute::Unique(fields) => write!(f, "@@unique([{}])", fields.join(", ")),
            ModelAttribute::Index(fields) => write!(f, "@@index([{}])", fields.join(", ")),
        }
    }
}
