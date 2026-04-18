# 创建和使用 Skill

本教程将详细说明如何创建和使用符合 agentskills.io 标准的 Skill。

## 什么是 Skill？

Skill 是 Aetheris 中的基本功能单元，是根据 agentskills.io 行业标准定义的可重用功能模块。每个 Skill 都有自己的输入、输出和执行逻辑。

## Skill 结构

每个 Skill 都有以下结构：

```
skill-name/
├── SKILL.md          # Skill 定义文件
├── references/       # 参考文档和示例
└── (可选) code/      # 自定义代码
```

## 创建 Skill

### 步骤 1：创建 Skill 目录

```bash
mkdir -p examples/agentskills/your-skill-name
cd examples/agentskills/your-skill-name
```

### 步骤 2：创建 SKILL.md 文件

SKILL.md 文件是 Skill 的核心定义文件，包含以下部分：

1. **YAML Frontmatter** - 包含 Skill 的基本信息
2. **功能概述** - 描述 Skill 的功能和用途
3. **适用场景** - 说明 Skill 适用的场景
4. **输入规范** - 定义 Skill 的输入参数
5. **执行流程** - 描述 Skill 的执行步骤
6. **输出规范** - 定义 Skill 的输出格式
7. **约束与安全** - 说明 Skill 的约束和安全考虑
8. **示例** - 提供 Skill 的使用示例

#### YAML Frontmatter

```yaml
---
name: your-skill-name
description: Description of your skill. Use when you need to do something specific.
version: 1.0.0
author: Your Name
license: Apache-2.0
tags: [tag1, tag2, tag3]
compatibility: No special requirements
timeout: 30
allowed-tools: [LLM]
metadata:
  emoji: "🔧"
  retry_config:
    max_attempts: 1
    initial_delay_ms: 500
    max_delay_ms: 1000
    backoff_multiplier: 1
  sandbox_level: low
---
```

#### 功能概述

```markdown
# Your Skill Name

## 功能概述

描述您的 Skill 的功能和用途。
```

#### 适用场景

```markdown
## 适用场景

说明您的 Skill 适用的场景。
```

#### 输入规范

```markdown
## 输入规范

定义您的 Skill 的输入参数：

- `param1` (type, required): 描述
- `param2` (type, optional): 描述
```

#### 执行流程

```markdown
## 执行流程

1. 接收输入参数
2. 验证输入参数
3. 执行逻辑
4. 返回结果
```

#### 输出规范

```markdown
## 输出规范

- 成功返回：
  - `result`: 结果
- 失败返回：
  - `error`: 错误信息
```

#### 约束与安全

```markdown
## 约束与安全

说明您的 Skill 的约束和安全考虑。
```

#### 示例

```markdown
## 示例

### 示例 1：基本使用

输入：
```json
{
  "param1": "value1",
  "param2": "value2"
}
```

输出：
```json
{
  "result": "success"
}
```
```
```

### 步骤 3：验证 Skill

确保：
- 目录名与 `name` 字段一致
- YAML Frontmatter 格式正确
- 包含所有 7 个强制章节
- `description` 至少 50 字符

## 使用 Skill

### 通过 API 使用 Skill

```bash
curl -X POST http://localhost:3000/api/v1/skills/your-skill-name/execute \
  -H "Content-Type: application/json" \
  -d '{
    "param1": "value1",
    "param2": "value2"
  }'
```

### 通过 Agent 使用 Skill

在 Agent 配置文件中添加 Skill：

```yaml
skills:
  - name: your-skill-name
    version: 1.0.0
    enabled: true
```

### 通过 IM 平台使用 Skill

在 IM 平台中发送包含 Skill 名称和参数的消息：

```
使用 your-skill-name 技能，参数：{"param1": "value1", "param2": "value2"}
```

## 示例 Skill

Aetheris 提供 12 个开箱即用的示例 Skill：

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

## 最佳实践

1. **遵循 agentskills.io 标准**：确保您的 Skill 符合 agentskills.io 行业标准
2. **提供详细的文档**：为您的 Skill 提供详细的文档，包括功能概述、适用场景、输入输出规范等
3. **测试您的 Skill**：在部署 Skill 之前，确保您已经充分测试了 Skill 的功能和性能
4. **优化执行时间**：尽量优化 Skill 的执行时间，确保 Skill 能够快速响应
5. **考虑安全性**：在设计 Skill 时，考虑安全性，确保 Skill 不会执行危险操作
6. **使用适当的 sandbox 级别**：根据 Skill 的功能，设置适当的 sandbox 级别
7. **提供清晰的错误信息**：当 Skill 执行失败时，提供清晰的错误信息
8. **使用版本控制**：为您的 Skill 使用版本控制，确保版本兼容性

## 故障排除

### 常见问题

1. **Skill 执行失败**：检查 Skill 配置是否正确，确保输入参数符合要求
2. **Skill 响应缓慢**：检查 LLM 配置是否正确，确保网络连接正常
3. **Skill 无法加载**：检查 Skill 目录结构是否正确，确保 SKILL.md 文件格式正确

### 日志查看

查看 Aetheris 服务日志以获取更多信息：

```bash
# 开发模式运行时，日志会直接输出到终端
# 生产模式运行时，日志会输出到 logs/ 目录
```

## 下一步

- [化工生产排产](chemical-production-scheduling.md) - 了解化工生产排产的完整解决方案
- [DevOps 自动化](devops-automation.md) - 了解如何实现 DevOps 自动化
- [AI 应用开发](ai-application-development.md) - 了解如何基于 Aetheris 构建 AI 应用
- [API 文档](../api/README.md) - 了解完整的 API 参考
- [用户指南](../user-guide/README.md) - 了解更全面的使用指导
