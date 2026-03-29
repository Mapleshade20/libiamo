# Libiamo Backend

## Quick Start

The fastest way to get started is to use the automated setup script:

```bash
bash scripts/setup-and-run.sh
```

This script will:
- ✅ Check all prerequisites (PostgreSQL, Rust, sqlx-cli)
- ✅ Create/configure PostgreSQL user and database
- ✅ Auto-generate or update `.env` file with credentials
- ✅ Run database migrations
- ✅ Start the development server

The server will run at `http://127.0.0.1:8090`

---

## Manual Setup (Alternative)

### Prerequisites

1.  **PostgreSQL 18**: Ensure you have PostgreSQL 18 installed and running.
2.  **Rust & Cargo**: Install from https://rustup.rs/
3.  **SQLX CLI**: Install the SQLx command-line tool:
    ```bash
    cargo install sqlx-cli --no-default-features --features native-tls,postgres
    ```

### Step-by-Step Setup

1.  **Set up database and user** (one-time setup):
    ```bash
    bash scripts/setup-db.sh
    ```

2.  **Configure environment variables**: Copy `.env.example` to `.env`:
    ```bash
    cp .env.example .env
    ```
    Then update the credentials in `.env` if needed.

3.  **Run migrations**: Apply the schema to your database:
    ```bash
    sqlx migrate run
    ```

4.  **Start the server**:
    ```bash
    cargo run
    ```

---

## Development

### Running Tests

```bash
cargo test
```

### Checking Code

```bash
cargo check
```

### Building for Production

```bash
cargo build --release
```

---

## Scripts

For more information about available scripts and their usage, see [scripts/README.md](scripts/README.md).
