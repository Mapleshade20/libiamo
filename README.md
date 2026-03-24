# Libiamo Backend

## Getting Started

### Prerequisites

1.  **PostgreSQL**: Ensure you have PostgreSQL installed and running on your machine.
2.  **UUID v7 Extension**: This project uses UUID v7 for time-ordered primary keys. You must install the `pg_uuidv7` extension:
    ```bash
    # Install build dependencies (Ubuntu/Debian)
    sudo apt update
    sudo apt install postgresql-server-dev-all build-essential git

    # Clone and install the extension
    git clone https://github.com/fboulnois/pg_uuidv7.git
    cd pg_uuidv7
    make
    sudo make install

    # Restart PostgreSQL service
    sudo systemctl restart postgresql
    ```
3.  **SQLX CLI**: Install the SQLx command-line tool for database management:
    ```bash
    cargo install sqlx-cli --no-default-features --features native-tls,postgres
    ```

### Database Setup

1.  **Environment Variables**: Create a `.env` file in the `libiamo-backend` root directory and add your database connection string:
    ```env
    DATABASE_URL=postgres://your_username:your_password@localhost:5432/libiamo_db
    ```
    *Replace `your_username`, `your_password`, and `libiamo_db` with your actual local PostgreSQL credentials.*

2.  **Initialize Database**: Create the project database:
    ```bash
    sqlx database create
    ```

3.  **Run Migrations**: Apply the schema (tables, types, and domains) to your database:
    ```bash
    sqlx migrate run
    ```

### Development

To run the backend server:
```bash
cargo run
```
The server will be available at `http://127.0.0.1:3000`.
