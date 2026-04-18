# 代码生成示例应用

## 概述

代码生成示例应用展示了Aetheris的AI辅助编程能力，支持多种编程语言的代码生成。该应用集成了代码生成、文件操作和Web搜索技能，能够根据用户需求生成高质量的代码。

## 功能特性

- **多语言支持**：支持Python、Rust、JavaScript、TypeScript等多种编程语言
- **框架集成**：可指定使用特定的框架（如React、Django等）
- **自动测试**：生成的代码包含单元测试
- **文档注释**：自动添加详细的文档注释
- **代码优化**：遵循最佳实践，确保代码质量

## 目录结构

```
examples/code-generation-app/
├── code_generator_agent.yaml  # 代理配置文件
├── demo.py                    # 演示脚本
└── README.md                  # 本文件
```

## 配置与启动

### 1. 配置代理

将代理配置文件复制到配置目录：

```bash
# 创建配置目录（如果不存在）
mkdir -p ~/.aetheris/agents

# 复制代理配置
cp examples/code-generation-app/code_generator_agent.yaml ~/.aetheris/agents/
```

### 2. 配置技能

确保代码生成技能已配置：

```bash
# 复制技能配置
mkdir -p ~/.aetheris/agentskills
cp examples/agentskills/code-generation/SKILL.md ~/.aetheris/agentskills/
```

### 3. 启动Aetheris

```bash
aetheris start
```

## 使用演示

运行演示脚本查看代码生成能力：

```bash
cd examples/code-generation-app
python demo.py
```

演示脚本会生成以下类型的代码：
- Python：快速排序算法
- Rust：二维向量结构体
- JavaScript：防抖函数
- TypeScript：用户信息接口

## API调用示例

### 生成Python代码

```python
import requests

url = "http://localhost:8080/api/v1/agent/code-generator-app/invoke"
payload = {
    "skill": "code_generation",
    "parameters": {
        "description": "创建一个函数，实现快速排序算法",
        "language": "python",
        "include_tests": true,
        "include_docs": true
    }
}

response = requests.post(url, json=payload)
print(response.json())
```

### 生成Rust代码

```python
import requests

url = "http://localhost:8080/api/v1/agent/code-generator-app/invoke"
payload = {
    "skill": "code_generation",
    "parameters": {
        "description": "创建一个结构体表示二维向量，并实现基本运算",
        "language": "rust",
        "include_tests": true,
        "include_docs": true
    }
}

response = requests.post(url, json=payload)
print(response.json())
```

## 注意事项

- 需要配置LLM访问权限
- 生成的代码可能需要根据具体需求进行调整
- 复杂的代码生成可能需要更长的处理时间

## 故障排除

- 确保Aetheris服务正在运行
- 检查LLM配置是否正确
- 验证技能文件是否正确放置

## 扩展建议

- 添加更多编程语言支持
- 集成代码审查功能
- 实现代码重构建议
- 添加性能优化分析