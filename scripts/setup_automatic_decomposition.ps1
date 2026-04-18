# 自动任务分解系统 - Windows 初始化脚本
# 版本: 1.0.0
# 日期: 2026-04-15

Write-Host ""
Write-Host "╔═══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     Aetheris 自动任务分解系统 - 初始化设置                    ║" -ForegroundColor Cyan
Write-Host "╚═══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# 步骤 1: 检查项目目录
Write-Host "[1/5] 检查项目目录..." -ForegroundColor Yellow
$projectRoot = "d:\aetheris\aetheris"
if (-not (Test-Path $projectRoot)) {
    Write-Host "错误: 项目目录不存在: $projectRoot" -ForegroundColor Red
    exit 1
}
Set-Location $projectRoot
Write-Host "✓ 项目目录确认" -ForegroundColor Green

# 步骤 2: 创建配置目录
Write-Host ""
Write-Host "[2/5] 创建配置目录..." -ForegroundColor Yellow
$templateDir = "config\decomposition_templates"
if (-not (Test-Path $templateDir)) {
    New-Item -ItemType Directory -Path $templateDir -Force | Out-Null
    Write-Host "✓ 创建目录: $templateDir" -ForegroundColor Green
} else {
    Write-Host "✓ 目录已存在: $templateDir" -ForegroundColor Green
}

# 步骤 3: 检查模板文件
Write-Host ""
Write-Host "[3/5] 检查分解模板文件..." -ForegroundColor Yellow
$templateFile = "$templateDir\chemical_production_order.yaml"
if (Test-Path $templateFile) {
    Write-Host "✓ 模板文件已存在: $templateFile" -ForegroundColor Green
} else {
    Write-Host "错误: 模板文件不存在: $templateFile" -ForegroundColor Red
    Write-Host "请确保已从 .trae/documents/ 复制模板文件" -ForegroundColor Yellow
    exit 1
}

# 步骤 4: 创建示例代码目录
Write-Host ""
Write-Host "[4/5] 创建示例代码目录..." -ForegroundColor Yellow
$exampleDir = "examples\automatic_decomposition"
if (-not (Test-Path $exampleDir)) {
    New-Item -ItemType Directory -Path $exampleDir -Force | Out-Null
    Write-Host "✓ 创建目录: $exampleDir" -ForegroundColor Green
} else {
    Write-Host "✓ 目录已存在: $exampleDir" -ForegroundColor Green
}

# 步骤 5: 检查示例代码
Write-Host ""
Write-Host "[5/5] 检查示例代码..." -ForegroundColor Yellow
$matcherFile = "$exampleDir\template_matcher.rs"
if (Test-Path $matcherFile) {
    Write-Host "✓ 示例代码已存在: $matcherFile" -ForegroundColor Green
} else {
    Write-Host "警告: 示例代码不存在: $matcherFile" -ForegroundColor Yellow
}

# 完成
Write-Host ""
Write-Host "╔═══════════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║                    初始化完成！                                  ║" -ForegroundColor Green
Write-Host "╚═══════════════════════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""
Write-Host "接下来的步骤:" -ForegroundColor Cyan
Write-Host "1. 查看使用说明: 打开 .trae\documents\automatic_decomposition_quick_start.md" -ForegroundColor White
Write-Host "2. 配置 Agent: 确保 agents/ 目录下有 6 个 Agent 配置文件" -ForegroundColor White
Write-Host "3. 启动 Aetheris: 运行 cargo run" -ForegroundColor White
Write-Host "4. 测试功能: 提交任务 '我们厂接到一个新的化工生产订单需要生成最终的生产报告'" -ForegroundColor White
Write-Host ""
Write-Host "提示: 如需帮助，请查看 .trae\documents\ 下的文档" -ForegroundColor Gray
Write-Host ""
