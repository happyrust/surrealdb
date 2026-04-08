# SurrealDB 自动部署到 Ubuntu 服务器

本文档说明如何使用 GitHub Actions 将 SurrealDB 编译并自动部署到你的 Ubuntu 服务器。

## 前置条件

- **GitHub** 上的仓库且已启用 Actions（GitHub 只在本仓库托管在 github.com 时运行工作流；若主仓库在 Gitee，需同步或 mirror 到 GitHub）
- Ubuntu 服务器（支持 SSH 访问）

上游最新代码可定期执行：`git fetch upstream main && git merge upstream/main`（参考 [surrealdb/main 提交历史](https://github.com/surrealdb/surrealdb/commits/main/)）。

## 一、服务器端准备

### 1. 创建部署目录

```bash
sudo mkdir -p /opt/surrealdb
sudo chown $USER:$USER /opt/surrealdb
```

### 2. 配置 SSH 密钥（推荐）

在服务器上生成或使用现有 SSH 密钥，将**公钥**添加到服务器的 `~/.ssh/authorized_keys`：

```bash
# 本地生成密钥对（若尚未有）
ssh-keygen -t ed25519 -C "github-deploy" -f ~/.ssh/github_deploy -N ""

# 将公钥复制到服务器
ssh-copy-id -i ~/.ssh/github_deploy.pub user@your-server

# 私钥内容用于下一步的 DEPLOY_KEY
cat ~/.ssh/github_deploy
```

### 3. （可选）配置 systemd 服务

若希望部署后自动重启 SurrealDB 服务，可创建 systemd 单元：

```bash
sudo tee /etc/systemd/system/surrealdb.service << 'EOF'
[Unit]
Description=SurrealDB Database
After=network.target

[Service]
Type=simple
User=root
ExecStart=/opt/surrealdb/surreal start --log info --user root --pass root file:/var/lib/surrealdb/data
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# 创建数据目录
sudo mkdir -p /var/lib/surrealdb/data
sudo systemctl daemon-reload
sudo systemctl enable surrealdb
sudo systemctl start surrealdb
```

服务名为 `surrealdb`，将在后续的 `DEPLOY_SERVICE` 中使用。

## 二、GitHub 配置

### 1. 添加 Secrets

进入仓库 **Settings → Secrets and variables → Actions**，点击 **New repository secret**，添加：

| Secret 名称 | 必填 | 说明 |
|------------|------|------|
| `DEPLOY_HOST` | ✅ | 服务器 IP 或域名（如 `192.168.1.100` 或 `db.example.com`）|
| `DEPLOY_USER` | ✅ | SSH 用户名（如 `root` 或 `ubuntu`）|
| `DEPLOY_KEY` | 二选一 | SSH 私钥完整内容（推荐，比密码更安全）|
| `DEPLOY_PASSWORD` | 二选一 | SSH 密码（若不用密钥则必填）|
| `DEPLOY_PATH` | ❌ | 部署目录，默认 `/opt/surrealdb` |
| `DEPLOY_PORT` | ❌ | SSH 端口，默认 `22` |
| `DEPLOY_SERVICE` | ❌ | systemd 服务名，部署后自动执行 `systemctl restart` |

### 2. 触发部署

**方式一：手动触发**

1. 打开 **Actions** 标签
2. 选择 **Deploy to Ubuntu**
3. 点击 **Run workflow**
4. 可选：
   - `deploy-branch`: 要部署的分支（默认 `main`）
   - `features`: 构建启用的 features（留空则使用默认集合）

**方式二：推送自动部署**

推送到 `main` 分支时，工作流会自动构建并部署。

## 三、构建说明

### 默认 Features

为兼容 GitHub 标准 runner，默认构建**不包含** `storage-tikv` 和 `ml`（需要额外依赖）。包含：

- `allocator`, `storage-mem`, `storage-surrealkv`, `storage-rocksdb`
- `scripting`, `http`, `surrealism`, `graphql`, `cli`

### 自定义 Features

如需启用 TiKV、ML 等，在手动触发时填写 `features` 输入，例如：

```
storage-tikv,jwks,ml
```

注意：`storage-tikv` 和 `ml` 的构建依赖较复杂，可能需使用项目中的 `build-linux` action（Docker 方式）。

## 四、验证部署

```bash
ssh user@your-server
/opt/surrealdb/surreal version
# 或
/opt/surrealdb/surreal start --log info memory
```

## 五、常见问题

### 构建超时

SurrealDB 编译耗时较长（约 15–30 分钟），若超时可考虑：

- 使用自托管 runner
- 或临时关闭 `push` 触发，仅保留手动触发

### SSH 连接失败

- 确认 `DEPLOY_HOST`、`DEPLOY_USER`、`DEPLOY_KEY` 或 `DEPLOY_PASSWORD` 正确
- 确认服务器防火墙放行 SSH 端口
- 检查 SSH 私钥格式（含 `-----BEGIN` / `-----END` 整段）

### 权限不足

确保部署目录对 `DEPLOY_USER` 可写，或使用有权限的用户（如 `root`）部署。
