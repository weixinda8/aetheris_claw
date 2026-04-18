---
name: quickstart-guide
description: Aetheris 快速开始指南 - 从安装到创建第一个符合 agentskills.io 标准的技能
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [quickstart, guide, tutorial, agentskills, skill]
compatibility: Aetheris 1.0+
---

# Aetheris 快速开始指南

> **采用 agentskills.io 行业标准**

本指南将帮助您快速上手 Aetheris，从安装到创建第一个符合 agentskills.io 标准的技能。

## 目录

1. [系统要求](#系统要求)
2. [安装 Aetheris](#安装-aetheris)
3. [运行示例](#运行示例)
4. [创建您的第一个 Skill](#创建您的第一个-skill)
5. [在 Aetheris 中使用 Skill](#在-aetheris-中使用-skill)
6. [下一步](#下一步)

## 系统要求

- **Rust**: 1.94 或更高版本（使用 Rust 2024 Edition）
- **操作系统**: Windows 10/11, macOS 11+, Linux (Ubuntu 20.04+)
- **内存**: 最低 4GB RAM，推荐 8GB+
- **磁盘**: 至少 5GB 可用空间

## 安装 Aetheris

### 1. 安装 Rust

如果您尚未安装 Rust，请访问 [rustup.rs](https://rustup.rs/) 安装：

```bash
# Windows (PowerShell)
Invoke-RestMethod -Uri https://win.rustup.rs/x86_64 | Invoke-Expression

# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 克隆项目

```bash
git clone https://github.com/aetheris/aetheris.git
cd aetheris
```

### 3. 构建项目

```bash
# 开发模式
cargo build

# 生产模式
cargo build --release
```

### 4. 配置 LLM 提供商

复制配置文件示例（DeepSeek 为默认推荐）：

```bash
# Windows (PowerShell)
Copy-Item config\llm.example.yaml config\llm.yaml

# macOS / Linux
cp config/llm.example.yaml config/llm.yaml
```

编辑 `config/llm.yaml`，设置您的 DeepSeek API Key：

```yaml
provider: deepseek
api_key: your-deepseek-api-key-here
api_base: https://api.deepseek.com/v1
model: deepseek-chat
temperature: 0.7
max_tokens: 2000
timeout_seconds: 30
```

**获取 DeepSeek API Key**：访问 https://platform.deepseek.com/ 注册并获取 API Key

**DeepSeek 可用模型**：
- `deepseek-chat` - 通用对话模型（默认）
- `deepseek-coder` - 代码专用模型

## 运行示例

### 查看示例 Skills

Aetheris 包含 12 个符合 agentskills.io 标准的示例 Skill：

```bash
ls examples/agentskills/
```

示例列表：
- `chemical-reagent-manage` - 化学试剂管理
- `code-generation` - 代码生成
- `data-analysis` - 数据分析
- `database-query` - 数据库查询
- `email-composer` - 邮件撰写
- `file-operations` - 文件操作
- `lab-report-audit` - 实验报告审核
- `meeting-assistant` - 会议助手
- `predictive-maintenance` - 预测性维护
- `production-monitoring` - 生产监控
- `report-generation` - 报告生成
- `web-search` - 网络搜索

### 运行 Aetheris

```bash
# 开发模式
cargo run

# 生产模式
cargo run --release
```

服务将在 `http://localhost:3000` 启动。

## 创建您的第一个 Skill

让我们创建一个简单的 "Hello World" Skill，它会根据用户输入返回个性化问候。

### 步骤 1：创建 Skill 目录

```bash
mkdir -p examples/agentskills/hello-world
cd examples/agentskills/hello-world
```

### 步骤 2：创建 SKILL.md 文件

创建 `SKILL.md` 文件，内容如下：

```yaml
---
name: hello-world
description: Generate personalized greetings based on user input. Use when greeting users, welcoming new members, or creating friendly messages.
version: 1.0.0
author: Your Name
license: Apache-2.0
tags: [greeting, hello, welcome, message, friendly]
compatibility: No special requirements
timeout: 30
allowed-tools: [LLM]
metadata:
  emoji: "👋"
  retry_config:
    max_attempts: 1
    initial_delay_ms: 500
    max_delay_ms: 1000
    backoff_multiplier: 1
  sandbox_level: low
---

# Hello World Skill

## 功能概述

根据用户输入生成个性化问候语。支持多种语言和风格。

## 适用场景

- 问候用户
- 欢迎新成员
- 创建友好消息
- 客户服务开场
- 社区互动

## 输入规范

- `name` (string, required): 用户名称
- `language` (string, optional): 语言，可选值：zh, en, ja, ko，默认 zh
- `style` (string, optional): 风格，可选值：formal, casual, friendly, professional，默认 friendly
- `context` (string, optional): 额外上下文信息

## 执行流程

1. 接收用户输入参数
2. 验证必填字段
3. 根据语言和风格选择模板
4. 生成个性化问候
5. 返回结果

## 输出规范

- 成功返回：
  - `greeting`: 生成的问候语
  - `language`: 使用的语言
  - `style`: 使用的风格
- 失败返回：
  - `error`: 错误信息

## 约束与安全

- 不收集用户隐私信息
- 问候语必须友好和尊重
- 避免使用敏感词汇
- 支持多种语言字符编码

## 示例

### 示例 1：中文友好问候

输入：
```json
{
  "name": "张三",
  "language": "zh",
  "style": "friendly"
}
```

输出：
```json
{
  "greeting": "你好，张三！很高兴见到你！有什么我可以帮助你的吗？",
  "language": "zh",
  "style": "friendly"
}
```

### 示例 2：英文正式问候

输入：
```json
{
  "name": "John",
  "language": "en",
  "style": "formal"
}
```

输出：
```json
{
  "greeting": "Hello, John. It is a pleasure to meet you. How may I assist you today?",
  "language": "en",
  "style": "formal"
}
```
```

### 步骤 3：验证 Skill

确保：
- 目录名 `hello-world` 与 `name` 字段一致
- YAML Frontmatter 格式正确
- 包含所有 7 个强制章节
- `description` 至少 50 字符

## 在 Aetheris 中使用 Skill

### 1. 将 Skill 放置在正确位置

将您的 Skill 目录放在 `examples/agentskills/` 下，或者 Aetheris 配置的 Skill 目录中。

### 2. 启动 Aetheris

```bash
cargo run
```

### 3. 通过 API 使用 Skill

```bash
curl -X POST http://localhost:3000/api/skills/hello-world/execute \
  -H "Content-Type: application/json" \
  -d '{
    "name": "张三",
    "language": "zh",
    "style": "friendly"
  }'
```

## 下一步

- 阅读 [Agent Skills 官方规范](./AGENTSKILLS_IO_SPEC.md) 了解完整的标准
- 查看 [最佳实践](./BEST_PRACTICES.md) 学习如何编写高质量的 Skill
- 探索 [示例 Skills](./examples/agentskills/) 了解更多实际应用
- 阅读 [README.md](./README.md) 了解 Aetheris 的完整功能

## 获得帮助

如果您遇到问题：
- 查看 [常见问题](./README.md#常见问题解答-faq)
- 提交 [GitHub Issue](https://github.com/aetheris/aetheris/issues)
- 加入我们的社区讨论

## 相关资源

- [Agent Skills 官方网站](https://agentskills.io)
- [Aetheris 文档](./README.md)
- [示例技能库](./examples/agentskills/)
- [最佳实践指南](./BEST_PRACTICES.md)
