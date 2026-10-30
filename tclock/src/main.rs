use eframe::egui;
use egui::{Color32, Stroke, Vec2};
use std::f32::consts::PI;
use std::time::Duration;

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

struct ClockApp;

impl ClockApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self
    }
}

impl eframe::App for ClockApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(33));

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                if response.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                response.context_menu(|ui| {
                    if ui.button("退出").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                draw_clock(ui.painter(), rect);
            });
    }
}

fn draw_clock(painter: &egui::Painter, rect: egui::Rect) {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) / 2.0 - 6.0;
    if radius <= 8.0 {
        return;
    }

    use chrono::Timelike;
    let now = chrono::Local::now();
    let h = now.hour() as f32 % 12.0;
    let m = now.minute() as f32;
    let s = now.second() as f32 + now.nanosecond() as f32 / 1e9;

    painter.circle_filled(center + Vec2::new(0.0, 3.0), radius, Color32::from_black_alpha(45));
    painter.circle_filled(center, radius, Color32::from_white_alpha(235));

    painter.circle_stroke(center, radius, Stroke::new(2.0_f32, Color32::from_gray(55)));
    painter.circle_stroke(center, radius - 6.0, Stroke::new(1.0_f32, Color32::from_gray(190)));

    for i in 0..60 {
        let angle = i as f32 * 6.0 * PI / 180.0;
        let is_hour = i % 5 == 0;
        let (outer, inner, width) = if is_hour {
            (radius - 9.0, radius - 18.0, 2.5_f32)
        } else {
            (radius - 9.0, radius - 13.0, 1.0_f32)
        };
        let p1 = center + Vec2::new(outer * angle.sin(), -outer * angle.cos());
        let p2 = center + Vec2::new(inner * angle.sin(), -inner * angle.cos());
        let col = if is_hour { Color32::from_gray(45) } else { Color32::from_gray(120) };
        painter.line_segment([p1, p2], Stroke::new(width, col));
    }

    let font = egui::FontId::proportional((radius * 0.17).max(9.0));
    for (i, txt) in [(0, "12"), (3, "3"), (6, "6"), (9, "9")] {
        let angle = i as f32 * 30.0 * PI / 180.0;
        let r = radius - 26.0;
        let pos = center + Vec2::new(r * angle.sin(), -r * angle.cos());
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            txt,
            font.clone(),
            Color32::from_gray(45),
        );
    }

    let hand = |angle_deg: f32, len: f32, tail: f32, width: f32, color: Color32| {
        let a = angle_deg * PI / 180.0;
        let dir = Vec2::new(a.sin(), -a.cos());
        let start = center - dir * tail;
        let end = center + dir * len;
        painter.line_segment([start, end], Stroke::new(width, color));
        painter.circle_filled(end, width / 2.0, color);
        painter.circle_filled(start, width / 4.0, color);
    };

    hand((h + m / 60.0 + s / 3600.0) * 30.0, radius * 0.50, radius * 0.10, radius * 0.055,
         Color32::from_rgb(28, 28, 38));
    hand((m + s / 60.0) * 6.0, radius * 0.72, radius * 0.12, radius * 0.040,
         Color32::from_rgb(28, 28, 38));
    hand(s * 6.0, radius * 0.85, radius * 0.18, radius * 0.015,
         Color32::from_rgb(215, 55, 55));

    painter.circle_filled(center, radius * 0.045, Color32::from_rgb(215, 55, 55));
    painter.circle_filled(center, radius * 0.020, Color32::WHITE);
}
