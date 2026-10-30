use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::os::raw::{c_char, c_int, c_ulong, c_void};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gdk_pixbuf::{InterpType, Pixbuf};
use gio::prelude::*;
use gtk::prelude::*;
use gtk::Inhibit;

const BASE: i32 = 48; // 静止态图标显示尺寸
const MAX_SCALE: f64 = 3.0; // 悬停放大支持的最大倍数
const INDICATOR_H: i32 = 16; // 圆点绘制区边长（含上下留白 + 容纳光晕），固定占位防止图标跳动
const DOT_R: f64 = 3.0; // 圆点半径（小）
const CELL_W: i32 = BASE + 12; // 图标槽位宽度（紧凑；悬停放大时横向溢出覆盖相邻图标，营造弹性效果）
const CELL_H: i32 = (BASE as f64 * MAX_SCALE).ceil() as i32 + INDICATOR_H + 12; // 竖向预留放大空间，避免裁切
const BAR_PAD: i32 = 6; // 底部栏与窗口边缘内边距（紧凑）
const BAR_TOP_PAD: i32 = 16; // 底部栏顶部内边距（让出图标上方一点点空间）
const HIRES: i32 = 144; // 图标高分辨率渲染，再缩小显示，保证清晰

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Position {
    Bottom,
    Top,
    Left,
    Right,
}

struct AnimState {
    base: Pixbuf,
    image: gtk::Image,
    source: Option<glib::source::SourceId>,
    scale: f64,
    from: f64,
    to: f64,
    start: Instant,
    dur: Duration,
}

// ===== libwnck FFI（只用少量 C 函数，轮询刷新，不接信号）=====
#[link(name = "wnck-3")]
unsafe extern "C" {
    fn wnck_screen_get_default() -> *mut c_void;
    fn wnck_screen_get_windows(screen: *mut c_void) -> *mut GList;
    fn wnck_window_get_class_group(win: *mut c_void) -> *mut c_void;
    fn wnck_class_group_get_id(cg: *mut c_void) -> *const c_char;
    fn wnck_window_get_window_type(win: *mut c_void) -> c_int;
    fn wnck_window_get_name(win: *mut c_void) -> *const c_char;
    fn wnck_window_get_xid(win: *mut c_void) -> c_ulong;
    fn wnck_window_activate(win: *mut c_void, timestamp: u32);
    fn wnck_window_close(win: *mut c_void, timestamp: u32);
    fn wnck_screen_get_active_window(screen: *mut c_void) -> *mut c_void;
}

#[repr(C)]
struct GList {
    data: *mut c_void,
    next: *mut GList,
    prev: *mut GList,
}

unsafe fn cstr(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    std::ffi::CStr::from_ptr(p).to_str().ok().map(|s| s.to_string())
}

#[derive(Clone)]
struct WindowInfo {
    xid: u64,
    title: String,
}

// 启动期构建：StartupWMClass(小写) -> .desktop id 的反查表
fn build_wmclass_map() -> HashMap<String, String> {
    let mut m: HashMap<String, String> = HashMap::new();
    for app in gio::AppInfo::all() {
        if let Some(da) = app.downcast_ref::<gio::DesktopAppInfo>() {
            if let Some(wm) = da.string("StartupWMClass") {
                if let Some(id) = da.id() {
                    let id = id.split('/').last().unwrap_or(&id).to_string();
                    m.insert(wm.to_lowercase(), id);
                }
            }
        }
    }
    m
}

// 将窗口的 WM class(res_class) 解析为 dock 使用的 id：
// 1) 与已固定项按文件名匹配；2) 按 StartupWMClass 反查系统 .desktop；
// 3) 退而求其次要求 <class>.desktop 真实存在；否则忽略（避免无法启动的齿轮图标）。
fn resolve_running_id(cgid: &str, launchers: &[String], wmclass: &HashMap<String, String>) -> String {
    let cg_low = cgid.to_lowercase();
    for l in launchers {
        let base = l.trim_end_matches(".desktop").to_lowercase();
        if base == cg_low {
            return l.clone();
        }
    }
    if let Some(id) = wmclass.get(&cg_low) {
        return id.clone();
    }
    if cg_low.is_empty() {
        String::new()
    } else {
        let synth = format!("{}.desktop", cg_low);
        if gio::DesktopAppInfo::new(&synth).is_some() {
            synth
        } else {
            String::new()
        }
    }
}

