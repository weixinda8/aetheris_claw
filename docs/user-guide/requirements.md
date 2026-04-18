# 系统要求

本页面列出了运行 Aetheris 所需的系统要求和依赖项。

## 硬件要求

| 组件 | 最低要求 | 推荐要求 |
|------|---------|---------|
| CPU | 4 核处理器 | 8 核或更多 |
| 内存 | 4GB RAM | 8GB+ RAM |
| 磁盘 | 5GB 可用空间 | 10GB+ 可用空间 |
| 网络 | 稳定的网络连接 | 稳定的网络连接 |

## 软件要求

### 操作系统

- **Windows**: Windows 10/11 (64位)
- **macOS**: macOS 11+ (Big Sur 或更高版本)
- **Linux**: Ubuntu 20.04+ 或其他兼容的 Linux 发行版

### 依赖项

- **Rust**: 1.94 或更高版本（使用 Rust 2024 Edition）
- **Git**: 用于克隆代码库
- **Docker** (可选): 用于运行技能沙箱
- **PostgreSQL** (可选): 用于持久化存储
- **Qdrant** (可选): 用于向量数据库

## 网络要求

- 访问互联网以下载依赖项和与 LLM 提供商通信
- 可选：端口 3000 开放（用于 API 服务）

## LLM 提供商要求

Aetheris 支持以下 LLM 提供商：

- **DeepSeek** (推荐): 需要 API Key
- **OpenAI**: 需要 API Key
- **Anthropic**: 需要 API Key
- **Azure OpenAI**: 需要 API Key 和端点 URL

## 浏览器要求（用于前端界面）

- Google Chrome 90+
- Mozilla Firefox 88+
- Microsoft Edge 90+
- Safari 14+
