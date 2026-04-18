# 化工生产排产

本教程将详细说明如何使用 Aetheris 实现化工生产排产的完整解决方案。

## 概述

化工生产排产是一个复杂的任务，需要考虑多种因素，如原料库存、设备状态、生产能力等。Aetheris 通过多个 Agent 的协同工作，可以自动完成这一复杂任务。

## 系统架构

化工生产排产系统由以下组件组成：

1. **DataAgent** - 分析订单需求和原料库存
2. **IndustrialAgent** - 检查设备状态和预测维护
3. **OpsAgent** - 生成生产排产计划
4. **ComplianceAgent** - 审核相关化验报告
5. **OfficeAgent** - 生成最终生产报告

## 配置步骤

### 步骤 1：配置 IndustrialAgent

编辑 `examples/agents/industrial_agent.yaml` 文件：

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

### 步骤 2：配置其他 Agent

确保 DataAgent、OpsAgent、ComplianceAgent 和 OfficeAgent 也已正确配置。

### 步骤 3：配置任务分解模板

编辑 `config/decomposition_templates/chemical_production_order.yaml` 文件：

```yaml
name: chemical_production_order
description: 化工生产订单任务分解模板
author: Aetheris Team
version: 1.0.0

# 任务分解规则
tasks:
  - id: analyze_order
    name: 分析订单需求和原料库存
    agent: data_agent
    dependencies: []
    skills:
      - data-analysis
  - id: check_equipment
    name: 检查设备状态和预测维护
    agent: industrial_agent
    dependencies: [analyze_order]
    skills:
      - predictive-maintenance
  - id: generate_schedule
    name: 生成生产排产计划
    agent: ops_agent
    dependencies: [analyze_order, check_equipment]
    skills:
      - production-scheduling
  - id: audit_reports
    name: 审核相关化验报告
    agent: compliance_agent
    dependencies: [analyze_order]
    skills:
      - lab-report-audit
  - id: generate_report
    name: 生成最终生产报告
    agent: office_agent
    dependencies: [check_equipment, generate_schedule, audit_reports]
    skills:
      - report-generation
```

## 使用方法

### 通过 IM 平台使用

在企业微信、钉钉或飞书中发送消息：

```
我们厂接到一个新的化工生产订单需要生成最终的生产报告
```

系统会自动：
1. 解析自然语言
2. 识别化工生产订单
3. 分解为 5 个子任务
4. 5 个 Agent 协同执行
5. 生成最终生产报告

### 通过 API 使用

```bash
curl -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "agent": "industrial"
  }'
```

## 执行流程

1. **DataAgent** 分析订单需求和原料库存
2. **IndustrialAgent** 检查设备状态和预测维护
3. **OpsAgent** 生成生产排产计划
4. **ComplianceAgent** 审核相关化验报告
5. **OfficeAgent** 生成最终生产报告

## 输出结果

系统会生成以下输出：

1. **生产排产计划** - 详细的生产排产计划
2. **设备状态报告** - 设备状态检查结果和预测维护建议
3. **原料库存分析** - 原料库存分析结果
4. **化验报告审核** - 化验报告审核结果
5. **最终生产报告** - 包含所有信息的最终生产报告

## 监控和管理

### 查看任务状态

```bash
curl http://localhost:3000/api/v1/tasks/task-123
```

### 查看审计日志

```bash
curl http://localhost:3000/api/v1/audit/events/task-123
```

### 查看系统指标

```bash
curl http://localhost:3000/api/v1/telemetry/metrics
```

## 最佳实践

1. **定期更新设备状态**：确保设备状态数据及时更新，以便 IndustrialAgent 能够做出准确的预测
2. **维护原料库存数据**：确保原料库存数据准确，以便 DataAgent 能够做出正确的分析
3. **定期审核化验报告**：确保化验报告及时审核，以便 ComplianceAgent 能够做出正确的判断
4. **优化生产排产算法**：根据实际情况优化生产排产算法，提高生产效率
5. **监控系统性能**：定期监控系统性能，确保系统能够正常运行

## 故障排除

### 常见问题

1. **任务执行失败**：检查 LLM 配置是否正确，确保 API Key 有效
2. **排产计划不合理**：检查设备状态和原料库存数据是否准确
3. **系统响应缓慢**：检查系统资源是否充足，考虑增加服务器资源

### 日志查看

查看 Aetheris 服务日志以获取更多信息：

```bash
# 开发模式运行时，日志会直接输出到终端
# 生产模式运行时，日志会输出到 logs/ 目录
```

## 下一步

- [DevOps 自动化](devops-automation.md) - 了解如何实现 DevOps 自动化
- [AI 应用开发](ai-application-development.md) - 了解如何基于 Aetheris 构建 AI 应用
- [Agent 协同](agent-coordination.md) - 了解如何实现多个 Agent 的协同工作
- [API 文档](../api/README.md) - 了解完整的 API 参考
- [用户指南](../user-guide/README.md) - 了解更全面的使用指导
