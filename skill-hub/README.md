# Aetheris Skill Hub

Aetheris Skill Hub 是一个独立的技能发现、评分和分发服务，部署在官方服务器上，为所有 Aetheris 用户提供服务。

## 功能特性

- 技能搜索和发现
- 技能评分和评价
- 技能安装和更新
- 社区贡献流程
- 企业级权限和安全审核
- PostgreSQL 数据存储
- Qdrant 向量搜索
- 完整的可观测性

## 技术栈

- **语言**: Rust
- **Web 框架**: Axum
- **数据库**: PostgreSQL
- **向量搜索**: Qdrant
- **可观测性**: OpenTelemetry + Prometheus
- **容器化**: Docker + Kubernetes

## 快速开始

### 本地开发

1. 复制环境变量文件:
```bash
cp .env.example .env
```

2. 启动依赖服务:
```bash
docker-compose up postgres qdrant -d
```

3. 运行数据库迁移:
```bash
cargo run -- migrate
```

4. 启动服务:
```bash
cargo run
```

### Docker 部署

使用 docker-compose 启动完整的服务栈:

```bash
docker-compose up -d
```

服务将在 http://localhost:8080 启动。

### Kubernetes 部署

1. 创建命名空间:
```bash
kubectl create namespace aetheris
```

2. 部署配置:
```bash
kubectl apply -f k8s/
```

## API 端点

- `GET /api/health` - 健康检查
- `GET /api/skills` - 列出技能
- `GET /api/skills/:id` - 获取技能详情
- `GET /api/skills/:id/download` - 下载技能
- `POST /api/skills/executions` - 记录执行结果
- `GET /api/stats` - 获取统计数据

## 目录结构

```
skill-hub/
├── src/
│   ├── api/              # API 模块
│   │   ├── handlers.rs   # API 处理函数
│   │   ├── models.rs     # 数据模型
│   │   └── mod.rs        # API 路由和状态
│   ├── config/           # 配置模块
│   ├── migrations/       # 数据库迁移
│   ├── utils/            # 工具模块
│   ├── constants.rs      # 常量定义
│   ├── lib.rs            # 库入口
│   └── main.rs           # 主程序
├── k8s/                  # Kubernetes 配置
├── config/               # 配置文件
├── examples/             # 示例文件
├── Dockerfile            # Docker 构建文件
├── docker-compose.yml    # Docker Compose 配置
└── Cargo.toml            # Rust 项目配置
```

## 许可证

MIT
