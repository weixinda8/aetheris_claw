# API 文档

本文档提供了 Aetheris 的完整 API 参考，包括所有端点、参数和响应格式。

## 基础信息

### 基础 URL

```
http://localhost:3000/api/v1
```

### 认证

Aetheris 使用 JWT 进行认证。获取 JWT token 后，在请求头中包含：

```
Authorization: Bearer <your-token>
```

### 响应格式

所有 API 响应都采用 JSON 格式，包含以下字段：

- `success` - 布尔值，表示请求是否成功
- `data` - 请求返回的数据（如果成功）
- `error` - 错误信息（如果失败）
- `code` - 错误代码（如果失败）

## 公共路由

### 健康检查

**GET /health**

检查系统健康状态。

**响应**：

```json
{
  "success": true,
  "data": {
    "status": "ok",
    "version": "1.0.0",
    "components": {
      "llm": "ok",
      "database": "ok",
      "skills": "ok",
      "agents": "ok"
    }
  }
}
```

### 登录

**POST /auth/login**

获取 JWT token。

**请求体**：

```json
{
  "username": "admin",
  "password": "password"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "token": "<jwt-token>",
    "expires_at": "2024-01-01T00:00:00Z"
  }
}
```

### WebSocket 连接

**GET /ws**

建立 WebSocket 连接，用于实时通信。

### Prometheus 指标

**GET /metrics**

获取 Prometheus 格式的系统指标。

## 受保护路由

### 任务管理

#### 创建任务

**POST /tasks**

创建新任务。

**请求体**：

```json
{
  "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
  "agent": "industrial",
  "priority": "high",
  "metadata": {
    "order_id": "12345"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "task-123",
    "status": "pending",
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "agent": "industrial",
    "priority": "high",
    "metadata": {
      "order_id": "12345"
    },
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 列出任务

**GET /tasks**

**查询参数**：

- `status` - 任务状态（可选）
- `agent` - Agent 名称（可选）
- `page` - 页码（默认 1）
- `limit` - 每页数量（默认 10）

**响应**：

```json
{
  "success": true,
  "data": {
    "tasks": [
      {
        "id": "task-123",
        "status": "completed",
        "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
        "agent": "industrial",
        "created_at": "2024-01-01T00:00:00Z",
        "completed_at": "2024-01-01T00:05:00Z"
      }
    ],
    "total": 1,
    "page": 1,
    "limit": 10
  }
}
```

#### 获取任务详情

**GET /tasks/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "task-123",
    "status": "completed",
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "agent": "industrial",
    "output": "生产排产计划已生成，设备状态检查完成，原料库存分析完成，化验报告审核完成，最终生产报告已生成。",
    "created_at": "2024-01-01T00:00:00Z",
    "completed_at": "2024-01-01T00:05:00Z"
  }
}
```

#### 暂停任务

**PUT /tasks/:id/pause**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "task-123",
    "status": "paused",
    "paused_at": "2024-01-01T00:02:00Z"
  }
}
```

#### 恢复任务

**PUT /tasks/:id/resume**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "task-123",
    "status": "in_progress",
    "resumed_at": "2024-01-01T00:03:00Z"
  }
}
```

#### 取消任务

**DELETE /tasks/:id/cancel**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "task-123",
    "status": "cancelled",
    "cancelled_at": "2024-01-01T00:01:00Z"
  }
}
```

### Agent 管理

#### 列出 Agent

**GET /agents**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "name": "code_agent",
      "description": "代码领域 Agent，负责代码审查、生成和优化",
      "status": "active"
    },
    {
      "name": "data_agent",
      "description": "数据领域 Agent，负责数据分析、处理和可视化",
      "status": "active"
    }
  ]
}
```

#### 获取 Agent 详情

