# Libiamo Backend Scripts

This directory contains utility scripts for database setup and server management.

## Scripts

### 🚀 `setup-and-run.sh` - Recommended for First-Time Setup

**Complete one-command setup and server startup**

```bash
bash setup-and-run.sh
```

This script will:
- ✅ Check all prerequisites (PostgreSQL, Rust, sqlx-cli)
- ✅ Create/configure PostgreSQL user and database
- ✅ Auto-generate or update `.env` file with credentials
- ✅ Run database migrations
- ✅ Start the development server

**Output:**
- Server runs at `http://127.0.0.1:8090`
- Press `Ctrl+C` to stop the server

---

### 🗄️ `setup-db.sh` - Database Setup Only

**Set up PostgreSQL database and user without starting the server**

```bash
bash setup-db.sh
```

Use this when you:
- Only need to create/reset the database
- Want to run database setup separately from server startup
- Are resetting the database for testing

**Next steps after running:**
1. Update your `.env` file (if needed)
2. Run migrations: `sqlx migrate run`
3. Start the server: `cargo run` or use `setup-and-run.sh`

---

## Requirements

Both scripts require:
- **PostgreSQL 18** - Database server
- **Rust & Cargo** - For building and managing dependencies
- **sqlx-cli** - Will be installed automatically if missing
- **sudo access** - Required for PostgreSQL user creation

## Environment Setup

The scripts use these PostgreSQL credentials (configurable within the script):
- **User:** `libiamo_user`
- **Password:** `ShouldContainCapSmallNum123`
- **Database:** `libiamo_db`
- **Host:** `localhost`
- **Port:** `5432`

To modify these values, edit the configuration variables at the top of each script.

## Troubleshooting

### "PostgreSQL not found"
```bash
# Install PostgreSQL (Ubuntu/Debian)
sudo apt-get install postgresql postgresql-contrib

# Or macOS with Homebrew
brew install postgresql
```

### "Cargo not found"
```bash
# Install Rust from https://rustup.rs/
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### "sqlx-cli not found"
The script will attempt to install it automatically. If it fails, run manually:
```bash
cargo install sqlx-cli --no-default-features --features native-tls,postgres
```

### Password authentication failed
Run the database setup script again:
```bash
bash scripts/setup-db.sh
```

Or use the complete setup:
```bash
bash scripts/setup-and-run.sh
```
