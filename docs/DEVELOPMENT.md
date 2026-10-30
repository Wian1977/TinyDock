# TinyDock 开发笔记（Developer Notes）

> TinyDock：用 Rust (gtk-rs 0.16 / GTK3) 编写的轻量 dock，主打「弹性缩放」悬停动画。
> 随包附带 tclock（Rust + eframe 的桌面模拟时钟）。最后更新：2026-10-30

## 1. 已实现功能

- **弹性悬停放大**：移入图标 `easeOutBack` 弹性放大，移出弹性缩回；动画从「当前缩放值」平滑接管（中断旧定时器再起新动画），快速进出不卡中间态。
- **放大倍数可调**：右键空白处 →「悬停放大」子菜单可选 **1× / 2× / 3×**（实际值 1.45 / 2.0 / 3.0），选择持久化到 `~/.config/tinydock/hover_scale`。
- **运行应用检测**：通过 libwnck-3 FFI（500ms 轮询）识别每个应用运行的窗口；只对 NORMAL 类型窗口计数，避免桌面/面板等杂项。
- **左键预览 / 切换**：左键弹出该应用所有窗口列表，点击任一项激活对应窗口；单窗口时直接聚焦（不弹菜单）。
- **右键图标菜单**：打开新窗口 / 关闭窗口（仅 1 个窗口时）/ 关闭全部窗口 / 从 dock 移除。
- **右键空白处菜单**：添加应用…（选 .desktop）/ 悬停放大倍数 / 退出。
- **StartupWMClass 反查**：用 `gio::AppInfo::all()` 建立 `StartupWMClass → .desktop id` 映射，避免匹配不到时出现多余“齿轮”图标。
- **底部圆点指示**：用底部小圆点表示运行状态——无打开窗口：无点；有窗口但非焦点：白点；拥有当前焦点窗口：蓝点。圆点固定占位，图标不跳动。
- **图标清晰**：按 `HIRES=144` 高分辨率渲染，静止时缩小到 `BASE=48` 显示，避免栅格放大发虚。
- **底边对齐放大**：图标底边对齐在格子里，放大时向上生长（不向下压栏）。
- **窗口属性**：无边框、不出现在任务栏/分页器、置顶、不抢焦点、底部栏为半透明圆角（由 cairo 自绘，窗口其余区域透明）。

## 2. 编译与运行

工具链：rustc/cargo 1.99.x，国内可配置 rsproxy.cn 镜像加速。
系统依赖：libgtk-3-dev、libwnck-3-dev、pkg-config（dock）；tclock 另需 libxkbcommon-dev、libwayland-dev、libx11-dev、libgl1-mesa-dev、libglu1-mesa-dev、libasound2-dev。

```bash
cargo build            # debug
cargo build --release  # release（推荐）
```

- dock 二进制：`target/release/tinydock`
- tclock 源码位于仓库 `tclock/` 子目录，随 `make deb` / `make install` 一并构建。
- 直接运行：`target/release/tinydock`（需在本机桌面会话，依赖 DISPLAY 与窗口管理器）。
- 开机自启：安装后写入系统/用户自启（tinydock 与 tclock 均随登录启动）。

## 3. 常用调整（改完重新 `cargo build --release`）

- **停靠位置**：`main.rs` 中 `let pos = Position::Bottom;` → 改 `Left` / `Right` / `Top`。
- **尺寸相关常量**（文件顶部）：
  - `BASE = 48`：静止态图标显示尺寸
  - `MAX_SCALE = 3.0`：悬停放大支持的最大倍数（决定固定格子大小）
  - `CELL_W = BASE + 12`：图标槽位**宽度**（刻意收窄，紧凑；悬停放大时横向溢出覆盖相邻图标，即经典弹性 dock 效果）
  - `CELL_H = (BASE*MAX_SCALE).ceil() + INDICATOR_H + 12`：槽位**高度**，预留最大放大空间，避免裁切；宽/高拆开是为了“紧凑 + 不裁切”兼得
  - `INDICATOR_H = 16`：底部圆点固定占位高度（固定占位防止图标跳动）
  - `BAR_PAD = 6`：底部栏与窗口边缘内边距（已收紧，更紧凑）
  - `HIRES = 144`：图标渲染精度（越大越清晰、内存略增）
  - **更紧凑**：容器间距 `4`、四边边距 `6`；槽位宽度 `CELL_W=60`（原 ~180 见方），图标更贴近。
- **放大倍数选项**：`on_right` 里的 `[("1×", 1.45), ("2×", 2.0), ("3×", 3.0)]`；修改后 `MAX_SCALE` 要 ≥ 最大值否则 3× 会被裁切。
- **动画手感**：`ease_out_back` 里的 `c1`（默认 1.70158，越大弹得越夸张，过冲约 10%）；时长在 `make_icon` 的 `animate(.., Duration::from_millis(200))` / `animate(.., 1.0, Duration::from_millis(250))`。
- **放大上限保护**：`animate` 内每帧尺寸被 `clamp` 到 `[BASE-4, CELL_H-6]`，避免过冲时图标超出格子被裁切/抖动。

