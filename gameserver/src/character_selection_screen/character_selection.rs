use std::num::NonZeroU32;

use common::guid::{self, Guid};
use log::error;
use packets::character_info::{Equipment, GearInfo};
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
    pub flags: u32,
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
                flags: 0,                                                             // TODO
                first_login: model.first_login,
                equipment: EquipmentVisualItems {
                    head: if let Some(id) = model.equipment_head_id {
                        // This only works atm because server DB and game data DB are the same, in future this will require to be rewritten
                        match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    neck: if let Some(id) = model.equipment_neck_id {
                        // This only works atm because server DB and game data DB are the same, in future this will require to be rewritten
                        match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    shoulders: if let Some(id) = model.equipment_shoulders_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    body: if let Some(id) = model.equipment_body_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    chest: if let Some(id) = model.equipment_chest_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    waist: if let Some(id) = model.equipment_waist_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    legs: if let Some(id) = model.equipment_legs_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    feet: if let Some(id) = model.equipment_feet_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    wrists: if let Some(id) = model.equipment_wrists_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    hands: if let Some(id) = model.equipment_hands_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    finger1: if let Some(id) = model.equipment_finger1_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    finger2: if let Some(id) = model.equipment_finger2_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    trinket1: if let Some(id) = model.equipment_trinket1_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    trinket2: if let Some(id) = model.equipment_trinket2_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    back: if let Some(id) = model.equipment_back_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    mainhand: if let Some(id) = model.equipment_mainhand_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    offhand: if let Some(id) = model.equipment_offhand_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    ranged: if let Some(id) = model.equipment_ranged_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
                    } else {
                        None
                    },
                    tabard: if let Some(id) = model.equipment_tabard_id {
                       match sqlx::query!("SELECT display_id, inventory_type FROM item JOIN item_prototype ON item.prototype_id = item_prototype.id WHERE item.id = ?", id).fetch_one(db).await {
                            Ok(v) => Some(VisualItem {
                                display_id: v.display_id as u32,
                                inventory_type: v.inventory_type as u8
                            }),
                            Err(sqlx::Error::RowNotFound) => {
                                error!("Tried to load an item without a prototype in the database! (item_id: {})", id);
                                None
                            },
                            Err(e) => return Err(e)
                        }
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
            name: self.name,
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
            guild_id: self.guild_id.map_or(0, |v| v.get()),
            flags: self.flags,
            first_login: self.first_login,
            pet_display_id: 0, // TODO
            pet_level: 0,
            pet_family: 0,
            equipment: Equipment {
                head: if let Some(v) = self.equipment.head {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                neck: if let Some(v) = self.equipment.neck {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                shoulders: if let Some(v) = self.equipment.shoulders {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                body: if let Some(v) = self.equipment.body {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                chest: if let Some(v) = self.equipment.chest {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                waist: if let Some(v) = self.equipment.waist {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                legs: if let Some(v) = self.equipment.legs {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                feet: if let Some(v) = self.equipment.feet {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                wrists: if let Some(v) = self.equipment.wrists {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                hands: if let Some(v) = self.equipment.hands {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                finger1: if let Some(v) = self.equipment.finger1 {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                finger2: if let Some(v) = self.equipment.finger2 {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                trinket1: if let Some(v) = self.equipment.trinket1 {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                trinket2: if let Some(v) = self.equipment.trinket2 {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                back: if let Some(v) = self.equipment.back {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                mainhand: if let Some(v) = self.equipment.mainhand {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                offhand: if let Some(v) = self.equipment.offhand {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                ranged: if let Some(v) = self.equipment.ranged {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
                tabard: if let Some(v) = self.equipment.tabard {
                    GearInfo {
                        display_id: v.display_id,
                        inventory_type: v.inventory_type,
                    }
                } else {
                    GearInfo {
                        display_id: 0,
                        inventory_type: 0,
                    }
                },
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
    pub display_id: u32,
    pub inventory_type: u8,
}
