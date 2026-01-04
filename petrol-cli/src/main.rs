use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Parser, Subcommand};
use color_eyre::eyre::{eyre, Result, WrapErr};
use inflector::Inflector;
use petrol_client::PetrolClient;
use petrol_codegen::generate;
use petrol_core::schema::{
    DatasourceBlock, Field, FieldAttribute, FieldType, GeneratorBlock, Model, ScalarType, Schema,
    TypeModifiers,
};
use petrol_parser::parse_schema_file;
use sqlx::{postgres::PgPoolOptions, Row};
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about = "Petrol - Prisma-style ORM for Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a starter schema.petrol file
    Init {
        #[arg(long, default_value = "schema.petrol")]
        schema: PathBuf,
    },
    /// Generate Rust code from schema.petrol
    Generate {
        #[arg(long, default_value = "schema.petrol")]
        schema: PathBuf,
        #[arg(long, default_value = "src/petrol/mod.rs")]
        out: PathBuf,
    },
    /// Validate schema without generating output
    Validate {
        #[arg(long, default_value = "schema.petrol")]
        schema: PathBuf,
    },
    /// Apply schema to database (prototyping push)
    Push {
        #[arg(long, default_value = "schema.petrol")]
        schema: PathBuf,
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,
    },
    /// Introspect database into schema.petrol
    Pull {
        #[arg(long, default_value = "schema.petrol")]
        schema: PathBuf,
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,
    },
    /// Format schema.petrol deterministically
    Format {
        #[arg(long, default_value = "schema.petrol")]
        schema: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { schema } => handle_init(schema)?,
        Commands::Generate { schema, out } => handle_generate(schema, out)?,
        Commands::Validate { schema } => handle_validate(schema)?,
        Commands::Push {
            schema,
            database_url,
        } => handle_push(schema, &database_url).await?,
        Commands::Pull {
            schema,
            database_url,
        } => handle_pull(schema, &database_url).await?,
        Commands::Format { schema } => handle_format(schema)?,
    }

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

fn handle_init(schema_path: PathBuf) -> Result<()> {
    if schema_path.exists() {
        warn!("schema already exists at {:?}", schema_path);
        return Ok(());
    }

    let parent = schema_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent).wrap_err("failed to create schema directory")?;
    fs::write(&schema_path, DEFAULT_SCHEMA.trim_start())?;
    info!("created schema at {:?}", schema_path);
    Ok(())
}

fn handle_generate(schema_path: PathBuf, out_path: PathBuf) -> Result<()> {
    let schema = parse_schema_file(&schema_path)?;
    let rendered = generate(&schema)?;

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&out_path, rendered)?;
    if let Ok(status) = Command::new("rustfmt").arg(&out_path).status() {
        if !status.success() {
            warn!("rustfmt exited with status {:?}", status.code());
        }
    }

    info!("generated client at {:?}", out_path);
    Ok(())
}

fn handle_validate(schema_path: PathBuf) -> Result<()> {
    let schema = parse_schema_file(&schema_path)?;
    schema.validate()?;
    println!("Schema is valid ✅");
    Ok(())
}

async fn handle_push(schema_path: PathBuf, database_url: &str) -> Result<()> {
    let schema = parse_schema_file(&schema_path)?;
    schema.validate()?;
    let client = PetrolClient::new(database_url).await?;
    client.apply_schema(&schema).await?;
    println!("Schema pushed to database");
    Ok(())
}

async fn handle_pull(schema_path: PathBuf, database_url: &str) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await?;

    let schema = introspect_schema(&pool, database_url).await?;
    fs::write(&schema_path, schema.to_string())?;
    println!("Schema written to {:?}", schema_path);
    Ok(())
}

fn handle_format(schema_path: PathBuf) -> Result<()> {
    let schema = parse_schema_file(&schema_path)?;
    fs::write(&schema_path, schema.to_string())?;
    Ok(())
}

async fn introspect_schema(pool: &sqlx::PgPool, database_url: &str) -> Result<Schema> {
    let rows = sqlx::query(
        r#"
        SELECT
            table_name,
            column_name,
            data_type,
            is_nullable,
            column_default
        FROM information_schema.columns
        WHERE table_schema = 'public'
        ORDER BY table_name, ordinal_position
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut tables: BTreeMap<String, Vec<DbColumn>> = BTreeMap::new();
    for row in rows {
        let table: String = row.try_get("table_name")?;
        let column = DbColumn {
            name: row.try_get("column_name")?,
            data_type: row.try_get("data_type")?,
            is_nullable: row.try_get::<String, _>("is_nullable")? == "YES",
            default: row.try_get("column_default").ok(),
        };
        tables.entry(table).or_default().push(column);
    }

    let models = tables
        .into_iter()
        .map(|(table, columns)| build_model_from_columns(&table, &columns))
        .collect::<Result<Vec<_>, _>>()?;

    let datasource = DatasourceBlock {
        name: "db".into(),
        provider: "postgresql".into(),
        url: Some(database_url.to_string()),
        raw_url: Some(format!("\"{}\"", database_url)),
        connection_limit: None,
        pool_timeout_seconds: None,
    };

    let generator = GeneratorBlock::new("petrol-client-rust");

    Ok(Schema {
        datasource,
        generator,
        models,
    })
}

fn build_model_from_columns(table: &str, columns: &[DbColumn]) -> Result<Model> {
    let model_name = table.to_class_case();
    let fields = columns
        .iter()
        .map(|col| build_field(col))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Model {
        name: model_name,
        fields,
        attributes: Vec::new(),
    })
}

fn build_field(col: &DbColumn) -> Result<Field> {
    let scalar = map_sql_to_scalar(&col.data_type)
        .ok_or_else(|| eyre!("unsupported SQL type {}", col.data_type))?;

    let mut attributes = Vec::new();
    if col.name == "id" {
        attributes.push(FieldAttribute::Id);
    }
    if let Some(default) = &col.default {
        if default.contains("nextval") {
            attributes.push(FieldAttribute::Default(
                petrol_core::schema::DefaultValue::AutoIncrement,
            ));
        }
    }

    Ok(Field {
        name: col.name.clone(),
        r#type: FieldType::Scalar(
            scalar,
            TypeModifiers {
                optional: col.is_nullable,
                list: false,
            },
        ),
        attributes,
    })
}

fn map_sql_to_scalar(data_type: &str) -> Option<ScalarType> {
    match data_type {
        "integer" | "int4" => Some(ScalarType::Int),
        "bigint" | "int8" => Some(ScalarType::BigInt),
        "double precision" | "float8" => Some(ScalarType::Float),
        "numeric" | "decimal" => Some(ScalarType::Decimal),
        "text" | "character varying" | "varchar" => Some(ScalarType::String),
        "boolean" => Some(ScalarType::Boolean),
        "timestamp with time zone" | "timestamp without time zone" => Some(ScalarType::DateTime),
        "date" => Some(ScalarType::Date),
        "uuid" => Some(ScalarType::Uuid),
        "json" | "jsonb" => Some(ScalarType::Json),
        "bytea" => Some(ScalarType::Bytes),
        _ => None,
    }
}

struct DbColumn {
    name: String,
    data_type: String,
    is_nullable: bool,
    default: Option<String>,
}

const DEFAULT_SCHEMA: &str = r#"
// Petrol schema template
datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

generator client {
  provider = "petrol-client-rust"
}

model User {
  id        Int      @id @default(autoincrement())
  email     String   @unique
  username  String?
  createdAt DateTime @default(now())
}
"#;
