# tclock

一个轻量的桌面模拟时钟挂件。Rust + eframe(egui) 实现，180×180 圆形透明表盘，支持拖动、右键退出、开机自启。

---

## 1. 项目结构

```
~/workspace/tclock/
├── Cargo.toml        # 依赖声明
├── Cargo.lock        # 依赖版本锁定
├── src/
│   └── main.rs       # 全部源码
├── target/           # 编译产物（可 cargo clean 删除）
└── README.md         # 本文档
```

已安装位置：

| 路径 | 作用 |
| :--- | :--- |
| `~/.local/bin/tclock` | 可执行文件 |
| `~/.local/share/applications/tclock.desktop` | 应用菜单入口 |
| `~/.config/autostart/tclock.desktop` | 开机自启动条目 |

---

## 2. 依赖

`Cargo.toml`：

```toml
[package]
name = "tclock"
version = "0.2.0"
edition = "2021"

[dependencies]
eframe = "0.32"
chrono = "0.4"
```

| crate | 用途 |
| :--- | :--- |
| `eframe` | 桌面应用框架，封装窗口创建、事件循环、OpenGL 渲染 |
| `chrono` | 读取本地时间（时、分、秒、纳秒） |

`eframe` 会自动引入 `egui`（即时模式 GUI 库），代码中通过 `eframe::egui` 引用，不单独声明。

---

## 3. 代码结构

`src/main.rs` 分为四部分：程序入口、应用状态、UI 更新、表盘绘制。

### 3.1 程序入口 `main()`

```rust
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([180.0, 180.0])
            .with_min_inner_size([120.0, 120.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(true)
            .with_taskbar(false),
        ..Default::default()
    };
    eframe::run_native(
        "tclock",
        options,
        Box::new(|cc| Ok(Box::new(ClockApp::new(cc)))),
    )
}
```

`ViewportBuilder` 各项配置的作用：

| 方法 | 效果 |
| :--- | :--- |
| `with_inner_size([180.0, 180.0])` | 默认窗口尺寸 180×180 像素 |
| `with_min_inner_size([120.0, 120.0])` | 允许拖拽缩小的下限 |
| `with_decorations(false)` | 去掉标题栏和边框，只剩表盘 |
| `with_transparent(true)` | 窗口背景透明（依赖桌面合成器） |
| `with_always_on_top()` | 始终置顶 |
| `with_resizable(true)` | 允许拖拽边缘调整大小 |
| `with_taskbar(false)` | 不在任务栏显示图标 |

`run_native` 的闭包在 eframe 0.32 中要求返回 `Result<Box<dyn App>, Box<dyn Error>>`，所以是 `Ok(Box::new(...))`。

### 3.2 应用状态 `ClockApp`

```rust
struct ClockApp;

impl ClockApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self
    }
}
```

时钟是无状态的——每帧都重新读取当前时间并重绘，所以 `ClockApp` 不需要保存任何字段。

### 3.3 UI 更新 `update()`

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    ctx.request_repaint_after(Duration::from_millis(33));
    ...
}
```

关键逻辑：

1. **帧率控制**：`request_repaint_after(33ms)` 约等于 30 FPS，秒针平滑走动，同时不会占满 CPU。
2. **透明清屏**：`clear_color` 返回 `[0.0, 0.0, 0.0, 0.0]`，alpha 为 0，窗口背景完全透明。
3. **拖动**：`response.drag_started()` 时发送 `ViewportCommand::StartDrag`，整个窗口任何位置都能拖动。
4. **右键菜单**：`response.context_menu()` 提供“退出”选项，发送 `ViewportCommand::Close`。

### 3.4 表盘绘制 `draw_clock()`

绘制顺序（后画的覆盖先画的）：

| 顺序 | 内容 | 实现 |
| :--- | :--- | :--- |
| 1 | 阴影 | `circle_filled` 偏移 (0,3)，黑色 alpha 45 |
| 2 | 表盘底色 | 白色 alpha 235 |
| 3 | 外圈描边 | 两道 `circle_stroke` |
| 4 | 60 个刻度 | 循环 `line_segment`，5 的倍数为粗刻度 |
| 5 | 数字 12/3/6/9 | `painter.text`，位置按角度计算 |
| 6 | 时针 | `hand()` 闭包，长度 50% 半径 |
| 7 | 分针 | 长度 72% 半径 |
| 8 | 秒针 | 长度 85% 半径，红色 |
| 9 | 中心圆点 | 红色 + 白色内点 |

**角度计算**：屏幕坐标 y 轴向下，所以 `Vec2::new(sin(θ), -cos(θ))` 得到的是指向表盘上方的方向向量。时针角度：

```rust
(h + m / 60.0 + s / 3600.0) * 30.0
```

每小时 30°，加上分、秒的贡献，时针会平滑地落在两个整点之间，而不是跳跃。

**指针绘制闭包** `hand()`：从 `center - dir * tail` 到 `center + dir * len` 画一条线，两端各画一个圆点让指针末端圆润。`tail` 是指针向后延伸的长度，让指针穿过中心，视觉上更像真实钟表。

---

## 4. 构建与安装

### 4.1 编译

```bash
cd ~/workspace/tclock
cargo build --release
```

产物在 `target/release/tclock`。

### 4.2 安装

```bash
install -m 755 target/release/tclock ~/.local/bin/tclock
```

### 4.3 一键安装脚本（可选）

创建 `~/workspace/tclock/install.sh`：

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release
install -m 755 target/release/tclock "$HOME/.local/bin/tclock"
echo "已安装到 ~/.local/bin/tclock"
```

