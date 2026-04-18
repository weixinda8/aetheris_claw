#!/bin/bash

# Aetheris 健康检查与自动恢复脚本
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="/var/log"
LOG_FILE="$LOG_DIR/aetheris_health.log"

# 创建日志目录（如果不存在）
sudo mkdir -p "$LOG_DIR"
sudo touch "$LOG_FILE"
sudo chown "$USER:$USER" "$LOG_FILE" 2>/dev/null || true

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "$LOG_FILE"
}

check_service() {
    local service=$1
    if ! docker ps --filter "name=$service" --filter "status=running" | grep -q "$service"; then
        log "警告: $service 未运行，尝试重启..."
        cd "$PROJECT_DIR"
        if [ -f "$PROJECT_DIR/.env.production" ] && [ -f "$PROJECT_DIR/docker-compose.production.yml" ]; then
            docker compose --env-file .env.production -f docker-compose.production.yml start "$service"
        else
            if [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
                docker compose start "$service"
            fi
        fi
        return 1
    fi
    return 0
}

check_api() {
    if ! curl -s -f http://localhost:3000/api/health > /dev/null; then
        log "警告: Aetheris API 响应异常，尝试重启应用..."
        cd "$PROJECT_DIR"
        if [ -f "$PROJECT_DIR/.env.production" ] && [ -f "$PROJECT_DIR/docker-compose.production.yml" ]; then
            docker compose --env-file .env.production -f docker-compose.production.yml restart aetheris
        else
            if [ -f "$PROJECT_DIR/docker-compose.yml" ]; then
                docker compose restart aetheris
            fi
        fi
        return 1
    fi
    return 0
}

# 执行检查
log "开始健康检查..."
check_service "aetheris-postgres"
check_service "aetheris-qdrant"
check_service "aetheris-app"
check_api
log "健康检查完成"