**GET /agents/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "name": "industrial_agent",
    "description": "工业制造领域 Agent，负责设备监控、预测维护和生产排产",
    "status": "active",
    "skills": [
      "predictive-maintenance",
      "production-monitoring",
      "lab-report-audit"
    ],
    "tools": [
      "industrial-connector",
      "data-visualization"
    ]
  }
}
```

### 审计事件

#### 列出审计事件

**GET /audit/events**

**查询参数**：

- `task_id` - 任务 ID（可选）
- `user_id` - 用户 ID（可选）
- `action` - 操作类型（可选）
- `start_time` - 开始时间（可选）
- `end_time` - 结束时间（可选）
- `page` - 页码（默认 1）
- `limit` - 每页数量（默认 10）

**响应**：

```json
{
  "success": true,
  "data": {
    "events": [
      {
        "id": "event-123",
        "task_id": "task-123",
        "user_id": "user-123",
        "action": "task_created",
        "details": "Task created: 我们厂接到一个新的化工生产订单需要生成最终的生产报告",
        "timestamp": "2024-01-01T00:00:00Z"
      }
    ],
    "total": 1,
    "page": 1,
    "limit": 10
  }
}
```

#### 获取任务审计

**GET /audit/events/:task\_id**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "event-123",
      "task_id": "task-123",
      "user_id": "user-123",
      "action": "task_created",
      "details": "Task created: 我们厂接到一个新的化工生产订单需要生成最终的生产报告",
      "timestamp": "2024-01-01T00:00:00Z"
    },
    {
      "id": "event-124",
      "task_id": "task-123",
      "user_id": "system",
      "action": "task_completed",
      "details": "Task completed successfully",
      "timestamp": "2024-01-01T00:05:00Z"
    }
  ]
}
```

### 遥测指标

#### 获取指标

**GET /telemetry/metrics**

**响应**：

```json
{
  "success": true,
  "data": {
    "cpu_usage": 40,
    "memory_usage": 60,
    "disk_usage": 30,
    "network_usage": 20,
    "tasks": {
      "total": 100,
      "completed": 95,
      "failed": 5,
      "pending": 0
    },
    "agents": {
      "active": 6,
      "inactive": 0
    }
  }
}
```

### 可观测性

#### 获取系统指标

**GET /observability/system-metrics**

**响应**：

```json
{
  "success": true,
  "data": {
    "cpu": {
      "usage": 40,
      "cores": 8
    },
    "memory": {
      "usage": 60,
      "total": 16384
    },
    "disk": {
      "usage": 30,
      "total": 102400
    },
    "network": {
      "in": 1024,
      "out": 512
    }
  }
}
```

#### 列出任务指标

**GET /observability/task-metrics**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "task_id": "task-123",
      "execution_time": 300,
      "memory_usage": 128,
      "cpu_usage": 50,
      "status": "completed"
    }
  ]
}
```

#### 获取任务指标

**GET /observability/task-metrics/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "task_id": "task-123",
    "execution_time": 300,
    "memory_usage": 128,
    "cpu_usage": 50,
    "status": "completed",
    "steps": [
      {
        "name": "analyze_order",
        "execution_time": 60,
        "status": "completed"
      },
      {
        "name": "check_equipment",
        "execution_time": 90,
        "status": "completed"
      }
    ]
  }
}
```

#### 列出告警

**GET /observability/alerts**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "alert-123",
      "type": "high_cpu_usage",
      "severity": "warning",
      "message": "CPU usage is above 80%",
      "status": "active",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### 创建告警

**POST /observability/alerts**

**请求体**：

```json
{
  "type": "custom_alert",
  "severity": "error",
  "message": "Custom alert message",
  "metadata": {
    "source": "monitoring_system"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "alert-124",
    "type": "custom_alert",
    "severity": "error",
    "message": "Custom alert message",
    "status": "active",
    "metadata": {
      "source": "monitoring_system"
    },
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 解决告警

**PUT /observability/alerts/:id/resolve**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "alert-123",
    "status": "resolved",
    "resolved_at": "2024-01-01T00:05:00Z"
  }
}
```

### 管道管理

#### 创建管道

**POST /pipelines**

**请求体**：