```bash
chmod +x install.sh
./install.sh
```

---

## 5. 桌面集成

### 5.1 应用菜单入口

`~/.local/share/applications/tclock.desktop`：

```ini
[Desktop Entry]
Type=Application
Name=tclock
Comment=Desktop analog clock widget
Exec=~/.local/bin/tclock
Icon=clock
Terminal=false
Categories=Utility;
```

创建后刷新菜单缓存：

```bash
update-desktop-database ~/.local/share/applications
```

### 5.2 开机自启动

`~/.config/autostart/tclock.desktop`：

```ini
[Desktop Entry]
Type=Application
Name=tclock
Exec=sh -c "sleep 3 && exec ~/.local/bin/tclock"
Icon=clock
Terminal=false
X-GNOME-Autostart-enabled=true
X-Cinnamon-Autostart-enabled=true
NoDisplay=true
```

`sleep 3` 等桌面加载完成后再启动，避免登录瞬间资源竞争导致偶尔不出现。

---

## 6. 使用说明

| 操作 | 效果 |
| :--- | :--- |
| 拖动窗口任意位置 | 移动时钟 |
| 拖拽窗口边缘 | 调整大小 |
| 右键 | 弹出菜单，可选“退出” |
| `pkill tclock` | 命令行强制退出 |
| `pgrep -a tclock` | 确认是否在运行 |

从终端启动且希望关闭终端后继续运行：

```bash
setsid tclock >/dev/null 2>&1 &
```

---

## 7. 常见问题

**Q：背景是黑色而不是透明？**
桌面合成器未启用。检查“系统设置 → 通用 → 合成器”，确保开启。

**Q：编译报 `with_taskbar` 方法不存在？**
`eframe` 版本低于 0.31。确认 `Cargo.toml` 里是 `eframe = "0.32"`，然后 `cargo update`。

**Q：编译报 `run_native` 返回类型不匹配？**
eframe 0.32 要求闭包返回 `Result`，写法必须是 `Box::new(|cc| Ok(Box::new(ClockApp::new(cc))))`。

**Q：菜单里找不到 tclock？**
运行 `update-desktop-database ~/.local/share/applications`，或 `Alt+F2` 输入 `r` 重启 Cinnamon。

**Q：如何彻底卸载？**

```bash
rm -f ~/.local/bin/tclock
rm -f ~/.local/share/applications/tclock.desktop
rm -f ~/.config/autostart/tclock.desktop
rm -rf ~/workspace/tclock
```

`~/.cargo/` 和 `~/.rustup/` 是 Rust 环境本身，如果不再开发其他 Rust 项目可以一并删除。

---

## 8. 分发说明

`target/release/tclock` 是原生二进制，可直接拷给其他 x86_64 Linux 用户运行，前提是对方 glibc 版本不低于编译机、且已安装 X11/Wayland 基础库（桌面 Linux 默认都有）。

如需更宽的兼容性，可静态链接 C 库后重新编译：

```bash
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release
```
