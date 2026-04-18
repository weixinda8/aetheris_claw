# OpenClaw vs Aetheris Skill 配置格式对比与转换指南

## 📋 概述

本指南详细对比了 OpenClaw 的 `SKILL.md` 格式和 Aetheris 的 `.skill.yaml` 格式，并提供了从 OpenClaw 格式转换到 Aetheris 格式的最佳实践。

---

## 🔍 格式对比总览

| 维度 | OpenClaw (SKILL.md) | Aetheris (.skill.yaml) |
|------|---------------------|------------------------|
| **文件结构** | 同名文件夹 + SKILL.md（单一文件） | 独立的 .skill.yaml 文件 |
| **元数据格式** | YAML Frontmatter（文件头部） | YAML 根节点 |
| **指令格式** | Markdown 正文（自然语言） | 结构化 YAML（当前无流程指令） |
| **流程位置** | 在 Skill 文件中 | 在 Agent 配置中（system_prompt） |
| **设计哲学** | 技能就是完整的智能体 | Agent 是协调者，Skill 是能力单元 |

---

## 📝 详细字段映射表

### A. 元数据字段映射

| OpenClaw SKILL.md | Aetheris .skill.yaml | 说明 |
|-------------------|---------------------|------|
| `name` (Frontmatter) | `metadata.id` | 唯一标识符 |
| - | `metadata.name` | 显示名称（中文） |
| `description` (Frontmatter) | `metadata.description` | 简短描述 |
| - | `metadata.long_description` | 详细描述 |
| `version` (Frontmatter) | `metadata.version` | 版本号 |
| `author` (Frontmatter) | `metadata.author` | 作者 |
| - | `metadata.created_at` | 创建时间（ISO 8601） |
| - | `metadata.updated_at` | 更新时间（ISO 8601） |
| `user-invocable` (Frontmatter) | - | Aetheris 暂无对应字段 |
| `disable-model-invocation` (Frontmatter) | - | Aetheris 暂无对应字段 |
| `triggers` (Frontmatter) | `metadata.keywords` | 触发关键词 |
| - | `metadata.tags` | 标签 |
| - | `metadata.categories` | 分类 |
| - | `metadata.skill_type` | 技能类型（Builtin/Custom） |
| - | `metadata.priority` | 优先级（High/Medium/Low） |
| - | `metadata.deprecated` | 是否弃用 |
| `metadata.openclaw.emoji` | - | Aetheris 暂无对应字段 |

### B. 安全与依赖字段映射

| OpenClaw SKILL.md | Aetheris .skill.yaml | 说明 |
|-------------------|---------------------|------|
| `permissions` (Frontmatter) | `spec.permissions` | 权限列表 |
| `metadata.openclaw.requires.env` | `spec.env_vars` | 依赖环境变量 |
| `metadata.openclaw.requires.bins` | - | Aetheris 暂无对应字段 |
| - | `spec.security.required_capabilities` | 必需能力 |
| - | `spec.security.sandbox_level` | 沙箱级别 |
| - | `spec.dependencies` | 依赖的其他 Skill |
| - | `spec.timeout_seconds` | 超时时间（秒） |
| - | `spec.retry_config` | 重试配置 |

### C. 接口定义字段映射

| OpenClaw SKILL.md | Aetheris .skill.yaml | 说明 |
|-------------------|---------------------|------|
| `## 参数` 部分 | `spec.parameters` | 参数定义（结构化） |
| - | `spec.returns` | 返回值定义（结构化） |
| - | `spec.examples` | 使用示例 |
| `## 执行步骤` 部分 | - | Aetheris 流程在 Agent 中 |
| `## 输出格式` 部分 | `spec.returns` + 示例 | 返回值说明 |
| `## 安全规则` 部分 | `spec.security` | 安全配置 |

### D. 实现字段映射

| OpenClaw SKILL.md | Aetheris .skill.yaml | 说明 |
|-------------------|---------------------|------|
| - | `spec.implementation.type` | 实现类型（builtin/remote） |
| - | `spec.implementation.name` | 实现名称 |

---

## 🔄 完整转换示例 1：化验报告审核

### OpenClaw 格式（SKILL.md）

