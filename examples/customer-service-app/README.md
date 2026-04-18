# 客户服务示例应用

## 概述

客户服务示例应用展示了Aetheris的智能客服能力，支持多渠道客服、问题分类和自动回复。该应用集成了Web搜索、邮件编写和文件操作技能，能够理解客户问题，提供准确的解答，并在需要时升级问题。

## 功能特性

- **多渠道支持**：支持企业微信、钉钉、飞书等多个IM平台
- **问题分类**：自动分类客户问题，提高处理效率
- **知识库查询**：基于向量数据库快速检索相关信息
- **智能回复**：基于LLM提供专业、友好的回答
- **邮件处理**：自动生成邮件回复，提升客服效率
- **多轮对话**：支持多轮对话，记住客户历史问题
- **人工介入**：在需要时可以触发人工介入流程

## 目录结构

```
examples/customer-service-app/
├── customer_service_agent.yaml  # 代理配置文件
└── README.md                     # 本文件
```

## 配置与启动

### 1. 配置代理

将代理配置文件复制到配置目录：

```bash
# 创建配置目录（如果不存在）
mkdir -p ~/.aetheris/agents

# 复制代理配置
cp examples/customer-service-app/customer_service_agent.yaml ~/.aetheris/agents/
```

### 2. 配置技能

确保所需技能已配置：

```bash
# 复制技能配置
mkdir -p ~/.aetheris/agentskills
cp examples/agentskills/web-search/SKILL.md ~/.aetheris/agentskills/
cp examples/agentskills/email-composer/SKILL.md ~/.aetheris/agentskills/
cp examples/agentskills/file-operations/SKILL.md ~/.aetheris/agentskills/
```

### 3. 配置IM平台

根据实际情况修改环境变量：

```bash
# 企业微信配置
export WECHAT_WORK_WEBHOOK="https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=your-key"
export WECHAT_WORK_SECRET="your-secret"
export WECHAT_WORK_CORP_ID="your-corp-id"

# 钉钉配置
export DINGTALK_WEBHOOK="https://oapi.dingtalk.com/robot/send?access_token=your-token"
export DINGTALK_SECRET="your-secret"

# 飞书配置
export FEISHU_WEBHOOK="https://open.feishu.cn/open-apis/bot/v2/hook/your-hook"
export FEISHU_APP_ID="your-app-id"
export FEISHU_APP_SECRET="your-app-secret"
```

### 4. 配置向量数据库

确保Qdrant向量数据库已启动：

```bash
docker run -d -p 6333:6333 qdrant/qdrant
```

### 5. 启动Aetheris

```bash
aetheris start
```

## 使用演示

### 1. 企业微信集成

将企业微信机器人的webhook配置好后，客户可以直接在企业微信中提问。

### 2. 钉钉集成

将钉钉机器人的webhook配置好后，客户可以直接在钉钉中提问。

### 3. 飞书集成

将飞书机器人的webhook配置好后，客户可以直接在飞书中提问。

## API调用示例

### 发送消息给客服

```python
import requests

url = "http://localhost:8080/api/v1/agent/customer-service-app/chat"
payload = {
    "message": "我的订单什么时候发货？",
    "user_id": "user_001",
    "channel": "wechat_work"
}

response = requests.post(url, json=payload)
print(response.json())
```

### 查看对话历史

```python
import requests

url = "http://localhost:8080/api/v1/agent/customer-service-app/history"
params = {
    "user_id": "user_001",
    "limit": 10
}

response = requests.get(url, params=params)
print(response.json())
```

### 触发人工介入

```python
import requests

url = "http://localhost:8080/api/v1/agent/customer-service-app/escalate"
payload = {
    "ticket_id": "ticket_001",
    "reason": "客户要求人工客服",
    "user_id": "user_001"
}

response = requests.post(url, json=payload)
print(response.json())
```

## 常见问题分类

应用支持自动识别以下常见问题类型：

- **订单查询**：订单状态、发货时间、物流信息
- **产品咨询**：产品功能、使用方法、规格参数
- **技术支持**：故障排查、使用指导、Bug反馈
- **退款售后**：退款申请、退换货、投诉处理
- **账户问题**：登录问题、密码重置、账户安全
- **其他问题**：无法分类的问题，自动转人工

## 注意事项

- 需要配置LLM访问权限
- 需要配置IM平台的Webhook地址
- 向量数据库需要预先导入知识库
- 复杂问题可能需要人工介入

## 故障排除

- 确保Aetheris服务正在运行
- 检查IM平台Webhook配置是否正确
- 验证技能文件是否正确放置
- 检查向量数据库连接是否正常

## 扩展建议

- 添加更多IM平台支持（如Slack、Discord等）
- 集成语音客服功能
- 实现情感分析，识别客户情绪
- 添加工单管理系统
- 实现客服质量评估
- 添加知识库自动更新功能
