use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::VisibilityRange,
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::Checked,
    ui_widgets::{
        Activate, Button, Checkbox, Slider, SliderRange, SliderThumb, SliderValue, TrackClick,
        ValueChange, checkbox_self_update, observe, slider_self_update,
    },
};
use wow_adt::RootAdt;
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use wow_mpq::PatchChain;

use crate::{
    map_loader::{AdtPosition, RenderedGroundEffect, RenderedObject, TerrainEditor},
    mpq_read_file,
};

const PANEL: Color = Color::srgba(0.055, 0.065, 0.075, 0.94);
const TEXT: Color = Color::srgb(0.91, 0.93, 0.94);
const MUTED_TEXT: Color = Color::srgb(0.62, 0.67, 0.70);
const BORDER: Color = Color::srgb(0.23, 0.27, 0.29);
const ACCENT: Color = Color::srgb(0.20, 0.72, 0.53);
const TRACK: Color = Color::srgb(0.16, 0.19, 0.20);

#[derive(Resource)]
pub struct RenderSettings {
    pub render_adts: bool,
    pub render_objects: bool,
    pub render_ground_effects: bool,
    pub adt_distance: f32,
    pub object_distance: f32,
    pub ground_effect_distance: f32,
    pub edit_mode: EditMode,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EditMode {
    #[default]
    Heightmap,
    AlphaMap,
}

#[derive(Component)]
pub(crate) struct AdtCheckbox;

impl Default for AdtCheckbox {
    fn default() -> Self {
        Self
    }
}

#[derive(Component)]
pub(crate) struct ObjectCheckbox;

impl Default for ObjectCheckbox {
    fn default() -> Self {
        Self
    }
}

#[derive(Component)]
pub(crate) struct GroundEffectCheckbox;

impl Default for GroundEffectCheckbox {
    fn default() -> Self {
        Self
    }
}

#[derive(Component)]
pub(crate) struct CheckboxMark;

#[derive(Component)]
pub(crate) struct AdtDistanceLabel;

#[derive(Component)]
pub(crate) struct ObjectDistanceLabel;

#[derive(Component)]
pub(crate) struct GroundEffectDistanceLabel;

#[derive(Component)]
pub(crate) struct DistanceSlider;

#[derive(Component)]
pub(crate) struct DistanceSliderThumb;

#[derive(Component)]
pub(crate) struct EditModeButton(pub(crate) EditMode);

#[derive(Component)]
pub(crate) struct AlphaSliderContainer;

#[derive(Component)]
pub(crate) struct AlphaSlider {
    pub(crate) layer: usize,
}

#[derive(Component)]
pub(crate) struct TextureControl {
    images: Vec<Handle<Image>>,
}

#[derive(Component)]
pub(crate) struct UiRoot;

pub fn setup_render_controls(mut commands: Commands, settings: Res<RenderSettings>) {
    commands
        .spawn((
            Name::new("UI Root"),
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                right: px(16),
                width: px(280),
                padding: UiRect::all(px(16)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            GlobalZIndex(100),
            UiRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Name::new("Render controls"),
                Node {
                    width: px(280),
                    padding: UiRect::all(px(16)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(BORDER),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("RENDER"),
                    TextFont::from_font_size(13.0),
                    TextColor(MUTED_TEXT),
                ));
                spawn_checkbox(panel, "ADT terrain", settings.render_adts, AdtCheckbox);
                spawn_checkbox(panel, "Objects", settings.render_objects, ObjectCheckbox);
                spawn_checkbox(
                    panel,
                    "Ground effects",
                    settings.render_ground_effects,
                    GroundEffectCheckbox,
                );
                spawn_edit_mode_control(panel, settings.edit_mode);
                spawn_distance_control(
                    panel,
                    "ADT distance",
                    settings.adt_distance,
                    500.0,
                    75_000.0,
                    AdtDistanceLabel,
                    observe(
                        |event: On<ValueChange<f32>>, mut settings: ResMut<RenderSettings>| {
                            settings.adt_distance = event.value;
                        },
                    ),
                );
                spawn_distance_control(
                    panel,
                    "Object distance",
                    settings.object_distance,
                    100.0,
                    20_000.0,
                    ObjectDistanceLabel,
                    observe(
                        |event: On<ValueChange<f32>>, mut settings: ResMut<RenderSettings>| {
                            settings.object_distance = event.value;
                        },
                    ),
                );
                spawn_distance_control(
                    panel,
                    "Ground effect distance",
                    settings.ground_effect_distance,
                    32.0,
                    600.0,
                    GroundEffectDistanceLabel,
                    observe(
                        |event: On<ValueChange<f32>>, mut settings: ResMut<RenderSettings>| {
                            settings.ground_effect_distance = event.value;
                        },
                    ),
                );
            });
        });
}

