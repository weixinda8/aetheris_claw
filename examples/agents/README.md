# Aetheris Agent 配置示例

本目录包含了 Aetheris Agent 的配置示例文件，展示了如何使用 YAML 和 JSON5 格式配置不同类型的 Agent。

## 📁 文件列表

| 文件 | 格式 | 说明 |
|------|------|------|
| `code_agent.yaml` | YAML | 代码助手配置 |
| `office_agent.yaml` | YAML | 办公助手配置 |
| `data_agent.yaml` | YAML | 数据分析师配置 |
| `ops_agent.yaml` | YAML | 运维助手配置 |
| `code_agent.json5` | JSON5 | 代码助手配置 (JSON5 格式) |

## 🤖 Agent 类型说明

### 1. 代码助手 (code_agent)
- **用途**: 软件开发、代码审查、调试、重构
- **推荐模型**: GPT-4
- **启用技能**: 代码生成、文件操作、网页搜索
- **IM 平台**: 企业微信
- **安全配置**: Docker 沙箱，规则拦截

### 2. 办公助手 (office_agent)
- **用途**: 文档处理、日程安排、团队协作
- **推荐模型**: GPT-3.5-turbo
- **启用技能**: 文件操作
- **IM 平台**: 钉钉、飞书
- **安全配置**: 人工干预启用

### 3. 数据分析师 (data_agent)
- **用途**: 数据分析、报告生成、数据可视化
- **推荐模型**: GPT-4
- **启用技能**: 文件操作、网页搜索
- **IM 平台**: 飞书
- **安全配置**: Docker 沙箱，Python/SQL 命令白名单

### 4. 运维助手 (ops_agent)
- **用途**: 系统监控、故障排查、自动化运维
- **推荐模型**: GPT-4
- **启用技能**: 文件操作、网页搜索
- **IM 平台**: 企业微信、微信
- **安全配置**: Docker 沙箱，Kubectl/Docker 等命令白名单

## 🚀 使用方法

### 验证配置

```bash
aetheris agent validate examples/agents/code_agent.yaml
```

### 创建 Agent

```bash
# 从 YAML 配置创建
aetheris agent create examples/agents/code_agent.yaml

# 从 JSON5 配置创建
aetheris agent create examples/agents/code_agent.json5
```

### 从模板创建

```bash
aetheris agent template code_agent \
  --name "我的代码助手" \
  --description "自定义代码助手" \
  --var model=gpt-4
```

## 🔧 配置说明

### 环境变量替换

配置文件支持 `${VAR_NAME}` 语法进行环境变量替换：

```yaml
channels:
  wechat_work:
    webhook_url: "${WECHAT_WORK_WEBHOOK}"
    secret: "${WECHAT_WORK_SECRET}"
```

### 四大 IM 平台配置

| 平台 | 必需配置项 |
|------|-----------|
| 企业微信 | webhook_url, secret, corp_id, agent_id |
| 钉钉 | webhook_url, secret |
| 飞书 | webhook_url, app_id, app_secret |
| 微信 | ilink_url, app_id, app_secret |

## 📝 配置文件结构

```yaml
meta:              # 元数据
persona:           # 角色设定
model:             # 模型配置
skills:            # 技能配置
channels:          # 渠道配置
memory:            # 记忆配置
security:          # 安全配置
scheduler:         # 调度器配置
```

详细的配置说明请参考代码文档。

## 🎯 下一步

- 查看更多模板: `aetheris agent templates`
- 从模板创建 Agent: `aetheris agent template`
- 列出所有 Agent: `aetheris agent list`
