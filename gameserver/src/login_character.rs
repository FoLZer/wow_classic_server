use std::num::NonZeroU32;

use common::guid::{self, Guid};
use packets::character_info::{Equipment, GearInfo};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection};

use crate::game_data::{ValidClass, ValidGender, ValidRace};

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
    pub equipment: Equipment,
}

impl NewCharacter {
    pub async fn insert(
        &self,
        db: &DatabaseConnection,
        account_id: u32,
    ) -> Result<(), sea_orm::DbErr> {
        gameserver_entity::character::ActiveModel {
            id: ActiveValue::NotSet,
            account_id: ActiveValue::Set(account_id as i32),
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
            area: ActiveValue::Set(self.area as i32),
            map: ActiveValue::Set(self.map as i32),
            position_x: ActiveValue::Set(self.position_x),
            position_y: ActiveValue::Set(self.position_y),
            position_z: ActiveValue::Set(self.position_z),
            orientation: ActiveValue::Set(self.orientation),
            guild_id: ActiveValue::Set(self.guild_id as i32),
            flags: ActiveValue::Set(self.flags as i32),
            first_login: ActiveValue::Set(self.first_login),
            display_id: ActiveValue::Set(self.display_id as i32),
            equipment_head_display_id: ActiveValue::Set(self.equipment.head.display_id as i32),
            equipment_head_inventory_type: ActiveValue::Set(
                self.equipment.head.inventory_type as i32,
            ),
            equipment_neck_display_id: ActiveValue::Set(self.equipment.neck.display_id as i32),
            equipment_neck_inventory_type: ActiveValue::Set(
                self.equipment.neck.inventory_type as i32,
            ),
            equipment_shoulders_display_id: ActiveValue::Set(
                self.equipment.shoulders.display_id as i32,
            ),
            equipment_shoulders_inventory_type: ActiveValue::Set(
                self.equipment.shoulders.inventory_type as i32,
            ),
            equipment_body_display_id: ActiveValue::Set(self.equipment.body.display_id as i32),
            equipment_body_inventory_type: ActiveValue::Set(
                self.equipment.body.inventory_type as i32,
            ),
            equipment_chest_display_id: ActiveValue::Set(self.equipment.chest.display_id as i32),
            equipment_chest_inventory_type: ActiveValue::Set(
                self.equipment.chest.inventory_type as i32,
            ),
            equipment_waist_display_id: ActiveValue::Set(self.equipment.waist.display_id as i32),
            equipment_waist_inventory_type: ActiveValue::Set(
                self.equipment.waist.inventory_type as i32,
            ),
            equipment_legs_display_id: ActiveValue::Set(self.equipment.legs.display_id as i32),
            equipment_legs_inventory_type: ActiveValue::Set(
                self.equipment.legs.inventory_type as i32,
            ),
            equipment_feet_display_id: ActiveValue::Set(self.equipment.feet.display_id as i32),
            equipment_feet_inventory_type: ActiveValue::Set(
                self.equipment.feet.inventory_type as i32,
            ),
            equipment_wrists_display_id: ActiveValue::Set(self.equipment.wrists.display_id as i32),
            equipment_wrists_inventory_type: ActiveValue::Set(
                self.equipment.wrists.inventory_type as i32,
            ),
            equipment_hands_display_id: ActiveValue::Set(self.equipment.hands.display_id as i32),
            equipment_hands_inventory_type: ActiveValue::Set(
                self.equipment.hands.inventory_type as i32,
            ),
            equipment_finger1_display_id: ActiveValue::Set(
                self.equipment.finger1.display_id as i32,
            ),
            equipment_finger1_inventory_type: ActiveValue::Set(
                self.equipment.finger1.inventory_type as i32,
            ),
            equipment_finger2_display_id: ActiveValue::Set(
                self.equipment.finger2.display_id as i32,
            ),
            equipment_finger2_inventory_type: ActiveValue::Set(
                self.equipment.finger2.inventory_type as i32,
            ),
            equipment_trinket1_display_id: ActiveValue::Set(
                self.equipment.trinket1.display_id as i32,
            ),
            equipment_trinket1_inventory_type: ActiveValue::Set(
                self.equipment.trinket1.inventory_type as i32,
            ),
            equipment_trinket2_display_id: ActiveValue::Set(
                self.equipment.trinket2.display_id as i32,
            ),
            equipment_trinket2_inventory_type: ActiveValue::Set(
                self.equipment.trinket2.inventory_type as i32,
            ),
            equipment_back_display_id: ActiveValue::Set(self.equipment.back.display_id as i32),
            equipment_back_inventory_type: ActiveValue::Set(
                self.equipment.back.inventory_type as i32,
            ),
            equipment_mainhand_display_id: ActiveValue::Set(
                self.equipment.mainhand.display_id as i32,
            ),
            equipment_mainhand_inventory_type: ActiveValue::Set(
                self.equipment.mainhand.inventory_type as i32,
            ),
            equipment_offhand_display_id: ActiveValue::Set(
                self.equipment.offhand.display_id as i32,
            ),
            equipment_offhand_inventory_type: ActiveValue::Set(
                self.equipment.offhand.inventory_type as i32,
            ),
            equipment_ranged_display_id: ActiveValue::Set(self.equipment.ranged.display_id as i32),
            equipment_ranged_inventory_type: ActiveValue::Set(
                self.equipment.ranged.inventory_type as i32,
            ),
            equipment_tabard_display_id: ActiveValue::Set(self.equipment.tabard.display_id as i32),
            equipment_tabard_inventory_type: ActiveValue::Set(
                self.equipment.tabard.inventory_type as i32,
            ),
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
    pub equipment: Equipment,
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
            equipment: Equipment {
                head: GearInfo {
                    display_id: v.equipment_head_display_id as u32,
                    inventory_type: v.equipment_head_inventory_type as u8,
                },
                neck: GearInfo {
                    display_id: v.equipment_neck_display_id as u32,
                    inventory_type: v.equipment_neck_inventory_type as u8,
                },
                shoulders: GearInfo {
                    display_id: v.equipment_shoulders_display_id as u32,
                    inventory_type: v.equipment_shoulders_inventory_type as u8,
                },
                body: GearInfo {
                    display_id: v.equipment_body_display_id as u32,
                    inventory_type: v.equipment_body_inventory_type as u8,
                },
                chest: GearInfo {
                    display_id: v.equipment_chest_display_id as u32,
                    inventory_type: v.equipment_chest_inventory_type as u8,
                },
                waist: GearInfo {
                    display_id: v.equipment_waist_display_id as u32,
                    inventory_type: v.equipment_waist_inventory_type as u8,
                },
                legs: GearInfo {
                    display_id: v.equipment_legs_display_id as u32,
                    inventory_type: v.equipment_legs_inventory_type as u8,
                },
                feet: GearInfo {
                    display_id: v.equipment_feet_display_id as u32,
                    inventory_type: v.equipment_feet_inventory_type as u8,
                },
                wrists: GearInfo {
                    display_id: v.equipment_wrists_display_id as u32,
                    inventory_type: v.equipment_wrists_inventory_type as u8,
                },
                hands: GearInfo {
                    display_id: v.equipment_hands_display_id as u32,
                    inventory_type: v.equipment_hands_inventory_type as u8,
                },
                finger1: GearInfo {
                    display_id: v.equipment_finger1_display_id as u32,
                    inventory_type: v.equipment_finger1_inventory_type as u8,
                },
                finger2: GearInfo {
                    display_id: v.equipment_finger2_display_id as u32,
                    inventory_type: v.equipment_finger2_inventory_type as u8,
                },
                trinket1: GearInfo {
                    display_id: v.equipment_trinket1_display_id as u32,
                    inventory_type: v.equipment_trinket1_inventory_type as u8,
                },
                trinket2: GearInfo {
                    display_id: v.equipment_trinket2_display_id as u32,
                    inventory_type: v.equipment_trinket2_inventory_type as u8,
                },
                back: GearInfo {
                    display_id: v.equipment_back_display_id as u32,
                    inventory_type: v.equipment_back_inventory_type as u8,
                },
                mainhand: GearInfo {
                    display_id: v.equipment_mainhand_display_id as u32,
                    inventory_type: v.equipment_mainhand_inventory_type as u8,
                },
                offhand: GearInfo {
                    display_id: v.equipment_offhand_display_id as u32,
                    inventory_type: v.equipment_offhand_inventory_type as u8,
                },
                ranged: GearInfo {
                    display_id: v.equipment_ranged_display_id as u32,
                    inventory_type: v.equipment_ranged_inventory_type as u8,
                },
                tabard: GearInfo {
                    display_id: v.equipment_tabard_display_id as u32,
                    inventory_type: v.equipment_tabard_inventory_type as u8,
                },
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
            equipment: self.equipment,
            first_bag_display_id: 0, // TODO
            first_bag_inventory_type: 0,
        }
    }
}
