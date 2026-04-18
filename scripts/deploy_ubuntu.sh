#!/bin/bash
set -e

# ============================================
# Aetheris 生产级 Ubuntu 部署脚本
# 版本: 1.0.0
# 日期: 2026-04-16
# ============================================

echo "=========================================="
echo "  Aetheris 生产级部署脚本"
echo "=========================================="
echo ""

# ============================================
# 步骤 1: 检查系统环境
# ============================================
echo "[1/10] 检查系统环境..."

if [ "$EUID" -eq 0 ]; then 
    echo "请不要以 root 用户运行此脚本，请使用普通用户并配置 sudo"
    exit 1
fi

if ! command -v lsb_release &> /dev/null; then
    echo "错误: 无法检测操作系统版本"
    exit 1
fi

OS_VERSION=$(lsb_release -rs)
echo "检测到 Ubuntu 版本: $OS_VERSION"
echo ""

# ============================================
# 步骤 2: 更新系统并安装基础工具
# ============================================
echo "[2/10] 更新系统并安装基础工具..."

sudo apt update
sudo apt upgrade -y

sudo apt install -y \
    curl \
    wget \
    git \
    vim \
    htop \
    net-tools \
    ca-certificates \
    gnupg \
    lsb-release \
    apt-transport-https

echo "基础工具安装完成"
echo ""

# ============================================
# 步骤 3: 安装 Docker
# ============================================
echo "[3/10] 安装 Docker..."

if ! command -v docker &> /dev/null; then
    echo "正在安装 Docker..."
    
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg

    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu \
      $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

    sudo apt update
    sudo apt install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

    sudo systemctl start docker
    sudo systemctl enable docker

    echo "Docker 安装完成"
else
    echo "Docker 已安装，版本: $(docker --version)"
fi

echo ""

# ============================================
# 步骤 4: 配置 Docker（生产级）
# ============================================
echo "[4/10] 配置 Docker（生产级）..."

sudo mkdir -p /etc/docker

sudo tee /etc/docker/daemon.json > /dev/null << 'EOF'
{
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "100m",
    "max-file": "3"
  },
  "storage-driver": "overlay2",
  "live-restore": true,
  "userland-proxy": false,
  "no-new-privileges": true
}
EOF

sudo systemctl restart docker

echo "Docker 配置完成"
echo ""

# ============================================
# 步骤 5: 将当前用户添加到 docker 组
# ============================================
echo "[5/10] 配置用户权限..."

if ! groups | grep -q docker; then
    sudo usermod -aG docker $USER
    echo "已将用户 $USER 添加到 docker 组"
    echo "请重新登录或运行: newgrp docker"
    echo ""
fi

# ============================================
# 步骤 6: 克隆项目（如果还没有）
# ============================================
echo "[6/10] 获取 Aetheris 项目..."

if [ ! -d "aetheris" ]; then
    git clone https://github.com/aetheris/aetheris.git
    cd aetheris
else
    echo "项目已存在，正在更新..."
    cd aetheris
    git pull
fi

echo ""

# ============================================
# 步骤 7: 创建生产环境配置
# ============================================
echo "[7/10] 创建生产环境配置..."

if [ ! -f ".env.production" ]; then
    if [ -f ".env.example" ]; then
        cp .env.example .env.production
        echo "已创建 .env.production，请编辑配置"
    else
        cat > .env.production << 'EOF'
# Aetheris 生产环境配置
RUST_LOG=info
RUST_BACKTRACE=0

# 数据库配置
POSTGRES_USER=aetheris
POSTGRES_PASSWORD=$(openssl rand -hex 16)
POSTGRES_DB=aetheris
DATABASE_URL=postgresql://aetheris:${POSTGRES_PASSWORD}@postgres:5432/aetheris

# 向量数据库配置
QDRANT_URL=http://qdrant:6333

# LLM 提供商配置（DeepSeek为默认推荐）
LLM_PROVIDER=deepseek
DEEPSEEK_API_KEY=sk-your-deepseek-api-key-here
DEEPSEEK_API_BASE=https://api.deepseek.com/v1
DEEPSEEK_MODEL=deepseek-chat

# 安全配置
JWT_SECRET=$(openssl rand -hex 32)
ENCRYPTION_KEY=$(openssl rand -hex 32)

# IM 平台配置
WECHAT_WORK_ENABLED=false
DINGTALK_ENABLED=false
FEISHU_ENABLED=false
WECHAT_ENABLED=false
EOF
        echo "已生成默认 .env.production，请修改配置"
    fi
fi

echo ""

# ============================================
# 步骤 8: 创建生产级 docker-compose 配置
# ============================================
echo "[8/10] 创建生产级 Docker Compose 配置..."