```json
{
  "name": "chemical_production",
  "description": "化工生产管道",
  "steps": [
    {
      "name": "analyze_order",
      "agent": "data_agent",
      "skill": "data-analysis"
    },
    {
      "name": "check_equipment",
      "agent": "industrial_agent",
      "skill": "predictive-maintenance",
      "depends_on": ["analyze_order"]
    }
  ]
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "pipeline-123",
    "name": "chemical_production",
    "description": "化工生产管道",
    "steps": [
      {
        "name": "analyze_order",
        "agent": "data_agent",
        "skill": "data-analysis"
      },
      {
        "name": "check_equipment",
        "agent": "industrial_agent",
        "skill": "predictive-maintenance",
        "depends_on": ["analyze_order"]
      }
    ],
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 列出管道

**GET /pipelines**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "pipeline-123",
      "name": "chemical_production",
      "description": "化工生产管道",
      "status": "active",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### 获取管道详情

**GET /pipelines/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "pipeline-123",
    "name": "chemical_production",
    "description": "化工生产管道",
    "steps": [
      {
        "name": "analyze_order",
        "agent": "data_agent",
        "skill": "data-analysis"
      },
      {
        "name": "check_equipment",
        "agent": "industrial_agent",
        "skill": "predictive-maintenance",
        "depends_on": ["analyze_order"]
      }
    ],
    "status": "active",
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 更新管道

**PUT /pipelines/:id**

**请求体**：

```json
{
  "name": "chemical_production_v2",
  "description": "化工生产管道（版本 2）",
  "steps": [
    {
      "name": "analyze_order",
      "agent": "data_agent",
      "skill": "data-analysis"
    },
    {
      "name": "check_equipment",
      "agent": "industrial_agent",
      "skill": "predictive-maintenance",
      "depends_on": ["analyze_order"]
    },
    {
      "name": "generate_report",
      "agent": "office_agent",
      "skill": "report-generation",
      "depends_on": ["check_equipment"]
    }
  ]
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "pipeline-123",
    "name": "chemical_production_v2",
    "description": "化工生产管道（版本 2）",
    "steps": [
      {
        "name": "analyze_order",
        "agent": "data_agent",
        "skill": "data-analysis"
      },
      {
        "name": "check_equipment",
        "agent": "industrial_agent",
        "skill": "predictive-maintenance",
        "depends_on": ["analyze_order"]
      },
      {
        "name": "generate_report",
        "agent": "office_agent",
        "skill": "report-generation",
        "depends_on": ["check_equipment"]
      }
    ],
    "updated_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 删除管道

**DELETE /pipelines/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "pipeline-123",
    "status": "deleted"
  }
}
```

#### 启动管道

**POST /pipelines/:id/start**

**请求体**：

```json
{
  "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
  "metadata": {
    "order_id": "12345"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "pipeline-exec-123",
    "pipeline_id": "pipeline-123",
    "status": "in_progress",
    "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
    "metadata": {
      "order_id": "12345"
    },
    "started_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 停止管道

**POST /pipelines/:id/stop**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "pipeline-exec-123",
    "status": "stopped",
    "stopped_at": "2024-01-01T00:02:00Z"
  }
}
```

#### 获取管道指标

**GET /pipelines/:id/metrics**

**响应**：

```json
{
  "success": true,
  "data": {
    "pipeline_id": "pipeline-123",
    "executions": 10,
    "success_rate": 90,
    "average_execution_time": 300,
    "failed_executions": 1
  }
}
```

#### 获取管道日志

**GET /pipelines/:id/logs**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "timestamp": "2024-01-01T00:00:00Z",
      "level": "info",
      "message": "Pipeline started: chemical_production"
    },
    {
      "timestamp": "2024-01-01T00:05:00Z",
      "level": "info",
      "message": "Pipeline completed successfully"
    }
  ]
}
```

### 用户管理

#### 列出用户

**GET /users**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "user-123",
      "username": "admin",
      "role": "admin",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

### 告警规则

#### 创建告警规则

**POST /alert-rules**

**请求体**：

```json
{
  "name": "high_cpu_usage",
  "description": "CPU usage above 80%",
  "condition": "cpu_usage > 80",
  "severity": "warning",
  "enabled": true
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "rule-123",
    "name": "high_cpu_usage",
    "description": "CPU usage above 80%",
    "condition": "cpu_usage > 80",
    "severity": "warning",
    "enabled": true,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 列出告警规则

**GET /alert-rules**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "rule-123",
      "name": "high_cpu_usage",
      "description": "CPU usage above 80%",
      "severity": "warning",
      "enabled": true
    }
  ]
}
```

#### 获取告警规则

**GET /alert-rules/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "rule-123",
    "name": "high_cpu_usage",
    "description": "CPU usage above 80%",
    "condition": "cpu_usage > 80",
    "severity": "warning",
    "enabled": true,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 更新告警规则

**PUT /alert-rules/:id**

**请求体**：

```json
{
  "name": "high_cpu_usage",
  "description": "CPU usage above 85%",
  "condition": "cpu_usage > 85",
  "severity": "error",
  "enabled": true
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "rule-123",
    "name": "high_cpu_usage",
    "description": "CPU usage above 85%",
    "condition": "cpu_usage > 85",
    "severity": "error",
    "enabled": true,
    "updated_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 删除告警规则

**DELETE /alert-rules/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "rule-123",
    "status": "deleted"
  }
}
```

#### 静音告警规则

**POST /alert-rules/:id/mute**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "rule-123",
    "muted": true,
    "muted_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 取消静音告警规则

