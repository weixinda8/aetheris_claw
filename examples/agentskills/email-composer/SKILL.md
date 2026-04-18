---
name: email-composer
description: Compose, format, and send professional emails with templates, personalization, and attachments. Use when drafting emails, creating correspondence, or managing email communication.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [email, compose, communication, correspondence, template, personalization]
compatibility: Requires email service access
timeout: 300
allowed-tools: [Read, Write, LLM]
metadata:
  emoji: "📧"
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

# Email Composer Skill

## 功能概述

撰写、格式化和发送专业邮件，支持模板、个性化和附件。

## 适用场景

- 撰写商务邮件
- 创建邮件模板
- 个性化邮件内容
- 邮件格式优化
- 批量邮件撰写
- 邮件回复和跟进

## 输入规范

- `operation` (string, required): 操作类型，可选值：compose, reply, forward, template, batch
- `to` (array, required): 收件人邮箱列表
- `subject` (string, required): 邮件主题
- `purpose` (string, required): 邮件目的或内容描述
- `from` (string, optional): 发件人邮箱
- `cc` (array, optional): 抄送列表
- `bcc` (array, optional): 密送列表
- `template` (string, optional): 模板名称
- `personalization` (object, optional): 个性化数据
- `attachments` (array, optional): 附件路径列表
- `tone` (string, optional): 语气，可选值：formal, casual, friendly, professional，默认 professional

## 执行流程

1. 确定邮件目的和受众
2. 选择或创建邮件模板
3. 个性化邮件内容
4. 撰写邮件正文
5. 添加签名和附件
6. 格式优化和审查
7. 准备发送

## 输出规范

- 成功返回：
  - `email_id`: 邮件 ID
  - `to`: 收件人
  - `subject`: 主题
  - `body`: 邮件正文
  - `status`: 状态
  - `preview`: 预览内容

## 约束与安全

- 保护隐私信息
- 遵守反垃圾邮件法规
- 验证邮件地址
- 避免发送敏感信息
- 保留邮件发送记录

## 示例

### 示例 1：撰写商务邮件

输入：
```
{
  "operation": "compose",
  "to": ["client@company.com"],
  "subject": "Q2 项目进度更新",
  "purpose": "向客户汇报Q2项目进度，包括已完成工作、当前状态和下一步计划",
  "from": "project-manager@company.com",
  "tone": "professional"
}
```

输出：
```
{
  "email_id": "EMAIL-2026-0407-001",
  "to": ["client@company.com"],
  "subject": "Q2 项目进度更新",
  "body": "尊敬的客户：\n\n您好！\n\n谨以此邮件向您汇报我们Q2项目的最新进度...",
  "status": "draft",
  "preview": "尊敬的客户：您好！谨以此邮件向您汇报我们Q2项目的最新进度..."
}
```
