# Agent 管理

本指南将帮助您了解如何管理和配置 Aetheris 中的 Agent，包括 6 个默认领域 Agent 和自定义 Agent。

## 默认 Agent

Aetheris 提供 6 个开箱即用的领域 Agent：

| Agent | 职责 | 适用场景 |
|-------|------|----------|
| **CodeAgent** | 代码审查、生成、优化 | 软件开发团队 |
| **DataAgent** | 数据分析、处理、可视化 | 数据分析师 |
| **OpsAgent** | 运维、监控、部署 | DevOps 团队 |
| **OfficeAgent** | 文档处理、邮件回复、日程 | 行政办公 |
| **IndustrialAgent** | 设备监控、预测维护、排产 | 工业制造 |
| **ComplianceAgent** | 合规检查、审计报告、风控 | 法务合规 |

## Agent 配置

Agent 配置文件位于 `examples/agents/` 目录中，每个 Agent 都有自己的配置文件：

- `code_agent.yaml` - 代码 Agent 配置
- `data_agent.yaml` - 数据 Agent 配置
- `ops_agent.yaml` - 运维 Agent 配置
- `office_agent.yaml` - 办公 Agent 配置
- `industrial_agent.yaml` - 工业 Agent 配置
- `chemical_factory_scheduling_agent.yaml` - 化工生产排产 Agent 配置

### 配置示例

以下是一个 Agent 配置示例：

```yaml
name: industrial_agent
description: 工业制造领域 Agent，负责设备监控、预测维护和生产排产
author: Aetheris Team
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
    template: chemical_production_order
  # 执行策略
  execution_strategy:
    type: sequential
    timeout_seconds: 3600

# 技能配置
skills:
  - name: predictive-maintenance
    version: 1.0.0
    enabled: true
  - name: production-monitoring
    version: 1.0.0
    enabled: true
  - name: lab-report-audit
    version: 1.0.0
    enabled: true

# 工具配置
tools:
  - name: industrial-connector
    type: modbus
    enabled: true
  - name: data-visualization
    type: dashboard
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

## 创建自定义 Agent

### 步骤 1：创建 Agent 配置文件

创建一个新的 YAML 配置文件，例如 `custom_agent.yaml`，放在 `examples/agents/` 目录中。

### 步骤 2：配置 Agent

根据您的需求配置 Agent，包括核心配置、技能配置、工具配置等。

### 步骤 3：注册 Agent

编辑 `config/agent_registry.yaml` 文件，添加您的自定义 Agent：

```yaml
agents:
  - name: custom_agent
    path: examples/agents/custom_agent.yaml
    enabled: true
```

### 步骤 4：重启服务

重启 Aetheris 服务以加载新的 Agent 配置：

```bash
cargo run
```

## Agent 管理 API

### 列出所有 Agent

```bash
curl http://localhost:3000/api/agents
```

响应：

```json
[
  {
    "name": "code_agent",
    "description": "代码领域 Agent，负责代码审查、生成和优化",
    "status": "active"
  },
  {
    "name": "data_agent",
    "description": "数据领域 Agent，负责数据分析、处理和可视化",
    "status": "active"
  }
]
```

### 获取 Agent 详情

```bash
curl http://localhost:3000/api/agents/industrial_agent
```

响应：

```json
{
  "name": "industrial_agent",
  "description": "工业制造领域 Agent，负责设备监控、预测维护和生产排产",
  "status": "active",
  "skills": [
    "predictive-maintenance",
    "production-monitoring",
    "lab-report-audit"
  ],
  "tools": [
    "industrial-connector",
    "data-visualization"
  ]
}
```

## Agent 最佳实践

### 1. 选择合适的 Agent

根据任务类型选择合适的 Agent：
- 代码相关任务：CodeAgent
- 数据分析任务：DataAgent
- 运维相关任务：OpsAgent
- 文档处理任务：OfficeAgent
- 工业制造任务：IndustrialAgent
- 合规审计任务：ComplianceAgent

### 2. 配置合适的技能

为 Agent 配置合适的技能，确保 Agent 能够完成所需的任务。

### 3. 监控 Agent 性能

定期监控 Agent 的性能，包括执行时间、成功率、资源使用情况等。

### 4. 优化 Agent 配置

根据实际使用情况，优化 Agent 的配置，包括执行策略、资源限制等。

## 故障排除

### 常见问题

1. **Agent 执行失败**：检查 Agent 配置是否正确，确保技能和工具配置正确。
2. **Agent 响应缓慢**：检查 LLM 配置是否正确，确保网络连接正常。
3. **Agent 无法识别意图**：检查意图识别配置，调整阈值参数。

### 日志查看

查看 Aetheris 服务日志以获取更多信息：

```bash
# 开发模式运行时，日志会直接输出到终端
# 生产模式运行时，日志会输出到 logs/ 目录
```

## 下一步

- [Skill 管理](skill-management.md) - 了解如何管理和使用 Skill
- [故障排除](troubleshooting.md) - 了解常见问题和解决方案
- [API 文档](../api/README.md) - 了解完整的 API 参考