**POST /alert-rules/:id/unmute**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "rule-123",
    "muted": false,
    "unmuted_at": "2024-01-01T00:00:00Z"
  }
}
```

### 告警历史

#### 获取告警历史

**GET /alert-history**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "alert-hist-123",
      "alert_id": "alert-123",
      "type": "high_cpu_usage",
      "severity": "warning",
      "message": "CPU usage is above 80%",
      "status": "active",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### 确认告警

**POST /alert-history/:id/acknowledge**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "alert-hist-123",
    "status": "acknowledged",
    "acknowledged_at": "2024-01-01T00:00:00Z",
    "acknowledged_by": "user-123"
  }
}
```

#### 解决告警历史

**POST /alert-history/:id/resolve**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "alert-hist-123",
    "status": "resolved",
    "resolved_at": "2024-01-01T00:00:00Z",
    "resolved_by": "user-123"
  }
}
```

### 通知渠道

#### 创建通知渠道

**POST /notification-channels**

**请求体**：

```json
{
  "name": "email_notifications",
  "type": "email",
  "config": {
    "email": "admin@example.com"
  },
  "enabled": true
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "channel-123",
    "name": "email_notifications",
    "type": "email",
    "config": {
      "email": "admin@example.com"
    },
    "enabled": true,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 列出通知渠道

**GET /notification-channels**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "channel-123",
      "name": "email_notifications",
      "type": "email",
      "enabled": true
    }
  ]
}
```

### 升级策略

#### 创建升级策略

**POST /escalation-policies**

**请求体**：

```json
{
  "name": "critical_issues",
  "description": "Critical issues escalation policy",
  "steps": [
    {
      "type": "notify",
      "target": "user-123",
      "delay": 0
    },
    {
      "type": "notify",
      "target": "user-456",
      "delay": 300
    }
  ]
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "policy-123",
    "name": "critical_issues",
    "description": "Critical issues escalation policy",
    "steps": [
      {
        "type": "notify",
        "target": "user-123",
        "delay": 0
      },
      {
        "type": "notify",
        "target": "user-456",
        "delay": 300
      }
    ],
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 列出升级策略

**GET /escalation-policies**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "policy-123",
      "name": "critical_issues",
      "description": "Critical issues escalation policy"
    }
  ]
}
```

### 模型管理

#### 注册模型

**POST /models**

**请求体**：

```json
{
  "name": "deepseek-chat",
  "description": "DeepSeek chat model",
  "type": "llm",
  "provider": "deepseek",
  "config": {
    "api_key": "your-api-key",
    "api_base": "https://api.deepseek.com/v1",
    "model": "deepseek-chat"
  },
  "enabled": true
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "model-123",
    "name": "deepseek-chat",
    "description": "DeepSeek chat model",
    "type": "llm",
    "provider": "deepseek",
    "enabled": true,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 列出模型

**GET /models**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "model-123",
      "name": "deepseek-chat",
      "description": "DeepSeek chat model",
      "type": "llm",
      "provider": "deepseek",
      "enabled": true
    }
  ]
}
```

#### 获取模型

**GET /models/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "model-123",
    "name": "deepseek-chat",
    "description": "DeepSeek chat model",
    "type": "llm",
    "provider": "deepseek",
    "config": {
      "api_key": "your-api-key",
      "api_base": "https://api.deepseek.com/v1",
      "model": "deepseek-chat"
    },
    "enabled": true,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 更新模型

**PUT /models/:id**

**请求体**：

```json
{
  "name": "deepseek-chat-v2",
  "description": "DeepSeek chat model (version 2)",
  "config": {
    "api_key": "your-new-api-key",
    "api_base": "https://api.deepseek.com/v1",
    "model": "deepseek-chat"
  },
  "enabled": true
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "model-123",
    "name": "deepseek-chat-v2",
    "description": "DeepSeek chat model (version 2)",
    "config": {
      "api_key": "your-new-api-key",
      "api_base": "https://api.deepseek.com/v1",
      "model": "deepseek-chat"
    },
    "enabled": true,
    "updated_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 删除模型

**DELETE /models/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "model-123",
    "status": "deleted"
  }
}
```

### 推理

#### 运行推理

**POST /inference**

**请求体**：

```json
{
  "model_id": "model-123",
  "input": "你好，请问什么是 Aetheris？",
  "parameters": {
    "temperature": 0.7,
    "max_tokens": 1000
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "inference-123",
    "model_id": "model-123",
    "input": "你好，请问什么是 Aetheris？",
    "output": "Aetheris 是一个 AI 原生、自我进化、分布式、完全可信的复杂任务执行引擎，采用 agentskills.io 行业标准。",
    "execution_time": 1.2,
    "tokens_used": 50
  }
}
```

#### 获取推理指标

**GET /inference/metrics**

**响应**：

```json
{
  "success": true,
  "data": {
    "total_requests": 100,
    "successful_requests": 95,
    "failed_requests": 5,
    "average_execution_time": 1.5,
    "total_tokens_used": 5000
  }
}
```

### 异常检测

#### 检测异常

**POST /anomaly-detection/detect**

**请求体**：

```json
{
  "data": [1.0, 2.0, 3.0, 100.0, 5.0],
  "model_id": "anomaly-model-123",
  "threshold": 0.9
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "anomalies": [
      {
        "index": 3,
        "value": 100.0,
        "score": 0.95
      }
    ]
  }
}
```

#### 列出异常

**GET /anomaly-detection/anomalies**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "anomaly-123",
      "timestamp": "2024-01-01T00:00:00Z",
      "value": 100.0,
      "score": 0.95,
      "status": "detected"
    }
  ]
}
```

