# 安装指南

本指南将帮助您安装 Aetheris，从获取代码到构建和运行服务。

## 步骤 1：安装 Rust

Aetheris 是用 Rust 编写的，因此您需要安装 Rust 开发环境。

### Windows (PowerShell)

```powershell
Invoke-RestMethod -Uri https://win.rustup.rs/x86_64 | Invoke-Expression
```

### macOS / Linux

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安装完成后，重启终端以应用环境变量更改。

## 步骤 2：克隆代码库

```bash
git clone https://github.com/aetheris/aetheris.git
cd aetheris
```

## 步骤 3：构建项目

### 开发模式

```bash
cargo build
```

### 生产模式

```bash
cargo build --release
```

## 步骤 4：配置 LLM 提供商

复制 LLM 配置文件示例：

### Windows (PowerShell)

```powershell
Copy-Item config\llm.example.yaml config\llm.yaml
```

### macOS / Linux

```bash
cp config/llm.example.yaml config/llm.yaml
```

编辑 `config/llm.yaml` 文件，设置您的 LLM 提供商 API Key：

```yaml
# DeepSeek 配置示例
provider: deepseek
api_key: your-deepseek-api-key-here
api_base: https://api.deepseek.com/v1
model: deepseek-chat
temperature: 0.7
max_tokens: 2000
timeout_seconds: 30
```

## 步骤 5：运行服务

### 开发模式

```bash
cargo run
```

### 生产模式

```bash
cargo run --release
```

服务将在 `http://localhost:3000` 启动。

## 步骤 6：验证安装

打开浏览器访问 `http://localhost:3000/api/health`，您应该看到以下响应：

```json
{
  "status": "ok",
  "version": "1.0.0"
}
```

## 下一步

- [配置指南](configuration.md) - 了解如何配置 Aetheris
- [基本使用](basic-usage.md) - 了解如何使用 Aetheris 的基本功能
- [IM 平台集成](im-integration.md) - 了解如何集成 IM 平台
