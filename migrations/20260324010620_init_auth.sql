-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- 定义 UUID v7 生成函数
CREATE OR REPLACE FUNCTION uuidv7() RETURNS uuid AS $$
DECLARE
    v_time bigint := (extract(epoch from now()) * 1000)::bigint;
    v_bytes bytea := decode(lpad(to_hex(v_time), 12, '0'), 'hex') || gen_random_bytes(10);
BEGIN
    v_bytes := set_byte(v_bytes, 6, (get_byte(v_bytes, 6) & 15) | 112); -- set version to 7
    v_bytes := set_byte(v_bytes, 8, (get_byte(v_bytes, 8) & 63) | 128); -- set variant to 10xx
    RETURN encode(v_bytes, 'hex')::uuid;
END;
$$ LANGUAGE plpgsql VOLATILE;

-- ENUM and DOMAIN types
CREATE TYPE user_role AS ENUM ('learner', 'admin');

CREATE TYPE token_purpose AS ENUM ('email_verification', 'password_reset');

CREATE TYPE language_code AS ENUM ('en', 'es', 'fr'); -- BCP 47

CREATE DOMAIN native_language_code AS VARCHAR(5)
CHECK (
    VALUE ~ '^[a-z]{2}(-[A-Z]{2})?$'
);

-- Users Table
CREATE TABLE users (
    id                  INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    email               VARCHAR(128) UNIQUE NOT NULL,
    password_hash       TEXT NOT NULL,
    is_verified         BOOLEAN NOT NULL DEFAULT FALSE,
    role                user_role NOT NULL,

    nickname            VARCHAR(64),
    avatar_url          VARCHAR(512),
    target_language     language_code NOT NULL,
    native_language     native_language_code NOT NULL,
    timezone            VARCHAR(64) NOT NULL DEFAULT 'UTC',
    gems_balance        INT DEFAULT 0,
    level_self_assign   INT CHECK (level_self_assign BETWEEN 1 AND 5),

    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at          TIMESTAMPTZ
);

-- Recovery / Password Reset Tokens
CREATE TABLE auth_tokens (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id     INT NOT NULL REFERENCES users(id),

    token_hash  TEXT NOT NULL,
    purpose     token_purpose NOT NULL,
    used_at     TIMESTAMPTZ,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Login Sessions
CREATE TABLE auth_sessions (
    -- id 将作为 Cookie 的值发给用户
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id     INT NOT NULL REFERENCES users(id),

    ip_address  TEXT,
    user_agent  TEXT,

    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT now()
);

-- Indexes
CREATE INDEX idx_tokens_expires_at ON auth_tokens(expires_at);
CREATE INDEX idx_sessions_expires_at ON auth_sessions(expires_at);