```yaml
---
name: lab-report-audit
description: 审核化工化验原始记录与检测报告，校验指标合规、数据准确、签字闭环、生成审核日志
version: 1.0.0
author: Claw
user-invocable: true
permissions: [file_read, file_write, excel]
triggers: [审核, 校验, 报告, 化验]
metadata:
  openclaw:
    emoji: "✅"
    requires:
      env: []
      bins: []
---

# 化验报告审核技能

## 功能说明
对化工原料/中控/成品化验原始记录、检测报告进行合规性、准确性审核，驳回不合格项，生成审核意见与正式报告。

## 参数
- `报告路径` (string): 待审核文件路径，支持 .xlsx/.md/.pdf
- `样品类型` (string): 原料/中控/成品
- `标准依据` (string): GB/HG/企标编号

## 执行步骤
1. 读取文件：校验文件存在、格式合法、非空
2. 提取核心数据：样品编号、指标、结果、标准值、检验员
3. 逐项比对：结果是否在标准范围内、有效数字合规、无涂改
4. 异常判定：超标/超差/数据异常 → 标记驳回、注明原因
5. 合格处理：签署审核意见、生成正式报告、归档
6. 日志记录：保存审核轨迹、可100%溯源

## 输出
- 成功：生成 审核-{{报告名}}.md、正式报告.pdf
- 失败：返回 驳回原因清单、异常指标明细
- 日志：~/化验室/审核日志/audit-{{today}}.csv
```

### Aetheris 格式（lab_report_audit.skill.yaml）

```yaml
api_version: "v1"
kind: "AgentSkill"
metadata:
  id: "lab_report_audit"
  name: "化验报告审核"
  version: "1.0.0"
  author: "Aetheris Team"
  created_at: "2026-04-07T00:00:00Z"
  updated_at: "2026-04-07T00:00:00Z"
  description: "审核化工化验原始记录与检测报告，校验指标合规、数据准确、签字闭环、生成审核日志"
  long_description: "对化工原料/中控/成品化验原始记录、检测报告进行合规性、准确性审核，驳回不合格项，生成审核意见与正式报告。支持 Excel、Markdown、PDF 等多种格式的审核，生成审核日志便于追溯。"
  category: "lab"
  tags:
    - "lab"
    - "audit"
    - "chemical"
    - "report"
    - "quality"
  categories:
    - "quality"
    - "lab"
  skill_type: "Builtin"
  priority: "High"
  keywords:
    - "审核"
    - "校验"
    - "报告"
    - "化验"
    - "合规"
  deprecated: false
spec:
  parameters:
    - name: "operation"
      type: "string"
      required: true
      enum: ["full_audit", "data_extraction", "compliance_check", "report_generation"]
      description: "操作类型：完整审核、数据提取、合规性检查、报告生成"
    - name: "report_path"
      type: "string"
      required: true
      description: "待审核文件路径，支持 .xlsx/.md/.pdf"
    - name: "sample_type"
      type: "string"
      required: true
      enum: ["raw_material", "intermediate", "finished_product"]
      description: "样品类型：原料/中控/成品"
    - name: "standard_reference"
      type: "string"
      required: true
      description: "标准依据，如 GB/HG/企标编号"
    - name: "enable_signature_check"
      type: "boolean"
      required: false
      default: true
      description: "是否启用签字闭环检查"
    - name: "output_dir"
      type: "string"
      required: false
      default: "./audit_results"
      description: "审核结果输出目录"
  returns:
    type: "object"
    description: "审核结果"
    properties:
      success:
        type: "boolean"
        description: "审核操作是否成功"
      message:
        type: "string"
        description: "结果消息"
      data:
        type: "object"
        description: "返回数据"
        properties:
          audit_id:
            type: "string"
            description: "审核 ID"
          audit_status:
            type: "string"
            enum: ["passed", "rejected", "pending"]
            description: "审核状态"
          sample_info:
            type: "object"
            description: "样品信息"
            properties:
              sample_id:
                type: "string"
                description: "样品编号"
              sample_type:
                type: "string"
                description: "样品类型"
              inspector:
                type: "string"
                description: "检验员"
          indicators:
            type: "array"
            description: "指标审核结果"
            items:
              type: "object"
              properties:
                name:
                  type: "string"
                  description: "指标名称"
                result:
                  type: "number"
                  description: "检测结果"
                standard_min:
                  type: "number"
                  description: "标准最小值"
                standard_max:
                  type: "number"
                  description: "标准最大值"
                status:
                  type: "string"
                  enum: ["compliant", "non_compliant", "warning"]
                  description: "状态"
                comment:
                  type: "string"
                  description: "审核意见"
          rejection_items:
            type: "array"
            description: "驳回项清单"
            items:
              type: "object"
              properties:
                item:
                  type: "string"
                  description: "驳回项"
                reason:
                  type: "string"
                  description: "驳回原因"
          signature_status:
            type: "object"
            description: "签字检查状态"
            properties:
              inspector_signed:
                type: "boolean"
                description: "检验员签字"
              reviewer_signed:
                type: "boolean"
                description: "审核人签字"
              approver_signed:
                type: "boolean"
                description: "批准人签字"
          report_files:
            type: "object"
            description: "生成的报告文件"
            properties:
              audit_report:
                type: "string"
                description: "审核报告路径"
              formal_report:
                type: "string"
                description: "正式报告路径"
              audit_log:
                type: "string"
                description: "审核日志路径"
  examples:
    - name: "完整审核流程"
      description: "执行从读取文件到生成报告的完整审核流程"
      parameters:
        operation: "full_audit"
        report_path: "./lab_records/成品化验-2026-04-07.xlsx"
        sample_type: "finished_product"
        standard_reference: "GB/T 12345-2023"
        enable_signature_check: true
        output_dir: "./audit_results"
      returns:
        success: true
        message: "审核完成！"
        data:
          audit_id: "AUDIT-2026-0407-001"
          audit_status: "passed"
          sample_info:
            sample_id: "SAM-2026-04-06"
            sample_type: "finished_product"
            inspector: "张三"
          indicators:
            - name: "水分"
              result: 0.12
              standard_min: 0.0
              standard_max: 0.5
              status: "compliant"
              comment: "符合要求"
            - name: "纯度"
              result: 99.85
              standard_min: 99.5
              standard_max: 100.0
              status: "compliant"
              comment: "符合要求"
            - name: "pH"
              result: 6.8
              standard_min: 6.5
              standard_max: 7.5
              status: "compliant"
              comment: "符合要求"
          rejection_items: []
          signature_status:
            inspector_signed: true
            reviewer_signed: true
            approver_signed: true
          report_files:
            audit_report: "./audit_results/审核-成品化验-2026-04-07.md"
            formal_report: "./audit_results/正式报告-成品化验-2026-04-07.pdf"
            audit_log: "./audit_logs/audit-2026-04-07.csv"
  security:
    required_capabilities: ["file_system_access", "excel_processing"]
    sandbox_level: "medium"
  implementation:
    type: "builtin"
    name: "lab_report_audit"
  dependencies:
    - "excel_processor"
    - "pdf_extractor"
  env_vars:
    - "LAB_QUALITY_STANDARD_DB"
  permissions:
    - "file.read"
    - "file.write"
    - "excel.read"
    - "excel.write"
  timeout_seconds: 300
  retry_config:
    max_attempts: 2
    initial_delay_ms: 1000
    max_delay_ms: 5000
    backoff_multiplier: 1.5
    retry_on:
      - "file_not_found"
      - "excel_format_error"
```

