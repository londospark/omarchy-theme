//! imgui-rs x omarchy-theme demo (headless frame + notes for wiring a real
//! backend). Run: cargo run -p omarchy-theme-imgui --example imgui_rs_demo
//!
//! In a real app (imgui-glfw-gl3 etc.), the only two theming lines are:
//!   apply(ctx.style_mut(), &theme);            // at startup
//!   if theme.changed() { theme.reload()?; apply(ctx.style_mut(), &theme); }

use omarchy_theme::Theme;

fn main() -> Result<(), omarchy_theme::Error> {
    let mut ctx = imgui::Context::create();
    let mut theme = match Theme::current() {
        Ok(t) => t.with_font(),
        Err(_) => {
            println!("no live Omarchy theme; using the catppuccin palette as stand-in");
            let palette = omarchy_theme::Palette::resolve(
                include_str!("../../../conformance/fixtures/catppuccin.colors.toml"),
                false,
            );
            Theme::from_palette(palette)
        }
    };
    omarchy_theme_imgui::apply(ctx.style_mut(), &theme);


    ctx.io_mut().display_size = [128.0, 128.0];
    // build the default font atlas the way a renderer backend would
    ctx.fonts().build_rgba32_texture();

    for i in 0..3 {
        if theme.changed() {
            theme.reload()?;
            omarchy_theme_imgui::apply(ctx.style_mut(), &theme);
            println!("frame {i}: retinted to {:?}", theme.name);
        }
        let ui = ctx.frame();
        let title = imgui::ImString::new(format!("themed by omarchy-theme {}", theme.palette.mode()));
        ui.window(&title).build(|| {});
        ctx.render(); // no renderer attached; safe, we only exercise state
    }

    let style = ctx.style();
    println!(
        "Text color: ({:.3}, {:.3}, {:.3}) == foreground {:?}",
        style[imgui::StyleColor::Text][0],
        style[imgui::StyleColor::Text][1],
        style[imgui::StyleColor::Text][2],
        theme.palette.color("foreground"),
    );
    println!("theme followed: {}", theme.name.as_deref().unwrap_or("synthetic"));
    Ok(())
}
