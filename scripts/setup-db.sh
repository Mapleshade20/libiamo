#!/bin/bash

# Libiamo Database Setup Script
# This script creates the database and user for local development

set -e

echo "🔧 Setting up Libiamo Database..."

# PostgreSQL credentials from .env
DB_USER="libiamo_user"
DB_PASSWORD="ShouldContainCapSmallNum123"
DB_NAME="libiamo_db"
DB_HOST="localhost"

# Create the database user if it doesn't exist
echo "📝 Creating PostgreSQL user '$DB_USER'..."
sudo -u postgres psql << EOF
-- Drop user if exists (with CASCADE to remove owned objects)
DO \$\$
BEGIN
  IF EXISTS (SELECT FROM pg_user WHERE usename = '$DB_USER') THEN
    ALTER ROLE "$DB_USER" WITH PASSWORD '$DB_PASSWORD';
    RAISE NOTICE 'User password updated';
  ELSE
    CREATE ROLE "$DB_USER" WITH LOGIN PASSWORD '$DB_PASSWORD';
    RAISE NOTICE 'User created';
  END IF;
END
\$\$;

-- Grant privileges
ALTER ROLE "$DB_USER" CREATEDB;
EOF

# Create the database if it doesn't exist
echo "📦 Creating database '$DB_NAME'..."
sudo -u postgres psql << EOF
SELECT 'CREATE DATABASE "$DB_NAME"' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '$DB_NAME')\gexec
EOF

# Grant privileges on database
echo "🔐 Granting privileges..."
sudo -u postgres psql << EOF
GRANT ALL PRIVILEGES ON DATABASE "$DB_NAME" TO "$DB_USER";
ALTER DATABASE "$DB_NAME" OWNER TO "$DB_USER";
EOF

echo "✅ Database setup complete!"
echo ""
echo "Next steps:"
echo "1. Update your .env file with:"
echo "   DATABASE_URL=\"postgres://$DB_USER:$DB_PASSWORD@$DB_HOST:5432/$DB_NAME\""
echo "2. Run migrations: sqlx migrate run"
echo "3. Start the server: cargo run"
