#!/usr/bin/env python3
"""
代码生成示例应用演示脚本
展示AI辅助编程能力，支持多种编程语言的代码生成
"""

import json
import requests
import time

def generate_code(description, language, framework=None, include_tests=True, include_docs=True):
    """生成代码的函数"""
    url = "http://localhost:8080/api/v1/agent/code-generator-app/invoke"
    payload = {
        "skill": "code_generation",
        "parameters": {
            "description": description,
            "language": language,
            "framework": framework,
            "include_tests": include_tests,
            "include_docs": include_docs
        }
    }
    
    try:
        response = requests.post(url, json=payload, timeout=60)
        response.raise_for_status()
        return response.json()
    except requests.exceptions.RequestException as e:
        print(f"请求失败: {e}")
        return None

def save_code_to_file(code, filename):
    """保存代码到文件"""
    try:
        with open(filename, 'w', encoding='utf-8') as f:
            f.write(code)
        print(f"代码已保存到: {filename}")
    except Exception as e:
        print(f"保存文件失败: {e}")

def demo_python_code_generation():
    """演示Python代码生成"""
    print("\n=== 演示 Python 代码生成 ===")
    description = "创建一个函数，实现快速排序算法，包含详细的文档注释和单元测试"
    result = generate_code(description, "python")
    
    if result and "code" in result:
        print("生成的Python代码:")
        print(result["code"])
        save_code_to_file(result["code"], "quick_sort.py")
    else:
        print("代码生成失败")

def demo_rust_code_generation():
    """演示Rust代码生成"""
    print("\n=== 演示 Rust 代码生成 ===")
    description = "创建一个结构体表示二维向量，并实现加法、减法和点积运算，包含单元测试"
    result = generate_code(description, "rust")
    
    if result and "code" in result:
        print("生成的Rust代码:")
        print(result["code"])
        save_code_to_file(result["code"], "vector2d.rs")
    else:
        print("代码生成失败")

def demo_javascript_code_generation():
    """演示JavaScript代码生成"""
    print("\n=== 演示 JavaScript 代码生成 ===")
    description = "创建一个函数，实现debounce（防抖）功能，用于优化事件处理"
    result = generate_code(description, "javascript")
    
    if result and "code" in result:
        print("生成的JavaScript代码:")
        print(result["code"])
        save_code_to_file(result["code"], "debounce.js")
    else:
        print("代码生成失败")

def demo_typescript_code_generation():
    """演示TypeScript代码生成"""
    print("\n=== 演示 TypeScript 代码生成 ===")
    description = "创建一个接口表示用户信息，包含姓名、年龄和邮箱字段，并实现一个验证函数"
    result = generate_code(description, "typescript")
    
    if result and "code" in result:
        print("生成的TypeScript代码:")
        print(result["code"])
        save_code_to_file(result["code"], "user.ts")
    else:
        print("代码生成失败")

def main():
    """主函数"""
    print("代码生成示例应用演示")
    print("=" * 50)
    
    # 运行各种语言的代码生成演示
    demo_python_code_generation()
    time.sleep(2)  # 避免请求过于频繁
    
    demo_rust_code_generation()
    time.sleep(2)
    
    demo_javascript_code_generation()
    time.sleep(2)
    
    demo_typescript_code_generation()
    
    print("\n" + "=" * 50)
    print("演示完成！")

if __name__ == "__main__":
    main()