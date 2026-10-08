use assets::AssetPlugin;
use audio::AudioPlugin;
use bevy::{
    log::LogPlugin,
    prelude::*,
    render::{
        RenderPlugin as BevyRenderPlugin,
        pipelined_rendering::PipelinedRenderingPlugin,
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
    },
};
use bots::BotsPlugin;
use console::ConsolePlugin;
use frame::RuntimeRole;
use hud::HudPlugin;
use net::NetPlugin;
use render::RenderPlugin;
use replay::ReplayPlugin;
use session::SessionPlugin;
use ui::UiPlugin;

pub fn add_runtime_plugins(app: &mut App) {
    add_runtime_plugins_with_role(app, RuntimeRole::Listen);
}

pub fn add_runtime_plugins_with_role(app: &mut App, role: RuntimeRole) {
    let net = match role {
        RuntimeRole::Listen => NetPlugin::listen(),
        RuntimeRole::Dedicated => NetPlugin::dedicated(),
        RuntimeRole::Client => NetPlugin::client(),
        RuntimeRole::Replay => NetPlugin::replay(),
    };
    if crate::bench::enabled() {
        // Unfocused benchmarks must not inherit the window runner's 60 Hz sleep.
        app.insert_resource(bevy::winit::WinitSettings::continuous());
    }
    app.insert_resource(audio::AudioRuntime::new(
        role != RuntimeRole::Dedicated && !audio::AudioSilent::active(),
    ));
    app.add_plugins(AssetPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(ConsolePlugin)
        .add_plugins(net)
        .add_plugins((BotsPlugin, HudPlugin))
        .add_plugins(AudioPlugin)
        .add_plugins(ReplayPlugin)
        .add_plugins(RenderPlugin)
        .add_plugins(SessionPlugin);
    app.insert_resource(bevy::render::error_handler::RenderErrorHandler(log_then_quit));
    // iOS: ask a 120 Hz display for its full refresh during matches.
    crate::promotion::register(app);
    // iOS: no auto-lock while a map loads or a match runs.
    crate::idle_timer::register(app);
    // Render setup can finish after Startup, so watch for the resource.
    app.add_systems(
        First,
        note_gpu_bc.run_if(resource_added::<bevy::image::CompressedImageFormatSupport>),
    );

    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });

    app.edit_schedule(First, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app.edit_schedule(Last, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });

    if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
        render_app.edit_schedule(bevy::render::renderer::RenderGraph, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        render_app.edit_schedule(bevy::core_pipeline::Core3d, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        render_app.edit_schedule(bevy::core_pipeline::Core2d, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        render_app.edit_schedule(bevy::render::Render, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        render_app.edit_schedule(bevy::render::ExtractSchedule, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
    }
}

pub fn assemble_listen_app() -> App {
    let mut app = App::new();
    app.add_plugins(default_plugins_with_quiet_log(WindowPlugin {
        primary_window: None,
        ..default()
    }));
    add_runtime_plugins(&mut app);
    app
}

/// Bevy's default quits the app on any render error but only logs through tracing, which
/// never reaches `iw4l-boot.log`: a graphics failure looked like a silent `code=1` exit.
/// Same policy as the default (quit), plus the error type and text in the boot log.
fn log_then_quit(
    error: &bevy::render::error_handler::RenderError,
    main_world: &mut World,
    _render_world: &mut World,
) -> bevy::render::error_handler::RenderErrorPolicy {
    // StopRendering keeps the error state, so Bevy calls this every frame until the app
    // exits: log it once.
    static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        let text = format!(
            "RENDER ERROR {:?}: {} | footprint {} MiB, iOS still allows {} MiB",
            error.ty,
            error.description,
            diag::ios_env::footprint_bytes().unwrap_or(0) / (1024 * 1024),
            diag::ios_env::available_bytes().unwrap_or(0) / (1024 * 1024),
        );
        diag::boot_crumb(&text);
        diag::flush();
    }
    main_world.write_message(AppExit::error());
    bevy::render::error_handler::RenderErrorPolicy::StopRendering
}

/// Tells the texture decoder whether the GPU samples BC directly, so iOS can
/// keep BC1/2/3/5 compressed (4-8x smaller) instead of expanding to RGBA8.
fn note_gpu_bc(support: Res<bevy::image::CompressedImageFormatSupport>) {
    asset_material::set_gpu_bc_textures(
        support.0.contains(bevy::image::CompressedImageFormats::BC),
    );
}

pub fn default_plugins_with_quiet_log(mut window: WindowPlugin) -> bevy::app::PluginGroupBuilder {
    if let Some(primary) = window.primary_window.as_mut() {
        primary.desired_maximum_frame_latency = core::num::NonZeroU32::new(frame_latency());
    }
    let mut wgpu = WgpuSettings::default();
    // BC is required everywhere but iOS. Only Apple9+ iPhone/iPad GPUs have it
    // (A15/A16 do not), so iOS takes it when the adapter offers it (the default
    // priority requests every adapter feature) and decodes to RGBA8 otherwise;
    // see `note_gpu_bc`.
    #[cfg(not(target_os = "ios"))]
    {
        wgpu.features |= WgpuFeatures::TEXTURE_COMPRESSION_BC;
    }
    wgpu.features |= WgpuFeatures::TEXTURE_FORMAT_16BIT_NORM
        | WgpuFeatures::POLYGON_MODE_LINE
        | WgpuFeatures::TEXTURE_BINDING_ARRAY
        | WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
        | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY;
    let plugins = DefaultPlugins
        .set(window)
        .set(LogPlugin {
            filter: "warn,iw4l=info".into(),
            level: bevy::log::Level::WARN,
            ..default()
        })
        .set(BevyRenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(wgpu)),
            ..default()
        });
    #[cfg(target_os = "ios")]
    let plugins = plugins.add(crate::ios_input::IosInputPlugin);
    if pipelined_rendering() {
        plugins
    } else {
        plugins.disable::<PipelinedRenderingPlugin>()
    }
}

