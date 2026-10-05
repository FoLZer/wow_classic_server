use std::num::NonZeroU32;

use sqlx::{Pool, Sqlite};

use crate::{game_data::GameDataAccessor, objects::item_prototype::ItemPrototype};

pub struct CreaturePrototype {
    pub health: Option<u32>,
    pub name: String,
    pub sub_name: Option<String>,
    pub max_health: u32,
    pub level: std::ops::RangeInclusive<u32>,
    pub faction: u32,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub power: u8,

    pub equipment_mainhand: Option<ItemPrototype>,
    pub equipment_offhand: Option<ItemPrototype>,
    pub equipment_ranged: Option<ItemPrototype>,

    pub base_attack_time: u32,
    pub offhand_attack_time: u32,
    pub ranged_attack_time: u32,

    pub combat_reach: f32,
    pub display_id: u32,
    pub native_display_id: u32,
    pub initial_mount_display_id: Option<u32>,

    pub flags: u32,
    pub r#type: u32,
    pub family: Option<NonZeroU32>,
    pub rank: u32,
    pub civilian: u16,
}

impl CreaturePrototype {
    pub async fn load_from_db(
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        id: u32,
    ) -> Result<Self, sqlx::Error> {
        let model = sqlx::query!("SELECT * FROM creature WHERE id = ?", id)
            .fetch_one(db)
            .await?;

        Ok(Self {
            health: model.health.map(|v| v as u32),
            name: model.name,
            sub_name: model.sub_name,
            max_health: model.max_health as u32,
            level: (model.level_min as u32)..=(model.level_max as u32),
            faction: model.faction as u32,
            race: model.race as u8,
            class: model.class as u8,
            gender: model.gender as u8,
            power: model.power as u8,

            equipment_mainhand: if let Some(id) = model.equipment_mainhand_id {
                match game_data_accessor.get_item_prototype(id as u32).await {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                }
            } else {
                None
            },
            equipment_offhand: if let Some(id) = model.equipment_offhand_id {
                match game_data_accessor.get_item_prototype(id as u32).await {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                }
            } else {
                None
            },
            equipment_ranged: if let Some(id) = model.equipment_ranged_id {
                match game_data_accessor.get_item_prototype(id as u32).await {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                }
            } else {
                None
            },

            base_attack_time: model.base_attack_time as u32,
            offhand_attack_time: model.offhand_attack_time as u32,
            ranged_attack_time: model.ranged_attack_time as u32,

            combat_reach: model.combat_reach as f32,
            display_id: model.display_id as u32,
            native_display_id: model.native_display_id as u32,
            initial_mount_display_id: model.initial_mount_display_id.map(|v| v as u32),

            flags: model.flags as u32,
            r#type: model.r#type as u32,
            family: NonZeroU32::new(model.family.unwrap_or(0) as u32),
            rank: model.rank as u32,
            civilian: model.civilian as u16,
        })
    }
}
