use std::num::NonZeroU32;

use common::guid::{self, Guid};
use packets::character_info::{Equipment, GearInfo};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection};

use crate::{
    character::EquipmentItems,
    game_data::{StartEquipment, ValidClass, ValidGender, ValidRace},
};

// TEMPORARY NAME
// This structure only exists before player logs in,
// it is a convinience struct for database interaction
pub struct NewCharacter {
    pub name: String,
    pub race: ValidRace,
    pub class: ValidClass,
    pub gender: ValidGender,
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
    pub orientation: f32,
    pub guild_id: u32,
    pub flags: u32,
    pub first_login: bool,
    pub display_id: u32,
    pub equipment: StartEquipment,
}

impl NewCharacter {
    pub async fn insert(
        &self,
        db: &DatabaseConnection,
        account_id: u32,
    ) -> Result<(), sea_orm::DbErr> {
        gameserver_entity::character::ActiveModel {
            id: ActiveValue::NotSet,
            account_id: ActiveValue::Set(account_id as i64),
            name: ActiveValue::Set(self.name.clone()),
            race: ActiveValue::Set(self.race.get() as i8),
            class: ActiveValue::Set(self.class.get() as i8),
            gender: ActiveValue::Set(self.gender.get() as i8),
            skin: ActiveValue::Set(self.skin as i8),
            face: ActiveValue::Set(self.face as i8),
            hair_style: ActiveValue::Set(self.hairstyle as i8),
            hair_color: ActiveValue::Set(self.haircolor as i8),
            facial_hair: ActiveValue::Set(self.facialhair as i8),
            level: ActiveValue::Set(self.level as i8),
            area: ActiveValue::Set(self.area as i64),
            map: ActiveValue::Set(self.map as i64),
            position_x: ActiveValue::Set(self.position_x),
            position_y: ActiveValue::Set(self.position_y),
            position_z: ActiveValue::Set(self.position_z),
            orientation: ActiveValue::Set(self.orientation),
            guild_id: ActiveValue::Set(self.guild_id as i64),
            flags: ActiveValue::Set(self.flags as i64),
            first_login: ActiveValue::Set(self.first_login),
            display_id: ActiveValue::Set(self.display_id as i64),
            equipment_head: ActiveValue::Set(None),
            equipment_neck: ActiveValue::Set(None),
            equipment_shoulders: ActiveValue::Set(None),
            equipment_body: ActiveValue::Set(None),
            equipment_chest: ActiveValue::Set(None),
            equipment_waist: ActiveValue::Set(None),
            equipment_legs: ActiveValue::Set(None),
            equipment_feet: ActiveValue::Set(None),
            equipment_wrists: ActiveValue::Set(None),
            equipment_hands: ActiveValue::Set(None),
            equipment_finger1: ActiveValue::Set(None),
            equipment_finger2: ActiveValue::Set(None),
            equipment_trinket1: ActiveValue::Set(None),
            equipment_trinket2: ActiveValue::Set(None),
            equipment_back: ActiveValue::Set(None),
            equipment_mainhand: ActiveValue::Set(None),
            equipment_offhand: ActiveValue::Set(None),
            equipment_ranged: ActiveValue::Set(None),
            equipment_tabard: ActiveValue::Set(None),
            bag1: ActiveValue::Set(None),
            bag2: ActiveValue::Set(None),
            bag3: ActiveValue::Set(None),
            bag4: ActiveValue::Set(None),
            main_backpack1: ActiveValue::Set(None),
            main_backpack2: ActiveValue::Set(None),
            main_backpack3: ActiveValue::Set(None),
            main_backpack4: ActiveValue::Set(None),
            main_backpack5: ActiveValue::Set(None),
            main_backpack6: ActiveValue::Set(None),
            main_backpack7: ActiveValue::Set(None),
            main_backpack8: ActiveValue::Set(None),
            main_backpack9: ActiveValue::Set(None),
            main_backpack10: ActiveValue::Set(None),
            main_backpack11: ActiveValue::Set(None),
            main_backpack12: ActiveValue::Set(None),
            main_backpack13: ActiveValue::Set(None),
            main_backpack14: ActiveValue::Set(None),
            main_backpack15: ActiveValue::Set(None),
            main_backpack16: ActiveValue::Set(None),
            bank1: ActiveValue::Set(None),
            bank2: ActiveValue::Set(None),
            bank3: ActiveValue::Set(None),
            bank4: ActiveValue::Set(None),
            bank5: ActiveValue::Set(None),
            bank6: ActiveValue::Set(None),
            bank7: ActiveValue::Set(None),
            bank8: ActiveValue::Set(None),
            bank9: ActiveValue::Set(None),
            bank10: ActiveValue::Set(None),
            bank11: ActiveValue::Set(None),
            bank12: ActiveValue::Set(None),
            bank13: ActiveValue::Set(None),
            bank14: ActiveValue::Set(None),
            bank15: ActiveValue::Set(None),
            bank16: ActiveValue::Set(None),
            bank17: ActiveValue::Set(None),
            bank18: ActiveValue::Set(None),
            bank19: ActiveValue::Set(None),
            bank20: ActiveValue::Set(None),
            bank21: ActiveValue::Set(None),
            bank22: ActiveValue::Set(None),
            bank23: ActiveValue::Set(None),
            bank24: ActiveValue::Set(None),
            bank25: ActiveValue::Set(None),
            bank26: ActiveValue::Set(None),
            bank27: ActiveValue::Set(None),
            bank28: ActiveValue::Set(None),
            bank_bag1: ActiveValue::Set(None),
            bank_bag2: ActiveValue::Set(None),
            bank_bag3: ActiveValue::Set(None),
            bank_bag4: ActiveValue::Set(None),
            bank_bag5: ActiveValue::Set(None),
            bank_bag6: ActiveValue::Set(None),
            bank_bag7: ActiveValue::Set(None),
            vendor_buyback1: ActiveValue::Set(None),
            vendor_buyback2: ActiveValue::Set(None),
            vendor_buyback3: ActiveValue::Set(None),
            vendor_buyback4: ActiveValue::Set(None),
            vendor_buyback5: ActiveValue::Set(None),
            vendor_buyback6: ActiveValue::Set(None),
            vendor_buyback7: ActiveValue::Set(None),
            vendor_buyback8: ActiveValue::Set(None),
            vendor_buyback9: ActiveValue::Set(None),
            vendor_buyback10: ActiveValue::Set(None),
            vendor_buyback11: ActiveValue::Set(None),
            vendor_buyback12: ActiveValue::Set(None),
            keyring1: ActiveValue::Set(None),
            keyring2: ActiveValue::Set(None),
            keyring3: ActiveValue::Set(None),
            keyring4: ActiveValue::Set(None),
            keyring5: ActiveValue::Set(None),
            keyring6: ActiveValue::Set(None),
            keyring7: ActiveValue::Set(None),
            keyring8: ActiveValue::Set(None),
            keyring9: ActiveValue::Set(None),
            keyring10: ActiveValue::Set(None),
            keyring11: ActiveValue::Set(None),
            keyring12: ActiveValue::Set(None),
        }
        .insert(db)
        .await?;

        Ok(())
    }
}

