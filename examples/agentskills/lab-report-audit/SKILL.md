---
name: lab-report-audit
description: Audit chemical laboratory test records and reports, verify compliance, data accuracy, signature completeness, and generate audit logs. Use when reviewing lab reports, quality control, or compliance verification.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [lab, audit, report, chemical, quality, compliance, verification]
compatibility: Requires Excel and PDF processing
timeout: 300
allowed-tools: [Read, Write, Excel]
metadata:
  emoji: "✅"
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

# Lab Report Audit Skill

## 功能概述

对化工原料/中控/成品化验原始记录、检测报告进行合规性、准确性审核，驳回不合格项，生成审核意见与正式报告。

## 适用场景

- 化验原始记录审核
- 检测报告合规性检查
- 数据准确性验证
- 签字闭环确认
- 审核日志生成

## 输入规范

- `operation` (string, required): 操作类型，可选值：full_audit, data_extraction, compliance_check, report_generation
- `report_path` (string, required): 待审核文件路径，支持 .xlsx/.md/.pdf
- `sample_type` (string, required): 样品类型，可选值：raw_material, intermediate, finished_product
- `standard_reference` (string, required): 标准依据，如 GB/HG/企标编号
- `enable_signature_check` (boolean, optional): 是否启用签字闭环检查，默认 true
- `output_dir` (string, optional): 审核结果输出目录，默认 ./audit_results

## 执行流程

1. 读取文件：校验文件存在、格式合法、非空
2. 提取核心数据：样品编号、指标、结果、标准值、检验员
3. 逐项比对：结果是否在标准范围内、有效数字合规、无涂改
4. 异常判定：超标/超差/数据异常 → 标记驳回、注明原因
5. 合格处理：签署审核意见、生成正式报告、归档
6. 日志记录：保存审核轨迹、可100%溯源

## 输出规范

- 成功返回：审核结果
  - `audit_id`: 审核 ID
  - `audit_status`: 审核状态
  - `sample_info`: 样品信息
  - `indicators`: 指标审核结果
  - `rejection_items`: 驳回项清单
  - `signature_status`: 签字检查状态
  - `report_files`: 生成的报告文件

## 约束与安全

- 审核过程不可篡改原始数据
- 保留完整审核轨迹
- 遵守数据保密规定
- 审核意见需清晰明确

## 示例

### 示例 1：完整审核流程

输入：
```
{
  "operation": "full_audit",
  "report_path": "./lab_records/成品化验-2026-04-07.xlsx",
  "sample_type": "finished_product",
  "standard_reference": "GB/T 12345-2023",
  "enable_signature_check": true,
  "output_dir": "./audit_results"
}
```

输出：
```
{
  "audit_id": "AUDIT-2026-0407-001",
  "audit_status": "passed",
  "sample_info": {
    "sample_id": "SAM-2026-04-06",
    "sample_type": "finished_product",
    "inspector": "张三"
  },
  "indicators": [
    {
      "name": "水分",
      "result": 0.12,
      "standard_min": 0.0,
      "standard_max": 0.5,
      "status": "compliant",
      "comment": "符合要求"
    }
  ],
  "rejection_items": [],
  "signature_status": {
    "inspector_signed": true,
    "reviewer_signed": true,
    "approver_signed": true
  },
  "report_files": {
    "audit_report": "./audit_results/审核-成品化验-2026-04-07.md",
    "formal_report": "./audit_results/正式报告-成品化验-2026-04-07.pdf",
    "audit_log": "./audit_logs/audit-2026-04-07.csv"
  }
}
```
