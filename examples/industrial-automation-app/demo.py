#!/usr/bin/env python3
"""
工业自动化示例应用演示脚本
展示与工业设备集成能力，支持Modbus、OPC UA等工业协议
"""

import json
import requests
import time
import random

def invoke_industrial_skill(skill, parameters):
    """调用工业自动化相关技能"""
    url = "http://localhost:8080/api/v1/agent/industrial-automation-app/invoke"
    payload = {
        "skill": skill,
        "parameters": parameters
    }
    
    try:
        response = requests.post(url, json=payload, timeout=60)
        response.raise_for_status()
        return response.json()
    except requests.exceptions.RequestException as e:
        print(f"请求失败: {e}")
        return None

def demo_modbus_connection():
    """演示Modbus设备连接"""
    print("\n=== 演示 Modbus 设备连接 ===")
    parameters = {
        "device_name": "modbus_device",
        "action": "connect"
    }
    result = invoke_industrial_skill("production_monitoring", parameters)
    
    if result:
        print("Modbus设备连接结果:")
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print("Modbus设备连接失败")

def demo_read_sensor_data():
    """演示读取传感器数据"""
    print("\n=== 演示读取传感器数据 ===")
    parameters = {
        "device_name": "modbus_device",
        "action": "read",
        "registers": [
            {"address": 0, "count": 1, "type": "holding"},  # 温度
            {"address": 1, "count": 1, "type": "holding"},  # 压力
            {"address": 2, "count": 1, "type": "holding"},  # 流量
            {"address": 3, "count": 1, "type": "coil"}     # 状态
        ]
    }
    result = invoke_industrial_skill("production_monitoring", parameters)
    
    if result:
        print("传感器数据读取结果:")
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print("传感器数据读取失败")

def demo_production_monitoring():
    """演示生产状态监控"""
    print("\n=== 演示生产状态监控 ===")
    parameters = {
        "action": "monitor",
        "devices": ["modbus_device", "opcua_server"],
        "metrics": ["temperature", "pressure", "flow", "status"]
    }
    result = invoke_industrial_skill("production_monitoring", parameters)
    
    if result:
        print("生产状态监控结果:")
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print("生产状态监控失败")

def demo_predictive_maintenance():
    """演示预测性维护"""
    print("\n=== 演示预测性维护 ===")
    # 模拟传感器历史数据
    historical_data = []
    for i in range(30):
        historical_data.append({
            "timestamp": time.time() - (30 - i) * 3600,
            "temperature": 60 + random.uniform(-2, 2) + i * 0.1,  # 温度逐渐升高
            "pressure": 10 + random.uniform(-0.5, 0.5),
            "vibration": 0.5 + random.uniform(-0.1, 0.1) + i * 0.01  # 振动逐渐增加
        })
    
    parameters = {
        "action": "predict",
        "device_name": "modbus_device",
        "sensor_data": historical_data,
        "timeframe": "7d"
    }
    result = invoke_industrial_skill("predictive_maintenance", parameters)
    
    if result:
        print("预测性维护分析结果:")
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print("预测性维护分析失败")

def demo_industrial_scenario():
    """演示完整工业场景"""
    print("\n=== 演示完整工业场景 ===")
    print("1. 连接工业设备...")
    demo_modbus_connection()
    time.sleep(2)
    
    print("\n2. 读取传感器数据...")
    demo_read_sensor_data()
    time.sleep(2)
    
    print("\n3. 监控生产状态...")
    demo_production_monitoring()
    time.sleep(2)
    
    print("\n4. 预测性维护分析...")
    demo_predictive_maintenance()

def main():
    """主函数"""
    print("工业自动化示例应用演示")
    print("=" * 60)
    
    # 运行完整工业场景演示
    demo_industrial_scenario()
    
    print("\n" + "=" * 60)
    print("演示完成！")

if __name__ == "__main__":
    main()