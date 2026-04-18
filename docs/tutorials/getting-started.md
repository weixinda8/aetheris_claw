# 快速入门

本教程将帮助您快速上手 Aetheris，从安装到创建第一个任务。

## 步骤 1：安装 Aetheris

### 1.1 安装 Rust

Aetheris 是用 Rust 编写的，因此您需要安装 Rust 开发环境。

#### Windows (PowerShell)

```powershell
Invoke-RestMethod -Uri https://win.rustup.rs/x86_64 | Invoke-Expression
```

#### macOS / Linux

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安装完成后，重启终端以应用环境变量更改。

### 1.2 克隆代码库

```bash
git clone https://github.com/aetheris/aetheris.git
cd aetheris
```

### 1.3 构建项目

```bash
# 开发模式
cargo build

# 生产模式
cargo build --release
```

## 步骤 2：配置 LLM 提供商

Aetheris 需要配置 LLM 提供商以执行任务。我们推荐使用 DeepSeek 作为默认提供商。

### 2.1 复制配置文件

#### Windows (PowerShell)

```powershell
Copy-Item config\llm.example.yaml config\llm.yaml
```

#### macOS / Linux

```bash
cp config/llm.example.yaml config/llm.yaml
```

### 2.2 编辑配置文件

编辑 `config/llm.yaml` 文件，设置您的 DeepSeek API Key：

```yaml
provider: deepseek
api_key: your-deepseek-api-key-here
api_base: https://api.deepseek.com/v1
model: deepseek-chat
temperature: 0.7
max_tokens: 2000
timeout_seconds: 30
```

### 2.3 获取 DeepSeek API Key

访问 [DeepSeek 平台](https://platform.deepseek.com/) 注册并获取 API Key。

## 步骤 3：运行 Aetheris

### 3.1 启动服务

```bash
# 开发模式
cargo run

# 生产模式
cargo run --release
```

服务将在 `http://localhost:3000` 启动。

### 3.2 验证服务

打开浏览器访问 `http://localhost:3000/api/health`，您应该看到以下响应：

```json
{
  "status": "ok",
  "version": "1.0.0"
}
```

## 步骤 4：创建第一个任务

### 4.1 通过 API 创建任务

使用 curl 命令创建一个任务：

```bash
curl -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "agent": "industrial"
  }'
```

响应：

```json
{
  "success": true,
  "data": {
    "id": "task-123",
    "status": "pending",
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "agent": "industrial",
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

### 4.2 查看任务状态

```bash
curl http://localhost:3000/api/v1/tasks/task-123
```

响应：

```json
{
  "success": true,
  "data": {
    "id": "task-123",
    "status": "completed",
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "agent": "industrial",
    "output": "生产排产计划已生成，设备状态检查完成，原料库存分析完成，化验报告审核完成，最终生产报告已生成。",
    "created_at": "2024-01-01T00:00:00Z",
    "completed_at": "2024-01-01T00:05:00Z"
  }
}
```

## 步骤 5：通过 IM 平台使用

### 5.1 配置企业微信

1. 在企业微信管理后台创建一个应用
2. 配置应用的 webhook 地址为 `http://your-server:3000/api/webhook/wechat`
3. 在应用中发送消息："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
4. 应用会自动处理并回复结果

### 5.2 配置钉钉

1. 在钉钉中创建一个自定义机器人
2. 配置机器人的 webhook 地址为 `http://your-server:3000/api/webhook/dingtalk`
3. 在群聊中 @ 机器人并发送消息："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
4. 机器人会自动处理并回复结果

### 5.3 配置飞书

1. 在飞书中创建一个应用
2. 配置应用的 webhook 地址为 `http://your-server:3000/api/webhook/feishu`
3. 在应用中发送消息："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
4. 应用会自动处理并回复结果

## 步骤 6：创建第一个 Skill

### 6.1 创建 Skill 目录

```bash
mkdir -p examples/agentskills/hello-world
cd examples/agentskills/hello-world
```

### 6.2 创建 SKILL.md 文件

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

### 6.3 执行 Skill

重启 Aetheris 服务后，使用 curl 命令执行 Skill：

```bash
curl -X POST http://localhost:3000/api/v1/skills/hello-world/execute \
  -H "Content-Type: application/json" \
  -d '{
    "name": "张三",
    "language": "zh",
    "style": "friendly"
  }'
```

响应：

```json
{
  "success": true,
  "data": {
    "greeting": "你好，张三！很高兴见到你！有什么我可以帮助你的吗？",
    "language": "zh",
    "style": "friendly"
  }
}
```

## 下一步

- [创建和使用 Skill](creating-skills.md) - 深入了解如何创建和使用 Skill
- [化工生产排产](chemical-production-scheduling.md) - 了解化工生产排产的完整解决方案
- [DevOps 自动化](devops-automation.md) - 了解如何实现 DevOps 自动化
- [API 文档](../api/README.md) - 了解完整的 API 参考
- [用户指南](../user-guide/README.md) - 了解更全面的使用指导
