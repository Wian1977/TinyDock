# TinyDock 交接文档 / 待办

## 当前状态
- 唯一工作副本：730 主机 `/home/u/ops/tinydock/`（本地机 `/home/u/rust-dock` 已删除）
- 开发/构建环境已验证可用：
  - Rust 1.99.1（经 `rsproxy.cn` 镜像安装，非 apt 自带的 1.75）
  - cargo 已配置 rsproxy 镜像（`~/.cargo/config.toml`）
  - `tinydock`（GTK3）与 `tclock`（eframe）均可 `cargo build --release`
  - `./build-deb.sh 1.0` 可生成 `tinydock_1.0_amd64.deb`（24.04 / Mint 22 的 t64 变体，依赖含 `libgtk-3-0t64`）
- **Git 推送：已打通**（见下）

## 已解决（原阻塞项）
### SSH 公钥 + remote 地址
- 730 的 ed25519 公钥（~/.ssh/id_ed25519.pub）已加入 GitHub 账户 Wian1977：

  ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGigthuN5ihZgVSXzDl99n3dPzB3p67zZumrkSOabY4B u@u730

- **坑**：仓库 remote 原本是 `github.com:Wian1977/TinyDock.git`（缺用户名），git 会默认用本地用户 `u` 去连而被拒；必须显式写成 `git@github.com:Wian1977/TinyDock.git`。已 `git remote set-url` 修正。
- 验证：`ssh -T git@github.com` 显示 `Hi Wian1977!`；`git ls-remote origin` 正常。
- 注：最早那把 RSA 长密钥（id_rsa.pub）在**别的** GitHub 账户上，切勿再用。

## 其它备忘（非阻塞）
- 在 730 构建前需先 `source ~/.cargo/env`（rustup 装的 cargo 不在默认 PATH）
- 推送前确认当前 730 公钥与 GitHub 登记的一致（若重装 730 系统/重新生成密钥需同步更新 GitHub）
- Mint 20 暂无 CI 预编译包，需本地 `make deb DEB_SUFFIX=_ubuntu20`（用户暂不需要）
- Cargo.lock 在 730（cargo 1.99）下为 version 3，与仓库已提交的 version 4 仅有标记差异、无实际依赖变化，可忽略