// 轮询枚举窗口，按 id 聚合；返回 id -> 窗口列表
fn refresh_running(launchers: &[String], wmclass: &HashMap<String, String>) -> HashMap<String, Vec<WindowInfo>> {
    let mut map: HashMap<String, Vec<WindowInfo>> = HashMap::new();
    unsafe {
        let screen = wnck_screen_get_default();
        if screen.is_null() {
            return map;
        }
        let mut l = wnck_screen_get_windows(screen);
        while !l.is_null() {
            let win = (*l).data;
            if !win.is_null() {
                // 仅保留普通窗口(类型 0=NORMAL)，排除桌面/面板/对话框等，避免无关联程序出现在 dock
                if wnck_window_get_window_type(win) != 0 {
                    l = (*l).next;
                    continue;
                }
                let xid = wnck_window_get_xid(win) as u64;
                let title = cstr(wnck_window_get_name(win)).unwrap_or_default();
                let cg = wnck_window_get_class_group(win);
                if !cg.is_null() {
                    if let Some(cgid) = cstr(wnck_class_group_get_id(cg)) {
                        let id = resolve_running_id(&cgid, launchers, wmclass);
                        if !id.is_empty() {
                            map.entry(id).or_insert_with(Vec::new).push(WindowInfo { xid, title });
                        }
                    }
                }
            }
            l = (*l).next;
        }
    }
    map
}

// 取当前焦点窗口对应的 dock id（用于蓝点指示）；不匹配任何启动项则返回空串
fn active_id(launchers: &[String], wmclass: &HashMap<String, String>) -> String {
    unsafe {
        let screen = wnck_screen_get_default();
        if screen.is_null() {
            return String::new();
        }
        let win = wnck_screen_get_active_window(screen);
        if win.is_null() {
            return String::new();
        }
        let cg = wnck_window_get_class_group(win);
        if cg.is_null() {
            return String::new();
        }
        if let Some(cgid) = cstr(wnck_class_group_get_id(cg)) {
            resolve_running_id(&cgid, launchers, wmclass)
        } else {
            String::new()
        }
    }
}

fn activate_by_xid(xid: u64, t: u32) {
    unsafe {
        let screen = wnck_screen_get_default();
        if screen.is_null() {
            return;
        }
        let mut l = wnck_screen_get_windows(screen);
        while !l.is_null() {
            let win = (*l).data;
            if !win.is_null() && wnck_window_get_xid(win) as u64 == xid {
                wnck_window_activate(win, t);
                break;
            }
            l = (*l).next;
        }
    }
}

fn close_by_xid(xid: u64, t: u32) {
    unsafe {
        let screen = wnck_screen_get_default();
        if screen.is_null() {
            return;
        }
        let mut l = wnck_screen_get_windows(screen);
        while !l.is_null() {
            let win = (*l).data;
            if !win.is_null() && wnck_window_get_xid(win) as u64 == xid {
                wnck_window_close(win, t);
                break;
            }
            l = (*l).next;
        }
    }
}

// ===== 弹性缩放动画 =====
fn ease_out_back(t: f64) -> f64 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn scaled(base: &Pixbuf, size: i32) -> Pixbuf {
    base.scale_simple(size, size, InterpType::Bilinear)
        .unwrap_or_else(|| base.clone())
}

fn animate(state: Rc<RefCell<AnimState>>, to: f64, dur: Duration) {
    if let Some(s) = state.borrow_mut().source.take() {
        s.remove();
    }
    {
        let mut s = state.borrow_mut();
        let f = s.scale;
        s.to = to;
        s.from = f;
        s.dur = dur;
    }
    let start = Instant::now();
    state.borrow_mut().start = start;

    let st = state.clone();
    let sid = glib::timeout_add_local(Duration::from_millis(16), move || {
        let (from, to, start, dur, base, image) = {
            let s = st.borrow();
            (s.from, s.to, s.start, s.dur, s.base.clone(), s.image.clone())
        };
        let t = (start.elapsed().as_secs_f64() / dur.as_secs_f64()).min(1.0);
        let e = ease_out_back(t);
        let scale = from + (to - from) * e;
        let mut size = (BASE as f64 * scale).round() as i32;
        size = size.max(BASE - 4).min(CELL_H - 6); // 限制放大上限，避免超过格子被裁切/抖动
        if let Some(pb) = base.scale_simple(size, size, InterpType::Bilinear) {
            image.set_from_pixbuf(Some(&pb));
        }
        let mut s = st.borrow_mut();
        s.scale = scale;
        if t >= 1.0 {
            s.source = None;
            glib::Continue(false)
        } else {
            glib::Continue(true)
        }
    });
    state.borrow_mut().source = Some(sid);
}

