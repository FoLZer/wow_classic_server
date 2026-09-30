use std::path::PathBuf;

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    input_focus::tab_navigation::TabIndex,
    picking::hover::HoverMap,
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, Button, TextInput, observe},
};
use sqlx::{
    Column, QueryBuilder, Row, Sqlite, SqlitePool, TypeInfo,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tokio::runtime::{Builder, Runtime};

use crate::render_controls::{EditorMode, EditorModeChanged};

const BACKGROUND: Color = Color::srgb(0.035, 0.041, 0.047);
const PANEL: Color = Color::srgb(0.055, 0.065, 0.075);
const TEXT: Color = Color::srgb(0.91, 0.93, 0.94);
const MUTED_TEXT: Color = Color::srgb(0.62, 0.67, 0.70);
const BORDER: Color = Color::srgb(0.23, 0.27, 0.29);
const ACCENT: Color = Color::srgb(0.20, 0.72, 0.53);
const TRACK: Color = Color::srgb(0.16, 0.19, 0.20);
const PAGE_SIZE: i64 = 20;

pub struct ItemEditorPlugin {
    database_path: PathBuf,
}

impl ItemEditorPlugin {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

impl Plugin for ItemEditorPlugin {
    fn build(&self, app: &mut App) {
        let database = ItemDatabase::connect(self.database_path.clone());

        app.insert_resource(database)
            .init_resource::<ItemEditorState>()
            .add_systems(Startup, setup_item_editor)
            .add_observer(handle_item_editor_mode_changed)
            .add_systems(Update, (rebuild_item_editor, scroll_item_editor));
    }
}

#[derive(Clone)]
struct ItemSummary {
    id: i64,
    name: String,
}

#[derive(Clone, Copy)]
enum ColumnKind {
    Integer,
    Real,
    Text,
}

#[derive(Clone)]
struct ItemAttribute {
    name: String,
    value: String,
    kind: ColumnKind,
    editable: bool,
    required: bool,
}

#[derive(Resource)]
struct ItemEditorState {
    page: i64,
    total: i64,
    search: String,
    items: Vec<ItemSummary>,
    selected_item: Option<i64>,
    is_new_item: bool,
    attributes: Vec<ItemAttribute>,
    message: Option<String>,
    show_create_dialog: bool,
    auto_generated_item_id: Option<i64>,
    rebuild: bool,
}

impl Default for ItemEditorState {
    fn default() -> Self {
        Self {
            page: 0,
            total: 0,
            search: String::new(),
            items: Vec::new(),
            selected_item: None,
            is_new_item: false,
            attributes: Vec::new(),
            message: None,
            show_create_dialog: false,
            auto_generated_item_id: None,
            rebuild: true,
        }
    }
}

#[derive(Resource)]
struct ItemDatabase {
    runtime: Runtime,
    pool: Option<SqlitePool>,
    connection_error: Option<String>,
}

impl ItemDatabase {
    fn connect(path: PathBuf) -> Self {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("unable to create item database runtime");
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false);
        let result = runtime.block_on(SqlitePoolOptions::new().connect_with(options));
        let (pool, connection_error) = match result {
            Ok(pool) => (Some(pool), None),
            Err(error) => (
                None,
                Some(format!(
                    "Unable to open item database {}: {error}",
                    path.display()
                )),
            ),
        };
        Self {
            runtime,
            pool,
            connection_error,
        }
    }

    fn load_page(&self, state: &mut ItemEditorState) {
        let Some(pool) = &self.pool else {
            state.message = self.connection_error.clone();
            state.rebuild = true;
            return;
        };
        let pattern = format!("%{}%", state.search);
        let offset = state.page * PAGE_SIZE;
        let result = self.runtime.block_on(async {
            let total = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM item_prototype \
                 WHERE CAST(id AS TEXT) LIKE ? OR name LIKE ?",
            )
            .bind(&pattern)
            .bind(&pattern)
            .fetch_one(pool)
            .await?;
            let rows = sqlx::query(
                "SELECT id, name FROM item_prototype \
                 WHERE CAST(id AS TEXT) LIKE ? OR name LIKE ? \
                 ORDER BY id LIMIT ? OFFSET ?",
            )
            .bind(&pattern)
            .bind(&pattern)
            .bind(PAGE_SIZE)
            .bind(offset)
            .fetch_all(pool)
            .await?;
            let items = rows
                .into_iter()
                .map(|row| ItemSummary {
                    id: row.get("id"),
                    name: row.get("name"),
                })
                .collect();
            Ok::<_, sqlx::Error>((total, items))
        });
        match result {
            Ok((total, items)) => {
                state.total = total;
                state.items = items;
                state.message = None;
            }
            Err(error) => state.message = Some(format!("Unable to load items: {error}")),
        }
        state.rebuild = true;
    }

    fn next_item_id(&self) -> Result<i64, String> {
        let Some(pool) = &self.pool else {
            return Err(self
                .connection_error
                .clone()
                .unwrap_or_else(|| "Item database is unavailable".to_owned()));
        };
        self.runtime
            .block_on(
                sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(id), 0) + 1 FROM item_prototype")
                    .fetch_one(pool),
            )
            .map_err(|error| format!("Unable to generate an item ID: {error}"))
    }

    fn load_item(&self, state: &mut ItemEditorState, item_id: i64) {
        let Some(pool) = &self.pool else {
            return;
        };
        let result = self.runtime.block_on(async {
            let schema = sqlx::query("PRAGMA table_info(item_prototype)")
                .fetch_all(pool)
                .await?;
            let row = sqlx::query("SELECT * FROM item_prototype WHERE id = ?")
                .bind(item_id)
                .fetch_one(pool)
                .await?;
            let mut attributes = Vec::with_capacity(row.len());
            for column in row.columns() {
                let name = column.name().to_owned();
                let declared_type = column.type_info().name().to_ascii_uppercase();
                let kind = if declared_type.contains("CHAR") || declared_type.contains("TEXT") {
                    ColumnKind::Text
                } else if declared_type.contains("REAL")
                    || declared_type.contains("FLOA")
                    || declared_type.contains("DOUB")
                {
                    ColumnKind::Real
                } else {
                    ColumnKind::Integer
                };
                let value = match kind {
                    ColumnKind::Text => row
                        .try_get::<Option<String>, _>(name.as_str())?
                        .unwrap_or_default(),
                    ColumnKind::Real => row
                        .try_get::<Option<f64>, _>(name.as_str())?
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                    ColumnKind::Integer => row
                        .try_get::<Option<i64>, _>(name.as_str())?
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                };
                attributes.push(ItemAttribute {
                    editable: name != "id",
                    required: schema.iter().any(|schema_column| {
                        schema_column.get::<String, _>("name") == name
                            && schema_column.get::<i64, _>("notnull") != 0
                    }),
                    name,
                    value,
                    kind,
                });
            }
            Ok::<_, sqlx::Error>(attributes)
        });
        match result {
            Ok(attributes) => {
                state.selected_item = Some(item_id);
                state.is_new_item = false;
                state.attributes = attributes;
                state.message = None;
            }
            Err(error) => state.message = Some(format!("Unable to load item {item_id}: {error}")),
        }
        state.rebuild = true;
    }

    fn save_item(&self, state: &mut ItemEditorState, values: Vec<(String, String)>) {
        let (Some(pool), Some(item_id)) = (&self.pool, state.selected_item) else {
            return;
        };
        let original_attributes = state.attributes.clone();
        let mut attributes = original_attributes.clone();
        for (name, value) in values {
            if let Some(attribute) = attributes
                .iter_mut()
                .find(|attribute| attribute.name == name && attribute.editable)
            {
                attribute.value = value;
            }
        }
        if let Some(attribute_name) = attributes
            .iter()
            .find(|attribute| {
                attribute.required
                    && !matches!(attribute.kind, ColumnKind::Text)
                    && attribute.value.trim().is_empty()
            })
            .map(|attribute| attribute.name.clone())
        {
            state.attributes = attributes;
            state.message = Some(format!("{attribute_name} is required"));
            state.rebuild = true;
            return;
        }
        let is_new_item = state.is_new_item;
        let current_page = state.page;
        let result = self.runtime.block_on(async {
            let mut transaction = pool.begin().await?;
            if is_new_item {
                let mut query = QueryBuilder::<Sqlite>::new("INSERT INTO item_prototype (");
                {
                    let mut columns = query.separated(", ");
                    for attribute in &attributes {
                        columns.push(format!("\"{}\"", attribute.name.replace('"', "\"\"")));
                    }
                }
                query.push(") VALUES (");
                {
                    let mut values = query.separated(", ");
                    for attribute in &attributes {
                        match attribute.kind {
                            ColumnKind::Text
                                if !attribute.required && attribute.value.is_empty() =>
                            {
                                values.push_bind(Option::<String>::None);
                            }
                            ColumnKind::Text => {
                                values.push_bind(attribute.value.clone());
                            }
                            ColumnKind::Real
                                if !attribute.required && attribute.value.is_empty() =>
                            {
                                values.push_bind(Option::<f64>::None);
                            }
                            ColumnKind::Real => {
                                values.push_bind(attribute.value.parse::<f64>().map_err(|_| {
                                    sqlx::Error::Protocol(format!(
                                        "{} requires a decimal value",
                                        attribute.name
                                    ))
                                })?);
                            }
                            ColumnKind::Integer
                                if !attribute.required && attribute.value.is_empty() =>
                            {
                                values.push_bind(Option::<i64>::None);
                            }
                            ColumnKind::Integer => {
                                values.push_bind(attribute.value.parse::<i64>().map_err(|_| {
                                    sqlx::Error::Protocol(format!(
                                        "{} requires an integer value",
                                        attribute.name
                                    ))
                                })?);
                            }
                        };
                    }
                }
                query.push(")");
                query.build().execute(&mut *transaction).await?;
            } else {
                for attribute in &attributes {
                    let Some(original) = original_attributes
                        .iter()
                        .find(|original| original.name == attribute.name)
                    else {
                        continue;
                    };
                    if !attribute.editable || original.value == attribute.value {
                        continue;
                    }
                    let mut query = QueryBuilder::<Sqlite>::new("UPDATE item_prototype SET \"");
                    query.push(attribute.name.replace('"', "\"\""));
                    query.push("\" = ");
                    match attribute.kind {
                        ColumnKind::Text if !attribute.required && attribute.value.is_empty() => {
                            query.push_bind(Option::<String>::None)
                        }
                        ColumnKind::Text => query.push_bind(attribute.value.clone()),
                        ColumnKind::Real if !attribute.required && attribute.value.is_empty() => {
                            query.push_bind(Option::<f64>::None)
                        }
                        ColumnKind::Real => {
                            query.push_bind(attribute.value.parse::<f64>().map_err(|_| {
                                sqlx::Error::Protocol(format!(
                                    "{} requires a decimal value",
                                    attribute.name
                                ))
                            })?)
                        }
                        ColumnKind::Integer
                            if !attribute.required && attribute.value.is_empty() =>
                        {
                            query.push_bind(Option::<i64>::None)
                        }
                        ColumnKind::Integer => {
                            query.push_bind(attribute.value.parse::<i64>().map_err(|_| {
                                sqlx::Error::Protocol(format!(
                                    "{} requires an integer value",
                                    attribute.name
                                ))
                            })?)
                        }
                    };
                    query.push(" WHERE id = ").push_bind(item_id);
                    query.build().execute(&mut *transaction).await?;
                }
            }
            transaction.commit().await?;
            if is_new_item {
                let item_position = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM item_prototype WHERE id <= ?",
                )
                .bind(item_id)
                .fetch_one(pool)
                .await?;
                Ok::<i64, sqlx::Error>(item_position.saturating_sub(1) / PAGE_SIZE)
            } else {
                Ok::<i64, sqlx::Error>(current_page)
            }
        });
        match result {
            Ok(page) => {
                state.page = page;
                if is_new_item {
                    state.search.clear();
                }
                self.load_item(state, item_id);
                self.load_page(state);
                state.message = Some(if is_new_item {
                    format!("Created item {item_id}")
                } else {
                    format!("Saved item {item_id}")
                });
            }
            Err(error) => {
                state.attributes = attributes;
                state.message = Some(format!("Unable to save item {item_id}: {error}"));
                state.rebuild = true;
            }
        }
    }

    fn create_item_draft(&self, state: &mut ItemEditorState, requested_id: Option<i64>) {
        let Some(pool) = &self.pool else {
            return;
        };
        let result = self.runtime.block_on(async {
            let item_id = match requested_id {
                Some(item_id) => item_id,
                None => {
                    sqlx::query_scalar::<_, i64>(
                        "SELECT COALESCE(MAX(id), 0) + 1 FROM item_prototype",
                    )
                    .fetch_one(pool)
                    .await?
                }
            };
            let id_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM item_prototype WHERE id = ?)",
            )
            .bind(item_id)
            .fetch_one(pool)
            .await?;
            if id_exists {
                return Err(sqlx::Error::Protocol(format!(
                    "Item ID {item_id} already exists"
                )));
            }
            let schema = sqlx::query("PRAGMA table_info(item_prototype)")
                .fetch_all(pool)
                .await?;
            let mut attributes = Vec::with_capacity(schema.len());
            for row in schema {
                let name: String = row.get("name");
                let declared_type: String = row.get("type");
                let required = row.get::<i64, _>("notnull") != 0;
                let kind = if declared_type.to_ascii_uppercase().contains("TEXT") {
                    ColumnKind::Text
                } else if declared_type.to_ascii_uppercase().contains("REAL")
                    || declared_type.to_ascii_uppercase().contains("FLOA")
                    || declared_type.to_ascii_uppercase().contains("DOUB")
                {
                    ColumnKind::Real
                } else {
                    ColumnKind::Integer
                };
                let value = if name == "id" {
                    item_id.to_string()
                } else if name == "name" {
                    "New item".to_owned()
                } else if required {
                    match kind {
                        ColumnKind::Text => String::new(),
                        ColumnKind::Integer | ColumnKind::Real => "0".to_owned(),
                    }
                } else {
                    String::new()
                };
                attributes.push(ItemAttribute {
                    editable: name != "id",
                    required,
                    name,
                    value,
                    kind,
                });
            }
            Ok::<_, sqlx::Error>((item_id, attributes))
        });
        match result {
            Ok((item_id, attributes)) => {
                state.show_create_dialog = false;
                state.auto_generated_item_id = None;
                state.selected_item = Some(item_id);
                state.is_new_item = true;
                state.attributes = attributes;
                state.message = Some(format!("Item {item_id} is a draft until saved"));
                state.rebuild = true;
            }
            Err(error) => {
                state.message = Some(format!("Unable to create item draft: {error}"));
                state.rebuild = true;
            }
        }
    }
}

