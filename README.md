# Aetheris - AI-Native Complex Task Execution Engine

[简体中文](./README_CN.md) | English

[![GitHub License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.94%2B-orange.svg)](https://www.rust-lang.org)
[![Rust Edition](https://img.shields.io/badge/rust%20edition-2024-blue.svg)](https://blog.rust-lang.org/2024/10/03/Rust-1.82.0.html#rust-2024-edition)
[![agentskills.io](https://img.shields.io/badge/agentskills.io-v1.0-green.svg)](https://agentskills.io)

&gt; **AI-native, self-evolving, distributed, fully trusted complex task execution engine**
&gt;
&gt; 🌟 **Built with agentskills.io Industry Standard** 🌟

---

## 🔥 One-Line Pitch

用钉钉/飞书直接说"我们厂接到化工订单"，系统自动分解为 5 个任务，6 个 Agent 协同执行，零学习成本，无需任何命令！

---

## ⭐ Core Highlights

| Feature | Description |
|---------|-------------|
| 🧠 **Natural Language Input** | Zero learning curve, just talk naturally |
| 💼 **6 Domain Agents** | Code, Data, Ops, Office, Industrial, Compliance - out of the box |
| 🌟 **agentskills.io Standard** | Industry standard, cross-platform compatible |
| 🌐 **4 IM Platforms** | WeChat Work, DingTalk, FeiShu, Personal WeChat |
| 🏭 **Production-Grade** | Security sandboxes, audit logs, OpenTelemetry |

---

## 🎯 Who Is This For?

### 🏭 Industrial Manufacturing Enterprises

**Company Size**: Mid-to-large manufacturing (500-5000 employees)

**Core Value**:
- 30-50% production efficiency improvement
- 40-60% reduction in equipment downtime
- 50-70% fewer quality issues
- 40-60% shorter order processing cycles

**Use Cases**: Chemical production scheduling, predictive maintenance, quality control

---

### 💻 DevOps Teams

**Company Size**: Tech companies (100-2000 employees)

**Core Value**:
- 40-60% development efficiency improvement
- 50-70% reduction in operations costs
- 60-80% fewer code quality issues
- 70-90% less time on compliance audits

**Use Cases**: Code review, data analysis, operations automation, compliance audits

---

### 🧪 AI Application Developers

**Company Size**: Startups and developers (1-50 people)

**Core Value**:
- 5-10x faster development
- 70-80% lower learning curve
- 80-90% less technical debt
- 10-20x better ecosystem integration

**Use Cases**: Agent application development, Skill ecosystem building

---

## 💡 Real-World Use Cases

### 🏭 Chemical Production Order Intelligent Processing

**User Input (DingTalk/FeiShu)**:
```
我们厂接到一个新的化工生产订单需要生成最终的生产报告
```

**Automatic Processing Flow**:

1. **Natural Language Parsing**
   ```
   User Input → IntentParser → Intent (Chemical Production Order)
   ```

2. **Automatic Task Decomposition**
   ```
   Chemical Order → DecompositionTemplate → 5 Sub-Tasks
   ```

3. **5 Sub-Tasks**:
   | Sub-Task | Assigned To | Dependencies |
   |----------|-------------|--------------|
   | 1. Analyze order requirements and raw material inventory | DataAgent | None |
   | 2. Check equipment status and predictive maintenance | IndustrialAgent | 1 |
   | 3. Generate production scheduling plan | OpsAgent | 1, 2 |
   | 4. Audit relevant lab reports | ComplianceAgent | 1 |
   | 5. Generate final production report | OfficeAgent | 2, 3, 4 |

4. **Agent Coordination**:
   ```
   DataAgent → IndustrialAgent → OpsAgent
                                 ↓
   ComplianceAgent → OfficeAgent → Final Report
   ```

5. **Output Results**:
   ```
   ✅ Production scheduling plan generated
   ✅ Equipment status check completed
   ✅ Raw material inventory analyzed
   ✅ Lab reports audited
   ✅ Final production report generated
   ```

[View Complete Chemical Factory Solution](./.trae/documents/chemical_factory_intelligent_system_implementation.md)

---

### 💻 DevOps Automation

**Scenario**: Code review + operations automation

1. **CodeAgent**: Auto-review code, identify issues, suggest improvements
2. **DataAgent**: Analyze logs, generate insights, visualize metrics
3. **OpsAgent**: Monitor systems, auto-deploy, troubleshoot issues
4. **ComplianceAgent**: Auto-check compliance, generate audit reports

---

### 🧪 AI Application Development

**Scenario**: Build Agent applications with the Skill ecosystem

1. **12+ Example Skills**: Ready-to-use, production-grade skills
2. **agentskills.io Standard**: Write once, run everywhere
3. **6 Agent Configurations**: Complete configurations for each domain
4. **Complete Documentation**: Quick start, best practices, API docs

---

## 🚀 Quick Start - 3 Steps

### Step 1: See the Value

Just say in DingTalk/FeiShu/WeChat:
```
我们厂接到一个新的化工生产订单需要生成最终的生产报告
```

The system automatically:
1. ✅ Parses natural language
2. ✅ Identifies chemical production order
3. ✅ Decomposes into 5 sub-tasks
4. ✅ 6 Agents collaborate
5. ✅ Generates final production report

---

### Step 2: Install and Run

```bash
# Clone the repo
git clone https://github.com/aetheris/aetheris.git
cd aetheris

# Build the project
cargo build

# Configure LLM provider (DeepSeek is recommended as default)
Copy-Item config\llm.example.yaml config\llm.yaml  # Windows
# or
cp config/llm.example.yaml config/llm.yaml         # macOS/Linux

# Edit config/llm.yaml and set your DeepSeek API Key
# Get your API Key from: https://platform.deepseek.com/

# Run the service
cargo run
```

Service starts at http://localhost:3000

---

### Step 3: Explore More

- 📖 [Quick Start Guide](./QUICKSTART.md) - From installation to your first Skill
- 💡 [Best Practices](./BEST_PRACTICES.md) - Guide to writing high-quality Skills
- 📚 [12+ Example Skills](./examples/agentskills/) - Complete Skill examples
- 🎯 [Chemical Production Scheduling](./.trae/documents/chemical_factory_intelligent_system_implementation.md) - Complete industry solution

---

## 🛠️ System Requirements

- **Rust**: 1.94 or higher (Rust 2024 Edition)
- **OS**: Windows 10/11, macOS 11+, Linux (Ubuntu 20.04+)
- **Memory**: 4GB RAM minimum, 8GB+ recommended
- **Disk**: At least 5GB free space

---

## 💼 6 Domain Agents - Out of the Box

| Agent | Responsibilities | Use Cases |
|-------|------------------|-----------|
| **CodeAgent** | Code review, generation, optimization | Software development teams |
| **DataAgent** | Data analysis, processing, visualization | Data analysts |
| **OpsAgent** | Operations, monitoring, deployment | DevOps teams |
| **OfficeAgent** | Document processing, email replies, scheduling | Administrative work |
| **IndustrialAgent** | Equipment monitoring, predictive maintenance, scheduling | Industrial manufacturing |
| **ComplianceAgent** | Compliance checks, audit reports, risk control | Legal &amp; compliance |

Each Agent comes with complete configurations, skills, and examples!

---

## 🌟 agentskills.io Industry Standard

agentskills.io is an open standard led by Anthropic, with participation from OpenAI, Google, Microsoft, ByteDance, and others.

**Aetheris fully adopts this standard for**:
- ✅ Cross-platform compatibility: Write once, run everywhere
- ✅ Skill sharing: Integrate with the broader industry ecosystem
- ✅ Avoid technical debt: No vendor lock-in
- ✅ Lower learning curve: Unified standard, quick to adopt
- ✅ Rich ecosystem: 12+ complete example Skills

[View Full Specification](./AGENTSKILLS_IO_SPEC.md)

---

## 🌐 4 IM Platforms - Use Anywhere

| Platform | Support Status | Description |
|----------|---------------|-------------|
| **WeChat Work** | ✅ Full Support | Webhook + Message Parsing |
| **DingTalk** | ✅ Full Support | Webhook + Message Parsing |
| **FeiShu** | ✅ Full Support | Webhook + Message Parsing |
| **Personal WeChat** | ✅ Full Support | ILink Adapter + Message Parsing |

Users can input natural language on any platform, and the system processes it automatically!

---

## 🏗️ System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   Access Layer (API/Gateway)                  │
├─────────────────────────────────────────────────────────────┤
│                 Commander Layer (Commander)                   │
│  Intent Parsing ──┬── Planning ──┬── Execution ──┬── Reflection  │
├─────────────────────────────────────────────────────────────┤
│               Expert Legion Layer (Expert Legion)              │
│  CodeAgent  DataAgent  OpsAgent  OfficeAgent  IndustrialAgent  │
├─────────────────────────────────────────────────────────────┤
│              Skills &amp; Tools Layer (Skills &amp; Tools)             │
│  Skill Registry  ───  Tool Discovery  ───  Unified Call      │
├─────────────────────────────────────────────────────────────┤
│             Execution Runtime Layer (Execution Runtime)         │
│  Task Executor  ───  Docker Sandbox  ───  WASM Runtime        │
├─────────────────────────────────────────────────────────────┤
│              State &amp; Memory Layer (State &amp; Memory)              │
│  Short-Term  ───  Mid-Term  ───  Long-Term  ───  Vector DB    │
├─────────────────────────────────────────────────────────────┤
│            Security &amp; Compliance Layer (Security &amp; Compliance)      │
│  Audit Log  ───  Rule Block  ───  Human Intervention  ─── Compliance  │
├─────────────────────────────────────────────────────────────┤
│          Observability &amp; Control Layer (Observability &amp; Control)    │
│  Tracing  ───  Metrics  ───  Dashboard  ───  OpenTelemetry    │
└─────────────────────────────────────────────────────────────┘
```

---

## 📚 Documentation Navigation

| Document | Description |
|----------|-------------|
| [Documentation Index](./docs/README.md) | Complete documentation index |
| [User Guide](./docs/user-guide/README.md) | Detailed user guide |
| [API Documentation](./docs/api/README.md) | Complete API reference |
| [Tutorials](./docs/tutorials/README.md) | Step-by-step tutorials |
| [QUICKSTART.md](./QUICKSTART.md) | Complete quick start guide |
| [BEST_PRACTICES.md](./BEST_PRACTICES.md) | Skill development best practices |
| [AGENTSKILLS_IO_SPEC.md](./AGENTSKILLS_IO_SPEC.md) | Complete agentskills.io specification |
| [AGENT_CONFIG_SYSTEM_GUIDE.md](./AGENT_CONFIG_SYSTEM_GUIDE.md) | Agent configuration guide |

---

## 🤝 Ecosystem

- **12+ Example Skills**: Ready-to-use, production-grade skills
- **6 Agent Configurations**: Complete configurations for each domain
- **4 IM Platform Integrations**: WeChat Work, DingTalk, FeiShu, Personal WeChat
- **agentskills.io Standard**: Industry-standard skill format

---

## 🏆 Tech Stack

- **LLM Provider**: DeepSeek (default recommended), OpenAI, Anthropic, Azure OpenAI
- **Web Framework**: Axum 0.8.4
- **Async Runtime**: Tokio 1.47.1
- **gRPC**: Tonic 0.12.3
- **Database**: SQLx 0.8.6 (PostgreSQL)
- **Vector Database**: Qdrant Client 1.15.0
- **WASM Runtime**: Wasmtime 36.0.0
- **Scripting Language**: Rhai 1.22.2
- **Graph Algorithms**: Petgraph 0.8.0
- **Logging**: Tracing 0.1.44
- **OpenTelemetry**: 0.29
- **Containerization**: Bollard 0.19 (Docker)
- **Skill Standard**: agentskills.io v1.0

---

## 🤝 Contributing

Fork the project, create a branch, commit your changes, and create a Pull Request!

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 💬 Contact

- **GitHub**: [https://github.com/aetheris/aetheris](https://github.com/aetheris/aetheris)
- **Issues**: [GitHub Issues](https://github.com/aetheris/aetheris/issues)

---

&lt;div align="center"&gt;
  &lt;p&gt;&lt;strong&gt;Built with ❤️ by the Aetheris Team&lt;/strong&gt;&lt;/p&gt;
  &lt;p&gt;© 2024-2026 Aetheris Project. All rights reserved.&lt;/p&gt;
&lt;/div&gt;
