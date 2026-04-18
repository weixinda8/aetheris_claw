---
name: database-query
description: Execute database queries, retrieve data, and perform CRUD operations safely. Use when working with databases, fetching data, or managing database records.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [database, query, sql, data, crud, retrieval]
compatibility: Requires database connection
timeout: 300
allowed-tools: [Database, Read, Write]
metadata:
  emoji: "🗄️"
  requires:
    env: ["DATABASE_URL"]
    bins: []
  retry_config:
    max_attempts: 2
    initial_delay_ms: 1000
    max_delay_ms: 3000
    backoff_multiplier: 2
  sandbox_level: high
---

# Database Query Skill

## 功能概述

执行数据库查询、检索数据并安全地执行 CRUD 操作。

## 适用场景

- 数据查询和检索
- 数据插入和更新
- 数据删除（谨慎）
- 报表数据提取
- 数据验证
- 数据库维护操作

## 输入规范

- `operation` (string, required): 操作类型，可选值：select, insert, update, delete, execute
- `query` (string, required): SQL 查询语句或操作描述
- `parameters` (object, optional): 查询参数（用于参数化查询）
- `database` (string, optional): 数据库名称
- `limit` (integer, optional): 结果限制数量，默认 100
- `timeout_seconds` (integer, optional): 查询超时秒数，默认 60
- `dry_run` (boolean, optional): 是否为 dry run（不实际执行修改），默认 false

## 执行流程

1. 验证查询安全性
2. 创建数据库连接
3. 执行参数化查询
4. 获取查询结果
5. 格式化返回数据
6. 关闭连接
7. 记录操作日志

## 输出规范

- 成功返回：
  - `operation`: 执行的操作
  - `success`: true
  - `row_count`: 影响的行数
  - `results`: 查询结果（select 操作）
  - `columns`: 列名列表
  - `execution_time_ms`: 执行时间
- 失败返回：
  - `success`: false
  - `error`: 错误类型
  - `message`: 错误详情

## 约束与安全

- 使用参数化查询防止 SQL 注入
- 限制查询返回数量
- DELETE 和 UPDATE 操作需要额外确认
- 禁止未授权的架构修改
- 记录所有数据库操作
- 实施最小权限原则

## 示例

### 示例 1：查询数据

输入：
```
{
  "operation": "select",
  "query": "SELECT id, name, email, created_at FROM users WHERE status = $1 AND created_at > $2",
  "parameters": {"status": "active", "created_at": "2026-01-01"},
  "limit": 50
}
```

输出：
```
{
  "operation": "select",
  "success": true,
  "row_count": 42,
  "columns": ["id", "name", "email", "created_at"],
  "results": [
    {"id": 1, "name": "张三", "email": "zhang@example.com", "created_at": "2026-01-15"},
    {"id": 2, "name": "李四", "email": "li@example.com", "created_at": "2026-02-20"}
  ],
  "execution_time_ms": 125
}
```