const PIPELINED_RENDERING_ENV: &str = "IW4L_PIPELINED_RENDERING";

/// Overlap rendering with the next main frame. Extraction remains the ownership
/// boundary; the bounded render channel permits one outstanding frame.
/// Set IW4L_PIPELINED_RENDERING=0 for synchronous presentation.
///
/// Off by default on macOS and iOS: AppKit only lets the main thread touch the NSView
/// behind the Metal surface. Bevy hands `create_surfaces` back to the main
/// thread through the multi-threaded executor, which the single-threaded
/// `Render` schedule above bypasses, so the render thread would create it and
/// panic in `raw-window-metal`.
fn pipelined_rendering() -> bool {
    match std::env::var_os(PIPELINED_RENDERING_ENV) {
        None => !cfg!(any(target_os = "macos", target_os = "ios")),
        Some(_) => perf::switch(PIPELINED_RENDERING_ENV),
    }
}

const FRAME_LATENCY_ENV: &str = "IW4L_FRAME_LATENCY";

/// How many frames the surface is asked to let the CPU run ahead of the GPU.
///
/// wgpu calls this a hint and the backend is free to clamp it: on Vulkan it is
/// bound to the number of swapchain images, so a run that asked for two did
/// not necessarily get two, and only a measurement says which. One is the
/// default because it is what the runtime shipped; the variable exists so the
/// other arm needs no rebuild, and the manifest records the number that was
/// asked for — never the number the driver granted, which this process cannot
/// read back.
///
/// The value is a count, not a switch: anything unparseable or zero is the
/// default, and says so rather than silently picking an arm. Read once, so the
/// window and the manifest cannot disagree and the complaint is made once.
pub(crate) fn frame_latency() -> u32 {
    // iOS renders on the main thread (no pipelined rendering), so with one
    // frame in flight (two drawables) it waits on nextDrawable every frame and
    // snaps to 30/20/15 fps. Two frames in flight let CPU and GPU overlap.
    const DEFAULT: u32 = if cfg!(target_os = "ios") { 2 } else { 1 };
    static FRAMES: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *FRAMES.get_or_init(|| {
        let Some(asked) = std::env::var_os(FRAME_LATENCY_ENV) else {
            return DEFAULT;
        };
        match asked.to_str().map(str::trim).and_then(|v| v.parse().ok()) {
            Some(frames) if frames > 0 => frames,
            _ => {
                diag::warn!(
                    Launch,
                    "{FRAME_LATENCY_ENV}={} is not a frame count; using {DEFAULT}",
                    asked.to_string_lossy(),
                );
                DEFAULT
            }
        }
    })
}
