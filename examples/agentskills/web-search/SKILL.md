---
name: web-search
description: Search the internet for information, research topics, and retrieve current data. Use when asked about facts, news, research, or up-to-date information from the web.
version: 1.0.0
author: Aetheris Team
license: Apache-2.0
tags: [search, web, research, information, internet]
compatibility: Requires internet connectivity
timeout: 60
allowed-tools: [WebSearch, Read]
metadata:
  emoji: "🌐"
  requires:
    env: []
    bins: []
  retry_config:
    max_attempts: 3
    initial_delay_ms: 1000
    max_delay_ms: 5000
    backoff_multiplier: 2
  sandbox_level: low
---

# Web Search Skill

## 功能概述

在互联网上搜索信息，查找最新新闻、研究资料、事实数据和权威来源。支持多种搜索策略，包括精确匹配、关键词搜索、近期结果筛选等。

## 适用场景

- 查询最新新闻和事件
- 研究特定主题或领域
- 查找事实数据和统计信息
- 验证信息准确性
- 搜索学术论文和研究资料
- 查找产品信息和对比

## 输入规范

- `query` (string, required): 搜索查询关键词或问题
- `num_results` (integer, optional): 返回结果数量，默认 5，最大 20
- `time_range` (string, optional): 时间范围，可选值：day, week, month, year, all（默认 all）
- `safe_search` (boolean, optional): 是否启用安全搜索，默认 true

## 执行流程

1. 验证输入参数：确保查询不为空，参数在有效范围内
2. 配置搜索参数：根据用户需求设置时间范围、结果数量等
3. 执行搜索：调用 WebSearch 工具执行查询
4. 结果筛选：过滤无关或低质量结果
5. 结果排序：按相关性和时间排序
6. 摘要生成：为每个结果生成简明摘要
7. 格式输出：整理成结构化格式返回

## 输出规范

- 成功返回：包含查询结果的结构化数据
  - `results`: 搜索结果数组
    - `title`: 结果标题
    - `url`: 结果链接
    - `snippet`: 内容摘要
    - `source`: 来源网站
    - `date`: 发布时间（如有）
  - `query`: 原始查询
  - `total_results`: 总结果数
- 失败返回：包含错误信息
  - `error`: 错误类型
  - `message`: 错误详情

## 约束与安全

- 必须遵守 robots.txt 和网站服务条款
- 尊重网站访问频率限制
- 安全搜索默认启用，避免不当内容
- 不执行恶意或自动化批量搜索
- 结果来源需标注清楚，避免版权问题

## 示例

### 示例 1：基础搜索

输入：
```
{
  "query": "人工智能最新发展 2026",
  "num_results": 5,
  "time_range": "month"
}
```

输出：
```
{
  "query": "人工智能最新发展 2026",
  "total_results": 1547,
  "results": [
    {
      "title": "2026年人工智能技术发展报告",
      "url": "https://example.com/ai-report-2026",
      "snippet": "2026年AI技术在多模态学习、边缘计算、医疗应用等领域取得重大突破...",
      "source": "Tech Review",
      "date": "2026-04-01"
    },
    {
      "title": "GPT-5正式发布：功能与影响分析",
      "url": "https://example.com/gpt5-announcement",
      "snippet": "OpenAI发布最新一代大语言模型，在推理能力和多模态理解方面显著提升...",
      "source": "AI News",
      "date": "2026-03-28"
    }
  ]
}
```

### 示例 2：精确事实查询

输入：
```
{
  "query": "2025年中国GDP增长率",
  "num_results": 3
}
```

输出：
```
{
  "query": "2025年中国GDP增长率",
  "total_results": 892,
  "results": [
    {
      "title": "国家统计局：2025年中国GDP增长5.2%",
      "url": "https://example.com/stats-gdp-2025",
      "snippet": "根据国家统计局发布的数据，2025年中国国内生产总值同比增长5.2%...",
      "source": "国家统计局",
      "date": "2026-01-20"
    }
  ]
}
```