#### 获取异常详情

**GET /anomaly-detection/anomalies/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "anomaly-123",
    "timestamp": "2024-01-01T00:00:00Z",
    "value": 100.0,
    "score": 0.95,
    "status": "detected",
    "details": {
      "expected_value": 3.0,
      "deviation": 97.0
    }
  }
}
```

#### 获取异常可视化

**GET /anomaly-detection/visualization**

**响应**：

```json
{
  "success": true,
  "data": {
    "chart_data": [
      {"x": "2024-01-01T00:00:00Z", "y": 1.0, "anomaly": false},
      {"x": "2024-01-01T00:01:00Z", "y": 2.0, "anomaly": false},
      {"x": "2024-01-01T00:02:00Z", "y": 3.0, "anomaly": false},
      {"x": "2024-01-01T00:03:00Z", "y": 100.0, "anomaly": true},
      {"x": "2024-01-01T00:04:00Z", "y": 5.0, "anomaly": false}
    ]
  }
}
```

#### 训练异常检测模型

**POST /anomaly-detection/fit**

**请求体**：

```json
{
  "data": [1.0, 2.0, 3.0, 4.0, 5.0],
  "model_name": "anomaly-model-123"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "model_id": "anomaly-model-123",
    "status": "trained",
    "training_time": 2.5
  }
}
```

### 预测

#### 单步预测

**POST /forecasting/forecast**

**请求体**：

```json
{
  "data": [1.0, 2.0, 3.0, 4.0, 5.0],
  "model_id": "forecast-model-123",
  "horizon": 3
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "forecast": [6.0, 7.0, 8.0],
    "confidence_intervals": [
      [5.5, 6.5],
      [6.5, 7.5],
      [7.5, 8.5]
    ]
  }
}
```

#### 多步预测

**POST /forecasting/multi-step**

**请求体**：

```json
{
  "data": [1.0, 2.0, 3.0, 4.0, 5.0],
  "model_id": "forecast-model-123",
  "horizon": 3,
  "steps": 2
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "forecasts": [
      [6.0, 7.0, 8.0],
      [9.0, 10.0, 11.0]
    ]
  }
}
```

#### 自动选择模型

**POST /forecasting/auto-select**

**请求体**：

```json
{
  "data": [1.0, 2.0, 3.0, 4.0, 5.0],
  "horizon": 3
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "best_model": "linear_regression",
    "forecast": [6.0, 7.0, 8.0],
    "metrics": {
      "mae": 0.1,
      "rmse": 0.15
    }
  }
}
```

#### 获取预测历史

**GET /forecasting/history**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "forecast-123",
      "model_id": "forecast-model-123",
      "timestamp": "2024-01-01T00:00:00Z",
      "horizon": 3,
      "forecast": [6.0, 7.0, 8.0]
    }
  ]
}
```

### 知识图谱

#### 添加实体

**POST /knowledge-graph/entities**

**请求体**：

```json
{
  "id": "entity-123",
  "type": "equipment",
  "properties": {
    "name": "Pump A",
    "location": "Factory Floor 1",
    "status": "operational"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "entity-123",
    "type": "equipment",
    "properties": {
      "name": "Pump A",
      "location": "Factory Floor 1",
      "status": "operational"
    }
  }
}
```

#### 添加关系

**POST /knowledge-graph/relationships**

**请求体**：

```json
{
  "source": "entity-123",
  "target": "entity-456",
  "type": "connected_to",
  "properties": {
    "connection_type": "fluid"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "relationship-123",
    "source": "entity-123",
    "target": "entity-456",
    "type": "connected_to",
    "properties": {
      "connection_type": "fluid"
    }
  }
}
```

#### 搜索知识图谱

