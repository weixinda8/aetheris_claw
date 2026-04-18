# 配置指南

本指南将帮助您配置 Aetheris，包括 LLM 提供商、Agent、Skill 和其他系统设置。

## 配置文件结构

Aetheris 的配置文件位于 `config/` 目录中：

- `llm.yaml` - LLM 提供商配置
- `database-pool.toml` - 数据库连接池配置
- `resource-limits.toml` - 资源限制配置
- `performance-monitoring.toml` - 性能监控配置

## LLM 配置

### DeepSeek 配置

```yaml
provider: deepseek
api_key: your-deepseek-api-key-here
api_base: https://api.deepseek.com/v1
model: deepseek-chat
temperature: 0.7
max_tokens: 2000
timeout_seconds: 30
```

### OpenAI 配置

```yaml
provider: openai
api_key: your-openai-api-key-here
api_base: https://api.openai.com/v1
model: gpt-4
 temperature: 0.7
max_tokens: 2000
timeout_seconds: 30
```

### Anthropic 配置

```yaml
provider: anthropic
api_key: your-anthropic-api-key-here
api_base: https://api.anthropic.com/v1
model: claude-3-opus-20240229
temperature: 0.7
max_tokens: 2000
timeout_seconds: 30
```

### Azure OpenAI 配置

```yaml
provider: azure
api_key: your-azure-api-key-here
api_base: https://your-resource-name.openai.azure.com/
model: gpt-4
 temperature: 0.7
max_tokens: 2000
timeout_seconds: 30
```

## Agent 配置

Agent 配置文件位于 `examples/agents/` 目录中，例如：

- `code_agent.yaml` - 代码 Agent 配置
- `data_agent.yaml` - 数据 Agent 配置
- `ops_agent.yaml` - 运维 Agent 配置
- `office_agent.yaml` - 办公 Agent 配置
- `industrial_agent.yaml` - 工业 Agent 配置

## Skill 配置

Skill 配置文件位于 `examples/agentskills/` 目录中，每个 Skill 都有自己的目录和 `SKILL.md` 文件。

## 环境变量

Aetheris 支持通过环境变量覆盖配置：

- `DEEPSEEK_API_KEY` - DeepSeek API Key
- `OPENAI_API_KEY` - OpenAI API Key
- `ANTHROPIC_API_KEY` - Anthropic API Key
- `AZURE_OPENAI_API_KEY` - Azure OpenAI API Key
- `AZURE_OPENAI_API_BASE` - Azure OpenAI API 基础 URL

## 高级配置

### 数据库配置

编辑 `config/database-pool.toml`：

```toml
[pool]
max_connections = 10
min_connections = 2
connect_timeout = 30
acquire_timeout = 30
max_lifetime = 86400
idle_timeout = 3600
```

### 资源限制配置

编辑 `config/resource-limits.toml`：

```toml
[limits]
max_concurrent_tasks = 100
max_task_execution_time = 3600
max_skill_execution_time = 600
max_memory_per_task = 1024
max_cpu_per_task = 100
```

## 验证配置

启动 Aetheris 服务后，您可以通过以下 API 端点检查配置：

```bash
curl http://localhost:3000/api/config
```

## 下一步

- [基本使用](basic-usage.md) - 了解如何使用 Aetheris 的基本功能
- [IM 平台集成](im-integration.md) - 了解如何集成 IM 平台
- [Agent 管理](agent-management.md) - 了解如何管理和配置 Agent
