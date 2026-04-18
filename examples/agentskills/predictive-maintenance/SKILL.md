---
name: predictive-maintenance
description: Predict equipment failures, analyze health trends, and recommend maintenance schedules. Use when managing equipment reliability, planning maintenance, or preventing unplanned downtime.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [maintenance, predictive, equipment, reliability, failure, scheduling]
compatibility: Requires sensor data and historical records
timeout: 600
allowed-tools: [Read, Database, LLM]
metadata:
  emoji: "🔧"
  requires:
    env: []
    bins: []
  retry_config:
    max_attempts: 2
    initial_delay_ms: 2000
    max_delay_ms: 5000
    backoff_multiplier: 2
  sandbox_level: medium
---

# Predictive Maintenance Skill

## 功能概述

预测设备故障、分析健康趋势并推荐维护计划。

## 适用场景

- 设备健康评估
- 故障预测
- 维护计划优化
- 备件管理建议
- 风险分析
- 维护报告生成

## 输入规范

- `operation` (string, required): 操作类型，可选值：health_assessment, failure_prediction, maintenance_plan, risk_analysis, full_report
- `equipment_ids` (array, required): 设备 ID 列表
- `sensor_data` (object, optional): 实时传感器数据
- `historical_data` (string, optional): 历史数据路径
- `prediction_window_days` (integer, optional): 预测窗口天数，默认 30
- `maintenance_history` (array, optional): 历史维护记录

## 执行流程

1. 收集设备数据
2. 分析健康指标
3. 建立预测模型
4. 计算故障概率
5. 推荐维护方案
6. 生成维护计划
7. 输出报告

## 输出规范

- 成功返回：
  - `analysis_date`: 分析日期
  - `equipment_health`: 设备健康状态列表
  - `predictions`: 故障预测
  - `recommendations`: 维护建议
  - `maintenance_schedule`: 维护计划
  - `risk_summary`: 风险摘要

## 约束与安全

- 预测结果仅供参考
- 关键设备需人工确认
- 保留维护记录用于追溯
- 遵守设备安全规范

## 示例

### 示例 1：设备健康评估

输入：
```
{
  "operation": "health_assessment",
  "equipment_ids": ["EQ-001", "EQ-002", "EQ-003", "EQ-004"],
  "prediction_window_days": 30
}
```

输出：
```
{
  "analysis_date": "2026-04-07",
  "equipment_health": [
    {
      "equipment_id": "EQ-001",
      "name": "CNC机床1",
      "health_score": 95,
      "status": "healthy",
      "predicted_failure_days": 180,
      "recommendations": ["继续正常运行", "按计划进行预防性维护"]
    },
    {
      "equipment_id": "EQ-002",
      "name": "注塑机",
      "health_score": 68,
      "status": "warning",
      "predicted_failure_days": 30,
      "recommendations": ["检查液压系统", "更换密封圈", "安排近期维护"]
    },
    {
      "equipment_id": "EQ-004",
      "name": "焊接机器人",
      "health_score": 45,
      "status": "critical",
      "predicted_failure_days": 7,
      "recommendations": ["立即安排维护", "更换焊枪组件", "检查冷却系统"]
    }
  ],
  "maintenance_schedule": [
    {
      "priority": "urgent",
      "equipment_id": "EQ-004",
      "scheduled_date": "2026-04-09",
      "estimated_duration_hours": 8
    }
  ],
  "risk_summary": "设备健康预测完成，发现1台设备处于临界状态，建议立即安排维护"
}
```
