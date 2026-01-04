use petrol_core::{schema::Schema, sql::schema_to_tables, PetrolError};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use thiserror::Error;

pub use sqlx;

#[derive(Clone)]
pub struct PetrolClient {
    pool: PgPool,
}

impl PetrolClient {
    pub async fn new(database_url: &str) -> Result<Self, ClientError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<(), ClientError> {
        sqlx::query("SELECT 1")
            .execute(self.pool())
            .await?
            .rows_affected();
        Ok(())
    }

    pub async fn apply_schema(&self, schema: &Schema) -> Result<(), ClientError> {
        for table in schema_to_tables(schema) {
            let sql = table.to_sql();
            sqlx::query(&sql).execute(self.pool()).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Petrol(#[from] PetrolError),
}

impl ClientError {
    pub fn msg<T: Into<String>>(msg: T) -> Self {
        Self::Message(msg.into())
    }
}

pub mod prelude {
    pub use crate::{ClientError, PetrolClient};
    pub use sqlx;
}
