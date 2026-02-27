use egui::{Context, Visuals};

pub fn apply_ui_theme(ctx: &Context) {
    let visuals = Visuals::dark();
    ctx.set_visuals(visuals);
}