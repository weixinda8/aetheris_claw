# 技术架构

本文档详细说明 Aetheris 的系统架构，包括系统分层、核心组件和数据流。

## 系统分层

Aetheris 采用分层架构，从上到下依次为：

1. **接入层 (API/Gateway)** - 负责接收和处理外部请求
2. **指挥中枢层 (Commander)** - 负责意图解析、规划和执行
3. **专家军团层 (Expert Legion)** - 包含多个领域 Agent
4. **技能与工具层 (Skills & Tools)** - 提供各种技能和工具
5. **执行运行时层 (Execution Runtime)** - 负责任务执行
6. **状态与记忆层 (State & Memory)** - 管理系统状态和记忆
7. **安全与合规层 (Security & Compliance)** - 确保系统安全和合规
8. **观测与管控层 (Observability & Control)** - 监控和管理系统

## 核心组件

### 接入层

- **API Gateway** - 处理 HTTP/HTTPS 请求
- **WebSocket Server** - 处理 WebSocket 连接
- **IM 适配器** - 集成企业微信、钉钉、飞书等 IM 平台

### 指挥中枢层

- **Intent Parser** - 解析用户意图
- **Task Decomposer** - 分解任务为子任务
- **Planner** - 生成执行计划
- **Executor** - 执行任务
- **Reflection** - 反思和优化执行过程

### 专家军团层

- **CodeAgent** - 代码领域专家
- **DataAgent** - 数据领域专家
- **OpsAgent** - 运维领域专家
- **OfficeAgent** - 办公领域专家
- **IndustrialAgent** - 工业领域专家
- **ComplianceAgent** - 合规领域专家

### 技能与工具层

- **Skill Registry** - 管理技能
- **Tool Discovery** - 发现和管理工具
- **Unified Call** - 统一调用接口

### 执行运行时层

- **Task Executor** - 执行任务
- **Docker Sandbox** - 提供安全的执行环境
- **WASM Runtime** - 执行 WASM 代码

### 状态与记忆层

- **Short-Term Memory** - 短期记忆
- **Mid-Term Memory** - 中期记忆
- **Long-Term Memory** - 长期记忆
- **Vector DB** - 向量数据库

### 安全与合规层

- **Audit Log** - 审计日志
- **Rule Block** - 规则阻断
- **Human Intervention** - 人工干预
- **Compliance** - 合规检查

### 观测与管控层

- **Tracing** - 分布式追踪
- **Metrics** - 指标收集
- **Dashboard** - 仪表板
- **OpenTelemetry** - 开放遥测

## 数据流

1. **输入处理** - 接入层接收用户输入
2. **意图解析** - 指挥中枢层解析用户意图
3. **任务分解** - 指挥中枢层分解任务为子任务
4. **Agent 选择** - 指挥中枢层选择合适的 Agent
5. **Skill 执行** - Agent 执行相应的 Skill
6. **结果汇总** - 指挥中枢层汇总执行结果
7. **输出反馈** - 接入层返回结果给用户

## 通信机制

- **Agent Communication Bus** - Agent 之间的通信总线
- **Broadcast** - 广播通信
- **Point-to-Point** - 点对点通信
- **Queue** - 队列通信
- **Protocol** - 通信协议
- **Reliability** - 通信可靠性保障

## 扩展性

Aetheris 设计为高度可扩展的系统：

- **插件架构** - 支持插件扩展
- **Skill 生态** - 支持自定义 Skill
- **Agent 扩展** - 支持自定义 Agent
- **LLM 集成** - 支持多种 LLM 提供商
- **工具集成** - 支持自定义工具

## 安全性

Aetheris 采用多层次的安全措施：

- **Sandbox** - 技能执行沙箱
- **Audit Logging** - 审计日志
- **Rate Limiting** - 速率限制
- **Human Intervention** - 人工干预
- **Compliance Check** - 合规检查

## 可观测性

Aetheris 提供全面的可观测性：

- **Metrics** - 系统指标
- **Tracing** - 分布式追踪
- **Logging** - 日志记录
- **Alerts** - 告警系统
- **Dashboard** - 仪表板

## 部署架构

Aetheris 支持多种部署方式：

- **单机部署** - 适用于开发和测试
- **容器部署** - 使用 Docker 容器
- **Kubernetes 部署** - 适用于生产环境
- **云部署** - 部署到云平台

## 技术栈

- **LLM 提供商**: DeepSeek（默认推荐）、OpenAI、Anthropic、Azure OpenAI
- **Web 框架**: Axum 0.8.4
- **异步运行时**: Tokio 1.47.1
- **gRPC**: Tonic 0.12.3
- **数据库**: SQLx 0.8.6 (PostgreSQL)
- **向量数据库**: Qdrant Client 1.15.0
- **WASM 运行时**: Wasmtime 36.0.0
- **脚本语言**: Rhai 1.22.2
- **图算法**: Petgraph 0.8.0
- **日志**: Tracing 0.1.44
- **OpenTelemetry**: 0.29
- **容器化**: Bollard 0.19 (Docker)
- **Skill 标准**: agentskills.io v1.0

## 下一步

- [用户指南](../user-guide/README.md) - 了解如何使用 Aetheris
- [API 文档](../api/README.md) - 了解完整的 API 参考
- [教程](../tutorials/README.md) - 了解常见用例和示例教程
- [Skill 开发最佳实践](skill-best-practices.md) - 了解如何开发高质量的 Skill