pub(crate) fn ensure_texture_control(
    commands: &mut Commands,
    existing_controls: &Query<(Entity, &TextureControl)>,
    ui_root_query: &Query<(Entity, &UiRoot)>,
    adt: &RootAdt,
    chunk_index: usize,
    mpqs: &PatchChain,
    images: &mut Assets<Image>,
) {
    for (entity, control) in existing_controls {
        for image in &control.images {
            images.remove(image.id());
        }
        commands.entity(entity).despawn();
    }

    let Some(chunk) = adt.mcnk_chunks.get(chunk_index) else {
        return;
    };
    let texture_handles = (0..4)
        .map(|layer_index| {
            chunk
                .layers
                .as_ref()
                .and_then(|layers| layers.layers.get(layer_index))
                .and_then(|layer| adt.textures.get(layer.texture_id as usize))
                .and_then(|filepath| load_texture_preview(filepath, mpqs, images))
        })
        .collect::<Vec<_>>();
    let loaded_images = texture_handles.iter().flatten().cloned().collect();

    commands
        .entity(ui_root_query.single().unwrap().0)
        .with_children(|root| {
            root.spawn((
                Name::new("Texture controls"),
                TextureControl {
                    images: loaded_images,
                },
                Node {
                    width: px(280),
                    padding: UiRect::all(px(16)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(BORDER),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("TEXTURES"),
                    TextFont::from_font_size(13.0),
                    TextColor(MUTED_TEXT),
                ));
                panel
                    .spawn(Node {
                        display: Display::Grid,
                        width: percent(100),
                        grid_template_columns: RepeatedGridTrack::fr(2, 1.0),
                        grid_template_rows: RepeatedGridTrack::fr(2, 1.0),
                        row_gap: px(8),
                        column_gap: px(8),
                        ..default()
                    })
                    .with_children(|grid| {
                        for (layer, texture) in texture_handles.into_iter().enumerate() {
                            grid.spawn(Node {
                                min_width: px(0),
                                flex_direction: FlexDirection::Column,
                                row_gap: px(5),
                                ..default()
                            })
                            .with_children(|texture_control| {
                                texture_control.spawn((
                                    Node {
                                        width: percent(100),
                                        aspect_ratio: Some(1.0),
                                        border: UiRect::all(px(1)),
                                        ..default()
                                    },
                                    texture.map_or_else(
                                        || ImageNode::solid_color(TRACK),
                                        ImageNode::new,
                                    ),
                                    BorderColor::all(BORDER),
                                ));
                                if layer > 0 {
                                    spawn_alpha_slider(texture_control, layer);
                                }
                            });
                        }
                    });
            });
        });
}

fn spawn_edit_mode_control(parent: &mut ChildSpawnerCommands, selected: EditMode) {
    parent
        .spawn(Node {
            width: percent(100),
            height: px(30),
            margin: UiRect::top(px(3)),
            ..default()
        })
        .with_children(|control| {
            for (mode, label) in [
                (EditMode::Heightmap, "Heightmap points"),
                (EditMode::AlphaMap, "Alpha map points"),
            ] {
                control
                    .spawn((
                        EditModeButton(mode),
                        Button,
                        Hovered::default(),
                        Node {
                            width: percent(50),
                            height: percent(100),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(px(1)),
                            ..default()
                        },
                        BackgroundColor(if mode == selected { ACCENT } else { TRACK }),
                        BorderColor::all(BORDER),
                        observe(
                            move |_activate: On<Activate>, mut settings: ResMut<RenderSettings>| {
                                settings.edit_mode = mode;
                            },
                        ),
                    ))
                    .with_child((
                        Text::new(label),
                        TextFont::from_font_size(11.0),
                        TextColor(TEXT),
                    ));
            }
        });
}

fn spawn_alpha_slider(parent: &mut ChildSpawnerCommands, layer: usize) {
    parent.spawn((
        AlphaSliderContainer,
        Node {
            display: Display::None,
            width: percent(100),
            height: px(16),
            align_items: AlignItems::Center,
            ..default()
        },
        AlphaSlider { layer },
        Hovered::default(),
        Slider {
            track_click: TrackClick::Snap,
            ..default()
        },
        SliderValue(0.0),
        SliderRange::new(0.0, 255.0),
        TabIndex(0),
        observe(slider_self_update),
        children![
            (
                Node {
                    width: percent(100),
                    height: px(4),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(TRACK),
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(10),
                    top: px(3),
                    bottom: px(3),
                    ..default()
                },
                children![(
                    SliderThumb,
                    DistanceSliderThumb,
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(10),
                        height: px(10),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(ACCENT),
                )],
            )
        ],
    ));
}

