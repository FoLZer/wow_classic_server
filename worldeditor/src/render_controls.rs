use bevy::{
    camera::visibility::VisibilityRange,
    input_focus::tab_navigation::TabIndex,
    picking::hover::Hovered,
    prelude::*,
    ui::Checked,
    ui_widgets::{
        Checkbox, Slider, SliderRange, SliderThumb, SliderValue, TrackClick, ValueChange,
        checkbox_self_update, observe, slider_self_update,
    },
};

use crate::map_loader::{AdtPosition, RenderedObject};

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
    pub adt_distance: f32,
    pub object_distance: f32,
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
pub(crate) struct CheckboxMark;

#[derive(Component)]
pub(crate) struct AdtDistanceLabel;

#[derive(Component)]
pub(crate) struct ObjectDistanceLabel;

#[derive(Component)]
pub(crate) struct DistanceSlider;

#[derive(Component)]
pub(crate) struct DistanceSliderThumb;

pub fn setup_render_controls(mut commands: Commands, settings: Res<RenderSettings>) {
    commands
        .spawn((
            Name::new("Render controls"),
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
            BackgroundColor(PANEL),
            BorderColor::all(BORDER),
            GlobalZIndex(100),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("RENDER"),
                TextFont::from_font_size(13.0),
                TextColor(MUTED_TEXT),
            ));
            spawn_checkbox(panel, "ADT terrain", settings.render_adts, AdtCheckbox);
            spawn_checkbox(panel, "Objects", settings.render_objects, ObjectCheckbox);
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
        });
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
                    )
                ],
            ));
        });
}

pub fn update_slider_visuals(
    sliders: Query<
        (Entity, &SliderValue, &SliderRange),
        (Changed<SliderValue>, With<DistanceSlider>),
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
    checkbox_children: Query<&Children, Or<(With<AdtCheckbox>, With<ObjectCheckbox>)>>,
    mut marks: Query<
        (&mut Text, &mut BackgroundColor),
        (
            With<CheckboxMark>,
            Without<AdtDistanceLabel>,
            Without<ObjectDistanceLabel>,
        ),
    >,
    mut adt_labels: Query<
        &mut Text,
        (
            With<AdtDistanceLabel>,
            Without<CheckboxMark>,
            Without<ObjectDistanceLabel>,
        ),
    >,
    mut object_labels: Query<
        &mut Text,
        (
            With<ObjectDistanceLabel>,
            Without<CheckboxMark>,
            Without<AdtDistanceLabel>,
        ),
    >,
    mut settings: ResMut<RenderSettings>,
) {
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
    for mut label in &mut adt_labels {
        **label = format_distance(settings.adt_distance);
    }
    for mut label in &mut object_labels {
        **label = format_distance(settings.object_distance);
    }
}

fn update_checkbox_mark(
    checkbox: Entity,
    checked: bool,
    children: &Query<&Children, Or<(With<AdtCheckbox>, With<ObjectCheckbox>)>>,
    marks: &mut Query<
        (&mut Text, &mut BackgroundColor),
        (
            With<CheckboxMark>,
            Without<AdtDistanceLabel>,
            Without<ObjectDistanceLabel>,
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
        (With<AdtPosition>, Without<RenderedObject>),
    >,
    mut objects: Query<&mut Visibility, With<RenderedObject>>,
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
}

fn format_distance(value: f32) -> String {
    if value >= 1_000.0 {
        format!("{:.1} km", value / 1_000.0)
    } else {
        format!("{value:.0} m")
    }
}
