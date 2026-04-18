---
name: code-generation
description: Generate code in various programming languages based on requirements, specifications, or descriptions. Use when writing new code, implementing features, or creating code from scratch.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [code, generation, programming, development, coding, implementation]
compatibility: Requires LLM access
timeout: 300
allowed-tools: [LLM, CodeExecutor]
metadata:
  emoji: "💻"
  requires:
    env: []
    bins: []
  retry_config:
    max_attempts: 2
    initial_delay_ms: 1000
    max_delay_ms: 3000
    backoff_multiplier: 2
  sandbox_level: medium
---

# Code Generation Skill

## 功能概述

根据需求、规格说明或描述生成各种编程语言的代码。支持多种语言、框架和编码风格。

## 适用场景

- 从零开始编写新代码
- 实现功能模块
- 创建类和函数
- 生成单元测试
- 编写文档注释
- 代码补全和扩展

## 输入规范

- `description` (string, required): 代码功能描述或需求
- `language` (string, required): 编程语言，如 rust, python, javascript, typescript, java, go 等
- `framework` (string, optional): 框架名称，如 react, django, spring 等
- `style` (string, optional): 编码风格指南
- `include_tests` (boolean, optional): 是否包含单元测试，默认 true
- `include_docs` (boolean, optional): 是否包含文档注释，默认 true

## 执行流程

1. 分析需求描述
2. 选择合适的设计模式
3. 生成代码结构
4. 实现核心功能
5. 添加错误处理
6. 编写文档和注释
7. 生成测试代码
8. 代码格式化和优化

## 输出规范

- 成功返回：
  - `code`: 生成的代码
  - `language`: 编程语言
  - `files`: 文件列表（多文件时）
  - `explanation`: 代码说明
- 失败返回：
  - `error`: 错误信息

## 约束与安全

- 生成的代码需经人工审查
- 不执行未经授权的代码
- 遵循安全编码最佳实践
- 避免生成有安全漏洞的代码

## 示例

### 示例 1：生成 Rust 函数

输入：
```
{
  "description": "创建一个函数，计算两个数的最大公约数（GCD），使用欧几里得算法",
  "language": "rust",
  "include_tests": true,
  "include_docs": true
}
```

输出：
```
{
  "code": "/// 计算两个数的最大公约数（GCD）\n/// \n/// 使用欧几里得算法计算两个非负整数的最大公约数。\n/// \n/// # Arguments\n/// \n/// * `a` - 第一个非负整数\n/// * `b` - 第二个非负整数\n/// \n/// # Returns\n/// \n/// 返回 a 和 b 的最大公约数\n/// \n/// # Examples\n/// \n/// ```\n/// assert_eq!(gcd(48, 18), 6);\n/// assert_eq!(gcd(0, 5), 5);\n/// assert_eq!(gcd(7, 13), 1);\n/// ```\npub fn gcd(mut a: u64, mut b: u64) -> u64 {\n    while b != 0 {\n        (a, b) = (b, a % b);\n    }\n    a\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_gcd() {\n        assert_eq!(gcd(48, 18), 6);\n        assert_eq!(gcd(0, 5), 5);\n        assert_eq!(gcd(7, 13), 1);\n        assert_eq!(gcd(100, 100), 100);\n        assert_eq!(gcd(14, 21), 7);\n    }\n}",
  "language": "rust",
  "explanation": "实现了使用欧几里得算法的 GCD 计算函数，包含完整的文档注释和单元测试。"
}
```