fn load_texture_preview(
    filepath: &str,
    mpqs: &PatchChain,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let data = mpq_read_file(mpqs, filepath)
        .map_err(|error| warn!("Unable to read terrain texture {filepath}: {error}"))
        .ok()?;
    let blp = load_blp_from_buf(&data)
        .map_err(|error| warn!("Unable to parse terrain texture {filepath}: {error}"))
        .ok()?;
    let decoded = blp_to_image(&blp, 0)
        .map_err(|error| warn!("Unable to decode terrain texture {filepath}: {error}"))
        .ok()?;
    let mut image = Image::new(
        Extent3d {
            width: blp.header.width,
            height: blp.header.height,
            ..default()
        },
        TextureDimension::D2,
        decoded.into_rgba8().into_vec(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    Some(images.add(image))
}

fn spawn_checkbox<M: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    checked: bool,
    marker: M,
) {
    let mut checkbox = parent.spawn((
        marker,
        Checkbox,
        Hovered::default(),
        TabIndex(0),
        Node {
            align_items: AlignItems::Center,
            column_gap: px(9),
            ..default()
        },
        observe(checkbox_self_update),
    ));
    if checked {
        checkbox.insert(Checked);
    }
    checkbox.with_children(|row| {
        row.spawn((
            CheckboxMark,
            Text::new(if checked { "x" } else { "" }),
            TextFont::from_font_size(14.0),
            TextLayout::justify(Justify::Center),
            TextColor(TEXT),
            Node {
                width: px(18),
                height: px(18),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(if checked { ACCENT } else { TRACK }),
            BorderColor::all(BORDER),
        ));
        row.spawn((
            Text::new(label.to_owned()),
            TextFont::from_font_size(15.0),
            TextColor(TEXT),
        ));
    });
}

fn spawn_distance_control<M: Component, O: Bundle>(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    label_marker: M,
    on_change: O,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(7),
            margin: UiRect::top(px(3)),
            ..default()
        })
        .with_children(|control| {
            control
                .spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    width: percent(100),
                    ..default()
                })
                .with_children(|heading| {
                    heading.spawn((
                        Text::new(label.to_owned()),
                        TextFont::from_font_size(13.0),
                        TextColor(MUTED_TEXT),
                    ));
                    heading.spawn((
                        label_marker,
                        Text::new(format_distance(value)),
                        TextFont::from_font_size(13.0),
                        TextColor(TEXT),
                    ));
                });
            control.spawn((
                Node {
                    height: px(18),
                    width: percent(100),
                    align_items: AlignItems::Center,
                    ..default()
                },
                DistanceSlider,
                Hovered::default(),
                Slider {
                    track_click: TrackClick::Snap,
                    ..default()
                },
                SliderValue(value),
                SliderRange::new(min, max),
                TabIndex(0),
                observe(slider_self_update),
                on_change,
                children![
                    (
                        Node {
                            width: percent(100),
                            height: px(5),
                            border_radius: BorderRadius::all(px(3)),
                            ..default()
                        },
                        BackgroundColor(TRACK),
                    ),
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            right: px(14),
                            top: px(2),
                            bottom: px(2),
                            ..default()
                        },
                        children![(
                            SliderThumb,
                            DistanceSliderThumb,
                            Node {
                                position_type: PositionType::Absolute,
                                width: px(14),
                                height: px(14),
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BackgroundColor(ACCENT),
                        )],
                    )
                ],
            ));
        });
}

pub fn update_slider_visuals(
    sliders: Query<
        (Entity, &SliderValue, &SliderRange),
        (
            Changed<SliderValue>,
            Or<(With<DistanceSlider>, With<AlphaSlider>)>,
        ),
    >,
    children: Query<&Children>,
    mut thumbs: Query<&mut Node, With<DistanceSliderThumb>>,
) {
    for (slider, value, range) in &sliders {
        for child in children.iter_descendants(slider) {
            if let Ok(mut thumb) = thumbs.get_mut(child) {
                thumb.left = percent(range.thumb_position(value.0) * 100.0);
            }
        }
    }
}

