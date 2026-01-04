# Petrol ORM: Technical Specification & Documentation

**Project Name:** Petrol
**Tagline:** High-octane, type-safe database management for Rust.
**Target Databases:** PostgreSQL (specifically optimized for Supabase & NeonDB serverless environments).

---

## 1. High-Level Architecture

To replicate the Prisma experience in Rust, **Petrol** needs three distinct components:

1. **Petrol CLI (`petrol-cli`)**: The binary that developers run in their terminal (e.g., `petrol push`, `petrol generate`).
2. **Petrol Schema (`schema.petrol`)**: A custom DSL (Domain Specific Language) file where users define their models.
3. **Petrol Client (`petrol-client`)**: The runtime library that the generated code uses to talk to the database.

### Flow Diagram

```mermaid
graph TD
    A[User writes schema.petrol] --> B[Petrol CLI]
    B -- petrol generate --> C[Generated Rust Structs (types.rs)]
    B -- petrol push --> D[Supabase / NeonDB (Postgres)]
    B -- petrol pull --> E[Introspection Engine]
    E --> A
    C --> F[User's Rust App]

```

---

## 2. The Schema File (`schema.petrol`)

Instead of JSON or raw SQL, you need a readable configuration file. This will look very similar to Prisma but tailored for Rust types.

**Example `schema.petrol`:**

```graphql
datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL") // Supports Supabase/Neon connection strings
}

generator client {
  provider = "petrol-client-rust"
}

model User {
  id        Int      @id @default(autoincrement())
  email     String   @unique
  username  String?  // Optional type (Option<String>)
  isAdmin   Boolean  @default(false)
  createdAt DateTime @default(now())
}

model Post {
  id        Uuid     @id @default(uuid())
  title     String
  authorId  Int
  author    User     @relation(fields: [authorId], references: [id])
}

```

---

## 3. Core CLI Commands (The Workflow)

You need to implement the following commands in Rust using the `clap` crate.

### A. `petrol init`

* **Action:** Creates the initial folder structure and a default `schema.petrol`.
* **Rust Logic:** File system IO to write template files.

### B. `petrol push` (Schema -> Database)

This is the "Migration" step without creating migration files (prototyping mode).

* **Action:**
1. Parse `schema.petrol`.
2. Inspect the current state of the Supabase/Neon database.
3. Calculate the diff (e.g., "Table `User` is missing column `email`").
4. Generate and execute raw SQL `ALTER TABLE` commands to make the DB match the schema.


* **Tech Stack:** `sqlx` for executing queries, `schemars` or custom parser for the schema.

### C. `petrol pull` (Database -> Schema)

This is "Introspection".

* **Action:**
1. Connect to the DB using the connection string.
2. Query `information_schema` tables in Postgres to get all table names, columns, and types.
3. Reverse-engineer this metadata into the `schema.petrol` format.
4. Overwrite the local file.



### D. `petrol generate` (Schema -> Rust Code)

This is the most critical part for type safety.

* **Action:** Reads the schema and writes a Rust file (e.g., `src/petrol_client.rs`) containing structs and implementation blocks.
* **Tech Stack:** `syn` and `quote` crates (standard for Rust code generation).

---

## 4. Generated Code Structure (The "Type Safety" Magic)

When the user runs `petrol generate`, your tool should output Rust code that looks like this:

**Output file: `src/petrol/mod.rs**`

```rust
// 1. Type-Safe Models
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub username: Option<String>,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// 2. The Client
pub struct PetrolClient {
    pool: sqlx::PgPool,
}

impl PetrolClient {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::PgPool::connect(url).await?;
        Ok(Self { pool })
    }

    // Accessor for the User model
    pub fn user(&self) -> UserClient {
        UserClient { pool: &self.pool }
    }
}

// 3. Model-Specific Client (Fluid API)
pub struct UserClient<'a> {
    pool: &'a sqlx::PgPool,
}

impl<'a> UserClient<'a> {
    pub async fn create(&self, email: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            "INSERT INTO \"User\" (email) VALUES ($1) RETURNING *",
            email
        )
        .fetch_one(self.pool)
        .await
    }

    pub async fn find_many(&self) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as!(User, "SELECT * FROM \"User\"")
            .fetch_all(self.pool)
            .await
    }
}

```

---

## 5. Implementation Stack (How to build Petrol)

To build this tool in Rust, these are the crates you must use:

| Component | Recommended Crate | Purpose |
| --- | --- | --- |
| **CLI Argument Parsing** | `clap` | Handling commands like `push`, `pull`. |
| **Database Driver** | `sqlx` | Async Postgres driver. Supports connection pooling (vital for serverless/Neon). |
| **Parsing Schema** | `pest` or `nom` | To write the parser that reads your `.petrol` file. |
| **Code Generation** | `quote` & `proc-macro2` | To generate the Rust structs strings programmatically. |
| **Formatting** | `rustfmt-wrapper` | To make the generated code look clean. |
| **Async Runtime** | `tokio` | Required for all database operations. |

---

## 6. Connection String Handling (Supabase & Neon)

Since you want to support Supabase and Neon specifically, your `petrol push` command must handle **SSL modes** correctly.

**In `schema.petrol`:**

```bash
# Neon DB requires sslmode=require usually
url = "postgres://user:pass@ep-cool-db.us-east-1.aws.neon.tech/neondb?sslmode=require"

```

**In your Rust Logic:**
When parsing this URL, ensure `sqlx` is configured to respect the connection pool limits. Neon and Supabase are serverless, so they dislike idle connections.

* **Tip:** Implement a "Direct Connection" vs "Pooling Connection" logic.
* **Push/Pull:** Should use a direct connection (port 5432).
* **App Runtime:** Should ideally use the connection pooler (port 6543) if available.