if [ ! -f "docker-compose.production.yml" ]; then
    cat > docker-compose.production.yml << 'EOF'
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    container_name: aetheris-postgres
    restart: unless-stopped
    environment:
      POSTGRES_USER: ${POSTGRES_USER:-aetheris}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ${POSTGRES_DB:-aetheris}
      PGDATA: /var/lib/postgresql/data/pgdata
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d:ro
    ports:
      - "127.0.0.1:5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER:-aetheris}"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 10s
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '0.5'
          memory: 1G

  qdrant:
    image: qdrant/qdrant:v1.8.2
    container_name: aetheris-qdrant
    restart: unless-stopped
    ports:
      - "127.0.0.1:6333:6333"
      - "127.0.0.1:6334:6334"
    volumes:
      - qdrant_data:/qdrant/storage
    healthcheck:
      test: ["CMD-SHELL", "wget --no-verbose --tries=1 --spider http://localhost:6333/healthz || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '0.5'
          memory: 1G

  aetheris:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: aetheris-app
    restart: unless-stopped
    ports:
      - "127.0.0.1:3000:3000"
    environment:
      DATABASE_URL: ${DATABASE_URL}
      QDRANT_URL: ${QDRANT_URL:-http://qdrant:6333}
      RUST_LOG: ${RUST_LOG:-info}
      LLM_PROVIDER: ${LLM_PROVIDER:-deepseek}
      DEEPSEEK_API_KEY: ${DEEPSEEK_API_KEY}
      DEEPSEEK_API_BASE: ${DEEPSEEK_API_BASE:-https://api.deepseek.com/v1}
      DEEPSEEK_MODEL: ${DEEPSEEK_MODEL:-deepseek-chat}
      JWT_SECRET: ${JWT_SECRET}
      ENCRYPTION_KEY: ${ENCRYPTION_KEY}
      WECHAT_WORK_ENABLED: ${WECHAT_WORK_ENABLED:-false}
      DINGTALK_ENABLED: ${DINGTALK_ENABLED:-false}
      FEISHU_ENABLED: ${FEISHU_ENABLED:-false}
      WECHAT_ENABLED: ${WECHAT_ENABLED:-false}
    depends_on:
      postgres:
        condition: service_healthy
      qdrant:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "wget --no-verbose --tries=1 --spider http://localhost:3000/api/health || exit 1"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
    deploy:
      resources:
        limits:
          cpus: '4'
          memory: 8G
        reservations:
          cpus: '1'
          memory: 2G
    logging:
      driver: "json-file"
      options:
        max-size: "100m"
        max-file: "10"

volumes:
  postgres_data:
    driver: local
  qdrant_data:
    driver: local
EOF
    echo "已创建 docker-compose.production.yml"
fi

echo ""

# ============================================
# 步骤 9: 创建备份脚本
# ============================================
echo "[9/10] 创建备份脚本..."

mkdir -p scripts

cat > scripts/backup.sh << 'EOF'
#!/bin/bash
# Aetheris 数据库备份脚本

BACKUP_DIR="/opt/backups/aetheris"
DATE=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=7

mkdir -p $BACKUP_DIR

echo "开始备份: $DATE"

# 备份 PostgreSQL
if docker ps -q --filter "name=aetheris-postgres" | grep -q .; then
    docker exec aetheris-postgres pg_dump -U ${POSTGRES_USER:-aetheris} ${POSTGRES_DB:-aetheris} | gzip > $BACKUP_DIR/postgres_$DATE.sql.gz
    echo "PostgreSQL 备份完成: postgres_$DATE.sql.gz"
else
    echo "警告: PostgreSQL 容器未运行"
fi

# 清理旧备份
find $BACKUP_DIR -name "*.gz" -mtime +$RETENTION_DAYS -delete
echo "已清理 $RETENTION_DAYS 天前的备份"

echo "备份完成: $DATE"
EOF

chmod +x scripts/backup.sh

echo "备份脚本已创建: scripts/backup.sh"
echo ""

# ============================================
# 步骤 10: 部署完成提示
# ============================================
echo "[10/10] 部署准备完成！"
echo ""
echo "=========================================="
echo "  接下来的步骤："
echo "=========================================="
echo ""
echo "1. 编辑生产环境配置:"
echo "   vim .env.production"
echo ""
echo "2. 启动服务:"
echo "   docker compose --env-file .env.production -f docker-compose.production.yml up -d"
echo ""
echo "3. 查看服务状态:"
echo "   docker compose --env-file .env.production -f docker-compose.production.yml ps"
echo ""
echo "4. 查看日志:"
echo "   docker compose --env-file .env.production -f docker-compose.production.yml logs -f"
echo ""
echo "5. 验证部署:"
echo "   curl http://localhost:3000/api/health"
echo ""
echo "6. 设置定时备份（每天凌晨 2 点）:"
echo "   crontab -e"
echo "   添加: 0 2 * * * /opt/aetheris/scripts/backup.sh"
echo ""
echo "=========================================="
echo "  完整文档请查看:"
echo "  .trae/documents/ubuntu_production_deployment_plan.md"
echo "=========================================="
echo ""
