# Aetheris 技能市场设计文档

## 1. 概述

本设计文档描述了Aetheris技能市场的实现方案，包括数据库结构、API设计、技能管理功能等。技能市场将支持技能的完整生命周期管理，包括上传、下载、搜索、评分、版本管理和依赖管理。

## 2. 技术栈

- **后端**：Rust、Axum、SQLx
- **数据库**：PostgreSQL
- **向量数据库**：Qdrant（用于技能搜索）
- **认证**：JWT

## 3. 数据库结构

### 3.1 核心表结构

#### users 表
- `id`：UUID，主键
- `username`：VARCHAR(100)，唯一，用户名
- `email`：VARCHAR(255)，唯一，邮箱
- `password_hash`：VARCHAR(255)，密码哈希
- `role`：VARCHAR(50)，角色
- `is_active`：BOOLEAN，是否活跃
- `created_at`：TIMESTAMP WITH TIME ZONE，创建时间
- `updated_at`：TIMESTAMP WITH TIME ZONE，更新时间

#### skills 表
- `id`：UUID，主键
- `skill_id`：VARCHAR(255)，唯一，技能ID
- `name`：VARCHAR(100)，技能名称
- `description`：TEXT，技能描述
- `long_description`：TEXT，详细描述
- `version`：VARCHAR(50)，当前版本
- `author_id`：UUID，作者ID
- `author_name`：VARCHAR(100)，作者名称
- `category`：VARCHAR(100)，分类
- `categories`：TEXT[]，分类数组
- `tags`：TEXT[]，标签数组
- `status`：VARCHAR(50)，状态（draft、pending、published、deprecated、blocked）
- `call_mode`：VARCHAR(50)，调用模式
- `permission_level`：VARCHAR(50)，权限级别
- `priority`：VARCHAR(50)，优先级
- `required_permissions`：TEXT[]，所需权限
- `input_schema`：JSONB，输入 schema
- `output_schema`：JSONB，输出 schema
- `example_input`：JSONB，示例输入
- `example_output`：JSONB，示例输出
- `dependencies`：TEXT[]，依赖
- `is_active`：BOOLEAN，是否活跃
- `is_deprecated`：BOOLEAN，是否已弃用
- `deprecation_reason`：TEXT，弃用原因
- `metadata`：JSONB，元数据
- `download_count`：BIGINT，下载次数
- `average_rating`：NUMERIC(3, 2)，平均评分
- `rating_count`：INTEGER，评分次数
- `success_rate`：NUMERIC(5, 2)，成功率
- `execution_count`：BIGINT，执行次数
- `created_at`：TIMESTAMP WITH TIME ZONE，创建时间
- `updated_at`：TIMESTAMP WITH TIME ZONE，更新时间
- `published_at`：TIMESTAMP WITH TIME ZONE，发布时间
- `deprecated_at`：TIMESTAMP WITH TIME ZONE，弃用时间

#### skill_versions 表
- `id`：UUID，主键
- `skill_id`：UUID，技能ID
- `version`：VARCHAR(50)，版本号
- `content`：JSONB，技能内容
- `changelog`：TEXT，变更日志
- `created_at`：TIMESTAMP WITH TIME ZONE，创建时间

#### skill_reviews 表
- `id`：UUID，主键
- `skill_id`：UUID，技能ID
- `user_id`：UUID，用户ID
- `rating`：INTEGER，评分（1-5）
- `title`：VARCHAR(200)，评论标题
- `content`：TEXT，评论内容
- `helpful_count`：INTEGER，有用次数
- `created_at`：TIMESTAMP WITH TIME ZONE，创建时间
- `updated_at`：TIMESTAMP WITH TIME ZONE，更新时间

#### skill_downloads 表
- `id`：UUID，主键
- `skill_id`：UUID，技能ID
- `user_id`：UUID，用户ID
- `downloaded_at`：TIMESTAMP WITH TIME ZONE，下载时间
- `ip_address`：VARCHAR(50)，IP地址
- `user_agent`：TEXT，用户代理

#### skill_executions 表
- `id`：UUID，主键
- `skill_id`：UUID，技能ID
- `version`：VARCHAR(50)，版本号
- `success`：BOOLEAN，是否成功
- `executed_at`：TIMESTAMP WITH TIME ZONE，执行时间
- `execution_time_ms`：BIGINT，执行时间（毫秒）
- `error_message`：TEXT，错误信息

### 3.2 辅助表结构

