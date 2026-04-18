# 基本使用

本指南将帮助您了解如何使用 Aetheris 的基本功能，包括通过 API 和 IM 平台使用 Aetheris。

## 通过 API 使用

### 健康检查

```bash
curl http://localhost:3000/api/health
```

响应：

```json
{
  "status": "ok",
  "version": "1.0.0"
}
```

### 创建任务

```bash
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "agent": "industrial"
  }'
```

响应：

```json
{
  "id": "task-123",
  "status": "pending",
  "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
  "agent": "industrial",
  "created_at": "2024-01-01T00:00:00Z"
}
```

### 获取任务状态

```bash
curl http://localhost:3000/api/tasks/task-123
```

响应：

```json
{
  "id": "task-123",
  "status": "completed",
  "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
  "agent": "industrial",
  "created_at": "2024-01-01T00:00:00Z",
  "completed_at": "2024-01-01T00:05:00Z",
  "output": "生产排产计划已生成，设备状态检查完成，原料库存分析完成，化验报告审核完成，最终生产报告已生成。"
}
```

### 执行 Skill

```bash
curl -X POST http://localhost:3000/api/skills/hello-world/execute \
  -H "Content-Type: application/json" \
  -d '{
    "name": "张三",
    "language": "zh",
    "style": "friendly"
  }'
```

响应：

```json
{
  "greeting": "你好，张三！很高兴见到你！有什么我可以帮助你的吗？",
  "language": "zh",
  "style": "friendly"
}
```

## 通过 IM 平台使用

### 企业微信

1. 在企业微信中创建一个应用
2. 配置应用的 webhook 地址为 `http://your-server:3000/api/webhook/wechat`
3. 在应用中发送消息，例如："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
4. 系统会自动处理并回复结果

### 钉钉

1. 在钉钉中创建一个自定义机器人
2. 配置机器人的 webhook 地址为 `http://your-server:3000/api/webhook/dingtalk`
3. 在群聊中 @ 机器人并发送消息，例如："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
4. 机器人会自动处理并回复结果

### 飞书

1. 在飞书中创建一个应用
2. 配置应用的 webhook 地址为 `http://your-server:3000/api/webhook/feishu`
3. 在应用中发送消息，例如："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
4. 系统会自动处理并回复结果

### 个人微信

1. 配置 ILink 适配器
2. 在个人微信中发送消息，例如："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
3. 系统会自动处理并回复结果

## 任务管理

### 列出任务

```bash
curl http://localhost:3000/api/tasks
```

响应：

```json
[
  {
    "id": "task-123",
    "status": "completed",
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "agent": "industrial",
    "created_at": "2024-01-01T00:00:00Z",
    "completed_at": "2024-01-01T00:05:00Z"
  }
]
```

### 停止任务

```bash
curl -X POST http://localhost:3000/api/tasks/task-123/stop
```

响应：

```json
{
  "id": "task-123",
  "status": "stopped",
  "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
  "agent": "industrial",
  "created_at": "2024-01-01T00:00:00Z",
  "stopped_at": "2024-01-01T00:02:00Z"
}
```

## 下一步

- [IM 平台集成](im-integration.md) - 了解如何集成 IM 平台
- [Agent 管理](agent-management.md) - 了解如何管理和配置 Agent
- [Skill 管理](skill-management.md) - 了解如何管理和使用 Skill
