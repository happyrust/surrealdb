#!/usr/bin/env bash
# ===========================================================
# SurrealDB 跨平台编译 & 部署脚本 (Ubuntu 22 / x86_64)
# 编译工具: cargo-zigbuild (https://github.com/rust-cross/cargo-zigbuild)
# ===========================================================
set -euo pipefail

# === 默认配置 ===
readonly TARGET="x86_64-unknown-linux-gnu"
readonly BINARY_NAME="surreal"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

SERVER_HOST="${SERVER_HOST:-123.57.182.243}"
SERVER_USER="${SERVER_USER:-root}"
SERVER_PASS="${SERVER_PASS:-Happytest123_}"
REMOTE_BIN_DIR="/usr/local/bin"
SERVICE_NAME="surrealdb"
SERVICE_PORT="8000"
DATA_DIR="/var/lib/surrealdb"
SURREAL_USER="root"
SURREAL_PASS="root"

BUILD_MODE="debug"
SKIP_BUILD=false
SKIP_DEPLOY=false
BINARY_PATH=""

# === 帮助 ===
usage() {
    cat << HELP
用法: $(basename "$0") [选项]

选项:
  --release          使用 release 模式编译（默认: debug）
  --skip-build       跳过编译，直接部署已有二进制
  --skip-deploy      只编译，不部署到服务器
  --host   HOST      服务器地址       (默认: ${SERVER_HOST})
  --user   USER      SSH 用户名       (默认: ${SERVER_USER})
  --pass   PASS      SSH 密码
  --port   PORT      SurrealDB 端口   (默认: ${SERVICE_PORT})
  -h, --help         显示帮助

环境变量: SERVER_HOST, SERVER_USER, SERVER_PASS 可覆盖默认配置
HELP
}

# === 参数解析 ===
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)      BUILD_MODE="release"; shift ;;
        --skip-build)   SKIP_BUILD=true;      shift ;;
        --skip-deploy)  SKIP_DEPLOY=true;     shift ;;
        --host)         SERVER_HOST="$2";     shift 2 ;;
        --user)         SERVER_USER="$2";     shift 2 ;;
        --pass)         SERVER_PASS="$2";     shift 2 ;;
        --port)         SERVICE_PORT="$2";    shift 2 ;;
        -h|--help)      usage; exit 0 ;;
        *) echo "未知参数: $1" >&2; usage; exit 1 ;;
    esac
done

# === 彩色输出 ===
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'
info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[ OK ]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERR ]${NC}  $*" >&2; exit 1; }
step()  { echo -e "\n${BOLD}▶ $*${NC}"; }

# === 检查本地依赖 ===
check_deps() {
    step "检查本地依赖"

    # zig
    if ! command -v zig &>/dev/null; then
        warn "zig 未安装，尝试 brew 安装..."
        command -v brew &>/dev/null || err "请手动安装 zig: https://ziglang.org/download/"
        brew install zig
    fi
    ok "zig $(zig version)"

    # cargo-zigbuild
    if ! cargo zigbuild --version &>/dev/null 2>&1; then
        warn "cargo-zigbuild 未安装，正在安装..."
        cargo install cargo-zigbuild
    fi
    ok "cargo-zigbuild $(cargo zigbuild --version)"

    # Rust target
    if ! rustup target list --installed 2>/dev/null | grep -q "^${TARGET}"; then
        warn "Rust target ${TARGET} 未安装，正在添加..."
        rustup target add "${TARGET}"
    fi
    ok "Rust target: ${TARGET}"

    # sshpass（密码认证）
    if ! command -v sshpass &>/dev/null; then
        warn "sshpass 未安装，尝试 brew 安装..."
        command -v brew &>/dev/null || err "请手动安装 sshpass"
        brew install sshpass
    fi
    ok "sshpass 已就绪"
}

# === 编译 ===
do_build() {
    step "编译 SurrealDB (模式: ${BUILD_MODE}, 目标: ${TARGET})"

    cd "${PROJECT_ROOT}"

    local cargo_args=(zigbuild --target "${TARGET}")
    [[ "${BUILD_MODE}" == "release" ]] && cargo_args+=(--release)

    info "执行: cargo ${cargo_args[*]}"
    cargo "${cargo_args[@]}"

    BINARY_PATH="${PROJECT_ROOT}/target/${TARGET}/${BUILD_MODE}/${BINARY_NAME}"
    [[ -f "${BINARY_PATH}" ]] || err "编译输出未找到: ${BINARY_PATH}"

    local size
    size=$(du -sh "${BINARY_PATH}" | cut -f1)
    ok "编译成功: ${BINARY_PATH} (${size})"
}

# === SSH/SCP 辅助 ===
SSH_OPTS=(-o StrictHostKeyChecking=no -o ConnectTimeout=15 -o LogLevel=ERROR)

ssh_run() {
    sshpass -p "${SERVER_PASS}" ssh "${SSH_OPTS[@]}" "${SERVER_USER}@${SERVER_HOST}" "$@"
}

scp_upload() {
    sshpass -p "${SERVER_PASS}" scp "${SSH_OPTS[@]}" "$@"
}