// 预览/菜单贴着“1× 静止图标”的位置弹出：图标底边对齐在槽位底部，
// 因此用格子底边往上 BASE 高度即为静止图标区域，与当前放大倍数无关。
fn icon_rect(window: &gtk::Window, eb: &gtk::EventBox) -> gdk::Rectangle {
    let (ex, ey) = eb
        .translate_coordinates(window, 0, 0)
        .unwrap_or((0, 0));
    let alloc = eb.allocation();
    let h = BASE + INDICATOR_H; // 槽位底边减去图标+指示条，得到 1× 静止图标区域（与放大倍数无关）
    let top = ey + alloc.height() - h;
    let left = ex + (alloc.width() - BASE) / 2;
    gdk::Rectangle::new(left, top - 6, BASE, BASE) // 上移 6px，留出与图标之间的间隙，避免遮住图标
}

// ===== 配置（固定项 + 悬停倍数）=====
fn config_dir() -> std::path::PathBuf {
    let mut p = glib::user_config_dir();
    p.push("tinydock");
    p
}

fn launchers_path() -> std::path::PathBuf {
    let mut p = config_dir();
    p.push("launchers.txt");
    p
}

fn hover_scale_path() -> std::path::PathBuf {
    let mut p = config_dir();
    p.push("hover_scale");
    p
}

fn bar_transparent_path() -> std::path::PathBuf {
    let mut p = config_dir();
    p.push("bar_transparent");
    p
}

fn load_bar_transparent() -> bool {
    if let Ok(s) = std::fs::read_to_string(bar_transparent_path()) {
        s.trim() == "1"
    } else {
        false
    }
}

fn save_bar_transparent(on: bool) {
    save_text(&bar_transparent_path(), if on { "1" } else { "0" });
}

fn dot_shape_path() -> std::path::PathBuf {
    let mut p = config_dir();
    p.push("dot_shape");
    p
}

fn load_dot_shape() -> u8 {
    if let Ok(s) = std::fs::read_to_string(dot_shape_path()) {
        if s.trim() == "1" { 1 } else { 0 }
    } else {
        0
    }
}

fn save_dot_shape(v: u8) {
    save_text(&dot_shape_path(), if v == 1 { "1" } else { "0" });
}

fn save_text(path: &std::path::PathBuf, data: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, data);
}

fn save_launchers(ids: &[String]) {
    save_text(&launchers_path(), &ids.join("\n"));
}

fn load_launchers() -> Vec<String> {
    if let Ok(data) = std::fs::read_to_string(launchers_path()) {
        let v: Vec<String> = data
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !v.is_empty() {
            return v;
        }
    }
    let defaults = [
        "firefox.desktop",
        "firefox-esr.desktop",
        "org.gnome.Terminal.desktop",
        "gnome-terminal.desktop",
        "nemo.desktop",
        "thunar.desktop",
        "xed.desktop",
        "org.gnome.Nautilus.desktop",
    ];
    let existing: Vec<String> = defaults
        .iter()
        .filter_map(|id| gio::DesktopAppInfo::new(id).map(|_| id.to_string()))
        .collect();
    let seed = if existing.is_empty() {
        vec!["firefox.desktop".to_string()]
    } else {
        existing
    };
    save_launchers(&seed);
    seed
}

fn load_hover_scale() -> f64 {
    if let Ok(s) = std::fs::read_to_string(hover_scale_path()) {
        if let Ok(v) = s.trim().parse::<f64>() {
            return v;
        }
    }
    1.45
}

fn save_hover_scale(v: f64) {
    save_text(&hover_scale_path(), &v.to_string());
}

// 根据停靠方向返回（槽位宽, 槽位高）；尺寸固定、与当前放大倍数无关，保证切换倍数时 dock 位置不变
fn cell_size(pos: Position) -> (i32, i32) {
    match pos {
        Position::Bottom | Position::Top => (CELL_W, CELL_H),
        _ => (CELL_H, CELL_W),
    }
}

// ===== 图标加载（gtk-0.16 用 lookup_by_gicon -> IconInfo::load_icon）=====
fn load_icon(app: &gio::DesktopAppInfo, size: i32) -> Option<Pixbuf> {
    let theme = gtk::IconTheme::default()?;
    if let Some(icon) = app.icon() {
        if let Some(info) =
            theme.lookup_by_gicon(&icon, size, gtk::IconLookupFlags::FORCE_SIZE)
        {
            if let Ok(pb) = info.load_icon() {
                return Some(pb);
            }
        }
    }
    let fb = gio::ThemedIcon::new("application-x-executable");
    theme
        .lookup_by_gicon(&fb, size, gtk::IconLookupFlags::FORCE_SIZE)
        .and_then(|info| info.load_icon().ok())
}

fn fallback_pixbuf() -> Pixbuf {
    if let Some(theme) = gtk::IconTheme::default() {
        let fb = gio::ThemedIcon::new("application-x-executable");
        if let Some(info) =
            theme.lookup_by_gicon(&fb, HIRES, gtk::IconLookupFlags::FORCE_SIZE)
        {
            if let Ok(pb) = info.load_icon() {
                return pb;
            }
        }
    }
    Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, BASE, BASE).unwrap()
}

