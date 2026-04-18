# AI 应用开发

本教程将详细说明如何基于 Aetheris 构建 AI 应用，包括 Agent 开发、Skill 开发、LLM 集成等。

## 概述

Aetheris 是一个 AI 原生、自我进化、分布式、完全可信的复杂任务执行引擎，采用 agentskills.io 行业标准。它提供了一个完整的框架，用于构建和部署 AI 应用。

## 系统架构

AI 应用开发系统由以下组件组成：

1. **Agent** - 领域专家，负责处理特定领域的任务
2. **Skill** - 功能单元，执行具体的任务
3. **LLM** - 语言模型，提供智能处理能力
4. **工具** - 外部工具，扩展系统功能
5. **通信总线** - Agent 之间的通信机制

## 开发步骤

### 步骤 1：配置 LLM

编辑 `config/llm.yaml` 文件，配置 LLM 提供商：

```yaml
# DeepSeek 配置
provider: deepseek
api_key: your-deepseek-api-key-here
api_base: https://api.deepseek.com/v1
model: deepseek-chat
temperature: 0.7
max_tokens: 2000
timeout_seconds: 30
```

### 步骤 2：创建自定义 Agent

#### 2.1 创建 Agent 配置文件

创建 `examples/agents/custom_agent.yaml` 文件：

```yaml
name: custom_agent
description: 自定义 Agent，用于特定领域的任务
author: Your Name
version: 1.0.0

# 核心配置
core:
  # 意图识别配置
  intent_recognition:
    enabled: true
    threshold: 0.7
  # 任务分解配置
  task_decomposition:
    enabled: true
    template: custom_task
  # 执行策略
  execution_strategy:
    type: sequential
    timeout_seconds: 3600

# 技能配置
skills:
  - name: your-custom-skill
    version: 1.0.0
    enabled: true

# 工具配置
tools:
  - name: your-custom-tool
    type: custom
    enabled: true

# 资源限制
resource_limits:
  max_concurrent_tasks: 10
  max_execution_time: 3600
  max_memory: 1024

# 安全配置
security:
  sandbox_enabled: true
  audit_logging: true
  rate_limiting: true
  rate_limit: 100

# 监控配置
monitoring:
  enabled: true
  metrics: true
  tracing: true
  alerts: true
```

### 步骤 3：创建自定义 Skill

#### 3.1 创建 Skill 目录

```bash
mkdir -p examples/agentskills/your-custom-skill
cd examples/agentskills/your-custom-skill
```

#### 3.2 创建 SKILL.md 文件

创建 `SKILL.md` 文件：

````yaml
---
name: your-custom-skill
description: Your custom skill description. Use when you need to do something specific.
version: 1.0.0
author: Your Name
license: Apache-2.0
tags: [custom, skill, ai]
compatibility: No special requirements
timeout: 30
allowed-tools: [LLM]
metadata:
  emoji: "🤖"
  retry_config:
    max_attempts: 1
    initial_delay_ms: 500
    max_delay_ms: 1000
    backoff_multiplier: 1
  sandbox_level: low
---

# Your Custom Skill

## 功能概述

描述您的 Skill 的功能和用途。

## 适用场景

说明您的 Skill 适用的场景。

## 输入规范

- `param1` (type, required): 描述
- `param2` (type, optional): 描述

## 执行流程

1. 接收输入参数
2. 验证输入参数
3. 执行逻辑
4. 返回结果

## 输出规范

- 成功返回：
  - `result`: 结果
- 失败返回：
  - `error`: 错误信息

## 约束与安全

说明您的 Skill 的约束和安全考虑。

## 示例

### 示例 1：基本使用

输入：
```json
{
  "param1": "value1",
  "param2": "value2"
}
````

输出：

```json
{
  "result": "success"
}
```

```
```

### 步骤 4：注册 Agent

编辑 `config/agent_registry.yaml` 文件，添加您的自定义 Agent：

```yaml
agents:
  - name: custom_agent
    path: examples/agents/custom_agent.yaml
    enabled: true
```

## 使用方法

### 通过 API 使用

```bash
curl -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "input": "执行自定义任务",
    "agent": "custom"
  }'
```

### 通过 IM 平台使用

在企业微信、钉钉或飞书中发送消息：

```
执行自定义任务
```

## 执行流程

1. **意图识别** - 识别用户的意图
2. **任务分解** - 将任务分解为子任务
3. **Agent 选择** - 选择合适的 Agent 执行任务
4. **Skill 执行** - 执行相应的 Skill
5. **结果汇总** - 汇总执行结果

## 输出结果

系统会生成以下输出：

1. **任务执行结果** - 任务的执行结果
2. **Agent 执行日志** - Agent 的执行日志
3. **Skill 执行详情** - Skill 的执行详情

## 监控和管理

### 查看任务状态

```bash
curl http://localhost:3000/api/v1/tasks/task-123
```

### 查看 Agent 状态

```bash
curl http://localhost:3000/api/v1/agents/custom_agent
```

### 查看 Skill 状态

```bash
curl http://localhost:3000/api/v1/skills/your-custom-skill
```

## 最佳实践

1. **遵循 agentskills.io 标准**：确保您的 Skill 符合 agentskills.io 行业标准
2. **模块化设计**：将复杂任务分解为多个简单的 Skill
3. **错误处理**：为您的 Skill 添加适当的错误处理
4. **性能优化**：优化 Skill 的执行时间和资源使用
5. **安全性**：确保您的 Skill 不会执行危险操作
6. **测试**：充分测试您的 Skill，确保它能够正常工作
7. **文档**：为您的 Skill 提供详细的文档

## 故障排除

### 常见问题

1. **Agent 执行失败**：检查 Agent 配置是否正确，确保技能和工具配置正确
2. **Skill 执行失败**：检查 Skill 配置是否正确，确保输入参数符合要求
3. **LLM 响应缓慢**：检查 LLM 配置是否正确，确保网络连接正常
4. **系统响应缓慢**：检查系统资源是否充足，考虑增加服务器资源

### 日志查看

查看 Aetheris 服务日志以获取更多信息：

```bash
# 开发模式运行时，日志会直接输出到终端
# 生产模式运行时，日志会输出到 logs/ 目录
```

## 下一步

- [Agent 协同](agent-coordination.md) - 了解如何实现多个 Agent 的协同工作
- [IM 平台集成](im-integration.md) - 了解如何集成企业微信、钉钉、飞书等 IM 平台
- [数据治理](data-governance.md) - 了解如何使用 Aetheris 的数据治理功能
- [API 文档](../api/README.md) - 了解完整的 API 参考
- [用户指南](../user-guide/README.md) - 了解更全面的使用指导

