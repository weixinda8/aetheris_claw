---
name: production-monitoring
description: Monitor production lines, equipment status, quality metrics, and generate real-time monitoring reports. Use when overseeing manufacturing operations, tracking production KPIs, or managing factory floor operations.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [production, monitoring, manufacturing, factory, equipment, kpi]
compatibility: Requires industrial IoT access
timeout: 300
allowed-tools: [Read, Database, LLM]
metadata:
  emoji: "🏭"
  requires:
    env: []
    bins: []
  retry_config:
    max_attempts: 3
    initial_delay_ms: 2000
    max_delay_ms: 5000
    backoff_multiplier: 1.5
  sandbox_level: medium
---

# Production Monitoring Skill

## 功能概述

监控生产线、设备状态、质量指标并生成实时监控报告。

## 适用场景

- 生产线状态监控
- 设备健康状态跟踪
- 产量和效率统计
- 质量指标监测
- 异常报警和通知
- 生产报表生成

## 输入规范

- `operation` (string, required): 操作类型，可选值：realtime, snapshot, historical, alert, report
- `lines` (array, optional): 生产线 ID 列表（监控特定产线）
- `equipment` (array, optional): 设备 ID 列表（监控特定设备）
- `time_range` (object, optional): 时间范围，包含 start 和 end
- `metrics` (array, optional): 要监控的指标列表
- `thresholds` (object, optional): 告警阈值配置
- `output_format` (string, optional): 输出格式，默认 json

## 执行流程

1. 收集生产数据
2. 计算关键指标
3. 检测异常状态
4. 触发告警（如需要）
5. 生成监控报告
6. 可视化数据

## 输出规范

- 成功返回：
  - `timestamp`: 数据时间戳
  - `production_lines`: 生产线状态
  - `equipment_status`: 设备状态
  - `quality_metrics`: 质量指标
  - `alerts`: 告警列表
  - `summary`: 监控摘要

## 约束与安全

- 实时数据需准确
- 告警需及时响应
- 保留历史数据用于分析
- 遵守工业安全规范

## 示例

### 示例 1：实时生产监控

输入：
```
{
  "operation": "realtime",
  "lines": ["PL-001", "PL-002"],
  "metrics": ["output", "efficiency", "defect_rate"]
}
```

输出：
```
{
  "timestamp": "2026-04-07T10:30:00Z",
  "production_lines": [
    {
      "line_id": "PL-001",
      "name": "组装线A",
      "status": "running",
      "efficiency": 94.2,
      "output": 1250,
      "defect_rate": 0.8
    },
    {
      "line_id": "PL-002",
      "name": "组装线B",
      "status": "running",
      "efficiency": 88.5,
      "output": 980,
      "defect_rate": 1.2
    }
  ],
  "equipment_status": [
    {
      "equipment_id": "EQ-001",
      "name": "CNC机床1",
      "status": "normal",
      "temperature": 42.5,
      "vibration": 2.3
    }
  ],
  "alerts": [],
  "summary": "生产监控数据查询完成，2条产线正常运行"
}
```
