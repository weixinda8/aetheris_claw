# Aetheris 2.0 示例配置文件

本目录包含 Aetheris 2.0 的示例配置文件，帮助你快速开始使用统一数字员工与 Agent/Skill 生态系统。

## 目录结构

```
examples/
├── README.md                    # 本文件
├── default.soul.md             # 默认 SOUL 人格文件
├── aetheris.example.json5      # 示例配置文件
└── agentskills/                # AgentSkills 清单目录
    ├── web_search.skill.yaml      # Web 搜索 Skill
    ├── file_operations.skill.yaml # 文件操作 Skill
    └── code_generation.skill.yaml # 代码生成 Skill
```

## 快速开始

### 1. 配置文件

复制示例配置文件到你的配置目录：

```bash
# 创建配置目录
mkdir -p ~/.aetheris

# 复制配置文件
cp examples/aetheris.example.json5 ~/.aetheris/aetheris.json5

# 复制 SOUL 文件
mkdir -p ~/.aetheris/souls
cp examples/default.soul.md ~/.aetheris/souls/

# 复制 AgentSkills
mkdir -p ~/.aetheris/agentskills
cp examples/agentskills/*.skill.yaml ~/.aetheris/agentskills/
```

### 2. 使用 onboard 向导

运行 onboard 向导来完成初始化设置：

```bash
aetheris onboard
```

### 3. 验证配置

运行健康检查来验证配置：

```bash
aetheris doctor
```

## 配置文件说明

### SOUL.md - 数字员工人格文件

SOUL.md 是数字员工的人格定义文件，包含 YAML frontmatter 和 Markdown 内容。

**YAML frontmatter 字段：**
- `name`: 人格名称
- `version`: 版本号
- `author`: 作者
- `created_at`: 创建时间
- `updated_at`: 更新时间
- `personality`: 人格特征
  - `tone`: 语气风格
  - `style`: 对话风格
  - `language`: 语言
  - `humor_level`: 幽默程度 (0-1)
  - `formality`: 正式程度 (0-1)
- `capabilities`: 能力列表
- `skills`: 关联的 Skills 和优先级
- `preferences`: 偏好设置
- `tags`: 标签

**示例：**
```yaml
---
name: "Aetheris Assistant"
version: "1.0.0"
personality:
  tone: "friendly"
  style: "helpful"
---

我是 Aetheris Assistant，一个友好且专业的 AI 助手。
```

### aetheris.json5 - 主配置文件

JSON5 格式的配置文件，包含所有系统设置。

**主要配置项：**
- `openclaw`: OpenClaw 兼容设置
- `soul`: SOUL 人格系统配置
- `skills`: Skill 系统配置（优先级、AgentSkills、ClawHub）
- `security`: 安全模型配置
- `runtime`: 运行时配置
- `api`: API 服务配置

### AgentSkills 清单

AgentSkills 是 Skill 的清单格式，支持 YAML 和 JSON。

**结构：**
- `api_version`: API 版本
- `kind`: 资源类型
- `metadata`: 元数据（名称、版本、描述、分类、标签、优先级）
- `spec`: 规范定义
  - `parameters`: 参数定义
  - `returns`: 返回值定义
  - `examples`: 使用示例
  - `security`: 安全要求
  - `implementation`: 实现方式

## 6 级 Skill 优先级

1. **Mandatory** - 强制加载（核心功能）
2. **High** - 高优先级（预加载）
3. **Medium** - 中优先级（按需加载）
4. **Low** - 低优先级（懒加载）
5. **OnDemand** - 按需加载（仅在请求时）
6. **Disabled** - 禁用

## 四层安全模型

1. **RuleBlock** - 规则拦截
2. **SandboxIsolation** - 沙箱隔离
3. **ThreeLayerQualityCheck** - 三层质量检查
4. **AuditSigning** - 审计签名

## 下一步

- 查看 [Aetheris 2.0 产品需求文档](../.trae/specs/aetheris-20-unified-ecosystem/spec.md)
- 运行 `aetheris --help` 查看所有可用命令
- 访问 [ClawHub 市场](https://clawhub.aetheris.io) 发现更多 Skills