**POST /knowledge-graph/search**

**请求体**：

```json
{
  "query": "Pump A",
  "limit": 10
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "entities": [
      {
        "id": "entity-123",
        "type": "equipment",
        "properties": {
          "name": "Pump A",
          "location": "Factory Floor 1",
          "status": "operational"
        }
      }
    ]
  }
}
```

#### 获取维护案例

**POST /knowledge-graph/cases**

**请求体**：

```json
{
  "equipment_id": "entity-123",
  "limit": 10
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "cases": [
      {
        "id": "case-123",
        "equipment_id": "entity-123",
        "problem": "Pump not working",
        "solution": "Replace motor",
        "timestamp": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

#### 添加维护案例

**POST /knowledge-graph/cases/add**

**请求体**：

```json
{
  "equipment_id": "entity-123",
  "problem": "Pump making noise",
  "solution": "Lubricate bearings",
  "metadata": {
    "maintenance_tech": "John Doe"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "case-456",
    "equipment_id": "entity-123",
    "problem": "Pump making noise",
    "solution": "Lubricate bearings",
    "metadata": {
      "maintenance_tech": "John Doe"
    },
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

#### 获取图谱可视化

**GET /knowledge-graph/visualization**

**响应**：

```json
{
  "success": true,
  "data": {
    "nodes": [
      {
        "id": "entity-123",
        "label": "Pump A",
        "type": "equipment"
      },
      {
        "id": "entity-456",
        "label": "Tank B",
        "type": "storage"
      }
    ],
    "edges": [
      {
        "source": "entity-123",
        "target": "entity-456",
        "label": "connected_to"
      }
    ]
  }
}
```

### 自适应学习

#### 提交反馈

**POST /adaptive-learning/feedback**

**请求体**：

```json
{
  "task_id": "task-123",
  "model_id": "model-123",
  "feedback": "good",
  "comments": "The model provided accurate results"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "feedback-123",
    "task_id": "task-123",
    "model_id": "model-123",
    "feedback": "good",
    "comments": "The model provided accurate results",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

#### 列出反馈

**GET /adaptive-learning/feedback**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "feedback-123",
      "task_id": "task-123",
      "model_id": "model-123",
      "feedback": "good",
      "comments": "The model provided accurate results",
      "timestamp": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### 创建模型版本

**POST /adaptive-learning/versions**

**请求体**：

```json
{
  "model_id": "model-123",
  "version": "v2",
  "description": "Updated model with new training data"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "model_id": "model-123",
    "version": "v2",
    "description": "Updated model with new training data",
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 回滚模型

**POST /adaptive-learning/versions/:model\_id/:version\_id/rollback**

**响应**：

```json
{
  "success": true,
  "data": {
    "model_id": "model-123",
    "version": "v1",
    "status": "active",
    "rolled_back_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 开始 A/B 测试

**POST /adaptive-learning/ab-tests**

**请求体**：

```json
{
  "name": "model_comparison",
  "model_a": "model-123",
  "model_b": "model-456",
  "sample_size": 100
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "ab-test-123",
    "name": "model_comparison",
    "model_a": "model-123",
    "model_b": "model-456",
    "sample_size": 100,
    "status": "running",
    "started_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 获取 A/B 测试结果

**GET /adaptive-learning/ab-tests/:id/result**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "ab-test-123",
    "name": "model_comparison",
    "model_a": "model-123",
    "model_b": "model-456",
    "status": "completed",
    "results": {
      "model_a": {
        "success_rate": 0.9,
        "average_execution_time": 1.5
      },
      "model_b": {
        "success_rate": 0.95,
        "average_execution_time": 1.2
      }
    },
    "conclusion": "Model B performs better"
  }
}
```

### Agent 通信

#### 发送消息

**POST /agent-communication/send**

**请求体**：

```json
{
  "from_agent": "data_agent",
  "to_agent": "industrial_agent",
  "message": "Raw material inventory analyzed: sufficient for production",
  "metadata": {
    "order_id": "12345"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "message-123",
    "from_agent": "data_agent",
    "to_agent": "industrial_agent",
    "message": "Raw material inventory analyzed: sufficient for production",
    "metadata": {
      "order_id": "12345"
    },
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

#### 获取消息

**GET /agent-communication/messages/:agent\_id**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "id": "message-123",
      "from_agent": "data_agent",
      "to_agent": "industrial_agent",
      "message": "Raw material inventory analyzed: sufficient for production",
      "metadata": {
        "order_id": "12345"
      },
      "timestamp": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### 订阅主题

**POST /agent-communication/subscribe**

**请求体**：

```json
{
  "agent_id": "industrial_agent",
  "topic": "production_updates"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "agent_id": "industrial_agent",
    "topic": "production_updates",
    "subscribed_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 取消订阅主题

**POST /agent-communication/unsubscribe**

**请求体**：

```json
{
  "agent_id": "industrial_agent",
  "topic": "production_updates"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "agent_id": "industrial_agent",
    "topic": "production_updates",
    "unsubscribed_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 注册 Agent

**POST /agent-communication/agents/register**

**请求体**：

```json
{
  "agent_id": "custom_agent",
  "name": "Custom Agent",
  "description": "A custom agent for specific tasks"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "agent_id": "custom_agent",
    "name": "Custom Agent",
    "description": "A custom agent for specific tasks",
    "registered_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 注销 Agent

**DELETE /agent-communication/agents/:agent\_id/unregister**

**响应**：

```json
{
  "success": true,
  "data": {
    "agent_id": "custom_agent",
    "unregistered_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 列出主题

**GET /agent-communication/topics**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "name": "production_updates",
      "subscribers_count": 2
    },
    {
      "name": "maintenance_alerts",
      "subscribers_count": 1
    }
  ]
}
```

#### 获取主题订阅者

**GET /agent-communication/topics/:topic/subscribers**

**响应**：

```json
{
  "success": true,
  "data": [
    {
      "agent_id": "industrial_agent",
      "subscribed_at": "2024-01-01T00:00:00Z"
    },
    {
      "agent_id": "ops_agent",
      "subscribed_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

### 任务分解

#### 分解任务

**POST /task-decomposer/decompose**

**请求体**：

```json
{
  "input": "我们厂接到一个新的化工生产订单需要生成最终的生产报告",
  "agent": "industrial"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "tasks": [
      {
        "id": "subtask-1",
        "name": "分析订单需求和原料库存",
        "agent": "data_agent",
        "dependencies": []
      },
      {
        "id": "subtask-2",
        "name": "检查设备状态和预测维护",
        "agent": "industrial_agent",
        "dependencies": ["subtask-1"]
      },
      {
        "id": "subtask-3",
        "name": "生成生产排产计划",
        "agent": "ops_agent",
        "dependencies": ["subtask-1", "subtask-2"]
      },
      {
        "id": "subtask-4",
        "name": "审核相关化验报告",
        "agent": "compliance_agent",
        "dependencies": ["subtask-1"]
      },
      {
        "id": "subtask-5",
        "name": "生成最终生产报告",
        "agent": "office_agent",
        "dependencies": ["subtask-2", "subtask-3", "subtask-4"]
      }
    ]
  }
}
```

### Agent 匹配

#### 匹配 Agent

**POST /agent-matcher/match**

**请求体**：

```json
{
  "input": "我需要分析一些数据并生成报告"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "agents": [
      {
        "name": "data_agent",
        "score": 0.95
      },
      {
        "name": "office_agent",
        "score": 0.8
      }
    ]
  }
}
```

### 数据治理

#### 记录数据血缘

**POST /data-governance/lineage/record**

**请求体**：

```json
{
  "source": "raw_data",
  "target": "processed_data",
  "operation": "transformation",
  "metadata": {
    "job_id": "job-123",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "lineage-123",
    "source": "raw_data",
    "target": "processed_data",
    "operation": "transformation",
    "metadata": {
      "job_id": "job-123",
      "timestamp": "2024-01-01T00:00:00Z"
    },
    "recorded_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 获取数据血缘

**GET /data-governance/lineage/:id**

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "lineage-123",
    "source": "raw_data",
    "target": "processed_data",
    "operation": "transformation",
    "metadata": {
      "job_id": "job-123",
      "timestamp": "2024-01-01T00:00:00Z"
    },
    "recorded_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 查询上游数据

**POST /data-governance/lineage/query-upstream**

**请求体**：

```json
{
  "data_id": "processed_data",
  "depth": 3
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "data_id": "processed_data",
    "upstream": [
      {
        "id": "lineage-123",
        "source": "raw_data",
        "target": "processed_data",
        "operation": "transformation"
      }
    ]
  }
}
```

#### 查询下游数据

**POST /data-governance/lineage/query-downstream**

**请求体**：

```json
{
  "data_id": "raw_data",
  "depth": 3
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "data_id": "raw_data",
    "downstream": [
      {
        "id": "lineage-123",
        "source": "raw_data",
        "target": "processed_data",
        "operation": "transformation"
      }
    ]
  }
}
```

#### 获取数据血缘图

**GET /data-governance/lineage/:id/graph**

**响应**：

```json
{
  "success": true,
  "data": {
    "nodes": [
      {
        "id": "raw_data",
        "label": "Raw Data"
      },
      {
        "id": "processed_data",
        "label": "Processed Data"
      }
    ],
    "edges": [
      {
        "source": "raw_data",
        "target": "processed_data",
        "label": "transformation"
      }
    ]
  }
}
```

#### 获取影响分析

**POST /data-governance/lineage/impact-analysis**

**请求体**：

```json
{
  "data_id": "raw_data"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "data_id": "raw_data",
    "impacted": [
      {
        "data_id": "processed_data",
        "operation": "transformation"
      }
    ]
  }
}
```

#### 导出数据血缘

**POST /data-governance/lineage/export**

**请求体**：

```json
{
  "format": "json"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "lineage": [
      {
        "id": "lineage-123",
        "source": "raw_data",
        "target": "processed_data",
        "operation": "transformation"
      }
    ]
  }
}
```

#### 持久化数据血缘

**POST /data-governance/lineage/persist**

**请求体**：

```json
{
  "lineage": [
    {
      "source": "raw_data",
      "target": "processed_data",
      "operation": "transformation"
    }
  ]
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "status": "persisted",
    "count": 1
  }
}
```

#### 加载数据血缘

**POST /data-governance/lineage/load**

**请求体**：

```json
{
  "query": {
    "source": "raw_data"
  }
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "lineage": [
      {
        "id": "lineage-123",
        "source": "raw_data",
        "target": "processed_data",
        "operation": "transformation"
      }
    ]
  }
}
```

#### 数据分类

**POST /data-governance/classification/classify**

**请求体**：

```json
{
  "data": "This contains sensitive information like email: user@example.com",
  "model_id": "classification-model-123"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "classification": "PII",
    "confidence": 0.95,
    "entities": [
      {
        "type": "email",
        "value": "user@example.com",
        "start": 38,
        "end": 55
      }
    ]
  }
}
```

#### 数据掩码

**POST /data-governance/masking/mask**

**请求体**：

```json
{
  "data": "This contains sensitive information like email: user@example.com",
  "masking_type": "email"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "masked_data": "This contains sensitive information like email: u***@***********",
    "masked_count": 1
  }
}
```

## 管理员路由

### 用户管理

#### 创建用户

**POST /users**

**请求体**：

```json
{
  "username": "user1",
  "password": "password123",
  "role": "user"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "user-456",
    "username": "user1",
    "role": "user",
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### 更新用户角色

**PUT /users/:id/role**

**请求体**：

```json
{
  "role": "admin"
}
```

**响应**：

```json
{
  "success": true,
  "data": {
    "id": "user-456",
    "username": "user1",
    "role": "admin",
    "updated_at": "2024-01-01T00:00:00Z"
  }
}
```

## 错误代码

| 错误代码 | 描述      | 解决方案            |
| ---- | ------- | --------------- |
| 400  | 无效的请求参数 | 检查请求参数是否符合要求    |
| 401  | 未授权     | 检查 API Key 是否正确 |
| 403  | 禁止访问    | 检查用户权限是否足够      |
| 404  | 资源不存在   | 检查资源路径是否正确      |
| 500  | 内部服务器错误 | 查看日志以获取更多信息     |
| 502  | 网关错误    | 检查 LLM 服务是否可访问  |
| 503  | 服务不可用   | 检查系统资源是否充足      |
| 504  | 网关超时    | 检查 LLM 服务响应时间   |

## 速率限制

Aetheris 对 API 请求实施速率限制，以防止滥用：

- 公共 API：60 次请求/分钟/IP
- 受保护 API：120 次请求/分钟/用户
- 管理员 API：60 次请求/分钟/用户

如果超过速率限制，您将收到 429 错误。

## 最佳实践

1. **使用 HTTPS**：在生产环境中，始终使用 HTTPS 保护 API 通信
2. **合理使用速率限制**：遵守速率限制，避免过度请求
3. **处理错误**：正确处理 API 错误，尤其是 429（速率限制）和 500（内部错误）
4. **使用分页**：对于返回大量数据的 API，使用分页参数
5. **缓存响应**：对于频繁访问但不经常变化的数据，考虑缓存响应
6. **监控 API 使用**：监控 API 使用情况，及时发现异常

## 下一步

- [用户指南](../user-guide/README.md) - 了解如何使用 Aetheris
- [教程](../tutorials/README.md) - 了解常见用例和示例教程
- [参考](../reference/README.md) - 了解技术参考和最佳实践

