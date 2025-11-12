use packets::character_info::{Equipment, GearInfo};
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};

// This exists to provide an ability to switch data backend later if needed
// It's supposed to be easy to clone
#[derive(Clone)]
pub struct GameDataAccessor {
    db: DatabaseConnection,
}

impl GameDataAccessor {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn validate_race(&self, race: u8) -> Result<Option<ValidRace>, sea_orm::DbErr> {
        if gameserver_entity::race::Entity::find_by_id(race)
            .count(&self.db)
            .await?
            > 0
        {
            Ok(Some(ValidRace(race)))
        } else {
            Ok(None)
        }
    }

    pub async fn validate_class(&self, class: u8) -> Result<Option<ValidClass>, sea_orm::DbErr> {
        if gameserver_entity::class::Entity::find_by_id(class)
            .count(&self.db)
            .await?
            > 0
        {
            Ok(Some(ValidClass(class)))
        } else {
            Ok(None)
        }
    }

    pub async fn validate_gender(&self, gender: u8) -> Result<Option<ValidGender>, sea_orm::DbErr> {
        if gameserver_entity::gender::Entity::find_by_id(gender)
            .count(&self.db)
            .await?
            > 0
        {
            Ok(Some(ValidGender(gender)))
        } else {
            Ok(None)
        }
    }

    // Returns None if this race + class combination is invalid
    pub async fn get_character_start_data(
        &self,
        race: ValidRace,
        class: ValidClass,
    ) -> Result<Option<CharacterStartData>, sea_orm::DbErr> {
        let data =
            gameserver_entity::player_start_data::Entity::find_by_id((race.0 as i8, class.0 as i8))
                .one(&self.db)
                .await?;
        Ok(data.map(|data| CharacterStartData {
            area_id: data.area_id as u32,
            map_id: data.map_id as u32,
            position: (data.position_x, data.position_y, data.position_z),
            orientation: data.orientation,
            level: data.level as u8,
            start_equipment: Equipment {
                head: GearInfo {
                    display_id: data.equipment_head_display_id as u32,
                    inventory_type: data.equipment_head_inventory_type as u8,
                },
                neck: GearInfo {
                    display_id: data.equipment_neck_display_id as u32,
                    inventory_type: data.equipment_neck_inventory_type as u8,
                },
                shoulders: GearInfo {
                    display_id: data.equipment_shoulders_display_id as u32,
                    inventory_type: data.equipment_shoulders_inventory_type as u8,
                },
                body: GearInfo {
                    display_id: data.equipment_body_display_id as u32,
                    inventory_type: data.equipment_body_inventory_type as u8,
                },
                chest: GearInfo {
                    display_id: data.equipment_chest_display_id as u32,
                    inventory_type: data.equipment_chest_inventory_type as u8,
                },
                waist: GearInfo {
                    display_id: data.equipment_waist_display_id as u32,
                    inventory_type: data.equipment_waist_inventory_type as u8,
                },
                legs: GearInfo {
                    display_id: data.equipment_legs_display_id as u32,
                    inventory_type: data.equipment_legs_inventory_type as u8,
                },
                feet: GearInfo {
                    display_id: data.equipment_feet_display_id as u32,
                    inventory_type: data.equipment_feet_inventory_type as u8,
                },
                wrists: GearInfo {
                    display_id: data.equipment_wrists_display_id as u32,
                    inventory_type: data.equipment_wrists_inventory_type as u8,
                },
                hands: GearInfo {
                    display_id: data.equipment_hands_display_id as u32,
                    inventory_type: data.equipment_hands_inventory_type as u8,
                },
                finger1: GearInfo {
                    display_id: data.equipment_finger1_display_id as u32,
                    inventory_type: data.equipment_finger1_inventory_type as u8,
                },
                finger2: GearInfo {
                    display_id: data.equipment_finger2_display_id as u32,
                    inventory_type: data.equipment_finger2_inventory_type as u8,
                },
                trinket1: GearInfo {
                    display_id: data.equipment_trinket1_display_id as u32,
                    inventory_type: data.equipment_trinket1_inventory_type as u8,
                },
                trinket2: GearInfo {
                    display_id: data.equipment_trinket2_display_id as u32,
                    inventory_type: data.equipment_trinket2_inventory_type as u8,
                },
                back: GearInfo {
                    display_id: data.equipment_back_display_id as u32,
                    inventory_type: data.equipment_back_inventory_type as u8,
                },
                mainhand: GearInfo {
                    display_id: data.equipment_mainhand_display_id as u32,
                    inventory_type: data.equipment_mainhand_inventory_type as u8,
                },
                offhand: GearInfo {
                    display_id: data.equipment_offhand_display_id as u32,
                    inventory_type: data.equipment_offhand_inventory_type as u8,
                },
                ranged: GearInfo {
                    display_id: data.equipment_ranged_display_id as u32,
                    inventory_type: data.equipment_ranged_inventory_type as u8,
                },
                tabard: GearInfo {
                    display_id: data.equipment_tabard_display_id as u32,
                    inventory_type: data.equipment_tabard_inventory_type as u8,
                },
            },
            other_equipment: vec![], //TODO
        }))
    }
}

pub struct CharacterStartData {
    pub area_id: u32,
    pub map_id: u32,
    pub position: (f32, f32, f32),
    pub orientation: f32,
    pub level: u8,
    pub start_equipment: Equipment,

    //Any other equipment that should be put in player's bag
    pub other_equipment: Vec<GearInfo>,
}

#[derive(Clone, Copy)]
pub struct ValidRace(u8);
impl ValidRace {
    pub fn get(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct ValidClass(u8);
impl ValidClass {
    pub fn get(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct ValidGender(u8);
impl ValidGender {
    pub fn get(&self) -> u8 {
        self.0
    }
}
