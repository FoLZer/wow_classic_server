use packets::character_info::{Equipment, Guid};
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
