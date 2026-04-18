-- Add audit tables for Skill Hub
-- Version 002

-- Skill audit records table
CREATE TABLE IF NOT EXISTS skill_audit_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    skill_id UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    stage VARCHAR(50) NOT NULL,
    reviewer_id UUID REFERENCES users(id),
    status VARCHAR(50) NOT NULL DEFAULT 'in_progress',
    comments TEXT,
    findings JSONB,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for audit records
CREATE INDEX IF NOT EXISTS idx_skill_audit_records_skill_id ON skill_audit_records(skill_id);
CREATE INDEX IF NOT EXISTS idx_skill_audit_records_stage ON skill_audit_records(stage);
CREATE INDEX IF NOT EXISTS idx_skill_audit_records_status ON skill_audit_records(status);
CREATE INDEX IF NOT EXISTS idx_skill_audit_records_started_at ON skill_audit_records(started_at DESC);

-- Create updated_at trigger for skill_audit_records
CREATE TRIGGER update_skill_audit_records_updated_at BEFORE UPDATE ON skill_audit_records
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
