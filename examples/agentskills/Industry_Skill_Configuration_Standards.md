# 行业 Skill 配置文件格式标准汇总

**整理日期：** 2026-04-07  
**信息来源：** 行业公开标准、agentskills.io、SKILL.md v3.0、Microsoft、OpenAI、LangChain 等

---

## 📋 概述

目前行业内主要有以下几种 Skill/Tool 配置文件格式标准：

| 标准名称 | 主要推动者 | 格式 | 特点 |
|---------|-----------|------|------|
| **Agent Skills (agentskills.io)** | 社区驱动 | YAML Frontmatter + Markdown | 开放标准、跨平台兼容 |
| **SKILL.md v3.0** | 402md | YAML Frontmatter + Markdown | Claude 生态广泛使用 |
| **Microsoft Agent Framework** | Microsoft | YAML Frontmatter + Markdown | 企业级、Windows 生态 |
| **OpenAI Assistants API** | OpenAI | JSON | API 驱动、结构化 |
| **LangChain Tools** | LangChain | Python/JSON Schema | 代码优先、灵活 |

---

## 🌟 主流标准详解

### 1. Agent Skills (agentskills.io) - 开放标准

**官方网站：** https://agentskills.io  
**GitHub：** https://github.com/agentskills/agentskills

#### 1.1 目录结构（强制）

```
skill-name/
└── SKILL.md    # 必须：元数据 + 指示
```

**硬性规范：**
- 文件夹名称 `skill-name` 必须与 SKILL.md 中的 `name` 字段完全一致
- 仅支持 **小写字母、数字、连字符（kebab-case）**
- 不可使用中文、大写字母、下划线与特殊符号

#### 1.2 文件格式

`SKILL.md` 由两部分组成：
1. **YAML Frontmatter** - 文件头部的元数据
2. **Markdown 正文** - 自然语言指令

#### 1.3 完整模板

```markdown
---
name: expense-report
description: File and validate employee expense reports according to company policy. Use when asked about expense submissions, reimbursement rules, or spending limits.
license: Apache-2.0
compatibility: Requires python3
metadata:
  emoji: "📊"
  requires:
    env: [API_KEY]
    bins: [curl, jq]
---

# 费用报销技能

## 功能说明
一句话清晰说明本技能做什么、适用场景。

## 参数
- `employee_id` (string): 员工 ID
- `expense_date` (string): 报销日期
- `amount` (number): 报销金额

## 执行步骤
1. 第一步：做什么
2. 第二步：处理数据
3. 第三步：输出结果
```

---

### 2. SKILL.md v3.0 - Claude 生态标准

**GitHub：** https://github.com/402md/skillmd/blob/main/SPEC.md  
**版本：** v3.0（草案）  
**创建日期：** 2026-03-15  
**更新日期：** 2026-03-19

#### 2.1 核心规则

| 规则 | 要求 |
|------|------|
| **主文件** | 始终为 `SKILL.md`（精确名称，大小写敏感） |
| **文件夹名称** | kebab-case（小写、连字符；无空格、下划线或大写） |
| **Frontmatter name** | 必须与文件夹名称匹配 |

#### 2.2 Description 格式

```
description = "[What it does] + [When to use it] + [Key features]"
```

**示例：**
```yaml
description: File and validate employee expense reports according to company policy. Use when asked about expense submissions, reimbursement rules, or spending limits.
```

---

### 3. Microsoft Agent Framework - 企业级标准

**官方文档：** https://learn.microsoft.com/en-us/agent-framework/agents/skills

#### 3.1 格式特点

- YAML Frontmatter + Markdown 正文
- 支持 `license`、`compatibility` 等企业级字段
- 与 Windows、Azure 生态深度集成

#### 3.2 示例

```markdown
---
name: expense-report
description: File and validate employee expense reports according to company policy.
license: Apache-2.0
compatibility: Requires python3
metadata:
  allowed-tools: [file_system, http]
---

# 费用报销技能
...
```

---

### 4. OpenAI Assistants API - API 驱动标准

**官方文档：** https://platform.openai.com/docs/assistants/overview

#### 4.1 核心概念

| 组件 | 说明 |
|------|------|
| **Assistant** | AI 代理的配置或"蓝图"，持久化对象 |
| **Model** | 指定使用的模型（如 gpt-4） |
| **Instructions** | 系统提示，指导 Assistant 的个性、目标和响应方式 |
| **Tools** | 工具/函数调用定义 |
| **Response Format** | 响应格式（json_object、json_schema） |

#### 4.2 工具定义格式（JSON Schema）

```json
{
  "name": "search_database",
  "description": "Search the customer database for records matching the query",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search terms to look for"
      },
      "limit": {
        "type": "integer",
        "description": "Maximum number of results to return",
        "default": 10
      }
    },
    "required": ["query"]
  }
}
```

#### 4.3 结构化输出（JSON Schema）

```json
{
  "type": "json_schema",
  "json_schema": {
    "name": "expense_report",
    "strict": true,
    "schema": {
      "type": "object",
      "properties": {
        "employee_id": {"type": "string"},
        "amount": {"type": "number"},
        "approved": {"type": "boolean"}
      },
      "required": ["employee_id", "amount", "approved"],
      "additionalProperties": false
    }
  }
}
```

---

### 5. LangChain Tools - 代码优先标准

**官方文档：** https://python.langchain.com/docs/modules/agents/tools/custom_tools

