-- 为 long_term_memory 表添加内容全文搜索索引
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- 为 content 字段添加全文搜索索引，提高模糊查询性能
CREATE INDEX IF NOT EXISTS idx_long_term_content_trgm ON long_term_memory USING gin (content::text gin_trgm_ops);

-- 为 usage_count 和 last_accessed_at 添加复合索引，提高排序性能
CREATE INDEX IF NOT EXISTS idx_long_term_usage_accessed ON long_term_memory(usage_count DESC, last_accessed_at DESC NULLS LAST);

-- 为 mid_term_memory 表添加复合索引，提高按 session_id 和 created_at 查询的性能
CREATE INDEX IF NOT EXISTS idx_mid_term_session_created ON mid_term_memory(session_id, created_at DESC);

-- 为 task_chains 表添加复合索引，提高按 session_id 和 status 查询的性能
CREATE INDEX IF NOT EXISTS idx_task_chains_session_status ON task_chains(session_id, status);

-- 为 state_versions 表添加版本号索引，提高版本查询性能
CREATE INDEX IF NOT EXISTS idx_state_versions_version ON state_versions(entity_id, version DESC);

-- 优化 long_term_memory 表的查询性能，添加 memory_type 和 importance 的复合索引
CREATE INDEX IF NOT EXISTS idx_long_term_type_importance ON long_term_memory(memory_type, importance DESC);
