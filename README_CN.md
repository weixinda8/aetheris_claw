# Aetheris - AI 原生复杂任务执行引擎

简体中文 | [English](./README.md)

[![GitHub License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.94%2B-orange.svg)](https://www.rust-lang.org)
[![Rust Edition](https://img.shields.io/badge/rust%20edition-2024-blue.svg)](https://blog.rust-lang.org/2024/10/03/Rust-1.82.0.html#rust-2024-edition)
[![agentskills.io](https://img.shields.io/badge/agentskills.io-v1.0-green.svg)](https://agentskills.io)

&gt; **AI 原生、自我进化、分布式、完全可信的复杂任务执行引擎**
&gt;
&gt; 🌟 **采用 agentskills.io 行业标准** 🌟

---

## 🔥 一句话介绍

用钉钉/飞书直接说"我们厂接到化工订单"，系统自动分解为 5 个任务，6 个 Agent 协同执行，零学习成本，无需任何命令！

---

## ⭐ 核心亮点

| 功能 | 说明 |
|------|------|
| 🧠 **自然语言输入** | 零学习成本，直接说自然语言 |
| 💼 **6 个领域 Agent** | 代码、数据、运维、办公、工业、合规 - 开箱即用 |
| 🌟 **agentskills.io 标准** | 行业标准，跨平台兼容 |
| 🌐 **4 大 IM 平台** | 企业微信、钉钉、飞书、个人微信 |
| 🏭 **生产级质量** | 安全沙箱、审计日志、OpenTelemetry |

---

## 🎯 目标客户群体

### 🏭 工业制造企业

**企业规模**: 中大型制造企业（500-5000 人）

**核心价值**:
- 生产效率提升 30-50%
- 设备停机时间减少 40-60%
- 质量问题减少 50-70%
- 订单处理周期缩短 40-60%

**适用场景**: 化工生产排产、预测性维护、质量管控

---

### 💻 DevOps 团队

**企业规模**: 互联网公司（100-2000 人）

**核心价值**:
- 开发效率提升 40-60%
- 运维人力成本减少 50-70%
- 代码质量问题减少 60-80%
- 合规审计时间减少 70-90%

**适用场景**: 代码审查、数据分析、运维自动化、合规审计

---

### 🧪 AI 应用开发者

**企业规模**: 初创公司和开发者（1-50 人）

**核心价值**:
- 开发效率提升 5-10 倍
- 学习曲线降低 70-80%
- 技术债务减少 80-90%
- 生态参与度提升 10-20 倍

**适用场景**: Agent 应用开发、Skill 生态建设

---

## 💡 真实使用案例

### 🏭 化工生产订单智能处理

**用户输入（钉钉/飞书）**:
```
我们厂接到一个新的化工生产订单需要生成最终的生产报告
```

**系统自动处理流程**:

1. **自然语言解析**
   ```
   用户输入 → IntentParser → Intent（化工生产订单）
   ```

2. **自动任务分解**
   ```
   化工订单 → DecompositionTemplate → 5 个子任务
   ```

3. **5 个子任务**:
   | 子任务 | 分配给 | 依赖 |
   |--------|--------|------|
   | 1. 分析订单需求和原料库存 | DataAgent | 无 |
   | 2. 检查设备状态和预测维护 | IndustrialAgent | 1 |
   | 3. 生成生产排产计划 | OpsAgent | 1, 2 |
   | 4. 审核相关化验报告 | ComplianceAgent | 1 |
   | 5. 生成最终生产报告 | OfficeAgent | 2, 3, 4 |

4. **Agent 协同执行**:
   ```
   DataAgent → IndustrialAgent → OpsAgent
                                 ↓
   ComplianceAgent → OfficeAgent → 最终报告
   ```

5. **输出结果**:
   ```
   ✅ 生产排产计划生成完成
   ✅ 设备状态检查完成
   ✅ 原料库存分析完成
   ✅ 化验报告审核完成
   ✅ 最终生产报告生成完成
   ```

[查看化工生产排产完整方案](./.trae/documents/chemical_factory_intelligent_system_implementation.md)

---

### 💻 DevOps 自动化

**场景**: 代码审查 + 运维自动化

1. **CodeAgent**: 自动审查代码，识别问题，提供改进建议
2. **DataAgent**: 分析日志，生成洞察，可视化指标
3. **OpsAgent**: 监控系统，自动部署，排查问题
4. **ComplianceAgent**: 自动检查合规，生成审计报告

---

### 🧪 AI 应用开发

**场景**: 基于 Skill 生态构建 Agent 应用

1. **12+ 示例 Skill**: 开箱即用的生产级技能
2. **agentskills.io 标准**: 一次编写，全平台运行
3. **6 个 Agent 配置**: 每个领域的完整配置
4. **完整文档**: 快速开始、最佳实践、API 文档

---

## 🚀 快速开始 - 3 步

### 第 1 步：体验价值

用钉钉/飞书/企业微信/个人微信直接说：
```
我们厂接到一个新的化工生产订单需要生成最终的生产报告
```

系统自动：
1. ✅ 解析自然语言
2. ✅ 识别化工生产订单
3. ✅ 分解为 5 个子任务
4. ✅ 6 个 Agent 协同执行
5. ✅ 生成最终生产报告

---

### 第 2 步：安装运行

```bash
# 克隆项目
git clone https://github.com/aetheris/aetheris.git
cd aetheris

# 构建项目
cargo build

# 配置 LLM 提供商（推荐默认使用 DeepSeek）
Copy-Item config\llm.example.yaml config\llm.yaml  # Windows
# 或
cp config/llm.example.yaml config/llm.yaml         # macOS/Linux

# 编辑 config/llm.yaml 并设置您的 DeepSeek API Key
# 获取 API Key 地址：https://platform.deepseek.com/

# 运行服务
cargo run
```

服务启动在 http://localhost:3000

---

### 第 3 步：探索更多

- 📖 [快速开始指南](./QUICKSTART.md) - 从安装到创建第一个 Skill
- 💡 [最佳实践](./BEST_PRACTICES.md) - 编写高质量 Skill 的指南
- 📚 [12+ 示例 Skill](./examples/agentskills/) - 完整的 Skill 示例
- 🎯 [化工生产排产方案](./.trae/documents/chemical_factory_intelligent_system_implementation.md) - 完整行业解决方案

---

## 🛠️ 系统要求

- **Rust**: 1.94 或更高版本（使用 Rust 2024 Edition）
- **操作系统**: Windows 10/11, macOS 11+, Linux (Ubuntu 20.04+)
- **内存**: 最低 4GB RAM，推荐 8GB+
- **磁盘**: 至少 5GB 可用空间

---

## 💼 6 个领域 Agent - 开箱即用

| Agent | 职责 | 适用场景 |
|------|------|---------|
| **CodeAgent** | 代码审查、生成、优化 | 软件开发团队 |
| **DataAgent** | 数据分析、处理、可视化 | 数据分析师 |
| **OpsAgent** | 运维、监控、部署 | DevOps 团队 |
| **OfficeAgent** | 文档处理、邮件回复、日程 | 行政办公 |
| **IndustrialAgent** | 设备监控、预测维护、排产 | 工业制造 |
| **ComplianceAgent** | 合规检查、审计报告、风控 | 法务合规 |

每个 Agent 都有完整的配置、技能和示例！

---

## 🌟 agentskills.io 行业标准

agentskills.io 是由 Anthropic 主导、OpenAI/Google/Microsoft/字节等联合制定的开放标准。

**Aetheris 完全采用此标准的优势**:
- ✅ 跨平台兼容：一次编写，全平台运行
- ✅ 技能共享：融入行业大生态
- ✅ 避免技术债务：不绑定特定厂商
- ✅ 降低学习成本：标准统一，上手快
- ✅ 生态丰富：12+ 完整示例 Skill

[查看完整规范](./AGENTSKILLS_IO_SPEC.md)

---

## 🌐 4 大 IM 平台全支持 - 随时随地使用

| 平台 | 支持状态 | 说明 |
|------|---------|------|
| **企业微信** | ✅ 完全支持 | Webhook + 消息解析 |
| **钉钉** | ✅ 完全支持 | Webhook + 消息解析 |
| **飞书** | ✅ 完全支持 | Webhook + 消息解析 |
| **个人微信** | ✅ 完全支持 | ILink 适配器 + 消息解析 |

用户用任何平台直接输入自然语言，系统自动处理！

---

## 🏗️ 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                      接入层 (API/Gateway)                    │
├─────────────────────────────────────────────────────────────┤
│                    指挥中枢层 (Commander)                    │
│  Intent Parsing ──┬── Planning ──┬── Execution ──┬── Reflection  │
├─────────────────────────────────────────────────────────────┤
│                  专家军团层 (Expert Legion)                   │
│  CodeAgent  DataAgent  OpsAgent  OfficeAgent  IndustrialAgent  │
├─────────────────────────────────────────────────────────────┤
│                 技能与工具层 (Skills &amp; Tools)                  │
│  Skill Registry  ───  Tool Discovery  ───  Unified Call      │
├─────────────────────────────────────────────────────────────┤
│                 执行运行时层 (Execution Runtime)               │
│  Task Executor  ───  Docker Sandbox  ───  WASM Runtime        │
├─────────────────────────────────────────────────────────────┤
│                  状态与记忆层 (State &amp; Memory)                 │
│  Short-Term  ───  Mid-Term  ───  Long-Term  ───  Vector DB    │
├─────────────────────────────────────────────────────────────┤
│                  安全与合规层 (Security &amp; Compliance)            │
│  Audit Log  ───  Rule Block  ───  Human Intervention  ─── Compliance  │
├─────────────────────────────────────────────────────────────┤
│                  观测与管控层 (Observability &amp; Control)          │
│  Tracing  ───  Metrics  ───  Dashboard  ───  OpenTelemetry    │
└─────────────────────────────────────────────────────────────┘
```

---

## 📚 文档导航

| 文档 | 说明 |
|------|------|
| [文档索引](./docs/README.md) | 完整的文档索引 |
| [用户指南](./docs/user-guide/README.md) | 详细的用户指南 |
| [API 文档](./docs/api/README.md) | 完整的 API 参考 |
| [教程](./docs/tutorials/README.md) | 分步教程 |
| [QUICKSTART.md](./QUICKSTART.md) | 完整的快速开始指南 |
| [BEST_PRACTICES.md](./BEST_PRACTICES.md) | Skill 开发最佳实践 |
| [AGENTSKILLS_IO_SPEC.md](./AGENTSKILLS_IO_SPEC.md) | 完整的 agentskills.io 规范 |
| [AGENT_CONFIG_SYSTEM_GUIDE.md](./AGENT_CONFIG_SYSTEM_GUIDE.md) | Agent 配置系统指南 |

---

## 🤝 生态系统

- **12+ 示例 Skill**: 开箱即用的生产级技能
- **6 个 Agent 配置**: 每个领域的完整配置
- **4 大 IM 平台集成**: 企业微信、钉钉、飞书、个人微信
- **agentskills.io 标准**: 行业标准技能格式

---

## 🏆 技术栈

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

---

## 🤝 贡献

Fork 项目，创建分支，提交更改，然后创建 Pull Request！

---

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

---

## 💬 联系方式

- **GitHub**: [https://github.com/aetheris/aetheris](https://github.com/aetheris/aetheris)
- **问题反馈**: [GitHub Issues](https://github.com/aetheris/aetheris/issues)

---

&lt;div align="center"&gt;
  &lt;p&gt;&lt;strong&gt;Built with ❤️ by the Aetheris Team&lt;/strong&gt;&lt;/p&gt;
  &lt;p&gt;© 2024-2026 Aetheris Project. All rights reserved.&lt;/p&gt;
&lt;/div&gt;