#[derive(Component)]
struct ItemEditorRoot;

#[derive(Component)]
struct ItemScrollArea;

#[derive(Component)]
struct ItemSearchInput;

#[derive(Component)]
struct NewItemIdInput;

#[derive(Component)]
struct ItemRow(i64);

#[derive(Component)]
struct AttributeInput(String);

#[derive(Component, Clone, Copy)]
enum PageAction {
    Previous,
    Next,
}

#[derive(Component, Clone, Copy)]
enum ItemAction {
    Search,
    Add,
    CreateManual,
    CreateAuto,
    CancelCreate,
    Save,
}

fn setup_item_editor(mut commands: Commands, state: Res<ItemEditorState>) {
    spawn_item_editor(&mut commands, &state, false);
}

fn rebuild_item_editor(
    mut commands: Commands,
    mut state: ResMut<ItemEditorState>,
    mode: Res<EditorMode>,
    roots: Query<Entity, With<ItemEditorRoot>>,
) {
    if !state.rebuild {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_item_editor(&mut commands, &state, *mode == EditorMode::Items);
    state.rebuild = false;
}

fn spawn_item_editor(commands: &mut Commands, state: &ItemEditorState, visible: bool) {
    commands
        .spawn((
            ItemEditorRoot,
            Name::new("Item editor"),
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                bottom: px(38),
                display: if visible {
                    Display::Flex
                } else {
                    Display::None
                },
                padding: UiRect::all(px(16)),
                column_gap: px(16),
                ..default()
            },
            GlobalZIndex(100),
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|root| {
            root.spawn(Node {
                flex_grow: 1.0,
                min_width: px(420),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            })
            .with_children(|main| {
                main.spawn((
                    Text::new("Items"),
                    TextFont::from_font_size(22.0),
                    TextColor(TEXT),
                ));
                main.spawn((
                    Text::new("Double-click an item to edit its attributes"),
                    TextFont::from_font_size(12.0),
                    TextColor(MUTED_TEXT),
                ));
                main.spawn((
                    ItemScrollArea,
                    Node {
                        flex_grow: 1.0,
                        width: percent(100),
                        overflow: Overflow::scroll_y(),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    BorderColor::all(BORDER),
                    ScrollPosition(Vec2::ZERO),
                ))
                .with_children(|list| {
                    list.spawn((
                        Node {
                            min_height: px(32),
                            padding: UiRect::horizontal(px(10)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(TRACK),
                        children![
                            (
                                Text::new("ID"),
                                TextFont::from_font_size(12.0),
                                TextColor(MUTED_TEXT),
                                Node {
                                    width: px(90),
                                    ..default()
                                },
                            ),
                            (
                                Text::new("NAME"),
                                TextFont::from_font_size(12.0),
                                TextColor(MUTED_TEXT),
                            ),
                        ],
                    ));
                    for item in &state.items {
                        let selected = state.selected_item == Some(item.id);
                        list.spawn((
                            ItemRow(item.id),
                            Button,
                            Node {
                                min_height: px(34),
                                padding: UiRect::horizontal(px(10)),
                                align_items: AlignItems::Center,
                                border: UiRect::bottom(px(1)),
                                ..default()
                            },
                            BackgroundColor(if selected { ACCENT } else { PANEL }),
                            BorderColor::all(BORDER),
                            observe(open_item),
                            children![
                                (
                                    Text::new(item.id.to_string()),
                                    TextFont::from_font_size(13.0),
                                    TextColor(TEXT),
                                    Node {
                                        width: px(90),
                                        ..default()
                                    },
                                ),
                                (
                                    Text::new(item.name.clone()),
                                    TextFont::from_font_size(13.0),
                                    TextColor(TEXT),
                                ),
                            ],
                        ));
                    }
                });
                main.spawn(Node {
                    min_height: px(34),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|pagination| {
                    spawn_page_button(pagination, "Previous", PageAction::Previous);
                    let pages = (state.total + PAGE_SIZE - 1) / PAGE_SIZE;
                    pagination.spawn((
                        Text::new(format!(
                            "Page {} of {}  |  {} items",
                            state.page + 1,
                            pages.max(1),
                            state.total
                        )),
                        TextFont::from_font_size(12.0),
                        TextColor(MUTED_TEXT),
                    ));
                    spawn_page_button(pagination, "Next", PageAction::Next);
                });
            });

            root.spawn(Node {
                width: px(430),
                height: percent(100),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                border: UiRect::all(px(1)),
                ..default()
            })
            .insert(BackgroundColor(PANEL))
            .insert(BorderColor::all(BORDER))
            .with_children(|sidebar| {
                sidebar
                    .spawn(Node {
                        width: percent(100),
                        height: px(34),
                        column_gap: px(6),
                        ..default()
                    })
                    .with_children(|search| {
                        search.spawn((
                            ItemSearchInput,
                            TextInput,
                            EditableText::new(state.search.clone()),
                            TextCursorStyle::default(),
                            TextFont::from_font_size(13.0),
                            TextColor(TEXT),
                            TextLayout::no_wrap(),
                            TabIndex(0),
                            Node {
                                flex_grow: 1.0,
                                min_width: px(0),
                                padding: UiRect::horizontal(px(8)),
                                border: UiRect::all(px(1)),
                                align_items: AlignItems::Center,
                                overflow: Overflow::clip_x(),
                                ..default()
                            },
                            BackgroundColor(TRACK),
                            BorderColor::all(BORDER),
                        ));
                        spawn_action_button(search, "Search", ItemAction::Search);
                    });
                spawn_action_button(sidebar, "+  Add new item", ItemAction::Add);
                sidebar.spawn((
                    Text::new(state.selected_item.map_or_else(
                        || "ITEM ATTRIBUTES".to_owned(),
                        |id| {
                            if state.is_new_item {
                                format!("NEW ITEM {id} ATTRIBUTES")
                            } else {
                                format!("ITEM {id} ATTRIBUTES")
                            }
                        },
                    )),
                    TextFont::from_font_size(13.0),
                    TextColor(MUTED_TEXT),
                ));
                sidebar
                    .spawn((
                        ItemScrollArea,
                        Node {
                            flex_grow: 1.0,
                            width: percent(100),
                            min_height: px(0),
                            overflow: Overflow::scroll_y(),
                            flex_direction: FlexDirection::Column,
                            border: UiRect::all(px(1)),
                            ..default()
                        },
                        BorderColor::all(BORDER),
                        ScrollPosition(Vec2::ZERO),
                    ))
                    .with_children(|table| {
                        table.spawn((
                            Node {
                                min_height: px(30),
                                padding: UiRect::horizontal(px(8)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(TRACK),
                            children![
                                (
                                    Text::new("ATTRIBUTE"),
                                    TextFont::from_font_size(11.0),
                                    TextColor(MUTED_TEXT),
                                    Node {
                                        width: px(180),
                                        ..default()
                                    },
                                ),
                                (
                                    Text::new("VALUE"),
                                    TextFont::from_font_size(11.0),
                                    TextColor(MUTED_TEXT),
                                ),
                            ],
                        ));
                        for attribute in &state.attributes {
                            table
                                .spawn(Node {
                                    min_height: px(32),
                                    padding: UiRect::horizontal(px(8)),
                                    align_items: AlignItems::Center,
                                    border: UiRect::bottom(px(1)),
                                    ..default()
                                })
                                .insert(BorderColor::all(BORDER))
                                .with_children(|row| {
                                    row.spawn(Node {
                                        width: px(180),
                                        align_items: AlignItems::Center,
                                        ..default()
                                    })
                                    .with_children(|label| {
                                        label.spawn((
                                            Text::new(attribute.name.clone()),
                                            TextFont::from_font_size(12.0),
                                            TextColor(MUTED_TEXT),
                                        ));
                                        if attribute.required {
                                            label.spawn((
                                                Text::new(" *"),
                                                TextFont::from_font_size(12.0),
                                                TextColor(Color::srgb(0.95, 0.48, 0.42)),
                                            ));
                                        }
                                    });
                                    if attribute.editable {
                                        row.spawn((
                                            AttributeInput(attribute.name.clone()),
                                            TextInput,
                                            EditableText::new(attribute.value.clone()),
                                            TextCursorStyle::default(),
                                            TextFont::from_font_size(12.0),
                                            TextColor(TEXT),
                                            TextLayout::no_wrap(),
                                            TabIndex(1),
                                            Node {
                                                flex_grow: 1.0,
                                                min_width: px(0),
                                                height: px(27),
                                                padding: UiRect::horizontal(px(6)),
                                                border: UiRect::all(px(1)),
                                                align_items: AlignItems::Center,
                                                overflow: Overflow::clip_x(),
                                                ..default()
                                            },
                                            BackgroundColor(TRACK),
                                            BorderColor::all(BORDER),
                                        ));
                                    } else {
                                        row.spawn((
                                            Text::new(attribute.value.clone()),
                                            TextFont::from_font_size(12.0),
                                            TextColor(TEXT),
                                        ));
                                    }
                                });
                        }
                    });
                if state.selected_item.is_some() {
                    spawn_action_button(sidebar, "Save changes", ItemAction::Save);
                }
                if let Some(message) = &state.message {
                    sidebar.spawn((
                        Text::new(message.clone()),
                        TextFont::from_font_size(11.0),
                        TextColor(MUTED_TEXT),
                    ));
                }
            });

            if state.show_create_dialog {
                spawn_create_item_dialog(
                    root,
                    state.auto_generated_item_id,
                    state.message.as_deref(),
                );
            }
        });
}

fn spawn_create_item_dialog(
    parent: &mut ChildSpawnerCommands,
    auto_generated_id: Option<i64>,
    message: Option<&str>,
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                right: px(0),
                bottom: px(0),
                left: px(0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(110),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.68)),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(400),
                        padding: UiRect::all(px(18)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    BorderColor::all(BORDER),
                ))
                .with_children(|dialog| {
                    dialog.spawn((
                        Text::new("Create item"),
                        TextFont::from_font_size(18.0),
                        TextColor(TEXT),
                    ));
                    dialog.spawn((
                        Text::new(auto_generated_id.map_or_else(
                            || "Enter an unused item ID.".to_owned(),
                            |item_id| {
                                format!(
                                    "Enter an unused item ID, or use the next generated ID: {item_id}."
                                )
                            },
                        )),
                        TextFont::from_font_size(12.0),
                        TextColor(MUTED_TEXT),
                    ));
                    dialog.spawn((
                        NewItemIdInput,
                        TextInput,
                        EditableText::new(""),
                        TextCursorStyle::default(),
                        TextFont::from_font_size(13.0),
                        TextColor(TEXT),
                        TextLayout::no_wrap(),
                        TabIndex(0),
                        Node {
                            width: percent(100),
                            height: px(36),
                            padding: UiRect::horizontal(px(8)),
                            border: UiRect::all(px(1)),
                            align_items: AlignItems::Center,
                            overflow: Overflow::clip_x(),
                            ..default()
                        },
                        BackgroundColor(TRACK),
                        BorderColor::all(BORDER),
                    ));
                    if let Some(message) = message {
                        dialog.spawn((
                            Text::new(message),
                            TextFont::from_font_size(11.0),
                            TextColor(Color::srgb(0.95, 0.48, 0.42)),
                        ));
                    }
                    dialog
                        .spawn(Node {
                            width: percent(100),
                            column_gap: px(8),
                            justify_content: JustifyContent::FlexEnd,
                            ..default()
                        })
                        .with_children(|actions| {
                            spawn_action_button(
                                actions,
                                &auto_generated_id.map_or_else(
                                    || "Auto ID unavailable".to_owned(),
                                    |item_id| format!("Use ID {item_id}"),
                                ),
                                ItemAction::CreateAuto,
                            );
                            spawn_action_button(actions, "Cancel", ItemAction::CancelCreate);
                            spawn_action_button(actions, "Create", ItemAction::CreateManual);
                        });
                });
        });
}

fn spawn_page_button(parent: &mut ChildSpawnerCommands, label: &str, action: PageAction) {
    parent
        .spawn((
            action,
            Button,
            Node {
                width: px(90),
                height: px(30),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(TRACK),
            BorderColor::all(BORDER),
            observe(change_page),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(12.0),
            TextColor(TEXT),
        ));
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, label: &str, action: ItemAction) {
    parent
        .spawn((
            action,
            Button,
            Node {
                min_width: px(90),
                height: px(34),
                min_height: px(34),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(10)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(ACCENT),
            BorderColor::all(BORDER),
            observe(run_item_action),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(12.0),
            TextColor(TEXT),
        ));
}

fn handle_item_editor_mode_changed(
    transition: On<EditorModeChanged>,
    database: Res<ItemDatabase>,
    mut state: ResMut<ItemEditorState>,
    mut roots: Query<&mut Node, With<ItemEditorRoot>>,
) {
    if transition.current == EditorMode::Items {
        database.load_page(&mut state);
    } else {
        state.page = 0;
        state.total = 0;
        state.search.clear();
        state.items.clear();
        state.selected_item = None;
        state.is_new_item = false;
        state.attributes.clear();
        state.message = None;
        state.show_create_dialog = false;
        state.auto_generated_item_id = None;
        state.rebuild = true;
    }
    for mut node in &mut roots {
        node.display = if transition.current == EditorMode::Items {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn open_item(
    click: On<PointerClick>,
    rows: Query<&ItemRow>,
    database: Res<ItemDatabase>,
    mut state: ResMut<ItemEditorState>,
) {
    if click.button != PointerButton::Primary || click.count != 2 {
        return;
    }
    if let Ok(row) = rows.get(click.event_target()) {
        database.load_item(&mut state, row.0);
    }
}

fn change_page(
    activate: On<Activate>,
    actions: Query<&PageAction>,
    database: Res<ItemDatabase>,
    mut state: ResMut<ItemEditorState>,
) {
    let Ok(action) = actions.get(activate.event_target()) else {
        return;
    };
    state.page = page_after_action(state.page, state.total, *action);
    database.load_page(&mut state);
}

fn page_after_action(current_page: i64, total: i64, action: PageAction) -> i64 {
    let last_page = (total.saturating_sub(1)) / PAGE_SIZE;
    match action {
        PageAction::Previous => current_page.saturating_sub(1).max(0),
        PageAction::Next => current_page.saturating_add(1).min(last_page),
    }
}

fn editable_value(editable: &EditableText) -> String {
    editable.value().into_iter().collect()
}

fn run_item_action(
    activate: On<Activate>,
    actions: Query<&ItemAction>,
    search_inputs: Query<&EditableText, With<ItemSearchInput>>,
    new_item_id_inputs: Query<&EditableText, With<NewItemIdInput>>,
    attribute_inputs: Query<(&EditableText, &AttributeInput)>,
    database: Res<ItemDatabase>,
    mut state: ResMut<ItemEditorState>,
) {
    let Ok(action) = actions.get(activate.event_target()) else {
        return;
    };
    match action {
        ItemAction::Search => {
            if let Ok(input) = search_inputs.single() {
                state.search = editable_value(input);
                state.page = 0;
                state.selected_item = None;
                state.attributes.clear();
                database.load_page(&mut state);
            }
        }
        ItemAction::Add => {
            state.show_create_dialog = true;
            match database.next_item_id() {
                Ok(item_id) => {
                    state.auto_generated_item_id = Some(item_id);
                    state.message = None;
                }
                Err(error) => {
                    state.auto_generated_item_id = None;
                    state.message = Some(error);
                }
            }
            state.rebuild = true;
        }
        ItemAction::CreateManual => {
            let requested_id = new_item_id_inputs
                .single()
                .ok()
                .map(editable_value)
                .and_then(|value| value.trim().parse::<i64>().ok());
            if requested_id.is_some_and(|item_id| item_id > 0) {
                database.create_item_draft(&mut state, requested_id);
            } else {
                state.message = Some("Item ID must be a positive integer".to_owned());
                state.rebuild = true;
            }
        }
        ItemAction::CreateAuto => {
            if let Some(item_id) = state.auto_generated_item_id {
                database.create_item_draft(&mut state, Some(item_id));
            }
        }
        ItemAction::CancelCreate => {
            state.show_create_dialog = false;
            state.auto_generated_item_id = None;
            state.rebuild = true;
        }
        ItemAction::Save => {
            let values = attribute_inputs
                .iter()
                .map(|(input, attribute)| (attribute.0.clone(), editable_value(input)))
                .collect();
            database.save_item(&mut state, values);
        }
    }
}

fn scroll_item_editor(
    mut mouse_wheel: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    children: Query<&Children>,
    mut areas: Query<(Entity, &mut ScrollPosition, &ComputedNode), With<ItemScrollArea>>,
) {
    let delta = mouse_wheel
        .read()
        .map(|event| {
            let scale = if event.unit == MouseScrollUnit::Line {
                24.0
            } else {
                1.0
            };
            -event.y * scale
        })
        .sum::<f32>();
    if delta == 0.0 {
        return;
    }
    for (area, mut position, computed) in &mut areas {
        let hovered = hover_map.values().any(|pointer| {
            pointer.keys().any(|entity| {
                *entity == area
                    || children
                        .iter_descendants(area)
                        .any(|child| child == *entity)
            })
        });
        if hovered {
            let max_offset =
                (computed.content_size().y - computed.size().y) * computed.inverse_scale_factor();
            position.y = (position.y + delta).clamp(0.0, max_offset.max(0.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, time::SystemTime};

    use super::*;

    #[test]
    fn pagination_stays_within_available_pages() {
        assert_eq!(page_after_action(0, 40, PageAction::Previous), 0);
        assert_eq!(page_after_action(1, 40, PageAction::Previous), 0);
        assert_eq!(page_after_action(0, 40, PageAction::Next), 1);
        assert_eq!(page_after_action(1, 40, PageAction::Next), 1);
        assert_eq!(page_after_action(0, 0, PageAction::Next), 0);
    }

    #[test]
    fn item_database_supports_create_edit_and_search() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("worldeditor-items-{unique}.db"));
        File::create(&path).unwrap();
        let database = ItemDatabase::connect(path.clone());
        let pool = database.pool.as_ref().unwrap();
        database
            .runtime
            .block_on(
                sqlx::query(
                    "CREATE TABLE item_prototype (\
                        id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, \
                        class INT NOT NULL, \
                        name TEXT NOT NULL, \
                        description TEXT NOT NULL, \
                        ranged_mod_range FLOAT NOT NULL, \
                        duration INT\
                    )",
                )
                .execute(pool),
            )
            .unwrap();

        let mut state = ItemEditorState::default();
        database.create_item_draft(&mut state, Some(42));
        assert_eq!(state.selected_item, Some(42));
        assert!(state.is_new_item);
        assert!(
            state
                .attributes
                .iter()
                .find(|attribute| attribute.name == "name")
                .unwrap()
                .required
        );
        let item_count = database
            .runtime
            .block_on(
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM item_prototype").fetch_one(pool),
            )
            .unwrap();
        assert_eq!(item_count, 0);

        database.save_item(
            &mut state,
            vec![
                ("name".to_owned(), "Copper test sword".to_owned()),
                ("class".to_owned(), "7".to_owned()),
                ("ranged_mod_range".to_owned(), "1.5".to_owned()),
                ("duration".to_owned(), "30".to_owned()),
            ],
        );
        assert!(!state.is_new_item);
        assert_eq!(state.total, 1);
        assert_eq!(state.items[0].name, "Copper test sword");
        let description = database
            .runtime
            .block_on(
                sqlx::query_scalar::<_, String>(
                    "SELECT description FROM item_prototype WHERE id = 42",
                )
                .fetch_one(pool),
            )
            .unwrap();
        assert_eq!(description, "");

        state.search = "Copper".to_owned();
        database.load_page(&mut state);
        assert_eq!(state.total, 1);
        state.search = "missing".to_owned();
        database.load_page(&mut state);
        assert!(state.items.is_empty());

        assert_eq!(database.next_item_id().unwrap(), 43);
        database.create_item_draft(&mut state, None);
        assert_eq!(state.selected_item, Some(43));
        let item_count = database
            .runtime
            .block_on(
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM item_prototype").fetch_one(pool),
            )
            .unwrap();
        assert_eq!(item_count, 1);

        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
