# 故障排除

本指南将帮助您解决使用 Aetheris 时可能遇到的常见问题。

## 常见问题

### 1. 服务启动失败

**症状**：`cargo run` 命令执行后，服务无法启动。

**可能原因**：
- LLM 配置错误
- 端口被占用
- 依赖项缺失

**解决方案**：
1. 检查 LLM 配置文件 `config/llm.yaml` 是否正确
2. 确保端口 3000 未被其他服务占用
3. 运行 `cargo build` 检查依赖项是否正确安装

### 2. 任务执行失败

**症状**：创建任务后，任务状态显示为 `failed`。

**可能原因**：
- LLM API Key 无效
- LLM 服务不可用
- 任务输入格式错误

**解决方案**：
1. 检查 LLM API Key 是否正确
2. 验证 LLM 服务是否可访问
3. 检查任务输入是否符合要求

### 3. IM 平台集成失败

**症状**：在 IM 平台发送消息后，系统无响应。

**可能原因**：
- webhook 配置错误
- 网络连接问题
- 应用权限不足

**解决方案**：
1. 检查 webhook 配置是否正确
2. 确保服务器可以访问 IM 平台的 API
3. 检查应用权限是否正确配置

### 4. Skill 执行失败

**症状**：执行 Skill 后，返回错误信息。

**可能原因**：
- Skill 配置错误
- 输入参数不符合要求
- Skill 执行超时

**解决方案**：
1. 检查 Skill 配置文件是否正确
2. 确保输入参数符合 Skill 的要求
3. 调整 Skill 的超时设置

### 5. 性能问题

**症状**：系统响应缓慢，任务执行时间长。

**可能原因**：
- LLM 响应缓慢
- 系统资源不足
- 任务复杂度高

**解决方案**：
1. 选择响应速度更快的 LLM 提供商
2. 增加系统资源（内存、CPU）
3. 优化任务分解策略

## 日志查看

### 开发模式

在开发模式下，日志会直接输出到终端：

```bash
cargo run
```

### 生产模式

在生产模式下，日志会输出到 `logs/` 目录：

```bash
# 查看最新日志
cat logs/aetheris.log

# 实时查看日志
tail -f logs/aetheris.log
```

## 日志级别

Aetheris 支持以下日志级别：

- `trace` - 最详细的日志
- `debug` - 调试信息
- `info` - 一般信息
- `warn` - 警告信息
- `error` - 错误信息

您可以在 `config/llm.yaml` 文件中配置日志级别：

```yaml
# 日志配置
logging:
  level: info
  format: json
  file: logs/aetheris.log
```

## 健康检查

您可以通过以下 API 端点检查系统健康状态：

```bash
curl http://localhost:3000/api/health
```

响应：

```json
{
  "status": "ok",
  "version": "1.0.0",
  "components": {
    "llm": "ok",
    "database": "ok",
    "skills": "ok",
    "agents": "ok"
  }
}
```

## 系统状态

您可以通过以下 API 端点检查系统状态：

```bash
curl http://localhost:3000/api/status
```

响应：

```json
{
  "uptime": "10h 30m 45s",
  "tasks": {
    "total": 100,
    "completed": 95,
    "failed": 5,
    "pending": 0
  },
  "resources": {
    "cpu": "40%",
    "memory": "60%",
    "disk": "30%"
  }
}
```

## 常见错误代码

| 错误代码 | 描述 | 解决方案 |
|---------|------|----------|
| 400 | 无效的请求参数 | 检查请求参数是否符合要求 |
| 401 | 未授权 | 检查 API Key 是否正确 |
| 404 | 资源不存在 | 检查资源路径是否正确 |
| 500 | 内部服务器错误 | 查看日志以获取更多信息 |
| 502 | 网关错误 | 检查 LLM 服务是否可访问 |
| 503 | 服务不可用 | 检查系统资源是否充足 |
| 504 | 网关超时 | 检查 LLM 服务响应时间 |

## 联系支持

如果您遇到无法解决的问题，请：

1. 查看 [GitHub Issues](https://github.com/aetheris/aetheris/issues) 中是否有类似问题
2. 提交新的 [GitHub Issue](https://github.com/aetheris/aetheris/issues/new)，包括：
   - 详细的问题描述
   - 重现步骤
   - 错误日志
   - 系统环境信息
3. 加入我们的社区讨论，寻求帮助

## 下一步

- [API 文档](../api/README.md) - 了解完整的 API 参考
- [教程](../tutorials/README.md) - 了解常见用例和示例教程
- [参考](../reference/README.md) - 了解技术参考和最佳实践