fn load_icon_by_id(id: &str, size: i32) -> Pixbuf {
    gio::DesktopAppInfo::new(id)
        .and_then(|a| load_icon(&a, size))
        .unwrap_or_else(|| scaled(&fallback_pixbuf(), size))
}

fn launch_id(id: &str, t: u32) {
    if let Some(app) = gio::DesktopAppInfo::new(id) {
        let ctx = gdk::Display::default().and_then(|d| d.app_launch_context());
        if let Some(c) = ctx.as_ref() {
            c.set_timestamp(t);
        }
        if let Err(e) = app.launch(&[], ctx.as_ref()) {
            eprintln!("tinydock: failed to launch {}: {}", id, e);
        }
    } else {
        eprintln!("tinydock: desktop app not found: {}", id);
    }
}

// ===== Dock 状态 =====
struct IconHandle {
    eb: gtk::EventBox,
    sep: gtk::EventBox,
    indicator: gtk::DrawingArea,
    ind_state: Rc<RefCell<u8>>,
}

#[derive(Clone)]
struct DockState {
    window: gtk::Window,
    container: gtk::Box,
    pos: Position,
    bar_transparent: Rc<RefCell<bool>>,
    dot_shape: Rc<RefCell<u8>>,
    hover_scale: Rc<RefCell<f64>>,
    launchers: Rc<RefCell<Vec<String>>>,
    wmclass: HashMap<String, String>,
    running: Rc<RefCell<HashMap<String, Vec<WindowInfo>>>>,
    hidden: Rc<RefCell<HashSet<String>>>,
    icons: Rc<RefCell<HashMap<String, IconHandle>>>,
}

impl DockState {
    // 重新扫描窗口并同步 dock 上的图标（新增/移除/更新运行指示）
    fn refresh_and_reconcile(&self) {
        let launchers = self.launchers.borrow();
        let map = refresh_running(&launchers, &self.wmclass);
        drop(launchers);
        *self.running.borrow_mut() = map;
        self.reconcile();
    }

    fn reconcile(&self) {
        let launchers = self.launchers.borrow();
        let focused_id = active_id(&launchers, &self.wmclass);
        let running = self.running.borrow();
        let hidden = self.hidden.borrow();
        let mut show: HashSet<String> = HashSet::new();
        for id in launchers.iter() {
            if !hidden.contains(id) {
                show.insert(id.clone());
            }
        }
        for id in running.keys() {
            if !hidden.contains(id) {
                show.insert(id.clone());
            }
        }
        let running_keys: Vec<String> = running.keys().cloned().collect();
        drop(running);
        drop(launchers);

        // 已不再运行的应用，解除“隐藏”状态，便于下次启动重新出现
        let still: HashSet<String> =
            hidden.iter().cloned().filter(|id| running_keys.contains(id)).collect();
        drop(hidden);
        let mut hidden_mut = self.hidden.borrow_mut();
        *hidden_mut = still;
        drop(hidden_mut);

        let icons = self.icons.borrow();
        let to_remove: Vec<String> =
            icons.keys().cloned().filter(|id| !show.contains(id)).collect();
        drop(icons);
        for id in &to_remove {
            self.remove_icon(id);
        }

        let mut changed = !to_remove.is_empty();
        for id in show {
            if !self.icons.borrow().contains_key(&id) {
                self.make_icon(&id);
                changed = true;
            }
            self.update_indicator(&id, &focused_id);
        }
        if changed {
            place_window(&self.window, &self.pos);
        }
    }