---

## 🔄 完整转换示例 2：危化试剂管理

### OpenClaw 格式（SKILL.md）

```yaml
---
name: chemical-reagent-manage
description: 化工化验室危化试剂台账管理、库存盘点、有效期预警、采购申请生成
version: 1.0.0
user-invocable: true
permissions: [file_read, file_write, excel]
---

# 危化试剂管理技能

## 功能说明
管理化验室强酸、强碱、易燃、易爆试剂：库存核查、过期预警、消耗统计、生成采购单。

## 参数
- `操作类型` (string): 盘点/预警/采购
- `阈值` (number): 预警库存下限，默认 5

## 执行步骤
1. 读取台账：~/试剂管理/危化试剂台账.xlsx
2. 盘点：核对库存、位置、状态、有效期
3. 预警：筛选 过期/临期(<30天)/低于阈值 的试剂
4. 统计：月度消耗、领用记录、损耗率
5. 生成：采购申请单、整改清单、预警报表

## 安全规则
- 仅读写：~/试剂管理/ 目录
- 禁止删除：台账历史记录
- 敏感信息自动脱敏
```

### Aetheris 格式（chemical_reagent_manage.skill.yaml）

参见文件：`d:\aetheris\aetheris\examples\agentskills\chemical_reagent_manage.skill.yaml`

---

## 📝 转换最佳实践

### 1. 命名规范转换

| OpenClaw 规则 | Aetheris 规则 | 示例 |
|--------------|--------------|------|
| kebab-case（小写连字符） | snake_case（下划线） | `lab-report-audit` → `lab_report_audit` |
| 无显示名称字段 | 有中文显示名称 | - → `metadata.name: "化验报告审核"` |

