CREATE TABLE IF NOT EXISTS mid_term_memory (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL,
    content JSONB NOT NULL,
    tags TEXT[],
    importance DOUBLE PRECISION DEFAULT 0.5,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mid_term_session_id ON mid_term_memory(session_id);
CREATE INDEX IF NOT EXISTS idx_mid_term_tags ON mid_term_memory USING GIN(tags);
CREATE INDEX IF NOT EXISTS idx_mid_term_created_at ON mid_term_memory(created_at);

CREATE TABLE IF NOT EXISTS task_chains (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL,
    task_chain JSONB NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_task_chains_session_id ON task_chains(session_id);
CREATE INDEX IF NOT EXISTS idx_task_chains_status ON task_chains(status);

CREATE TABLE IF NOT EXISTS long_term_memory (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    memory_type TEXT NOT NULL,
    content JSONB NOT NULL,
    embedding vector(1536),
    tags TEXT[],
    importance DOUBLE PRECISION DEFAULT 0.5,
    usage_count INTEGER DEFAULT 0,
    last_accessed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_long_term_type ON long_term_memory(memory_type);
CREATE INDEX IF NOT EXISTS idx_long_term_tags ON long_term_memory USING GIN(tags);
CREATE INDEX IF NOT EXISTS idx_long_term_created_at ON long_term_memory(created_at);
CREATE INDEX IF NOT EXISTS idx_long_term_importance ON long_term_memory(importance DESC);

CREATE TABLE IF NOT EXISTS state_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL,
    entity_type TEXT NOT NULL,
    version BIGINT NOT NULL,
    state JSONB NOT NULL,
    change_message TEXT,
    created_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(entity_id, version)
);

CREATE INDEX IF NOT EXISTS idx_state_versions_entity ON state_versions(entity_id, entity_type);
CREATE INDEX IF NOT EXISTS idx_state_versions_created_at ON state_versions(created_at);

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_mid_term_memory_updated_at
    BEFORE UPDATE ON mid_term_memory
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_task_chains_updated_at
    BEFORE UPDATE ON task_chains
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_long_term_memory_updated_at
    BEFORE UPDATE ON long_term_memory
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
