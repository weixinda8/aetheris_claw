# Aetheris Python SDK Alpha

## 目录结构

```
python-sdk/
├── aetheris/
│   ├── __init__.py
│   ├── client.py      # HTTP 客户端
│   ├── models.py      # 数据模型
│   └── exceptions.py  # 异常处理
├── examples/
│   ├── basic_usage.py
│   └── agent_usage.py
├── setup.py
├── README.md
└── requirements.txt
```

## 核心功能

- 与 Aetheris API 通信
- 任务管理（创建、查询、停止）
- Agent 管理（列表、详情）
- 技能管理
- 工业协议集成

## 安装

```bash
pip install aetheris-sdk
```

## 基本使用

```python
from aetheris import AetherisClient

# 初始化客户端
client = AetherisClient(base_url="http://localhost:3000")

# 创建任务
response = client.create_task("分析销售数据并生成报告")
task_id = response["task_id"]

# 查询任务状态
status = client.get_task_status(task_id)
print(f"Task status: {status}")

# 列出所有 Agent
agents = client.list_agents()
for agent in agents:
    print(f"Agent: {agent['name']} ({agent['type']})")
```
