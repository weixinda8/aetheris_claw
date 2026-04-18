---
name: report-generation
description: Generate professional reports in various formats including markdown, PDF, and HTML. Use when creating business reports, technical documentation, or summary documents.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [report, generation, document, business, technical, summary]
compatibility: Requires document processing tools
timeout: 600
allowed-tools: [Read, Write, LLM]
metadata:
  emoji: "📋"
  requires:
    env: []
    bins: []
  retry_config:
    max_attempts: 2
    initial_delay_ms: 1000
    max_delay_ms: 3000
    backoff_multiplier: 2
  sandbox_level: low
---

# Report Generation Skill

## 功能概述

生成各种格式的专业报告，包括 markdown、PDF 和 HTML。

## 适用场景

- 业务报告
- 技术文档
- 项目总结
- 数据分析报告
- 进度报告
- 审计报告

## 输入规范

- `report_type` (string, required): 报告类型，可选值：business, technical, project, analysis, progress, audit
- `title` (string, required): 报告标题
- `sections` (array, required): 报告章节列表
- `data` (object, optional): 报告数据
- `template` (string, optional): 模板名称
- `output_format` (string, optional): 输出格式，可选值：markdown, pdf, html，默认 markdown
- `output_path` (string, optional): 输出路径

## 执行流程

1. 收集报告数据
2. 确定报告结构
3. 生成章节内容
4. 添加图表和可视化
5. 应用格式和样式
6. 导出为指定格式

## 输出规范

- 成功返回：
  - `report_title`: 报告标题
  - `format`: 输出格式
  - `content`: 报告内容
  - `file_path`: 生成的文件路径

## 约束与安全

- 确保数据准确性
- 遵循报告模板规范
- 清晰标注数据来源
- 保护敏感信息

## 示例

### 示例 1：生成项目进度报告

输入：
```
{
  "report_type": "progress",
  "title": "Q2 项目进度报告",
  "sections": ["执行摘要", "进度概述", "里程碑", "风险与问题", "下一步计划"],
  "data": {
    "project_name": "AI平台升级",
    "completion_rate": 0.75,
    "milestones": [
      {"name": "需求分析", "status": "completed"},
      {"name": "设计", "status": "completed"},
      {"name": "开发", "status": "in_progress"},
      {"name": "测试", "status": "pending"}
    ]
  },
  "output_format": "markdown"
}
```

输出：
```
{
  "report_title": "Q2 项目进度报告",
  "format": "markdown",
  "content": "# Q2 项目进度报告\n\n## 执行摘要\nAI平台升级项目总体进度75%...",
  "file_path": "./reports/Q2_项目进度报告.md"
}
```