#### skill_categories 表
- `id`：UUID，主键
- `name`：VARCHAR(100)，分类名称
- `description`：TEXT，分类描述
- `parent_id`：UUID，父分类ID
- `created_at`：TIMESTAMP WITH TIME ZONE，创建时间

#### skill_tags 表
- `id`：UUID，主键
- `name`：VARCHAR(50)，标签名称
- `usage_count`：INTEGER，使用次数
- `created_at`：TIMESTAMP WITH TIME ZONE，创建时间

#### review_helpfulness 表
- `id`：UUID，主键
- `review_id`：UUID，评论ID
- `user_id`：UUID，用户ID
- `is_helpful`：BOOLEAN，是否有用
- `created_at`：TIMESTAMP WITH TIME ZONE，创建时间

#### skill_audit_log 表
- `id`：UUID，主键
- `skill_id`：UUID，技能ID
- `action`：VARCHAR(100)，操作
- `performed_by`：UUID，执行用户
- `details`：JSONB，详细信息
- `performed_at`：TIMESTAMP WITH TIME ZONE，执行时间

## 4. API 设计

### 4.1 认证相关

- `POST /api/auth/register`：用户注册
- `POST /api/auth/login`：用户登录

### 4.2 技能管理

- `GET /api/skills`：获取技能列表（支持搜索、分页）
- `POST /api/skills`：创建技能
- `GET /api/skills/:id`：获取技能详情
- `PUT /api/skills/:id`：更新技能
- `GET /api/skills/:id/download`：下载技能
- `POST /api/skills/executions`：记录技能执行

### 4.3 技能版本管理

- `GET /api/skills/:id/versions`：获取技能版本列表
- `POST /api/skills/:id/versions`：创建技能版本
- `GET /api/skills/:id/versions/:version`：获取特定版本

### 4.4 技能评分和评论

- `GET /api/skills/:id/reviews`：获取技能评论列表
- `POST /api/skills/:id/reviews`：创建技能评论
- `GET /api/skills/:id/rating`：获取技能评分

### 4.5 技能审核

- `POST /api/skills/:id/submit-audit`：提交审核
- `PUT /api/skills/:id/status`：更新技能状态
- `GET /api/audit/queue`：获取审核队列

### 4.6 统计信息

- `GET /api/stats`：获取统计信息

## 5. 核心功能实现

### 5.1 技能上传

- 支持通过API上传技能
- 验证技能格式和内容
- 自动提取技能元数据
- 存储技能版本

### 5.2 技能下载

- 支持按ID下载技能
- 支持指定版本下载
- 记录下载统计信息

### 5.3 技能搜索

- 支持按名称、描述、标签搜索
- 支持按分类过滤
- 支持按评分、下载次数排序
- 使用Qdrant进行向量搜索

### 5.4 技能评分

- 支持1-5星评分
- 支持添加评论
- 计算平均评分
- 支持评论有用性投票

### 5.5 技能版本管理

- 支持多版本管理
- 记录版本变更日志
- 支持回滚到历史版本

### 5.6 依赖管理

- 记录技能依赖关系
- 验证依赖是否存在
- 支持依赖版本约束

### 5.7 生命周期管理

- 支持技能状态流转：draft → pending → published → deprecated
- 支持技能审核流程
- 支持技能禁用/启用

## 6. 性能优化

- 使用PostgreSQL索引优化查询性能
- 使用Qdrant向量数据库优化搜索性能
- 实现缓存机制减少数据库查询
- 支持分页查询避免一次性加载过多数据

## 7. 安全性

- 使用JWT进行身份验证
- 实现权限控制
- 对用户密码进行哈希处理
- 防止SQL注入攻击
- 验证技能内容安全性

## 8. 部署方案

- 支持Docker容器化部署
- 支持Kubernetes集群部署
- 提供配置文件管理
- 支持环境变量配置

## 9. 测试策略

- 单元测试：测试核心功能
- 集成测试：测试API接口
- 性能测试：测试系统性能
- 安全测试：测试系统安全性

## 10. 监控和日志

- 集成Prometheus监控
- 集成OpenTelemetry追踪
- 实现结构化日志
- 支持告警机制

## 11. 扩展性

- 支持插件系统
- 支持多语言技能
- 支持技能模板
- 支持技能推荐系统

## 12. 结论

本设计文档提供了Aetheris技能市场的完整实现方案，包括数据库结构、API设计、核心功能等。通过此方案，Aetheris将拥有一个功能完整、性能优异、安全可靠的技能市场，支持技能的完整生命周期管理，能够存储和检索至少100个技能。