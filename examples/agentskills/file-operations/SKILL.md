---
name: file-operations
description: Perform file system operations including reading, writing, deleting, copying, moving, and listing files and directories. Use when working with local files, directories, or file management tasks.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [file, filesystem, io, read, write, delete, copy, move]
compatibility: Requires file system access
timeout: 120
allowed-tools: [Read, Write, Delete, Bash]
metadata:
  emoji: "📁"
  requires:
    env: []
    bins: []
  retry_config:
    max_attempts: 2
    initial_delay_ms: 500
    max_delay_ms: 2000
    backoff_multiplier: 1.5
  sandbox_level: medium
---

# File Operations Skill

## 功能概述

执行文件系统操作，包括读取、写入、删除、复制、移动文件和目录，以及列出目录内容。支持多种文件格式和批量操作。

## 适用场景

- 读取本地文件内容
- 创建和写入新文件
- 删除文件或目录
- 复制和移动文件
- 列出目录内容
- 检查文件是否存在
- 获取文件元数据

## 输入规范

- `operation` (string, required): 操作类型，可选值：read, write, delete, copy, move, list, exists, metadata
- `path` (string, required): 目标文件或目录路径
- `content` (string, optional): 写入文件的内容（仅 write 操作需要）
- `destination` (string, optional): 目标路径（仅 copy 和 move 操作需要）
- `recursive` (boolean, optional): 是否递归操作，默认 false
- `overwrite` (boolean, optional): 是否覆盖已存在文件，默认 false

## 执行流程

1. 验证操作类型和必需参数
2. 检查路径有效性和权限
3. 执行指定的文件操作
4. 处理异常和错误情况
5. 返回操作结果

## 输出规范

- 成功返回：
  - `success`: true
  - `operation`: 执行的操作
  - `path`: 操作的路径
  - `result`: 操作结果（read 返回内容，list 返回文件列表，metadata 返回元数据）
- 失败返回：
  - `success`: false
  - `error`: 错误类型
  - `message`: 错误详情

## 约束与安全

- 仅允许访问授权的目录
- 禁止删除系统关键文件
- 递归操作需谨慎确认
- 大文件操作需考虑性能影响
- 保留操作日志用于审计

## 示例

### 示例 1：读取文件

输入：
```
{
  "operation": "read",
  "path": "/path/to/document.txt"
}
```

输出：
```
{
  "success": true,
  "operation": "read",
  "path": "/path/to/document.txt",
  "result": "文件内容..."
}
```

### 示例 2：写入文件

输入：
```
{
  "operation": "write",
  "path": "/path/to/new_file.txt",
  "content": "这是新文件的内容",
  "overwrite": true
}
```

输出：
```
{
  "success": true,
  "operation": "write",
  "path": "/path/to/new_file.txt",
  "result": "File written successfully"
}
```