    fn make_icon(&self, id: &str) {
        let app = gio::DesktopAppInfo::new(id);
        let base = app
            .as_ref()
            .and_then(|a| load_icon(a, HIRES))
            .unwrap_or_else(fallback_pixbuf);
        let image = gtk::Image::new();
        image.set_from_pixbuf(Some(&scaled(&base, BASE)));
        image.set_halign(gtk::Align::Center);
        image.set_valign(gtk::Align::End);
        image.set_vexpand(true);

        // 圆点：DrawingArea 自绘，保证正圆 + 可控光晕；状态 ind_state: 0无 / 1蓝（有窗口非焦点） / 2白+光晕（当前焦点）
        let ind_state = Rc::new(RefCell::new(0u8));
        let indicator = gtk::DrawingArea::new();
        indicator.set_halign(gtk::Align::Center);
        indicator.set_size_request(INDICATOR_H, INDICATOR_H); // 方形绘制区，居中得正圆；上下留白由该尺寸决定
        indicator.set_app_paintable(true);
        {
            let st_draw = ind_state.clone();
            let shape = self.dot_shape.clone();
            indicator.connect_draw(move |w, cr| {
                let s = *st_draw.borrow();
                if s != 0 {
                    let alloc = w.allocation();
                    let cx = alloc.width() as f64 / 2.0;
                    let cy = alloc.height() as f64 / 2.0;
                    let r = DOT_R;
                    let line = *shape.borrow() == 1;
                    let (rx, ry) = if line { (r * 2.4, r * 0.55) } else { (r, r) };
                    if s == 2 {
                        // 白点 + 更淡外光晕（仅作氛围，不增大视觉尺寸）
                        cr.save().ok();
                        cr.translate(cx, cy);
                        cr.scale(rx + 3.0, ry + 1.0);
                        cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
                        cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
                        cr.fill().ok();
                        cr.restore().ok();
                        cr.save().ok();
                        cr.translate(cx, cy);
                        cr.scale(rx, ry);
                        cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
                        cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                        cr.fill().ok();
                        cr.restore().ok();
                    } else {
                        // 蓝点（或线），无光晕
                        cr.save().ok();
                        cr.translate(cx, cy);
                        cr.scale(rx, ry);
                        cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
                        cr.set_source_rgba(74.0 / 255.0, 144.0 / 255.0, 217.0 / 255.0, 1.0);
                        cr.fill().ok();
                        cr.restore().ok();
                    }
                }
                Inhibit(false)
            });
        }

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.pack_start(&image, true, true, 0);
        vbox.pack_end(&indicator, false, false, 0);

        let eb = gtk::EventBox::new();
        eb.add(&vbox);
        let (cw, ch) = cell_size(self.pos);
        eb.set_size_request(cw, ch);
        eb.set_above_child(false);

        let anim = Rc::new(RefCell::new(AnimState {
            base,
            image: image.clone(),
            source: None,
            scale: 1.0,
            from: 1.0,
            to: 1.0,
            start: Instant::now(),
            dur: Duration::from_millis(200),
        }));

        let st = self.clone();
        let anim_e = anim.clone();
        eb.connect_enter_notify_event(move |_, _| {
            let anim_c = anim_e.clone();
            let hs = st.hover_scale.clone();
            animate(anim_c, *hs.borrow(), Duration::from_millis(200));
            Inhibit(false)
        });
        let anim_l = anim.clone();
        eb.connect_leave_notify_event(move |_, _| {
            let anim_c = anim_l.clone();
            animate(anim_c, 1.0, Duration::from_millis(250));
            Inhibit(false)
        });

        let st = self.clone();
        let idc = id.to_string();
        eb.connect_button_press_event(move |_, ev| {
            if ev.button() == 1 {
                st.clone().on_left(&idc, ev.time());
                Inhibit(true)
            } else if ev.button() == 3 {
                st.clone().on_right(&idc, ev.time());
                Inhibit(true)
            } else {
                Inhibit(false)
            }
        });

        // 图标之间的右键热区：透明、无色彩、全高，便于命中；右键弹出 dock 菜单，不影响图标的左/右键
        let sep = gtk::EventBox::new();
        sep.set_size_request(8, CELL_H); // 窄热区、全高，容易点中
        sep.set_visible(true);
        let st_sep = self.clone();
        sep.connect_button_press_event(move |_, ev| {
            if ev.button() == 3 {
                st_sep.show_bg_menu();
                Inhibit(true)
            } else {
                Inhibit(false)
            }
        });

        self.container.pack_start(&sep, false, false, 0);
        self.container.pack_start(&eb, false, false, 0);
        eb.show_all();
        self.icons.borrow_mut().insert(
            id.to_string(),
            IconHandle {
                eb,
                sep,
                indicator,
                ind_state,
            },
        );
    }

    fn remove_icon(&self, id: &str) {
        if let Some(h) = self.icons.borrow_mut().remove(id) {
            unsafe { h.sep.destroy(); }
            unsafe { h.eb.destroy(); }
        }
    }

    fn update_indicator(&self, id: &str, focused_id: &str) {
        let count = self.running.borrow().get(id).map(|v| v.len()).unwrap_or(0);
        if let Some(h) = self.icons.borrow().get(id) {
            // 状态：0 无点；1 蓝点（有窗口、非焦点）；2 白点+光晕（当前焦点窗口）
            let s: u8 = if count == 0 {
                0
            } else if id == focused_id {
                2
            } else {
                1
            };
            *h.ind_state.borrow_mut() = s;
            h.indicator.queue_draw();
        }
    }