#### 5.1 核心字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | str | 是 | 必须在提供给 LLM 或 agent 的工具集中唯一 |
| `description` | str | 否 | 描述工具的功能，作为 LLM 或 agent 的上下文 |
| `args_schema` | Pydantic Model | 否 | 参数 schema |
| `return_direct` | bool | 否 | 默认 false |

#### 5.2 定义方式 1：@tool 装饰器（最简单）

```python
from langchain.tools import tool

@tool
def search_database(query: str, limit: int = 10) -> str:
    """Search the customer database for records matching the query.
    
    Args:
        query: Search terms to look for
        limit: Maximum number of results to return
    """
    # 实现逻辑
    pass
```

#### 5.3 定义方式 2：BaseTool 子类（完整控制）

```python
from langchain_core.tools import BaseTool
from pydantic import BaseModel, Field

class CalculatorInput(BaseModel):
    a: int = Field(description="first number")
    b: int = Field(description="second number")

class CustomCalculatorTool(BaseTool):
    name = "calculator"
    description = "Calculate two numbers"
    args_schema: type[BaseModel] = CalculatorInput
    return_direct: bool = False

    def _run(self, a: int, b: int) -> int:
        return a + b
```

---

## 📊 标准对比总结

| 维度 | Agent Skills | SKILL.md v3.0 | Microsoft | OpenAI | LangChain |
|------|-------------|---------------|-----------|--------|-----------|
| **格式** | YAML+Markdown | YAML+Markdown | YAML+Markdown | JSON | Python/JSON |
| **学习曲线** | 平缓 | 平缓 | 平缓 | 中等 | 陡峭 |
| **AI 友好** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| **企业级** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **跨平台** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **代码集成** | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **生态规模** | 增长中 | 大（Claude） | 中等 | 很大 | 很大 |

---

## 🎯 关键洞察

### 1. 行业趋同：YAML Frontmatter + Markdown

**多个主流标准都采用相同的格式：**
- Agent Skills (agentskills.io)
- SKILL.md v3.0
- Microsoft Agent Framework

**为什么这种格式成为行业标准？**
1. ✅ **对人类友好**：Markdown 易读易写
2. ✅ **对 AI 友好**：自然语言指令，LLM 容易理解
3. ✅ **对机器友好**：YAML Frontmatter 提供结构化元数据
4. ✅ **无需特殊 DSL**：不需要学习新的领域特定语言
5. ✅ **工具生态好**：Git、编辑器、预览工具都支持

### 2. 命名规范统一：kebab-case

**所有标准都要求：**
- 文件夹名称 = SKILL.md 中的 name 字段
- 仅支持：小写字母、数字、连字符（kebab-case）
- 禁止：中文、大写字母、下划线、特殊符号

**示例：**
- ✅ 正确：`expense-report`、`lab-report-audit`
- ❌ 错误：`ExpenseReport`、`费用报销`、`skill_1`

### 3. Description 的黄金公式

**多个标准都推荐相同的 description 格式：**
```
[What it does] + [When to use it] + [Key features]
```

**示例：**
```yaml
description: File and validate employee expense reports according to company policy. Use when asked about expense submissions, reimbursement rules, or spending limits.
```

**为什么重要？**
- 这是 AI 决策的核心依据
- 越准确，AI 触发越稳定
- 必须清晰、完整、第三人称

### 4. 两种设计哲学

| 哲学 | 代表 | 特点 | 适用场景 |
|------|------|------|---------|
| **配置优先** | Agent Skills、SKILL.md、Microsoft | YAML+Markdown、零代码 | 快速开发、简单场景、非程序员 |
| **代码优先** | OpenAI、LangChain | JSON Schema、Python | 复杂场景、生产环境、程序员 |

---

## 💡 对 Aetheris 的启示

### 启示 1：考虑支持行业标准格式

Aetheris 可以考虑支持 **Agent Skills / SKILL.md** 格式，因为：
- ✅ 这是行业趋同的标准
- ✅ 跨平台兼容，可以共享技能
- ✅ 学习曲线平缓，易于 adoption

### 启示 2：保持 Aetheris 的企业级优势

Aetheris 当前的 `.skill.yaml` 格式在企业级特性上有优势：
- ✅ 强类型定义
- ✅ 超时、重试配置
- ✅ 安全沙箱
- ✅ 完整的元数据

### 启示 3：两种格式可以共存

- **生产环境**：使用 Aetheris `.skill.yaml` 格式（规范、安全）
- **快速开发**：支持 Agent Skills / SKILL.md 格式（简单、快速）
- **转换工具**：提供格式双向转换工具

---

## 📚 参考资源

| 资源 | 链接 |
|------|------|
| Agent Skills 规范 | https://github.com/agentskills/agentskills/blob/main/docs/specification.mdx |
| SKILL.md v3.0 | https://github.com/402md/skillmd/blob/main/SPEC.md |
| Microsoft Agent Skills | https://learn.microsoft.com/en-us/agent-framework/agents/skills |
| OpenAI Assistants API | https://platform.openai.com/docs/assistants/overview |
| LangChain Custom Tools | https://python.langchain.com/docs/modules/agents/tools/custom_tools |
| SKILL.md 文章 | https://automationswitch.com/articles/skillmd-files-the-agent-skills-directory |

---

**文档版本：** 1.0.0  
**最后更新：** 2026-04-07
