//! Render scale on iOS. An iPhone 17 Pro Max draws every frame at the screen's
//! full 2868x1320 (3.8 million pixels; the Mac's 1280x720 window is 0.9
//! million), and its logs show the graphics chip is what holds a match at 50-60
//! fps: 12-14 ms of GPU time a frame against 5-8 ms on the main thread.
//!
//! `IW4L_RENDER_SCALE` (0.2 to 1.0; off unless set) lowers the view's content
//! scale once the window exists, so the drawable and every pass drawn into it
//! shrink by that factor on each side; Core Animation stretches the result over
//! the screen. Window sizes in points, and with them the menus and the HUD
//! layout, stay as they were.
//!
//! # 3D / UI separation ("Nitidezza testo/UI") - STATE: WIRED (explicit Native/High/Medium and 3D scale < 0.6 only)
//! `text_sharpness::select` returns the 3D size and the UI size; they differ
//! only for an explicit mode with 3D scale < 0.6 (Auto/Off: same, as before).
//! The window takes the UI size, `render_gpu::set_scene_size_override` the 3D
//! size: colour pass viewport, projection and 2D command-buffer constants use
//! it, `scene_split::blit_scene` upscales it before post-fx / HUD. Notes:
//! - 3D size: colour pass viewport (`render_gpu/.../colour_submit/mod.rs`
//!   `extracted_view.viewport`), `view_parms.rs` `rt_w/rt_h` + camera.viewport
//!   in `render_frontend/src/prepare/scene/view_parms.rs`, scene depth
//!   (`scene_depth.rs:51`), a `MainPassResolutionOverride` on the Camera3d.
//! - UI size (full window): HUD / `draw_iw_tess` (`iw_tess.rs:436-467`),
//!   `Hud2dSurface` (`hud/src/surface.rs`), postfx/shellshock
//!   (`main_texture().size()`), and the scene must be upscaled into the whole
//!   main texture before the HUD pass (`ResolvedScene::copy` copies 1:1, it
//!   does not scale).
use bevy::prelude::*;

pub(crate) fn register(app: &mut App) {
    #[cfg(target_os = "ios")]
    ios::register(app);
    #[cfg(not(target_os = "ios"))]
    let _ = app;
}

#[cfg(target_os = "ios")]
mod ios {
    use bevy::ecs::system::NonSendMarker;
    use bevy::prelude::*;
    use bevy::window::PrimaryWindow;
    use winit::platform::ios::WindowExtIOS;

    use crate::text_sharpness::{select, SharpnessMode};

    const RENDER_SCALE_ENV: &str = "IW4L_RENDER_SCALE";

    /// The scale still to apply; removed once the window has it.
    #[derive(Resource)]
    struct PendingRenderScale(f64);

    pub(super) fn register(app: &mut App) {
        let Some(scale) = std::env::var(RENDER_SCALE_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<f64>().ok())
        else {
            // No render scale: still log the window size, so a screen that shows only
            // a corner of the picture (issue #8) can be read from the log.
            app.add_systems(Update, report_size);
            return;
        };
        if !(0.2..1.0).contains(&scale) {
            if scale != 1.0 {
                diag::warn!(
                    Launch,
                    "render scale: {RENDER_SCALE_ENV}={scale} is outside 0.2-1.0, drawing at full size"
                );
            }
            app.add_systems(Update, report_size);
            return;
        }
        app.insert_resource(PendingRenderScale(scale)).add_systems(
            Update,
            (
                apply.run_if(resource_exists::<PendingRenderScale>),
                report_size,
            ),
        );
    }

    fn apply(
        mut commands: Commands,
        pending: Res<PendingRenderScale>,
        primary: Query<Entity, With<PrimaryWindow>>,
        _main_thread: NonSendMarker,
    ) {
        let Ok(entity) = primary.single() else {
            return;
        };
        let applied = bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
            let winit = windows.get_window(entity)?;
            let native = winit
                .current_monitor()
                .map_or_else(|| winit.scale_factor(), |monitor| monitor.scale_factor());
            let before = winit.outer_size();
            // The window (swapchain, HUD, text) takes the UI scale; the 3D scale
            // is carried in `TextSharpness` for the scene consumers (see the
            // report: view_parms rt_w/rt_h, Hud2dSurface, scene_depth, postfx).
            let sel = select(
                SharpnessMode::from_env(),
                (before.width, before.height),
                pending.0,
            );
            winit.set_scale_factor(native * sel.ui_scale);
            Some((native, before, winit.outer_size(), sel))
        });
        // The winit window comes a frame or two after the entity.
        let Some((native, before, after, sel)) = applied else {
            return;
        };
        commands.remove_resource::<PendingRenderScale>();
        // Separation active only when the 3D area is smaller than the window.
        if sel.scene_size != sel.ui_size {
            render_gpu::set_scene_size_override(Some(UVec2::new(
                sel.scene_size.0,
                sel.scene_size.1,
            )));
        } else {
            render_gpu::set_scene_size_override(None);
        }
        commands.insert_resource(sel);
        diag::info!(
            Launch,
            "render 3D {}x{} | UI {}x{} | text scale {:.2}",
            sel.scene_size.0,
            sel.scene_size.1,
            sel.ui_size.0,
            sel.ui_size.1,
            sel.ui_scale
        );
        diag::info!(
            Launch,
            "render scale: {:.2} (content scale {native:.2} -> {:.2}), window {}x{} -> {}x{}",
            pending.0,
            native * sel.ui_scale,
            before.width,
            before.height,
            after.width,
            after.height
        );
    }

    /// Logs the size the game draws at whenever it changes. A layout pass that
    /// reports the screen's own scale puts the window back at full size, so this
    /// line, not the one above, says what the graphics chip is drawing.
    fn report_size(
        primary: Query<&Window, With<PrimaryWindow>>,
        mut last: Local<Option<(u32, u32)>>,
    ) {
        let Ok(window) = primary.single() else {
            return;
        };
        let size = (window.physical_width(), window.physical_height());
        if *last == Some(size) {
            return;
        }
        *last = Some(size);
        diag::info!(
            Launch,
            "window: drawing at {}x{} ({}x{} points, scale {:.2})",
            size.0,
            size.1,
            window.width(),
            window.height(),
            window.scale_factor()
        );
    }
}