// TEMPORARY NAME
// This is a convinience struct for database interaction
pub struct Character {
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
    pub guild_id: u32,
    pub flags: u32,
    pub first_login: bool,
    pub equipment: EquipmentItems,
}
impl Character {
    pub fn from_db(v: gameserver_entity::character::Model) -> Self {
        Self {
            guid: Guid::from_u32(NonZeroU32::new(v.id as u32).unwrap()), // Theoretically this should never be a problem but someone might be able to add a character with id 0 into the database,
            name: v.name,
            race: v.race as u8,
            class: v.class as u8,
            gender: v.gender as u8,
            skin: v.skin as u8,
            face: v.face as u8,
            hairstyle: v.hair_style as u8,
            haircolor: v.hair_color as u8,
            facialhair: v.facial_hair as u8,
            level: v.level as u8,
            area: v.area as u32,
            map: v.map as u32,
            position_x: v.position_x,
            position_y: v.position_y,
            position_z: v.position_z,
            guild_id: v.guild_id as u32,
            flags: v.flags as u32,
            first_login: v.first_login,
            equipment: EquipmentItems {
                head: None,
                neck: None,
                shoulders: None,
                body: None,
                chest: None,
                waist: None,
                legs: None,
                feet: None,
                wrists: None,
                hands: None,
                finger1: None,
                finger2: None,
                trinket1: None,
                trinket2: None,
                back: None,
                mainhand: None,
                offhand: None,
                ranged: None,
                tabard: None,
            },
        }
    }

    pub fn to_packet(self) -> packets::character_info::CharacterInfo {
        packets::character_info::CharacterInfo {
            guid: self.guid,
            name: self.name,
            race: self.race as u8,
            class: self.class as u8,
            gender: self.gender as u8,
            skin: self.skin as u8,
            face: self.face as u8,
            hairstyle: self.hairstyle as u8,
            haircolor: self.haircolor as u8,
            facialhair: self.facialhair as u8,
            level: self.level as u8,
            area: self.area as u32,
            map: self.map as u32,
            position_x: self.position_x,
            position_y: self.position_y,
            position_z: self.position_z,
            guild_id: self.guild_id as u32,
            flags: self.flags as u32,
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
