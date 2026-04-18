#!/bin/bash

# Aetheris 系统监控脚本
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "=========================================="
echo "  Aetheris 系统监控"
echo "=========================================="
echo ""

# 系统资源
echo "【系统资源】"
echo "CPU 使用率: $(top -bn1 | grep 'Cpu(s)' | sed 's/.*, *\([0-9.]*\)%* id.*/\1/' | awk '{print 100 - $1 "%"}')"
echo "内存使用: $(free -h | awk '/^Mem/ {print $3 "/" $2}')"
echo "磁盘使用: $(df -h / | awk 'NR==2 {print $5 " used (" $3 "/" $2 ")"}')"
echo ""

# Docker容器状态
echo "【Docker 容器】"
cd "$PROJECT_DIR"
if [ -f "$PROJECT_DIR/.env.production" ] && [ -f "$PROJECT_DIR/docker-compose.production.yml" ]; then
    docker compose --env-file .env.production -f docker-compose.production.yml ps
else
    echo "警告: 生产环境配置文件未找到，尝试使用默认配置"
    if [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
        docker compose ps
    fi
fi
echo ""

# 应用健康检查
echo "【应用健康】"
if curl -s -f http://localhost:3000/api/health > /dev/null; then
    echo "✅ Aetheris API: 健康"
else
    echo "❌ Aetheris API: 异常"
fi
echo ""

# 最近日志
echo "【最近日志】"
cd "$PROJECT_DIR"
if [ -f "$PROJECT_DIR/.env.production" ] && [ -f "$PROJECT_DIR/docker-compose.production.yml" ]; then
    docker compose --env-file .env.production -f docker-compose.production.yml logs --tail=20 aetheris
else
    echo "警告: 生产环境配置文件未找到，尝试使用默认配置"
    if [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
        docker compose logs --tail=20 aetheris
    fi
fi
