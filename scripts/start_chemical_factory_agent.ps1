# 化工厂智能排产系统启动脚本 (Windows)

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  化工厂智能排产系统启动" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# 1. 加载环境变量
if (Test-Path .env) {
    Write-Host "加载环境变量..." -ForegroundColor Yellow
    Get-Content .env | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') {
            [Environment]::SetEnvironmentVariable($matches[1], $matches[2])
        }
    }
}

# 2. 检查配置文件
$AGENT_CONFIG = "examples\agents\chemical_factory_scheduling_agent.yaml"
if (-not (Test-Path $AGENT_CONFIG)) {
    Write-Host "错误: 找不到 Agent 配置文件: $AGENT_CONFIG" -ForegroundColor Red
    exit 1
}

# 3. 验证配置
Write-Host "验证 Agent 配置..." -ForegroundColor Yellow
cargo run -- agent validate $AGENT_CONFIG
if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: Agent 配置验证失败" -ForegroundColor Red
    exit 1
}

# 4. 创建 Agent
Write-Host "创建化工厂排产 Agent..." -ForegroundColor Yellow
cargo run -- agent create $AGENT_CONFIG
if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: Agent 创建失败" -ForegroundColor Red
    exit 1
}

# 5. 启动 Aetheris
Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  启动 Aetheris 服务器..." -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "服务器将在 http://localhost:3000 启动" -ForegroundColor Green
Write-Host ""
Write-Host "按 Ctrl+C 停止服务器" -ForegroundColor Yellow
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

cargo run
