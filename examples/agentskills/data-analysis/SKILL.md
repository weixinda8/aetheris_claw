---
name: data-analysis
description: Analyze datasets, generate insights, create visualizations, and produce statistical reports. Use when working with data exploration, statistics, trends analysis, or data-driven decision making.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [data, analysis, statistics, visualization, insights, reporting]
compatibility: Requires data processing tools
timeout: 600
allowed-tools: [Read, Write, CSV, JSON, Excel, LLM]
metadata:
  emoji: "📊"
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

# Data Analysis Skill

## 功能概述

分析数据集、生成洞察、创建可视化、产生统计报告。支持多种数据格式和分析方法。

## 适用场景

- 数据探索和清洗
- 统计分析和假设检验
- 趋势和模式识别
- 数据可视化
- 报告生成
- A/B 测试分析

## 输入规范

- `data_source` (string, required): 数据源路径或数据内容
- `data_format` (string, required): 数据格式，可选值：csv, json, excel, parquet
- `analysis_type` (string, required): 分析类型，可选值：exploratory, statistical, trend, comparative, predictive
- `target_columns` (array, optional): 目标列名列表
- `group_by` (string, optional): 分组列名
- `time_column` (string, optional): 时间列名（用于趋势分析）
- `output_format` (string, optional): 输出格式，默认 markdown

## 执行流程

1. 加载和验证数据
2. 数据清洗和预处理
3. 探索性数据分析
4. 统计计算
5. 可视化生成
6. 洞察提取
7. 报告撰写

## 输出规范

- 成功返回：
  - `summary`: 分析摘要
  - `statistics`: 关键统计指标
  - `insights`: 洞察列表
  - `visualizations`: 可视化描述
  - `report`: 完整报告

## 约束与安全

- 保护敏感数据
- 避免错误的统计推断
- 清晰标注数据限制
- 注明分析局限性

## 示例

### 示例 1：销售数据分析

输入：
```
{
  "data_source": "./sales_data.csv",
  "data_format": "csv",
  "analysis_type": "exploratory",
  "group_by": "region",
  "time_column": "date"
}
```

输出：
```
{
  "summary": "分析了2025年全年销售数据，包含1200条记录，涵盖4个地区",
  "statistics": {
    "total_revenue": 12580000,
    "average_order_value": 256.5,
    "growth_rate": 0.15
  },
  "insights": [
    "华东地区贡献了45%的总营收",
    "Q4季度销售额环比增长28%",
    "产品A是销量最高的产品"
  ],
  "report": "# 销售数据分析报告\n\n## 执行摘要\n...\n"
}
```
