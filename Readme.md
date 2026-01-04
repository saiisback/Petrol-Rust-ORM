<div align="center">

# Petrol - RUST ORM

**High-octane, type-safe database management for Rust.**

[![Crates.io](https://img.shields.io/crates/v/petrol-cli.svg)](https://crates.io/crates/petrol-cli)
[![Documentation](https://docs.rs/petrol-client/badge.svg)](https://docs.rs/petrol-client)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/yourusername/petrol/workflows/CI/badge.svg)](https://github.com/yourusername/petrol/actions)
[![Discord](https://img.shields.io/discord/1234567890?color=7289da&label=discord)](https://discord.gg/petrol)

*The Prisma experience, reimagined for Rust.*

[Getting Started](#getting-started) •
[Documentation](#schema-reference) •
[Examples](#usage-example) •
[Contributing](#contributing)

</div>

---

## Table of Contents

- [About](#about)
- [Features](#features)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [CLI Commands](#cli-commands)
- [Schema Reference](#schema-reference)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [Roadmap](#roadmap)
- [License](#license)

---

## About

Petrol is a next-generation ORM for Rust that brings the developer experience of [Prisma](https://www.prisma.io/) to the Rust ecosystem. Define your schema once in a declarative DSL, and Petrol generates fully type-safe Rust code for interacting with your database.

### Target Databases

| Database | Status | Notes |
|----------|--------|-------|
| PostgreSQL | Supported | Primary target |
| Supabase | Optimized | Serverless-ready with connection pooling |
| NeonDB | Optimized | Serverless-ready with SSL support |

---

## Features

- **Declarative Schema** — Define your database models in a clean, readable `.petrol` schema file
- **100% Type-Safe** — Generated Rust structs with compile-time guarantees
- **Bidirectional Sync** — Push schema changes to DB or pull existing DB into schema
- **Async-First** — Built on `tokio` and `sqlx` for maximum performance
- **Zero Runtime Overhead** — All type checking happens at compile time
- **Serverless Ready** — Optimized connection handling for Supabase and NeonDB
- **Fluent API** — Chainable, expressive query builder

---

## Installation

### Prerequisites

- Rust 1.70 or later
- PostgreSQL 12+ (or Supabase/NeonDB account)

### Install CLI

```bash
cargo install petrol-cli
```

### Add Client Library

```bash
cargo add petrol-client
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
petrol-client = "0.1"
tokio = { version = "1", features = ["full"] }
```

---

## Getting Started

### 1. Initialize Project

```bash
petrol init
```

This creates a `schema.petrol` file in your project root.

### 2. Define Schema

```prisma
// schema.petrol

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
  isAdmin   Boolean  @default(false)
  createdAt DateTime @default(now())
  posts     Post[]
}

model Post {
  id        Uuid     @id @default(uuid())
  title     String
  content   String?
  published Boolean  @default(false)
  authorId  Int
  author    User     @relation(fields: [authorId], references: [id])
}
```

### 3. Push Schema to Database

```bash
petrol push
```

### 4. Generate Type-Safe Client

```bash
petrol generate
```

### 5. Usage Example

```rust
use petrol::PetrolClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PetrolClient::new(&std::env::var("DATABASE_URL")?).await?;
    
    // Create a new user (fully type-safe)
    let user = client
        .user()
        .create("alice@example.com")
        .await?;
    
    println!("Created user: {:?}", user);
    
    // Query all users
    let users = client.user().find_many().await?;
    
    for user in users {
        println!("User: {} (ID: {})", user.email, user.id);
    }
    
    Ok(())
}
```

---

## CLI Commands

| Command | Description |
|---------|-------------|
| `petrol init` | Initialize a new Petrol project with default schema |
| `petrol push` | Push schema changes to database (prototyping mode) |
| `petrol pull` | Introspect existing database and generate schema |
| `petrol generate` | Generate type-safe Rust client from schema |
| `petrol validate` | Validate schema syntax and relationships |
| `petrol format` | Format schema file |
| `petrol version` | Display version information |

### Command Examples

```bash
# Initialize with custom schema path
petrol init --schema ./db/schema.petrol

# Push with force flag (skip confirmation)
petrol push --force

# Generate to custom output directory
petrol generate --output ./src/db

# Pull from specific database
petrol pull --url "postgres://user:pass@localhost:5432/mydb"
```

---

## Schema Reference

### Data Types

| Petrol Type | Rust Type | PostgreSQL Type |
|-------------|-----------|-----------------|
| `Int` | `i32` | `INTEGER` |
| `BigInt` | `i64` | `BIGINT` |
| `Float` | `f64` | `DOUBLE PRECISION` |
| `Decimal` | `rust_decimal::Decimal` | `DECIMAL` |
| `String` | `String` | `TEXT` |
| `Boolean` | `bool` | `BOOLEAN` |
| `DateTime` | `chrono::DateTime<Utc>` | `TIMESTAMPTZ` |
| `Date` | `chrono::NaiveDate` | `DATE` |
| `Uuid` | `uuid::Uuid` | `UUID` |
| `Json` | `serde_json::Value` | `JSONB` |
| `Bytes` | `Vec<u8>` | `BYTEA` |

### Attributes

| Attribute | Description | Example |
|-----------|-------------|---------|
| `@id` | Primary key | `id Int @id` |
| `@unique` | Unique constraint | `email String @unique` |
| `@default(value)` | Default value | `@default(now())` |
| `@relation` | Define relationship | `@relation(fields: [authorId], references: [id])` |
| `@map("name")` | Map to different column name | `@map("user_email")` |
| `@@map("name")` | Map to different table name | `@@map("users")` |
| `@@unique([fields])` | Composite unique | `@@unique([email, username])` |
| `@@index([fields])` | Create index | `@@index([createdAt])` |

### Type Modifiers

| Modifier | Description | Rust Type |
|----------|-------------|-----------|
| `?` | Optional field | `Option<T>` |
| `[]` | Array/List | `Vec<T>` |

---

## Configuration

### Environment Variables

Create a `.env` file in your project root:

```bash
DATABASE_URL="postgres://user:password@localhost:5432/database"
```

### Supabase Configuration

```bash
# Direct connection (for migrations/push/pull)
DATABASE_URL="postgres://postgres:[PASSWORD]@db.[PROJECT].supabase.co:5432/postgres"

# Pooled connection (for application runtime - recommended)
DATABASE_URL="postgres://postgres:[PASSWORD]@db.[PROJECT].supabase.co:6543/postgres?pgbouncer=true"
```

### NeonDB Configuration

```bash
DATABASE_URL="postgres://user:pass@ep-cool-db.us-east-1.aws.neon.tech/neondb?sslmode=require"
```

### Connection Pool Settings

Configure in your schema:

```prisma
datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
  
  // Optional: Connection pool configuration
  connectionLimit = 5
  poolTimeout     = 10
}
```

---

## Architecture

### High-Level Overview

```mermaid
graph TB
    subgraph Developer
        A[schema.petrol]
        B[Rust Application]
    end
    
    subgraph Petrol CLI
        C[Parser]
        D[Code Generator]
        E[Migration Engine]
        F[Introspection Engine]
    end
    
    subgraph Generated
        G[types.rs]
        H[mod.rs]
    end
    
    subgraph Database
        I[(PostgreSQL)]
        J[(Supabase)]
        K[(NeonDB)]
    end
    
    A --> C
    C --> D
    C --> E
    C --> F
    
    D --> G
    D --> H
    
    E --> I
    E --> J
    E --> K
    
    F --> I
    F --> J
    F --> K
    F --> A
    
    G --> B
    H --> B
    B --> I
    B --> J
    B --> K
```

### CLI Command Flow

```mermaid
flowchart LR
    subgraph Input
        S[schema.petrol]
        DB[(Database)]
    end
    
    subgraph Commands
        INIT[petrol init]
        PUSH[petrol push]
        PULL[petrol pull]
        GEN[petrol generate]
    end
    
    subgraph Output
        SCHEMA[schema.petrol]
        CODE[Generated Rust Code]
        TABLES[Database Tables]
    end
    
    INIT --> SCHEMA
    S --> PUSH --> TABLES
    S --> GEN --> CODE
    DB --> PULL --> SCHEMA
```

### Generated Code Structure

```mermaid
classDiagram
    class PetrolClient {
        -pool: PgPool
        +new(url: &str) Result~Self~
        +user() UserClient
        +post() PostClient
    }
    
    class UserClient {
        -pool: &PgPool
        +create(email: &str) Result~User~
        +find_many() Result~Vec~User~~
        +find_unique(id: i32) Result~Option~User~~
        +update(id: i32, data: UserUpdate) Result~User~
        +delete(id: i32) Result~User~
    }
    
    class PostClient {
        -pool: &PgPool
        +create(data: PostCreate) Result~Post~
        +find_many() Result~Vec~Post~~
        +find_unique(id: Uuid) Result~Option~Post~~
    }
    
    class User {
        +id: i32
        +email: String
        +username: Option~String~
        +is_admin: bool
        +created_at: DateTime
    }
    
    class Post {
        +id: Uuid
        +title: String
        +content: Option~String~
        +author_id: i32
    }
    
    PetrolClient --> UserClient
    PetrolClient --> PostClient
    UserClient --> User
    PostClient --> Post
```

### Internal Component Architecture

```mermaid
graph LR
    subgraph petrol-cli
        CLI[CLI Entry]
        CMD[Command Handler]
    end
    
    subgraph petrol-parser
        LEX[Lexer]
        PARSE[Parser]
        AST[AST Builder]
    end
    
    subgraph petrol-codegen
        RUSTGEN[Rust Generator]
        SQLGEN[SQL Generator]
        FMT[Formatter]
    end
    
    subgraph petrol-core
        TYPES[Core Types]
        SCHEMA[Schema Types]
    end
    
    subgraph petrol-client
        POOL[Connection Pool]
        QUERY[Query Builder]
        EXEC[Executor]
    end
    
    CLI --> CMD
    CMD --> LEX --> PARSE --> AST
    AST --> RUSTGEN --> FMT
    AST --> SQLGEN
    
    TYPES --> SCHEMA
    SCHEMA --> AST
    
    POOL --> QUERY --> EXEC
```

---

## Project Structure

### Repository Layout

```
petrol/
├── petrol-cli/              # CLI binary crate
│   ├── src/
│   │   ├── main.rs
│   │   └── commands/
│   │       ├── init.rs
│   │       ├── push.rs
│   │       ├── pull.rs
│   │       └── generate.rs
│   └── Cargo.toml
│
├── petrol-client/           # Runtime client library
│   ├── src/
│   │   ├── lib.rs
│   │   ├── pool.rs
│   │   └── query.rs
│   └── Cargo.toml
│
├── petrol-parser/           # Schema parser
│   ├── src/
│   │   ├── lib.rs
│   │   ├── lexer.rs
│   │   ├── parser.rs
│   │   └── ast.rs
│   ├── schema.pest          # Grammar definition
│   └── Cargo.toml
│
├── petrol-codegen/          # Code generation engine
│   ├── src/
│   │   ├── lib.rs
│   │   ├── rust.rs
│   │   └── sql.rs
│   └── Cargo.toml
│
├── petrol-core/             # Shared types and utilities
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types.rs
│   │   └── error.rs
│   └── Cargo.toml
│
├── examples/                # Example projects
│   ├── basic/
│   └── with-axum/
│
├── tests/                   # Integration tests
├── docs/                    # Documentation
├── .github/                 # CI/CD workflows
├── Cargo.toml               # Workspace manifest
├── LICENSE
└── README.md
```

### Generated Output Structure

After running `petrol generate`, your project will contain:

```
your-project/
├── schema.petrol            # Schema definition
├── src/
│   ├── main.rs
│   └── petrol/              # Generated directory
│       ├── mod.rs           # Client implementation
│       └── types.rs         # Model structs
├── Cargo.toml
└── .env
```

---

## Contributing

We welcome contributions from the community. Please read our contributing guidelines before submitting a pull request.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/petrol.git
cd petrol

# Install development dependencies
rustup component add rustfmt clippy

# Build all packages
cargo build --workspace

# Run tests
cargo test --workspace

# Run lints
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --all
```

### Running the CLI Locally

```bash
# Run CLI commands during development
cargo run -p petrol-cli -- init
cargo run -p petrol-cli -- generate
cargo run -p petrol-cli -- push
```

### Testing

```bash
# Unit tests
cargo test --workspace

# Integration tests (requires PostgreSQL)
DATABASE_URL="postgres://localhost/petrol_test" cargo test --workspace -- --ignored

# Specific test
cargo test -p petrol-parser test_parse_model
```

### Pull Request Guidelines

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please ensure:
- All tests pass
- Code is formatted with `rustfmt`
- No clippy warnings
- Documentation is updated
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)

---

## Roadmap

### Version 0.1 (Current)
- [x] Schema parser
- [x] PostgreSQL support
- [x] Basic CRUD operations
- [x] Code generation

### Version 0.2 (Planned)
- [ ] Migration files support
- [ ] Transactions API
- [ ] Raw SQL queries
- [ ] Connection pooling improvements

### Version 0.3 (Future)
- [ ] MySQL support
- [ ] SQLite support
- [ ] Seeding support
- [ ] Studio (GUI)

See the [open issues](https://github.com/yourusername/petrol/issues) for a full list of proposed features and known issues.

---

## Tech Stack

| Component | Crate | Purpose |
|-----------|-------|---------|
| CLI Argument Parsing | `clap` | Command handling |
| Database Driver | `sqlx` | Async PostgreSQL driver |
| Schema Parsing | `pest` | Grammar-based parser |
| Code Generation | `quote`, `proc-macro2` | Rust code generation |
| Async Runtime | `tokio` | Async execution |
| Serialization | `serde` | Data serialization |
| Formatting | `rustfmt-wrapper` | Code formatting |

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

```
MIT License

Copyright (c) 2024 Petrol Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## Acknowledgments

- [Prisma](https://www.prisma.io/) — Inspiration for the developer experience
- [sqlx](https://github.com/launchbadge/sqlx) — The async SQL toolkit for Rust
- [tokio](https://tokio.rs/) — The async runtime for Rust
- [pest](https://pest.rs/) — The elegant parser

---

<div align="center">

**[Back to top](#petrol---rust-orm)**

</div>