# === 部署 ===
do_deploy() {
    step "部署到 ${SERVER_USER}@${SERVER_HOST}:${SERVICE_PORT}"

    # 确认二进制路径
    if [[ -z "${BINARY_PATH}" ]]; then
        BINARY_PATH="${PROJECT_ROOT}/target/${TARGET}/${BUILD_MODE}/${BINARY_NAME}"
    fi
    [[ -f "${BINARY_PATH}" ]] || err "未找到二进制: ${BINARY_PATH}\n  先编译或用 --skip-build=false 运行完整流程"

    local size
    size=$(du -sh "${BINARY_PATH}" | cut -f1)
    info "待部署: ${BINARY_PATH} (${size})"

    # 测试 SSH 连接
    info "测试 SSH 连接..."
    ssh_run "echo '连接成功'" || err "SSH 连接失败，请检查主机/凭据"
    ok "SSH 连接正常"

    # 远程系统信息
    local remote_info
    remote_info=$(ssh_run "uname -sr 2>/dev/null || echo unknown")
    ok "远程系统: ${remote_info}"

    # 停止旧服务
    info "停止旧服务 (如有)..."
    ssh_run "systemctl stop ${SERVICE_NAME} 2>/dev/null || true"

    # 上传二进制
    info "上传二进制文件..."
    scp_upload "${BINARY_PATH}" "${SERVER_USER}@${SERVER_HOST}:/tmp/${BINARY_NAME}.upload"
    ok "上传完成"

    # 生成远程安装脚本（本地变量在此展开为具体值）
    local tmp_script
    tmp_script=$(mktemp /tmp/surreal-deploy-XXXXXX.sh)
    trap "rm -f ${tmp_script}" RETURN

    # 注意：此 heredoc 无引号，${...} 均为本地变量展开；\$(...) 为远程变量
    cat > "${tmp_script}" << EOF
#!/bin/bash
set -euo pipefail

BIN_DST="${REMOTE_BIN_DIR}/${BINARY_NAME}"
DATA_DIR="${DATA_DIR}"
SERVICE="${SERVICE_NAME}"

echo ""
echo "=== [1/4] 安装二进制 ==="
mv /tmp/${BINARY_NAME}.upload "\${BIN_DST}"
chmod +x "\${BIN_DST}"
echo "已安装: \$(ls -lh \${BIN_DST})"
"\${BIN_DST}" --version 2>/dev/null || echo "(--version 不可用，跳过)"

echo ""
echo "=== [2/4] 创建数据目录 ==="
mkdir -p "\${DATA_DIR}"
chmod 700 "\${DATA_DIR}"
echo "数据目录: \${DATA_DIR}"

echo ""
echo "=== [3/4] 写入 systemd 服务配置 ==="
cat > /etc/systemd/system/\${SERVICE}.service << 'UNIT_EOF'
[Unit]
Description=SurrealDB - The ultimate multi-model database
Documentation=https://surrealdb.com/docs
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
Group=root
ExecStart=${REMOTE_BIN_DIR}/${BINARY_NAME} start \
    --log info \
    --user ${SURREAL_USER} \
    --pass ${SURREAL_PASS} \
    --bind 0.0.0.0:${SERVICE_PORT} \
    surrealkv://${DATA_DIR}/data
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
UNIT_EOF

echo "服务配置已写入: /etc/systemd/system/\${SERVICE}.service"

echo ""
echo "=== [4/4] 启动服务 ==="
systemctl daemon-reload
systemctl enable "\${SERVICE}"
systemctl restart "\${SERVICE}"
sleep 3

echo ""
echo "--- 服务状态 ---"
systemctl status "\${SERVICE}" --no-pager -l || true
echo ""
echo "--- 最近 20 行日志 ---"
journalctl -u "\${SERVICE}" -n 20 --no-pager || true
EOF

    # 上传并执行远程脚本
    info "上传安装脚本..."
    scp_upload "${tmp_script}" "${SERVER_USER}@${SERVER_HOST}:/tmp/surreal-install.sh"

    info "执行远程安装..."
    ssh_run "chmod +x /tmp/surreal-install.sh && /tmp/surreal-install.sh; rm -f /tmp/surreal-install.sh"

    echo ""
    ok "=================================================="
    ok "  SurrealDB 已成功部署！"
    ok "=================================================="
    ok "  主机:    ${SERVER_HOST}"
    ok "  端口:    ${SERVICE_PORT}"
    ok "  数据:    ${DATA_DIR}"
    ok "  管理:    systemctl {start|stop|restart|status} ${SERVICE_NAME}"
    ok "=================================================="
    ok "  WS:   ws://${SERVER_HOST}:${SERVICE_PORT}/rpc"
    ok "  HTTP: http://${SERVER_HOST}:${SERVICE_PORT}/health"
    ok "=================================================="
}

# === 主流程 ===
main() {
    echo ""
    echo "=============================================="
    echo "  SurrealDB 自动编译部署脚本 (Ubuntu 22)"
    echo "  工具: cargo-zigbuild | 目标: ${TARGET}"
    echo "=============================================="
    echo "  项目根目录: ${PROJECT_ROOT}"
    echo "  编译模式:   ${BUILD_MODE}"
    echo "  目标主机:   ${SERVER_USER}@${SERVER_HOST}:${SERVICE_PORT}"
    echo "=============================================="

    if [[ "${SKIP_BUILD}" == "false" ]]; then
        check_deps
        do_build
    else
        info "跳过编译 (--skip-build)"
    fi

    if [[ "${SKIP_DEPLOY}" == "false" ]]; then
        do_deploy
    else
        info "跳过部署 (--skip-deploy)"
        [[ -n "${BINARY_PATH}" ]] && ok "编译产物: ${BINARY_PATH}"
    fi
}

main