### 2. 参数定义转换

OpenClaw 的 Markdown 参数列表需要转换为 Aetheris 的结构化参数定义：

**OpenClaw 格式：**
```markdown
## 参数
- `报告路径` (string): 待审核文件路径，支持 .xlsx/.md/.pdf
- `样品类型` (string): 原料/中控/成品
- `是否加急` (boolean): 是否优先处理，默认 false
```

**Aetheris 格式：**
```yaml
parameters:
  - name: "report_path"
    type: "string"
    required: true
    description: "待审核文件路径，支持 .xlsx/.md/.pdf"
  - name: "sample_type"
    type: "string"
    required: true
    enum: ["raw_material", "intermediate", "finished_product"]
    description: "样品类型：原料/中控/成品"
  - name: "is_urgent"
    type: "boolean"
    required: false
    default: false
    description: "是否优先处理"
```

**关键转换点：**
- 参数名从中文改为英文 snake_case
- 可选参数标记 `required: false`
- 默认值通过 `default` 字段明确
- 枚举值通过 `enum` 字段结构化
- 类型严格化（string/number/boolean/array/object）

### 3. 流程指令处理

**重要差异：**
- OpenClaw：流程指令在 Skill 的 Markdown 正文中
- Aetheris：流程指令在 Agent 配置的 `system_prompt` 中

**转换策略：**
1. Skill 只定义接口（parameters、returns）
2. 流程逻辑移到 Agent 配置中
3. 可以将 OpenClaw 的执行步骤作为 Agent 的 system_prompt

**示例：**
```yaml
# Agent 配置示例
system_prompt: |
  你是一个化验报告审核助手。请按照以下步骤执行：
  
  1. 读取文件：调用 lab_report_audit Skill 的 data_extraction 操作
  2. 合规性检查：调用 lab_report_audit Skill 的 compliance_check 操作
  3. 报告生成：调用 lab_report_audit Skill 的 report_generation 操作
```

### 4. 安全配置转换

| OpenClaw | Aetheris | 说明 |
|---------|---------|------|
| `permissions: [file_read, file_write]` | `spec.permissions: ["file.read", "file.write"]` | 权限列表 |
| `metadata.openclaw.requires.env` | `spec.env_vars` | 环境变量依赖 |
| Markdown 中的安全规则 | `spec.security` + Agent 配置 | 安全规则 |

### 5. 示例转换

OpenClaw 没有显式的示例字段，Aetheris 建议提供完整的输入输出示例：

```yaml
examples:
  - name: "完整审核流程"
    description: "执行从读取文件到生成报告的完整审核流程"
    parameters:
      operation: "full_audit"
      report_path: "./lab_records/成品化验-2026-04-07.xlsx"
      sample_type: "finished_product"
      standard_reference: "GB/T 12345-2023"
    returns:
      success: true
      message: "审核完成！"
      data:
        audit_id: "AUDIT-2026-0407-001"
        audit_status: "passed"
```

---

## 🎯 设计哲学差异总结

### OpenClaw 的设计哲学
- **单一职责**：一个 Skill 文件就是一个完整的智能体
- **简单直观**：Markdown 正文，自然语言描述流程
- **快速上手**：学习曲线平缓，易于理解和编写
- **适合场景**：个人用户、简单场景、快速原型

### Aetheris 的设计哲学
- **分离关注点**：Agent 协调者 + Skill 能力单元
- **规范完整**：强类型定义、企业级特性
- **生产就绪**：超时、重试、安全、可观测性
- **适合场景**：企业用户、复杂场景、多 Skill 协调

### 关键洞察
两种设计**没有对错**，只是针对**不同的场景**：
- 如果需要**快速开发、简单场景**：OpenClaw 风格更合适
- 如果需要**企业级、复杂流程、生产环境**：Aetheris 风格更合适
- 两者可以**互补**：Aetheris 可以保持核心优势，同时可选支持 OpenClaw 风格

---

## 📂 相关文件

- Aetheris 示例目录：`d:\aetheris\aetheris\examples\agentskills\`
- 化验报告审核示例：`lab_report_audit.skill.yaml`
- 危化试剂管理示例：`chemical_reagent_manage.skill.yaml`
- 原始 OpenClaw 学习报告：`.trae/specs/openclaw_skill_template_study/spec.md`

---

**文档版本：** 1.0.0  
**最后更新：** 2026-04-07  
**作者：** Aetheris Team
