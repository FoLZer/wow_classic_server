use std::num::NonZeroU32;

use common::guid::{self, Guid};
use gamedata::equipment_slot::{VisibleEquipment, VisibleItem};
use log::error;
use packets::{bounded::Bounded, character_info::CharacterFlags};
use sqlx::{Pool, Sqlite};

pub struct CharacterSelection {
    pub guid: Guid<guid::Player>,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub skin: u8,
    pub face: u8,
    pub hairstyle: u8,
    pub haircolor: u8,
    pub facialhair: u8,
    pub level: u8,
    pub area: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub guild_id: Option<NonZeroU32>,
    pub flags: CharacterFlags,
    pub first_login: bool,
    pub equipment: EquipmentVisualItems,
}

impl CharacterSelection {
    pub async fn get_all_for_account(
        db: &Pool<Sqlite>,
        account_id: u32,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let models = sqlx::query!("SELECT * FROM character WHERE account_id = ?", account_id)
            .fetch_all(db)
            .await?;

        let mut characters = Vec::with_capacity(models.len());
        for model in models {
            characters.push(CharacterSelection {
                guid: Guid::from_u32(NonZeroU32::new(model.id as u32).unwrap()), // Theoretically this should never be a problem but someone might be able to add a character with id 0 into the database,
                name: model.name,
                race: model.race as u8,
                class: model.class as u8,
                gender: model.gender as u8,
                skin: model.skin as u8,
                face: model.face as u8,
                hairstyle: model.hair_style as u8,
                haircolor: model.hair_color as u8,
                facialhair: model.facial_hair as u8,
                level: model.level as u8,
                area: model.area as u32,
                map: model.map as u32,
                position_x: model.position_x as f32,
                position_y: model.position_y as f32,
                position_z: model.position_z as f32,
                guild_id: model.guild_id.map(|v| NonZeroU32::new(v as u32).unwrap()), // Same as above
                flags: CharacterFlags::new(),
                first_login: model.first_login,
                equipment: EquipmentVisualItems {
                    head: if let Some(id) = model.equipment_head_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    neck: if let Some(id) = model.equipment_neck_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    shoulders: if let Some(id) = model.equipment_shoulders_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    body: if let Some(id) = model.equipment_body_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    chest: if let Some(id) = model.equipment_chest_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    waist: if let Some(id) = model.equipment_waist_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    legs: if let Some(id) = model.equipment_legs_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    feet: if let Some(id) = model.equipment_feet_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    wrists: if let Some(id) = model.equipment_wrists_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    hands: if let Some(id) = model.equipment_hands_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    finger1: if let Some(id) = model.equipment_finger1_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    finger2: if let Some(id) = model.equipment_finger2_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    trinket1: if let Some(id) = model.equipment_trinket1_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    trinket2: if let Some(id) = model.equipment_trinket2_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    back: if let Some(id) = model.equipment_back_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    mainhand: if let Some(id) = model.equipment_mainhand_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    offhand: if let Some(id) = model.equipment_offhand_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    ranged: if let Some(id) = model.equipment_ranged_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                    tabard: if let Some(id) = model.equipment_tabard_id {
                        VisualItem::load(db, id).await?
                    } else {
                        None
                    },
                },
            })
        }

        Ok(characters)
    }

    pub fn to_packet(self) -> packets::character_info::CharacterInfo {
        packets::character_info::CharacterInfo {
            guid: self.guid,
            name: Bounded::try_from(self.name.as_str())
                .expect("character name exceeds the client limit"),
            race: self.race,
            class: self.class,
            gender: self.gender,
            skin: self.skin,
            face: self.face,
            hairstyle: self.hairstyle,
            haircolor: self.haircolor,
            facialhair: self.facialhair,
            level: self.level,
            area: self.area,
            map: self.map,
            position_x: self.position_x,
            position_y: self.position_y,
            position_z: self.position_z,
            guild_id: self.guild_id.map_or(0, |v: std::num::NonZero<u32>| v.get()),
            flags: self.flags,
            first_login: self.first_login,
            pet_display_id: 0, // TODO
            pet_level: 0,
            pet_family: 0,
            equipment: VisibleEquipment {
                head: self.equipment.head.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                neck: self.equipment.neck.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                shoulders: self.equipment.shoulders.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                body: self.equipment.body.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                chest: self.equipment.chest.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                waist: self.equipment.waist.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                legs: self.equipment.legs.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                feet: self.equipment.feet.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                wrists: self.equipment.wrists.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                hands: self.equipment.hands.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                finger1: self.equipment.finger1.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                finger2: self.equipment.finger2.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                trinket1: self.equipment.trinket1.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                trinket2: self.equipment.trinket2.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                back: self.equipment.back.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                mainhand: self.equipment.mainhand.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                offhand: self.equipment.offhand.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                ranged: self.equipment.ranged.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
                tabard: self.equipment.tabard.map(|v| VisibleItem {
                    display_id: v.display_id,
                    inventory_type: v.inventory_type,
                }),
            },
            first_bag_display_id: 0, // TODO
            first_bag_inventory_type: 0,
        }
    }
}

pub struct EquipmentVisualItems {
    pub head: Option<VisualItem>,
    pub neck: Option<VisualItem>,
    pub shoulders: Option<VisualItem>,
    pub body: Option<VisualItem>,
    pub chest: Option<VisualItem>,
    pub waist: Option<VisualItem>,
    pub legs: Option<VisualItem>,
    pub feet: Option<VisualItem>,
    pub wrists: Option<VisualItem>,
    pub hands: Option<VisualItem>,
    pub finger1: Option<VisualItem>,
    pub finger2: Option<VisualItem>,
    pub trinket1: Option<VisualItem>,
    pub trinket2: Option<VisualItem>,
    pub back: Option<VisualItem>,
    pub mainhand: Option<VisualItem>,
    pub offhand: Option<VisualItem>,
    pub ranged: Option<VisualItem>,
    pub tabard: Option<VisualItem>,
}

pub struct VisualItem {
    pub display_id: NonZeroU32,
    pub inventory_type: u8,
}

impl VisualItem {
    async fn load(db: &Pool<Sqlite>, id: i64) -> Result<Option<Self>, sqlx::Error> {
        // This only works atm because server DB and game data DB are the same, in future this will require to be rewritten
        match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
            Ok(v) => Ok(Some(Self {
                display_id: NonZeroU32::new(v.display_id as u32).unwrap(),
                inventory_type: v.inventory_type as u8
            })),
            Err(sqlx::Error::RowNotFound) => {
                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                Ok(None)
            },
            Err(e) => return Err(e)
        }
    }
}