pub fn update_render_controls(
    adt_checkboxes: Query<(Entity, Has<Checked>), With<AdtCheckbox>>,
    object_checkboxes: Query<(Entity, Has<Checked>), With<ObjectCheckbox>>,
    ground_effect_checkboxes: Query<(Entity, Has<Checked>), With<GroundEffectCheckbox>>,
    checkbox_children: Query<
        &Children,
        Or<(
            With<AdtCheckbox>,
            With<ObjectCheckbox>,
            With<GroundEffectCheckbox>,
        )>,
    >,
    mut marks: Query<
        (&mut Text, &mut BackgroundColor),
        (
            With<CheckboxMark>,
            Without<AdtDistanceLabel>,
            Without<ObjectDistanceLabel>,
            Without<GroundEffectDistanceLabel>,
        ),
    >,
    mut adt_labels: Query<
        &mut Text,
        (
            With<AdtDistanceLabel>,
            Without<CheckboxMark>,
            Without<ObjectDistanceLabel>,
            Without<GroundEffectDistanceLabel>,
        ),
    >,
    mut object_labels: Query<
        &mut Text,
        (
            With<ObjectDistanceLabel>,
            Without<CheckboxMark>,
            Without<AdtDistanceLabel>,
            Without<GroundEffectDistanceLabel>,
        ),
    >,
    mut ground_effect_labels: Query<
        &mut Text,
        (
            With<GroundEffectDistanceLabel>,
            Without<CheckboxMark>,
            Without<AdtDistanceLabel>,
            Without<ObjectDistanceLabel>,
        ),
    >,
    mut settings: ResMut<RenderSettings>,
    editor: Res<TerrainEditor>,
    mut mode_buttons: Query<(&EditModeButton, &mut BackgroundColor), Without<CheckboxMark>>,
    mut alpha_controls: Query<&mut Node, With<AlphaSliderContainer>>,
) {
    for (button, mut background) in &mut mode_buttons {
        background.0 = if button.0 == settings.edit_mode {
            ACCENT
        } else {
            TRACK
        };
    }
    for mut node in &mut alpha_controls {
        node.display =
            if settings.edit_mode == EditMode::AlphaMap && editor.has_active_alpha_point() {
                Display::Flex
            } else {
                Display::None
            };
    }
    for (entity, checked) in &adt_checkboxes {
        if settings.render_adts != checked {
            settings.render_adts = checked;
        }
        update_checkbox_mark(entity, checked, &checkbox_children, &mut marks);
    }
    for (entity, checked) in &object_checkboxes {
        if settings.render_objects != checked {
            settings.render_objects = checked;
        }
        update_checkbox_mark(entity, checked, &checkbox_children, &mut marks);
    }
    for (entity, checked) in &ground_effect_checkboxes {
        if settings.render_ground_effects != checked {
            settings.render_ground_effects = checked;
        }
        update_checkbox_mark(entity, checked, &checkbox_children, &mut marks);
    }
    for mut label in &mut adt_labels {
        **label = format_distance(settings.adt_distance);
    }
    for mut label in &mut object_labels {
        **label = format_distance(settings.object_distance);
    }
    for mut label in &mut ground_effect_labels {
        **label = format_distance(settings.ground_effect_distance);
    }
}

fn update_checkbox_mark(
    checkbox: Entity,
    checked: bool,
    children: &Query<
        &Children,
        Or<(
            With<AdtCheckbox>,
            With<ObjectCheckbox>,
            With<GroundEffectCheckbox>,
        )>,
    >,
    marks: &mut Query<
        (&mut Text, &mut BackgroundColor),
        (
            With<CheckboxMark>,
            Without<AdtDistanceLabel>,
            Without<ObjectDistanceLabel>,
            Without<GroundEffectDistanceLabel>,
        ),
    >,
) {
    let Ok(children) = children.get(checkbox) else {
        return;
    };
    for child in children.iter() {
        if let Ok((mut text, mut background)) = marks.get_mut(child) {
            **text = if checked { "x" } else { "" }.to_owned();
            background.0 = if checked { ACCENT } else { TRACK };
        }
    }
}

pub fn apply_render_visibility(
    settings: Res<RenderSettings>,
    mut adts: Query<
        (&mut Visibility, &mut VisibilityRange),
        (
            With<AdtPosition>,
            Without<RenderedObject>,
            Without<RenderedGroundEffect>,
        ),
    >,
    mut objects: Query<&mut Visibility, With<RenderedObject>>,
    mut ground_effects: Query<
        &mut Visibility,
        (With<RenderedGroundEffect>, Without<RenderedObject>),
    >,
) {
    if !settings.is_changed() {
        return;
    }
    for (mut visibility, mut range) in &mut adts {
        *visibility = if settings.render_adts {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        *range = VisibilityRange {
            use_aabb: true,
            ..VisibilityRange::abrupt(0.0, settings.adt_distance)
        };
    }
    for mut visibility in &mut objects {
        *visibility = if settings.render_objects {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut ground_effects {
        *visibility = if settings.render_ground_effects {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn format_distance(value: f32) -> String {
    if value >= 1_000.0 {
        format!("{:.1} km", value / 1_000.0)
    } else {
        format!("{value:.0} m")
    }
}
