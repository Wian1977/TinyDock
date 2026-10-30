# TinyDock 推广文案（复制即用）

## 一句话
A Super Easy Dock —— 用 Rust + GTK3 写的轻量级弹性 dock，悬停弹性放大、窗口指示圆点，随附 tclock 时钟。支持 Ubuntu 24.04 / Linux Mint 21·22。

## 中文文案（V2EX / 少数派 / 知乎 / Mint 中文论坛 / Linux Mint 论坛）

折腾桌面美化时写的小工具 **TinyDock**，分享给大家：

- 用 **Rust + GTK3** 实现，主打悬停时图标**弹性放大**（easeOutBack 过冲回弹），移出平滑缩回；
- 图标下方**小圆点指示窗口状态**：蓝=当前聚焦 / 白=有窗口 / 无=未运行；
- 右键菜单可切换放大倍数（1×/2×/3×）、添加应用、透明栏等；
- 随包附带 **tclock** 桌面时钟，开机自动启动；
- 已提供 Debian 包，**Ubuntu 24.04 / Linux Mint 21·22** 直接 `sudo dpkg -i` 即可。

GitHub（含下载）：https://github.com/Wian1977/TinyDock

## English（Reddit r/linux, r/linuxmint, r/unixporn, Hacker News Show）

**TinyDock** — a lightweight elastic-zoom dock written in Rust + GTK3.

- Hover to elastic-zoom icons (easeOutBack overshoot), smooth spring-back on leave;
- Indicator dots under icons show window state (blue = focused / white = running / none = not running);
- Right-click menu: zoom level (1×/2×/3×), add apps, transparent bar;
- Bundled **tclock** desk clock, autostarts on login;
- Prebuilt `.deb` for **Ubuntu 24.04 / Linux Mint 21 & 22**.

GitHub (with downloads): https://github.com/Wian1977/TinyDock

### Show HN version
Show HN: TinyDock – a lightweight elastic-zoom dock in Rust + GTK3. 
It does hover elastic zoom with spring-back, window-state indicator dots, a right-click 
menu, and bundles a tclock desk clock. Prebuilt debs for Ubuntu 24.04 / Linux Mint 21 & 22. 
Source + downloads: https://github.com/Wian1977/TinyDock

## 标签 / Hashtags
#linux #rust #gtk3 #dock #linuxmint #opensource #desktopricing

## 录屏 GIF 建议
在桌面录制：鼠标悬停 dock 图标观察弹性放大、移开回弹；再切到不同窗口看底部圆点颜色变化。
工具：Peek / Kooha（Wayland）、ffmpeg 命令行、或 OBS 录屏后转 GIF。建议 5–10 秒、宽 600px 左右。

## 国内渠道（Reddit 等境外站点不可达时）

GitHub 在国内访问慢且不稳，建议同步做以下动作：

- **Gitee 镜像仓库（最重要）**：登录 Gitee → 右上角「+」→ 导入仓库 → 填 `https://github.com/Wian1977/TinyDock` → 导入。国内克隆/浏览快；导入后到「发行版」上传 `.deb`，解决国内下载慢的痛点。
- **社区发布**：V2EX「分享创造」节点、开源中国(OSChina)、掘金、Linux Mint 中文论坛、少数派——直接复制上方中文文案。
- **视频演示**：在 B 站发一段悬停弹性放大 + 圆点变色的录屏（国内流量大、技术区接受度高），简介放 GitHub / Gitee 链接。
- **知乎**：包装成「用 Rust + GTK3 从零写一个弹性 dock」的经验文，顺带安利项目。

> 痛点提醒：很多国内用户连 GitHub 都打不开，所以**务必提供 Gitee 镜像或国内可访问的下载地址**，否则他们看到项目也装不上。

## GitHub 站内 / 生态曝光（不依赖外部社区）

即使不逛 Reddit 等外部社区，也能在 GitHub 生态内提升可见度；受众是能正常访问 GitHub 的海外用户，不受你本地网络影响。

- **Topics 话题页**：仓库已出现在 `github.com/topics/<词>`。建议在仓库 About 区补充关键词：
  `dock, launcher, gtk3, gtk-rs, rust, linux, linuxmint, wayland, x11, desktop, cinnamon, debian`。
  词越精准，被搜到的概率越高。
- **投稿 awesome 列表（最高杠杆）**：向以下高流量仓库提 PR，把 TinyDock 加进它们的清单，项目会直接出现在这些仓库的 README 里（曝光巨大）：
  - [awesome-rust](https://github.com/rust-unofficial/awesome-rust)（GUI/Apps 分类；若想进此列表，建议先把 crate 发到 crates.io）
  - [awesome-linux](https://github.com/iurisegtovich/awesome-linux)（桌面环境/工具分类）
  - 各类 GTK / 桌面美化清单
  下方提供了可直接复制的 PR 标题与描述。
- **完善 About**：仓库 About 区勾选 topics、填写 Website（可填 Release 页）、简介一句话。
- **crates.io（可选）**：`cargo publish` 把二进制发到 crates.io，进入 Rust 生态搜索；对 GUI 应用意义有限，可作为补充。

### awesome 列表 PR 文案（复制即用）

**PR 标题**：`Add TinyDock — a lightweight elastic-zoom dock in Rust + GTK3`

**PR 描述**：

```
TinyDock is a lightweight elastic-zoom dock written in Rust + GTK3.

- Hover to elastic-zoom icons (easeOutBack), smooth spring-back on leave
- Indicator dots show window state (focused / running / none)
- Right-click menu: zoom level, add apps, transparent bar
- Bundled tclock desk clock, autostarts on login
- Prebuilt .deb for Ubuntu 24.04 / Linux Mint 21 & 22

https://github.com/Wian1977/TinyDock
```