    fn on_left(&self, id: &str, t: u32) {
        let wins = self.running.borrow().get(id).cloned().unwrap_or_default();
        match wins.len() {
            0 => launch_id(id, t),
            1 => activate_by_xid(wins[0].xid, t),
            _ => self.show_preview(id, t),
        }
    }

    fn show_preview(&self, id: &str, t: u32) {
        let wins = self.running.borrow().get(id).cloned().unwrap_or_default();
        let menu = gtk::Menu::new();
        for w in wins {
            let row = gtk::MenuItem::new();
            let hb = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let pb = load_icon_by_id(id, 24);
            hb.pack_start(&gtk::Image::from_pixbuf(Some(&pb)), false, false, 0);
            hb.pack_start(&gtk::Label::new(Some(&w.title)), false, false, 0);
            row.add(&hb);
            let xid = w.xid;
            row.connect_activate(move |_| activate_by_xid(xid, t));
            menu.append(&row);
        }
        menu.show_all();
        if let Some(h) = self.icons.borrow().get(id) {
            if let Some(gwin) = self.window.window() {
                let rect = icon_rect(&self.window, &h.eb);
                menu.popup_at_rect(&gwin, &rect, gdk::Gravity::North, gdk::Gravity::South, None::<&gdk::Event>);
            }
        } else {
            menu.popup_at_pointer(None::<&gdk::Event>);
        }
    }

    fn on_right(&self, id: &str, t: u32) {
        let wins = self.running.borrow().get(id).cloned().unwrap_or_default();
        let menu = gtk::Menu::new();
        let st = self.clone();
        let idc = id.to_string();

        if wins.len() <= 1 {
            let open = gtk::MenuItem::with_label("打开新窗口");
            let ido = idc.clone();
            open.connect_activate(move |_| launch_id(&ido, t));
            menu.append(&open);
            if wins.len() == 1 {
                let xid = wins[0].xid;
                let close = gtk::MenuItem::with_label("关闭窗口");
                close.connect_activate(move |_| close_by_xid(xid, t));
                menu.append(&close);
            }
        } else {
            for w in wins {
                let row = gtk::MenuItem::new();
                let hb = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                let pb = load_icon_by_id(id, 24);
                hb.pack_start(&gtk::Image::from_pixbuf(Some(&pb)), false, false, 0);
                hb.pack_start(&gtk::Label::new(Some(&w.title)), false, false, 0);
                row.add(&hb);
                let xid = w.xid;
                row.connect_activate(move |_| activate_by_xid(xid, t));
                menu.append(&row);
            }
            let closeall = gtk::MenuItem::with_label("关闭全部窗口");
            let st2 = st.clone();
            let id2 = idc.clone();
            closeall.connect_activate(move |_| {
                for w in st2.running.borrow().get(&id2).cloned().unwrap_or_default() {
                    close_by_xid(w.xid, 0);
                }
            });
            menu.append(&closeall);
            let open = gtk::MenuItem::with_label("打开新窗口");
            let ido = idc.clone();
            open.connect_activate(move |_| launch_id(&ido, t));
            menu.append(&open);
        }

        if self.launchers.borrow().contains(&id.to_string()) {
            let st3 = st.clone();
            let id3 = idc.clone();
            let rm = gtk::MenuItem::with_label("从 dock 移除");
            rm.connect_activate(move |_| {
                st3.launchers.borrow_mut().retain(|x| x != &id3);
                save_launchers(&st3.launchers.borrow());
                st3.hidden.borrow_mut().insert(id3.clone());
                st3.remove_icon(&id3);
            });
            menu.append(&rm);
        }

        menu.show_all();
        if let Some(h) = self.icons.borrow().get(id) {
            if let Some(gwin) = self.window.window() {
                let rect = icon_rect(&self.window, &h.eb);
                menu.popup_at_rect(&gwin, &rect, gdk::Gravity::North, gdk::Gravity::South, None::<&gdk::Event>);
            }
        } else {
            menu.popup_at_pointer(None::<&gdk::Event>);
        }
    }

