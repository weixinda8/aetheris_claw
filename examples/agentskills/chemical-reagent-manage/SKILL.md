---
name: chemical-reagent-manage
description: Manage chemical reagent inventory, perform stock checks, expiry warnings, and generate purchase requests. Use when working with laboratory chemical management, inventory tracking, or safety compliance.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [chemical, reagent, inventory, laboratory, safety, expiry, purchase]
compatibility: Requires Excel file access
timeout: 300
allowed-tools: [Read, Write, Excel]
metadata:
  emoji: "🧪"
  requires:
    env: []
    bins: []
  retry_config:
    max_attempts: 2
    initial_delay_ms: 1000
    max_delay_ms: 3000
    backoff_multiplier: 2
  sandbox_level: high
---

# Chemical Reagent Management Skill

## 功能概述

管理化工化验室强酸、强碱、易燃、易爆试剂，包括库存核查、过期预警、消耗统计和采购申请生成。

## 适用场景

- 危化试剂库存盘点
- 试剂有效期预警
- 月度消耗统计
- 采购申请单生成
- 安全合规检查

## 输入规范

- `operation` (string, required): 操作类型，可选值：inventory_check, expiry_warning, purchase_request, consumption_statistics, full_flow
- `ledger_path` (string, optional): 危化试剂台账路径，默认 ~/试剂管理/危化试剂台账.xlsx
- `inventory_threshold` (integer, optional): 预警库存下限，默认 5
- `warning_threshold_days` (integer, optional): 临期预警天数阈值，默认 30
- `output_dir` (string, optional): 输出目录，默认 ~/试剂管理/output
- `statistics_month` (string, optional): 统计月份，格式 YYYY-MM

## 执行流程

1. 读取危化试剂台账文件
2. 根据操作类型执行相应处理
3. 库存盘点：核对库存、位置、状态、有效期
4. 有效期预警：筛选过期/临期/低于阈值的试剂
5. 消耗统计：月度消耗、领用记录、损耗率
6. 生成输出文件：采购申请单、整改清单、预警报表

## 输出规范

- 成功返回：包含操作结果的结构化数据
  - `task_id`: 任务 ID
  - `operation`: 执行的操作
  - `status`: 状态
  - `data`: 操作结果数据
  - `output_files`: 生成的文件列表
- 失败返回：包含错误信息

## 约束与安全

- 仅读写 ~/试剂管理/ 目录
- 禁止删除台账历史记录
- 敏感信息自动脱敏
- 严格遵守危化品管理规定

## 示例

### 示例 1：库存盘点

输入：
```
{
  "operation": "inventory_check",
  "ledger_path": "~/试剂管理/危化试剂台账.xlsx",
  "inventory_threshold": 5
}
```

输出：
```
{
  "task_id": "REAGENT-2026-0407-001",
  "operation": "inventory_check",
  "status": "success",
  "data": {
    "inventory_summary": {
      "total_items": 156,
      "below_threshold": 12,
      "total_value": 125800.50
    },
    "low_stock_items": [
      {
        "name": "浓硫酸",
        "current_stock": 3,
        "threshold": 5,
        "unit": "瓶"
      }
    ]
  },
  "output_files": [
    "~/试剂管理/output/库存盘点报告-2026-04.xlsx"
  ]
}
```
