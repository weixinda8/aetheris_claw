# 工业自动化示例应用

## 概述

工业自动化示例应用展示了Aetheris与工业设备的集成能力，支持Modbus、OPC UA等工业协议。该应用集成了生产监控和预测性维护技能，能够连接工业设备、读取传感器数据、监控生产状态，并提供预测性维护建议。

## 功能特性

- **多协议支持**：支持Modbus、OPC UA等工业协议
- **实时监控**：实时读取和监控传感器数据
- **生产状态分析**：分析生产设备的运行状态
- **预测性维护**：基于历史数据预测设备故障
- **数据可视化**：提供生产数据的可视化展示

## 目录结构

```
examples/industrial-automation-app/
├── industrial_agent.yaml  # 代理配置文件
├── demo.py                # 演示脚本
└── README.md              # 本文件
```

## 配置与启动

### 1. 配置代理

将代理配置文件复制到配置目录：

```bash
# 创建配置目录（如果不存在）
mkdir -p ~/.aetheris/agents

# 复制代理配置
cp examples/industrial-automation-app/industrial_agent.yaml ~/.aetheris/agents/
```

### 2. 配置技能

确保生产监控和预测性维护技能已配置：

```bash
# 复制技能配置
mkdir -p ~/.aetheris/agentskills
cp examples/agentskills/production-monitoring/SKILL.md ~/.aetheris/agentskills/
cp examples/agentskills/predictive-maintenance/SKILL.md ~/.aetheris/agentskills/
```

### 3. 配置工业设备

根据实际情况修改 `industrial_agent.yaml` 中的设备配置：

```yaml
industrial:
  protocols:
    - name: "modbus_device"
      type: "modbus"
      config:
        address: "192.168.1.100"  # 修改为实际设备IP
        port: 502
        slave_id: 1
        timeout: 5000
    - name: "opcua_server"
      type: "opcua"
      config:
        endpoint: "opc.tcp://localhost:4840"  # 修改为实际OPC UA服务器地址
        username: "user"  # 修改为实际用户名
        password: "password"  # 修改为实际密码
```

### 4. 启动Aetheris

```bash
aetheris start
```

## 使用演示

运行演示脚本查看工业自动化能力：

```bash
cd examples/industrial-automation-app
python demo.py
```

演示脚本会执行以下操作：
- 连接Modbus设备
- 读取传感器数据（温度、压力、流量、状态）
- 监控生产状态
- 执行预测性维护分析

## API调用示例

### 读取传感器数据

```python
import requests

url = "http://localhost:8080/api/v1/agent/industrial-automation-app/invoke"
payload = {
    "skill": "production_monitoring",
    "parameters": {
        "device_name": "modbus_device",
        "action": "read",
        "registers": [
            {"address": 0, "count": 1, "type": "holding"},  # 温度
            {"address": 1, "count": 1, "type": "holding"}   # 压力
        ]
    }
}

response = requests.post(url, json=payload)
print(response.json())
```

### 预测性维护分析

```python
import requests
import time
import random

# 模拟传感器历史数据
historical_data = []
for i in range(30):
    historical_data.append({
        "timestamp": time.time() - (30 - i) * 3600,
        "temperature": 60 + random.uniform(-2, 2) + i * 0.1,
        "pressure": 10 + random.uniform(-0.5, 0.5),
        "vibration": 0.5 + random.uniform(-0.1, 0.1) + i * 0.01
    })

url = "http://localhost:8080/api/v1/agent/industrial-automation-app/invoke"
payload = {
    "skill": "predictive_maintenance",
    "parameters": {
        "action": "predict",
        "device_name": "modbus_device",
        "sensor_data": historical_data,
        "timeframe": "7d"
    }
}

response = requests.post(url, json=payload)
print(response.json())
```

## 注意事项

- 需要配置实际的工业设备连接信息
- 某些工业协议可能需要额外的依赖库
- 预测性维护功能需要足够的历史数据

## 故障排除

- 确保工业设备网络连接正常
- 检查设备地址、端口和认证信息是否正确
- 验证技能文件是否正确放置

## 扩展建议

- 添加更多工业协议支持（如BACnet、Profibus等）
- 实现更复杂的生产调度算法
- 集成工业数据分析和可视化工具
- 添加远程控制功能