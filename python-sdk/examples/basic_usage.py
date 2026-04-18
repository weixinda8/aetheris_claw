from aetheris import AetherisClient

# 初始化客户端
client = AetherisClient(base_url="http://localhost:3000")

print("=== Aetheris Python SDK 基本使用示例 ===")

# 1. 创建任务
print("\n1. 创建任务...")
task_response = client.create_task(
    description="分析销售数据并生成季度报告",
    priority=8
)
task_id = task_response["id"]
print(f"创建的任务 ID: {task_id}")

# 2. 获取任务状态
print("\n2. 获取任务状态...")
task = client.get_task(task_id)
print(f"任务状态: {task.status}")
print(f"任务描述: {task.description}")
print(f"任务优先级: {task.priority}")

# 3. 列出所有任务
print("\n3. 列出所有任务...")
tasks = client.list_tasks()
print(f"总任务数: {len(tasks)}")
for t in tasks[:3]:  # 只显示前3个
    print(f"  - {t.id}: {t.description} ({t.status})")

# 4. 列出所有 Agent
print("\n4. 列出所有 Agent...")
agents = client.list_agents()
print(f"总 Agent 数: {len(agents)}")
for agent in agents:
    print(f"  - {agent.name} ({agent.agent_type}): {agent.status}")
    print(f"    能力: {', '.join(agent.capabilities[:3])}...")

# 5. 列出所有技能
print("\n5. 列出所有技能...")
skills = client.list_skills()
print(f"总技能数: {len(skills)}")
for skill in skills[:3]:  # 只显示前3个
    print(f"  - {skill.name} (v{skill.version})")
    print(f"    作者: {skill.author}")

print("\n=== 示例完成 ===")
