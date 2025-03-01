//! A compute shader that simulates Conway's Game of Life.
//!
//! Compute shaders use the GPU for computing arbitrary information, that may be independent of what
//! is rendered to the screen.

use bevy::{
    prelude::*,
    render::{
        extract_resource::{
            ExtractResource, ExtractResourcePlugin,
        },
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{
            RenderContext, RenderDevice, RenderQueue,
        },
        RenderApp, RenderStage,
    },
    window::WindowDescriptor,
};
use bevy_shader_utils::ShaderUtilsPlugin;
use std::borrow::Cow;

const SIZE: (u32, u32) = (1280, 720);
// const SIZE: (u32, u32) = (3840, 2160);
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            watch_for_changes: true,
            ..default()
        }))
        .add_plugin(ShaderUtilsPlugin)
        .add_plugin(GameOfLifeComputePlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    image.texture_descriptor.usage = TextureUsages::COPY_DST
        | TextureUsages::STORAGE_BINDING
        | TextureUsages::TEXTURE_BINDING;
    let image = images.add(image);

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(
                SIZE.0 as f32,
                SIZE.1 as f32,
            )),
            ..default()
        },
        texture: image.clone(),
        ..default()
    });
    commands.spawn(Camera2dBundle::default());

    commands.insert_resource(GameOfLifeImage(image));
}

pub struct GameOfLifeComputePlugin;

impl Plugin for GameOfLifeComputePlugin {
    fn build(&self, app: &mut App) {
        let render_device =
            app.world.resource::<RenderDevice>();

        let buffer = render_device.create_buffer(
            &BufferDescriptor {
                label: Some("time uniform buffer"),
                size: std::mem::size_of::<f32>() as u64,
                usage: BufferUsages::UNIFORM
                    | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            },
        );
        app.add_plugin(ExtractResourcePlugin::<
            GameOfLifeImage,
        >::default())
            .add_plugin(ExtractResourcePlugin::<
                ExtractedTime,
            >::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<GameOfLifePipeline>()
            .insert_resource(TimeMeta {
                buffer,
                bind_group: None,
            })
            .add_system_to_stage(
                RenderStage::Queue,
                queue_bind_group,
            )
            .add_system_to_stage(
                RenderStage::Prepare,
                prepare_time,
            );

        let mut render_graph =
            render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(
            "game_of_life",
            GameOfLifeNode::default(),
        );
        render_graph
            .add_node_edge(
                "game_of_life",
                bevy::render::main_graph::node::CAMERA_DRIVER,
            )
            .unwrap();
    }
}

// Resource is opt-in in main branch
#[derive(Resource, Clone, Deref, ExtractResource)]
struct GameOfLifeImage(Handle<Image>);

#[derive(Resource)]
struct GameOfLifeImageBindGroup(BindGroup);

fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<GameOfLifePipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    game_of_life_image: Res<GameOfLifeImage>,
    render_device: Res<RenderDevice>,
    time_meta: ResMut<TimeMeta>,
) {
    let view = &gpu_images[&game_of_life_image.0];
    let bind_group = render_device.create_bind_group(
        &BindGroupDescriptor {
            label: None,
            layout: &pipeline.texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &view.texture_view,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: time_meta
                        .buffer
                        .as_entire_binding(),
                },
            ],
        },
    );
    commands.insert_resource(GameOfLifeImageBindGroup(
        bind_group,
    ));
}

#[derive(Resource)]
pub struct GameOfLifePipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for GameOfLifePipeline {
    fn from_world(world: &mut World) -> Self {
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(std::mem::size_of::<f32>() as u64),
                        },
                        count: None,
                    }],
                });
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/flow.wgsl");
        let mut pipeline_cache =
            world.resource_mut::<PipelineCache>();
        let init_pipeline = pipeline_cache
            .queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: None,
                    layout: Some(vec![
                        texture_bind_group_layout.clone(),
                    ]),
                    shader: shader.clone(),
                    shader_defs: vec![],
                    entry_point: Cow::from("init"),
                },
            );
        let update_pipeline = pipeline_cache
            .queue_compute_pipeline(
                ComputePipelineDescriptor {
                    label: None,
                    layout: Some(vec![
                        texture_bind_group_layout.clone(),
                    ]),
                    shader,
                    shader_defs: vec![],
                    entry_point: Cow::from("update"),
                },
            );

        GameOfLifePipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum GameOfLifeState {
    Loading,
    Init,
    Update,
}

struct GameOfLifeNode {
    state: GameOfLifeState,
}

impl Default for GameOfLifeNode {
    fn default() -> Self {
        Self {
            state: GameOfLifeState::Loading,
        }
    }
}

impl render_graph::Node for GameOfLifeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline =
            world.resource::<GameOfLifePipeline>();
        let pipeline_cache =
            world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            GameOfLifeState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache
                        .get_compute_pipeline_state(
                            pipeline.init_pipeline,
                        )
                {
                    self.state = GameOfLifeState::Init;
                }
            }
            GameOfLifeState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache
                        .get_compute_pipeline_state(
                            pipeline.update_pipeline,
                        )
                {
                    self.state = GameOfLifeState::Update;
                }
            }
            GameOfLifeState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group =
            &world.resource::<GameOfLifeImageBindGroup>().0;
        let pipeline_cache =
            world.resource::<PipelineCache>();
        let pipeline =
            world.resource::<GameOfLifePipeline>();

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(
                &ComputePassDescriptor::default(),
            );

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            GameOfLifeState::Loading => {}
            GameOfLifeState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(
                        pipeline.init_pipeline,
                    )
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(
                    SIZE.0 / WORKGROUP_SIZE,
                    SIZE.1 / WORKGROUP_SIZE,
                    1,
                );
            }
            GameOfLifeState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(
                        pipeline.update_pipeline,
                    )
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(
                    SIZE.0 / WORKGROUP_SIZE,
                    SIZE.1 / WORKGROUP_SIZE,
                    1,
                );
            }
        }

        Ok(())
    }
}

#[derive(Resource, Default)]
struct ExtractedTime {
    seconds_since_startup: f32,
}

impl ExtractResource for ExtractedTime {
    type Source = Time;

    fn extract_resource(time: &Self::Source) -> Self {
        ExtractedTime {
            seconds_since_startup: time.elapsed_seconds(),
        }
    }
}

#[derive(Resource)]
struct TimeMeta {
    buffer: Buffer,
    bind_group: Option<BindGroup>,
}

// write the extracted time into the corresponding uniform buffer
fn prepare_time(
    time: Res<ExtractedTime>,
    time_meta: ResMut<TimeMeta>,
    render_queue: Res<RenderQueue>,
) {
    render_queue.write_buffer(
        &time_meta.buffer,
        0,
        bevy::core::cast_slice(&[
            time.seconds_since_startup
        ]),
    );
}
