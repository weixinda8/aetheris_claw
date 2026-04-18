---
name: meeting-assistant
description: Assist with meeting preparation, note-taking, summarization, action item tracking, and follow-up. Use when planning meetings, taking notes, or managing meeting outcomes.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [meeting, assistant, notes, summary, action, follow-up]
compatibility: Requires audio processing (optional)
timeout: 600
allowed-tools: [Read, Write, LLM, Audio]
metadata:
  emoji: "📅"
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

# Meeting Assistant Skill

## 功能概述

协助会议准备、记录笔记、总结内容、跟踪行动项和后续跟进。

## 适用场景

- 会议议程准备
- 会议记录和笔记
- 会议内容总结
- 行动项提取和跟踪
- 会议纪要生成
- 后续任务分配

## 输入规范

- `operation` (string, required): 操作类型，可选值：prepare_agenda, take_notes, summarize, extract_actions, generate_minutes
- `meeting_title` (string, required): 会议标题
- `participants` (array, optional): 参会人员列表
- `date` (string, optional): 会议日期
- `agenda_items` (array, optional): 议程项列表（prepare_agenda 时）
- `transcript` (string, optional): 会议记录或转录文本
- `previous_actions` (array, optional): 之前的行动项

## 执行流程

1. 收集会议信息
2. 准备会议材料
3. 记录关键要点
4. 提取行动项和责任人
5. 生成会议摘要
6. 创建会议纪要文档

## 输出规范

- 成功返回：
  - `operation`: 执行的操作
  - `meeting_info`: 会议信息
  - `content`: 生成的内容（议程、笔记、摘要等）
  - `action_items`: 行动项列表
  - `output_files`: 生成的文件

## 约束与安全

- 保护会议隐私
- 准确记录行动项
- 明确责任人和截止日期
- 保存会议记录用于追溯

## 示例

### 示例 1：生成会议纪要

输入：
```
{
  "operation": "generate_minutes",
  "meeting_title": "产品周会",
  "participants": ["张三", "李四", "王五"],
  "date": "2026-04-07",
  "transcript": "会议讨论了新功能开发进度..."
}
```

输出：
```
{
  "operation": "generate_minutes",
  "meeting_info": {
    "title": "产品周会",
    "date": "2026-04-07",
    "participants": ["张三", "李四", "王五"]
  },
  "content": "# 产品周会会议纪要\n\n## 会议概要\n...",
  "action_items": [
    {
      "description": "完成用户界面设计",
      "assignee": "张三",
      "deadline": "2026-04-14",
      "priority": "high"
    }
  ],
  "output_files": ["./meeting_minutes/产品周会-2026-04-07.md"]
}
```
