# IM 平台集成

本指南将帮助您集成 Aetheris 与各种 IM 平台，包括企业微信、钉钉、飞书和个人微信。

## 企业微信集成

### 步骤 1：创建企业微信应用

1. 登录 [企业微信管理后台](https://work.weixin.qq.com/)
2. 进入「应用管理」→「自建」→「创建应用」
3. 填写应用名称（例如：Aetheris）和描述
4. 上传应用图标
5. 点击「创建应用」

### 步骤 2：配置应用

1. 在应用详情页面，记录下「AgentId」和「Secret」
2. 在「可见范围」中选择可使用该应用的部门或成员
3. 在「API 接收消息」部分，点击「设置」
4. 填写「服务器地址(URL)」为 `http://your-server:3000/api/webhook/wechat`
5. 填写「Token」为一个随机字符串（例如：aetheris-wechat-token）
6. 填写「EncodingAESKey」为一个随机字符串
7. 点击「保存」

### 步骤 3：配置 Aetheris

编辑 `config/llm.yaml` 文件，添加企业微信配置：

```yaml
# 企业微信配置
wechat:
  corp_id: your-corp-id
  agent_id: your-agent-id
  secret: your-agent-secret
  token: aetheris-wechat-token
  encoding_aes_key: your-encoding-aes-key
```

### 步骤 4：验证集成

1. 重启 Aetheris 服务
2. 在企业微信中向应用发送消息，例如："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
3. 应用应该会自动处理并回复结果

## 钉钉集成

### 步骤 1：创建钉钉自定义机器人

1. 登录 [钉钉开放平台](https://open.dingtalk.com/)
2. 进入「开发者后台」→「应用开发」→「企业内部开发」→「机器人」
3. 点击「创建机器人」
4. 填写机器人名称（例如：Aetheris）和描述
5. 上传机器人头像
6. 点击「创建」

### 步骤 2：配置机器人

1. 在机器人详情页面，记录下「AppKey」和「AppSecret」
2. 在「服务器出口 IP」中添加您服务器的 IP 地址
3. 在「消息推送」部分，点击「设置」
4. 填写「消息接收地址」为 `http://your-server:3000/api/webhook/dingtalk`
5. 点击「保存」

### 步骤 3：配置 Aetheris

编辑 `config/llm.yaml` 文件，添加钉钉配置：

```yaml
# 钉钉配置
dingtalk:
  app_key: your-app-key
  app_secret: your-app-secret
```

### 步骤 4：验证集成

1. 重启 Aetheris 服务
2. 在钉钉群聊中 @ 机器人并发送消息，例如："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
3. 机器人应该会自动处理并回复结果

## 飞书集成

### 步骤 1：创建飞书应用

1. 登录 [飞书开放平台](https://open.feishu.cn/)
2. 进入「开发者后台」→「企业自建应用」→「创建应用」
3. 填写应用名称（例如：Aetheris）和描述
4. 上传应用图标
5. 点击「创建」

### 步骤 2：配置应用

1. 在应用详情页面，记录下「App ID」和「App Secret」
2. 在「权限管理」中添加必要的权限（例如：获取用户信息、发送消息等）
3. 在「事件订阅」部分，点击「设置」
4. 填写「请求地址」为 `http://your-server:3000/api/webhook/feishu`
5. 点击「保存」

### 步骤 3：配置 Aetheris

编辑 `config/llm.yaml` 文件，添加飞书配置：

```yaml
# 飞书配置
feishu:
  app_id: your-app-id
  app_secret: your-app-secret
```

### 步骤 4：验证集成

1. 重启 Aetheris 服务
2. 在飞书中向应用发送消息，例如："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
3. 应用应该会自动处理并回复结果

## 个人微信集成

### 步骤 1：配置 ILink 适配器

1. 下载并安装 ILink 适配器
2. 配置 ILink 适配器连接到个人微信
3. 设置 ILink 适配器的 webhook 地址为 `http://your-server:3000/api/webhook/wechat-personal`

### 步骤 2：配置 Aetheris

编辑 `config/llm.yaml` 文件，添加个人微信配置：

```yaml
# 个人微信配置
wechat_personal:
  enabled: true
```

### 步骤 3：验证集成

1. 重启 Aetheris 服务
2. 在个人微信中向 ILink 适配器发送消息，例如："我们厂接到一个新的化工生产订单需要生成最终的生产报告"
3. 系统应该会自动处理并回复结果

## 故障排除

### 常见问题

1. **webhook 配置失败**：检查服务器地址是否可访问，确保端口开放
2. **消息无响应**：检查 LLM 配置是否正确，确保 API Key 有效
3. **集成验证失败**：检查应用权限是否正确配置，确保网络连接正常

### 日志查看

查看 Aetheris 服务日志以获取更多信息：

```bash
# 开发模式运行时，日志会直接输出到终端
# 生产模式运行时，日志会输出到 logs/ 目录
```

## 下一步

- [Agent 管理](agent-management.md) - 了解如何管理和配置 Agent
- [Skill 管理](skill-management.md) - 了解如何管理和使用 Skill
- [故障排除](troubleshooting.md) - 了解常见问题和解决方案