    fn add_app_dialog(&self) {
        let dlg = gtk::FileChooserDialog::new(
            Some("选择 .desktop 文件"),
            Some(&self.window),
            gtk::FileChooserAction::Open,
        );
        dlg.add_button("取消", gtk::ResponseType::Cancel);
        dlg.add_button("添加", gtk::ResponseType::Accept);
        let _ = dlg.set_current_folder("/usr/share/applications");
        let filter = gtk::FileFilter::new();
        filter.add_pattern("*.desktop");
        dlg.set_filter(&filter);

        if dlg.run() == gtk::ResponseType::Accept {
            if let Some(path) = dlg.filename() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let id = name.to_string();
                    if !self.launchers.borrow().contains(&id) {
                        self.launchers.borrow_mut().push(id.clone());
                        save_launchers(&self.launchers.borrow());
                    }
                    self.hidden.borrow_mut().remove(&id);
                    self.refresh_and_reconcile();
                }
            }
        }
        unsafe {
            dlg.destroy();
        }
    }

    fn show_bg_menu(&self) {
        let st = self.clone();
        let menu = gtk::Menu::new();
        let add = gtk::MenuItem::with_label("添加应用…");
        let st2 = st.clone();
        add.connect_activate(move |_| st2.add_app_dialog());
        menu.append(&add);

        let quit = gtk::MenuItem::with_label("退出");
        quit.connect_activate(|_| gtk::main_quit());
        menu.append(&quit);

        // 底部栏：1 透明 / 2 不透明
        let bar_menu = gtk::Menu::new();
        let mut bg: Option<gtk::RadioMenuItem> = None;
        for (label, val) in [("透明", true), ("不透明", false)] {
            let item = gtk::RadioMenuItem::with_label(label);
            if let Some(g) = bg.as_ref() {
                item.join_group(Some(g));
            }
            bg = Some(item.clone());
            if *st.bar_transparent.borrow() == val {
                item.set_active(true);
            }
            let bt = st.bar_transparent.clone();
            let win = st.window.clone();
            item.connect_toggled(move |it| {
                if it.is_active() {
                    *bt.borrow_mut() = val;
                    save_bar_transparent(val);
                    win.queue_draw();
                }
            });
            bar_menu.append(&item);
        }
        let bar_item = gtk::MenuItem::with_label("底部栏");
        bar_item.set_submenu(Some(&bar_menu));
        menu.append(&bar_item);

        // 指示形状：圆点 / 线（线为横向压扁的椭圆）
        let shape_menu = gtk::Menu::new();
        let mut sg: Option<gtk::RadioMenuItem> = None;
        let cur_shape = *st.dot_shape.borrow();
        for (label, val) in [("圆点", 0u8), ("线", 1u8)] {
            let item = gtk::RadioMenuItem::with_label(label);
            if let Some(g) = sg.as_ref() {
                item.join_group(Some(g));
            }
            sg = Some(item.clone());
            if cur_shape == val {
                item.set_active(true);
            }
            let sh = st.dot_shape.clone();
            let st_icons = st.clone();
            item.connect_toggled(move |it| {
                if it.is_active() {
                    *sh.borrow_mut() = val;
                    save_dot_shape(val);
                    for h in st_icons.icons.borrow().values() {
                        h.indicator.queue_draw();
                    }
                }
            });
            shape_menu.append(&item);
        }
        let shape_item = gtk::MenuItem::with_label("指示形状");
        shape_item.set_submenu(Some(&shape_menu));
        menu.append(&shape_item);

        // 悬停放大倍数控制（1× / 2× / 3×）
        let scale_menu = gtk::Menu::new();
        let mut group: Option<gtk::RadioMenuItem> = None;
        let cur = *st.hover_scale.borrow();
        let mut first_item: Option<gtk::RadioMenuItem> = None;
        let mut matched = false;
        for (label, val) in [("1×", 1.45), ("2×", 2.0), ("3×", 3.0)] {
            let item = gtk::RadioMenuItem::with_label(label);
            if let Some(g) = group.as_ref() {
                item.join_group(Some(g));
            }
            group = Some(item.clone());
            if first_item.is_none() {
                first_item = Some(item.clone());
            }
            if (cur - val).abs() < 0.001 {
                item.set_active(true);
                matched = true;
            }
            // 关键：先 set_active 再 connect_toggled，避免构建菜单时 GTK 自动激活误写 hover_scale 文件
            let hs = st.hover_scale.clone();
            let v = val;
            item.connect_toggled(move |it| {
                if it.is_active() {
                    *hs.borrow_mut() = v;
                    save_hover_scale(v);
                    // 格子尺寸固定，修改倍数不再重排/重定位，dock 位置保持稳定
                }
            });
            scale_menu.append(&item);
        }
        if !matched {
            if let Some(f) = first_item {
                f.set_active(true);
            }
        }
        let scale_item = gtk::MenuItem::with_label("悬停放大");
        scale_item.set_submenu(Some(&scale_menu));
        menu.append(&scale_item);

        menu.show_all();
        menu.popup_at_pointer(None);
    }

    fn connect_bg_menu(&self) {
        let st = self.clone();
        self.window.connect_button_press_event(move |_, ev| {
            if ev.button() == 3 {
                st.show_bg_menu();
                Inhibit(true)
            } else {
                Inhibit(false)
            }
        });
    }
}

