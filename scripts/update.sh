#!/bin/bash
set -e

# Aetheris 自动更新脚本
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKUP_DIR="/opt/backups/aetheris"
DATE=$(date +%Y%m%d_%H%M%S)

echo "=========================================="
echo "  Aetheris 更新流程"
echo "=========================================="
echo ""

# 1. 备份当前版本
echo "[1/6] 创建备份..."
cd "$PROJECT_DIR"
if [ -f "$PROJECT_DIR/scripts/backup.sh" ]; then
    bash "$PROJECT_DIR/scripts/backup.sh"
else
    echo "警告: 备份脚本未找到，跳过备份"
fi

# 2. 拉取最新代码
echo "[2/6] 拉取最新代码..."
git fetch origin
git log --oneline -5 origin/master

# 3. 确认更新
read -p "是否继续更新？(y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "更新已取消"
    exit 1
fi

# 4. 停止服务
echo "[3/6] 停止服务..."
if [ -f "$PROJECT_DIR/.env.production" ] && [ -f "$PROJECT_DIR/docker-compose.production.yml" ]; then
    docker compose --env-file .env.production -f docker-compose.production.yml down
else
    echo "警告: 生产环境配置文件未找到，尝试使用默认配置"
    if [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
        docker compose down
    fi
fi

# 5. 切换到新版本
echo "[4/6] 更新代码..."
git checkout master
git pull origin master

# 可选: 切换到特定版本
# git checkout v1.1.0

# 6. 重新构建并启动
echo "[5/6] 重新构建并启动..."
if [ -f "$PROJECT_DIR/.env.production" ] && [ -f "$PROJECT_DIR/docker-compose.production.yml" ]; then
    docker compose --env-file .env.production -f docker-compose.production.yml up -d --build
else
    echo "警告: 生产环境配置文件未找到，尝试使用默认配置"
    if [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
        docker compose up -d --build
    fi
fi

# 7. 验证
echo "[6/6] 验证部署..."
sleep 30

if curl -s -f http://localhost:3000/api/health > /dev/null; then
    echo ""
    echo "✅ 更新成功！"
    echo ""
    if [ -f "$PROJECT_DIR/.env.production" ] && [ -f "$PROJECT_DIR/docker-compose.production.yml" ]; then
        echo "查看状态: docker compose --env-file .env.production -f docker-compose.production.yml ps"
        echo "查看日志: docker compose --env-file .env.production -f docker-compose.production.yml logs -f"
    else
        echo "查看状态: docker compose ps"
        echo "查看日志: docker compose logs -f"
    fi
else
    echo ""
    echo "❌ 健康检查失败，请查看日志"
    exit 1
fi