## 4. 文件结构

- `Cargo.toml`：依赖 gtk/gio/glib/gdk-pixbuf 0.16 + libwnck-3 FFI（build 里 pkg-config）
- `src/main.rs`：dock 全部逻辑（单文件）
- `tclock/`：随包附带的 tclock 时钟源码（Rust + eframe）
- `~/.config/tinydock/launchers.txt`：启动项列表（每行一个 .desktop 文件名，增删自动保存）
- `~/.config/tinydock/hover_scale`：悬停放大倍数（右键菜单写入）
- `~/.config/autostart/tinydock.desktop`、`tclock.desktop`：自启入口

## 5. 实现说明 / 踩坑记录

- gtk-rs 0.16 **没有** `IconTheme::load_gicon`，只能 `lookup_by_gicon()` → `IconInfo::load_icon()` 两步加载 `gio::Icon`。
- gdk-0.16 **移除了** `Screen::width()/height()`，定位改用主显示器几何：`gdk::Display::default()` → `primary_monitor()` → `geometry()`。
- 部件名设置方法是 `set_widget_name(...)`（doc alias 叫 set_name），不是 `set_name`。
- 不要 `use gtk::prelude::*` 全局导入（会与 `screen()`、`width()` 等多 trait 方法冲突）；当前用显式 trait 调用 `gtk::prelude::WidgetExt::screen(window)` 规避歧义。
- **菜单定位**：用 `menu.popup_at_rect(&gwin, &rect, Gravity::North, Gravity::South, None)`，其中 `rect` 由 `widget.translate_coordinates(&self.window, 0, 0)` 算出相对 dock 窗口坐标。不要尝试 `popup` 的手动 position 闭包——gtk-rs 0.16 下闭包无法满足 `Fn` 推断，编译必败。
- **底边放大**：image 用 `set_valign(End)` + `set_vexpand(true)`，vbox 里指示条 `pack_end`，放大时向上生长。
- **背景菜单悬停放大子菜单 bug 修复**：`RadioMenuItem` 必须先 `set_active` 再 `connect_toggled`。若先 connect 后 set_active，构建菜单时 GTK 自动激活会触发回调 `save_hover_scale` 误写配置文件，导致下次打开菜单时单选状态显示错误。修复后：join 分组 → set_active（匹配当前值，否则默认第一项）→ 最后 connect 回调。
- **切换倍数不漂移**：槽位尺寸 `cell_size(pos)` 是固定常量（由 `MAX_SCALE` 算出 `CELL_H`，宽度 `CELL_W`），不随当前倍数变化；倍数切换回调只改配置、不重排/不重定位。底部栏由 `window.connect_draw` 用 cairo 自绘（窗口其余透明），所以窗口可预留放大高度而不显示成巨型黑条。
- **圆点指示固定占位**：指示条始终 `set_size_request(12, INDICATOR_H)` 且保持可见，只切换 `running`(白)/`focused`(蓝)/`none`(透明) 三个 CSS 类；否则窗口数在 0↔N 之间变化时，图标会因圆点显隐而上下跳动（抖动）。
- **libwnck FFI**：用 `wnck_window_get_class_group` → `wnck_class_group_get_id` 取应用 id，`wnck_window_get_window_type` 过滤只保留 NORMAL 窗口。`wnck-3`（43.0）已移除 `wnck_application_get_id`，改用 class_group。`active_id()` 额外用 `wnck_screen_get_active_window` 取焦点窗口并解析成 dock id，用于蓝点（当前焦点窗口）。
- **StartupWMClass 反查**：`build_wmclass_map()` 遍历 `gio::AppInfo::all()`，读 `StartupWMClass` 与 `id()` 建映射，解决终端等程序匹配不到而多出齿轮图标的问题。

## 6. 已知注意 / 后续可优化

- dock 窗口为容纳最大放大，静止时比图标高（上方一段透明区域是放大的悬停热区）。若想 dock 更矮，可下调 `MAX_SCALE` 或加 XShape 输入遮罩只让栏区域接收鼠标。
- 高倍数（如 3×）悬停时，放大的图标可能向上延伸到预览下方、被预览挡住一部分——这是底边放大的固有表现；如需预览始终完整避开放大图标，可让 `icon_rect` 改为跟随“当前放大图标”的顶部（代价是 3× 时预览更高）。
- 多显示器仅按主显示器居中定位。
- 仅支持 GTK3 桌面；窗口状态依赖 libwnck-3 每 500ms 轮询。