fn monitor_geometry() -> (i32, i32, i32, i32) {
    if let Some(display) = gdk::Display::default() {
        if let Some(monitor) = display.primary_monitor() {
            let geo = monitor.geometry();
            return (geo.width(), geo.height(), geo.x(), geo.y());
        }
    }
    (1920, 1080, 0, 0)
}


fn place_window(window: &gtk::Window, pos: &Position) {
    let (sw, sh, ox, oy) = monitor_geometry();
    let (w, h) = window.size();
    let (x, y) = match pos {
        Position::Bottom => ((sw - w) / 2 + ox, sh - h + oy),
        Position::Top => ((sw - w) / 2 + ox, oy),
        Position::Left => (ox, (sh - h) / 2 + oy),
        Position::Right => (sw - w + ox, (sh - h) / 2 + oy),
    };
    window.move_(x.max(ox), y.max(oy));
}

fn main() {
    let _ = gtk::init();

    let pos = Position::Bottom;
    let launchers = Rc::new(RefCell::new(load_launchers()));
    let hover_scale = Rc::new(RefCell::new(load_hover_scale()));

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_widget_name("tinydock");
    window.set_decorated(false);
    window.set_resizable(false);
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.set_keep_above(true);
    window.set_accept_focus(false);
    window.set_focus_on_map(false);
    window.set_type_hint(gdk::WindowTypeHint::Dock);
    window.set_app_paintable(true);
    if let Some(screen) = gtk::prelude::WidgetExt::screen(&window) {
        if let Some(vis) = screen.rgba_visual() {
            window.set_visual(Some(&vis));
        }
    }

    let provider = gtk::CssProvider::new();
    let _ = provider.load_from_data(
        b"window#tinydock { background: transparent; }",
    );
    if let Some(screen) = gtk::prelude::WidgetExt::screen(&window) {
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let box_ = match pos {
        Position::Left | Position::Right => gtk::Box::new(gtk::Orientation::Vertical, 2),
        _ => gtk::Box::new(gtk::Orientation::Horizontal, 2),
    };
    box_.set_margin_top(6);
    box_.set_margin_bottom(6);
    box_.set_margin_start(6);
    box_.set_margin_end(6);
    window.add(&box_);

    // 自定义绘制底部栏：窗口其余区域保持透明，放大时图标向上溢出到透明区，
    // 既不会被裁切，也不会改变 dock 的窗口尺寸（位置因此稳定）。
    let bar_transparent = Rc::new(RefCell::new(load_bar_transparent()));
    let dot_shape = Rc::new(RefCell::new(load_dot_shape()));
    let bt = bar_transparent.clone();
    window.connect_draw(move |w, cr| {
        if *bt.borrow() {
            return Inhibit(false); // 透明模式：不绘制底部栏
        }
        let alloc = w.allocation();
        let wd = alloc.width() as f64;
        let ht = alloc.height() as f64;
        let pad = BAR_PAD as f64;
        let top_pad = BAR_TOP_PAD as f64;
        let bar_h = (BASE as f64) + top_pad + pad; // 顶边多留 top_pad，使图标上方与栏顶之间有一点空间
        let x = pad;
        let y = ht - bar_h - pad;
        let bw = wd - 2.0 * pad;
        let bh = bar_h;
        let r = 12.0;
        cr.save().ok();
        cr.set_source_rgba(28.0 / 255.0, 28.0 / 255.0, 30.0 / 255.0, 0.92);
        cr.move_to(x + r, y);
        cr.line_to(x + bw - r, y);
        cr.arc(x + bw - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        cr.line_to(x + bw, y + bh - r);
        cr.arc(x + bw - r, y + bh - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        cr.line_to(x + r, y + bh);
        cr.arc(x + r, y + bh - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        cr.line_to(x, y + r);
        cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
        cr.close_path();
        cr.fill().ok();
        cr.restore().ok();
        Inhibit(false)
    });

    let state = DockState {
        window: window.clone(),
        container: box_.clone(),
        pos,
        bar_transparent: bar_transparent.clone(),
        dot_shape: dot_shape.clone(),
        hover_scale: hover_scale.clone(),
        launchers: launchers.clone(),
        wmclass: build_wmclass_map(),
        running: Rc::new(RefCell::new(HashMap::new())),
        hidden: Rc::new(RefCell::new(HashSet::new())),
        icons: Rc::new(RefCell::new(HashMap::new())),
    };

    state.refresh_and_reconcile();
    place_window(&window, &pos);
    state.connect_bg_menu();

    let st = state.clone();
    glib::timeout_add_local(Duration::from_millis(500), move || {
        st.refresh_and_reconcile();
        glib::Continue(true)
    });

    window.show_all();
    window.set_keep_above(true);
    place_window(&window, &pos);

    gtk::main();
}
