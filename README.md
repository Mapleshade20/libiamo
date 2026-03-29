# Libiamo Backend

## Getting Started

### Prerequisites

1.  **PostgreSQL 18**: Ensure you have PostgreSQL 18 installed and running on your machine.
2.  **SQLX CLI**: Install the SQLx command-line tool for database management:
    ```bash
    cargo install sqlx-cli --no-default-features --features native-tls,postgres
    ```

### Database Setup

1.  **Environment Variables**: Create a `.env` file from the template `.env.example` in the root directory and set your database URL:

    ```env
    DATABASE_URL=postgres://your_username:your_password@localhost:5432/libiamo_db
    ```

2.  **Initialize Database**: Create the user first and then create its owned database:

    ```bash
    createdb libiamo_db -O libiamo_user
    ```

3.  **Run Migrations**: Apply the schema to the database:
    ```bash
    sqlx migrate run
    ```

### Development

To run the backend server:
```bash
cargo run
```
The server will be available at the specified address and port.
