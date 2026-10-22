//! 3D / UI separation. When a scene size is set, the colour pass draws the 3D
//! scene into the top-left `size` pixels of the full-size main texture and
//! `blit_scene` stretches that area over the whole texture (linear) before
//! post-fx and the HUD, which keep drawing at the full (UI) size.
//! The size is process-global and set once at startup (None = no separation).
//!
//! Ordering: `blit_scene` runs in [`SceneBlitSet`]; `draw_sun_effects` is
//! ordered `.after(SceneBlitSet)` so the sun is drawn on the stretched image.
//!
//! Limitation: the scene depth keeps the full size while the colour scene sits
//! in the top-left area until the blit, so depth-based effects after the blit
//! (DOF, floatz, sun occlusion) can be misaligned while separation is active.
//! Separation is on by default below a 3D scale of 0.6 (Auto = Medium).
//! Work-around: set the sharpness mode to Off (no separation).
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::core_pipeline::{Core3dSystems, tonemapping::tonemapping};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::render_resource::binding_types::{sampler, texture_2d};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::view::{ExtractedView, ViewTarget};

/// System set of `blit_scene`; later full-size effects order after it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneBlitSet;

static SCENE_SIZE: AtomicU64 = AtomicU64::new(0);

/// Sets (Some) or clears (None) the 3D scene size in physical pixels.
pub fn set_scene_size_override(size: Option<UVec2>) {
    let packed = size
        .filter(|s| s.x > 0 && s.y > 0)
        .map_or(0, |s| (u64::from(s.x) << 32) | u64::from(s.y));
    SCENE_SIZE.store(packed, Ordering::Relaxed);
}

/// The raw 3D scene size when separation is on.
pub fn scene_size_override() -> Option<UVec2> {
    let v = SCENE_SIZE.load(Ordering::Relaxed);
    (v != 0).then(|| UVec2::new((v >> 32) as u32, (v & 0xffff_ffff) as u32))
}

/// The scene size clamped to `target`; `target` itself when separation is off.
pub fn effective_scene_size(target: UVec2) -> UVec2 {
    match scene_size_override() {
        Some(s) => UVec2::new(s.x.min(target.x).max(1), s.y.min(target.y).max(1)),
        None => target,
    }
}

struct SceneCopy {
    texture: Texture,
    view: TextureView,
    format: TextureFormat,
    size: Extent3d,
}

#[derive(Resource)]
struct SceneBlit {
    shader: Handle<Shader>,
    layout: BindGroupLayoutDescriptor,
    sampler: Option<Sampler>,
    pipelines: HashMap<TextureFormat, CachedRenderPipelineId>,
    copy: Option<SceneCopy>,
}

fn blit_scene(
    view: ViewQuery<(&ViewTarget, &ExtractedView)>,
    mut state: ResMut<SceneBlit>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    mut context: RenderContext,
) {
    let Some(scene) = scene_size_override() else {
        return;
    };
    let (target, _view) = view.into_inner();
    let full = target.main_texture().size();
    let (w, h) = (scene.x.min(full.width), scene.y.min(full.height));
    if w == 0 || h == 0 || (w == full.width && h == full.height) {
        return;
    }
    let format = target.main_texture_format();
    let size = Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    let state = &mut *state;
    let id = *state.pipelines.entry(format).or_insert_with(|| {
        cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("scene_blit".into()),
            layout: vec![state.layout.clone()],
            vertex: VertexState {
                shader: state.shader.clone(),
                entry_point: Some("vertex".into()),
                ..default()
            },
            fragment: Some(FragmentState {
                shader_defs: Vec::new(),
                shader: state.shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            ..default()
        })
    });
    let Some(pipeline) = cache.get_render_pipeline(id) else {
        return;
    };
    if state
        .copy
        .as_ref()
        .is_none_or(|c| c.size != size || c.format != format)
    {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("scene_blit_source"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        state.copy = Some(SceneCopy {
            texture,
            view,
            format,
            size,
        });
    }
    let sampler = state
        .sampler
        .get_or_insert_with(|| {
            device.create_sampler(&SamplerDescriptor {
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..default()
            })
        })
        .clone();
    let Some(copy) = state.copy.as_ref() else {
        return;
    };
    context.command_encoder().copy_texture_to_texture(
        target.main_texture().as_image_copy(),
        copy.texture.as_image_copy(),
        size,
    );
    let bind = device.create_bind_group(
        "scene_blit",
        &cache.get_bind_group_layout(&state.layout),
        &BindGroupEntries::sequential((&copy.view, &sampler)),
    );
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("scene_blit"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: target.main_texture_view(),
            resolve_target: None,
            depth_slice: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        })],
        ..default()
    });
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &bind, &[]);
    pass.draw(0..3, 0..1);
}

pub(super) fn register(app: &mut App) {
    if app.get_sub_app(RenderApp).is_none() {
        return;
    }
    let shader = app
        .world_mut()
        .resource_mut::<Assets<Shader>>()
        .add(Shader::from_wgsl(
            include_str!("scene_blit.wgsl"),
            "scene_blit.wgsl",
        ));
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .insert_resource(SceneBlit {
            shader,
            layout: BindGroupLayoutDescriptor::new(
                "scene_blit",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
            sampler: None,
            pipelines: HashMap::new(),
            copy: None,
        })
        .add_systems(
            bevy::core_pipeline::Core3d,
            blit_scene
                .in_set(SceneBlitSet)
                .in_set(Core3dSystems::PostProcess)
                .after(tonemapping)
                .before(super::postfx::PostFxSet),
        );
}
