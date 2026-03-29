#!/bin/bash

# Libiamo Database Setup and Server Startup Script
# This script handles complete setup: DB creation, migrations, and server startup

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DB_USER="libiamo_user"
DB_PASSWORD="ShouldContainCapSmallNum123"
DB_NAME="libiamo_db"
DB_HOST="localhost"
DB_PORT="5432"
ENV_FILE=".env"

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Libiamo Backend - Setup & Run Script  ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""

# Check prerequisites
echo -e "${YELLOW}🔍 Checking prerequisites...${NC}"

# Check PostgreSQL
if ! command -v psql &> /dev/null; then
    echo -e "${RED}❌ PostgreSQL not found. Please install PostgreSQL 18 first.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ PostgreSQL found${NC}"

# Check Cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo not found. Please install Rust first.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Rust/Cargo found${NC}"

# Check sqlx-cli
if ! command -v sqlx &> /dev/null; then
    echo -e "${YELLOW}⚠️  sqlx-cli not found. Installing...${NC}"
    cargo install sqlx-cli --no-default-features --features native-tls,postgres
fi
echo -e "${GREEN}✓ sqlx-cli available${NC}"

echo ""
echo -e "${YELLOW}🗄️  Setting up PostgreSQL database...${NC}"

# Create/Update database user
echo "  📝 Creating/updating PostgreSQL user '$DB_USER'..."
sudo -u postgres psql << EOF > /dev/null 2>&1
DO \$\$
BEGIN
  IF EXISTS (SELECT FROM pg_user WHERE usename = '$DB_USER') THEN
    ALTER ROLE "$DB_USER" WITH PASSWORD '$DB_PASSWORD';
  ELSE
    CREATE ROLE "$DB_USER" WITH LOGIN PASSWORD '$DB_PASSWORD';
  END IF;
END
\$\$;

ALTER ROLE "$DB_USER" CREATEDB;
EOF

# Create database if it doesn't exist
echo "  📦 Creating database '$DB_NAME'..."
sudo -u postgres psql << EOF > /dev/null 2>&1
SELECT 'CREATE DATABASE "$DB_NAME"' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '$DB_NAME')\gexec
EOF

# Grant privileges
echo "  🔐 Granting privileges..."
sudo -u postgres psql << EOF > /dev/null 2>&1
GRANT ALL PRIVILEGES ON DATABASE "$DB_NAME" TO "$DB_USER";
ALTER DATABASE "$DB_NAME" OWNER TO "$DB_USER";
EOF

echo -e "${GREEN}✓ Database setup complete${NC}"

# Create or update .env file
echo ""
echo -e "${YELLOW}⚙️  Configuring environment variables...${NC}"

if [ ! -f "$ENV_FILE" ]; then
    echo "  Creating .env file..."
    cat > "$ENV_FILE" << EOF
# ============================================================================
# Database Configuration
# ============================================================================
DATABASE_URL="postgres://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME"
SQLX_OFFLINE=false

# ============================================================================
# Server Configuration
# ============================================================================
ADDRESS="127.0.0.1:8090"
RUST_LOG="info,libiamo=debug"
NO_COLOR=

# ============================================================================
# Authentication Token Configuration
# ============================================================================
TOKEN_EXPIRATION_HOURS=24

# ============================================================================
# Email Verification Configuration
# ============================================================================
SMTP_HOST="smtp.gmail.com"
SMTP_PORT="587"
SMTP_USERNAME="your-email@gmail.com"
SMTP_PASSWORD="ShouldContainCapSmallNum123"
FROM_EMAIL="noreply@libiamo.com"

# Frontend URL for email verification links
FRONTEND_URL="http://localhost:5173"
EOF
    echo -e "${GREEN}  ✓ .env file created${NC}"
else
    # Update DATABASE_URL in existing .env
    if grep -q "DATABASE_URL=" "$ENV_FILE"; then
        sed -i "s|^DATABASE_URL=.*|DATABASE_URL=\"postgres://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME\"|" "$ENV_FILE"
    else
        echo "DATABASE_URL=\"postgres://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME\"" >> "$ENV_FILE"
    fi
    
    # Ensure SQLX_OFFLINE is set to false
    if grep -q "SQLX_OFFLINE=" "$ENV_FILE"; then
        sed -i "s/^SQLX_OFFLINE=.*/SQLX_OFFLINE=false/" "$ENV_FILE"
    else
        echo "SQLX_OFFLINE=false" >> "$ENV_FILE"
    fi
    
    echo -e "${GREEN}  ✓ .env file updated${NC}"
fi

# Run migrations
echo ""
echo -e "${YELLOW}📜 Running database migrations...${NC}"
if sqlx migrate run; then
    echo -e "${GREEN}✓ Migrations completed${NC}"
else
    echo -e "${RED}⚠️  Migrations may have already been applied${NC}"
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Setup Complete! Starting Server...     ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}Server will run at: http://127.0.0.1:8090${NC}"
echo -e "${BLUE}Press Ctrl+C to stop the server${NC}"
echo ""

# Start the server
cargo run
