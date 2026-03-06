-- ============================================================
-- FILE:        V1__cloud_schema.sql
-- MODULE:      Phase 5 — Cloud Backend > Database Schema
-- TASK:        T-060 (cloud DB scaffold)
-- PLATFORM:    PostgreSQL 16
-- AUTHOR:      FocusMe Co-Pilot (Claude Opus)
-- GENERATED:   Session 7
-- DEPENDENCIES: PostgreSQL 16+, pgcrypto (gen_random_uuid)
-- KNOWN LIMITATIONS: No row-level security yet. Soft-delete via deleted_at
--                    column — no automatic purge.
-- ============================================================

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ================================================================
-- Users
-- ================================================================
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,                  -- Argon2id (m=65536, t=3, p=4)
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users (email);

-- ================================================================
-- Refresh Tokens (for JWT rotation)
-- ================================================================
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,                     -- SHA-256 of refresh token
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens (user_id);
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens (token_hash);

-- ================================================================
-- Devices (for multi-device sync)
-- ================================================================
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_name TEXT NOT NULL,
    platform TEXT NOT NULL,                       -- 'windows' | 'macos' | 'linux' | 'android'
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_sync TIMESTAMPTZ,                        -- last successful sync timestamp
    push_token TEXT,                              -- for future push notifications
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_devices_user ON devices (user_id);

-- ================================================================
-- Cloud Plans (authoritative copy for sync)
-- ================================================================
CREATE TABLE cloud_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    local_id TEXT NOT NULL,                       -- matches SQLite plan ID on device
    plan_json JSONB NOT NULL,                     -- full plan (schema: policy_schema_v1.json)
    version INTEGER NOT NULL DEFAULT 1,           -- optimistic concurrency control
    deleted_at TIMESTAMPTZ,                       -- soft-delete (NULL = active)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, local_id)
);

CREATE INDEX idx_cloud_plans_user ON cloud_plans (user_id);
CREATE INDEX idx_cloud_plans_updated ON cloud_plans (updated_at);

-- ================================================================
-- Sync Events (audit trail for conflict resolution)
-- ================================================================
CREATE TABLE sync_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,                     -- 'plan_created' | 'plan_updated' | 'plan_deleted'
    local_id TEXT NOT NULL,                       -- plan local_id this event relates to
    payload JSONB NOT NULL,                       -- event-specific data
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sync_events_user ON sync_events (user_id);
CREATE INDEX idx_sync_events_synced ON sync_events (synced_at);

-- ================================================================
-- Family Groups
-- ================================================================
CREATE TABLE family_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL DEFAULT 'My Family',
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_family_groups_owner ON family_groups (owner_id);

-- ================================================================
-- Family Members
-- ================================================================
CREATE TABLE family_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id UUID NOT NULL REFERENCES family_groups(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    email TEXT NOT NULL,                          -- may differ from users.email during invite
    role TEXT NOT NULL DEFAULT 'member',          -- 'owner' | 'admin' | 'member' | 'child'
    invite_token TEXT,                            -- NULL after accepted
    invite_expires_at TIMESTAMPTZ,
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_family_members_family ON family_members (family_id);
CREATE INDEX idx_family_members_user ON family_members (user_id);
CREATE INDEX idx_family_members_invite ON family_members (invite_token);

-- ================================================================
-- Shared Plans (plans shared within a family)
-- ================================================================
CREATE TABLE shared_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id UUID NOT NULL REFERENCES family_groups(id) ON DELETE CASCADE,
    plan_id UUID NOT NULL REFERENCES cloud_plans(id) ON DELETE CASCADE,
    shared_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    shared_with UUID REFERENCES users(id) ON DELETE CASCADE,   -- NULL = shared with entire family
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_shared_plans_family ON shared_plans (family_id);
CREATE INDEX idx_shared_plans_target ON shared_plans (shared_with);

-- ================================================================
-- Updated-at trigger (auto-update on row modification)
-- ================================================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER cloud_plans_updated_at
    BEFORE UPDATE ON cloud_plans
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
