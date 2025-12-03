use std::fmt::Display;

use log::error;
use packets::character_info::GearInfo;
use sqlx::{Pool, Sqlite};

use crate::objects::item_prototype::ItemPrototype;

// This exists to provide an ability to switch data backend later if needed
// It's supposed to be easy to clone
#[derive(Clone)]
pub struct GameDataAccessor {
    db: Pool<Sqlite>,
}

impl GameDataAccessor {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn validate_race(&self, race: u8) -> Result<Option<ValidRace>, sqlx::Error> {
        match sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM race WHERE id = ?)", race)
            .fetch_one(&self.db)
            .await
        {
            Ok(_) => Ok(Some(ValidRace(race))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn validate_class(&self, class: u8) -> Result<Option<ValidClass>, sqlx::Error> {
        match sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM class WHERE id = ?)", class)
            .fetch_one(&self.db)
            .await
        {
            Ok(_) => Ok(Some(ValidClass(class))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // Returns None if this race + class combination is invalid
    pub async fn get_character_start_data(
        &self,
        race: ValidRace,
        class: ValidClass,
    ) -> Result<Option<CharacterStartData>, sqlx::Error> {
        let data = match sqlx::query!(
            "SELECT * FROM player_start_data WHERE race = ? AND class = ?",
            race.0,
            class.0
        )
        .fetch_one(&self.db)
        .await
        {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => {
                return Ok(None);
            }
            Err(e) => {
                return Err(e);
            }
        };

        Ok(Some(CharacterStartData {
            area_id: data.area as u32,
            map_id: data.map as u32,
            position: (
                data.position_x as f32,
                data.position_y as f32,
                data.position_z as f32,
            ),
            orientation: data.orientation as f32,
            level: data.level as u8,
            start_equipment: StartEquipment {
                head: if let Some(id) = data.equipment_head_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_head_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                neck: if let Some(id) = data.equipment_neck_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_neck_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                shoulders: if let Some(id) = data.equipment_shoulders_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_shoulders_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                body: if let Some(id) = data.equipment_body_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_body_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                chest: if let Some(id) = data.equipment_chest_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_chest_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                waist: if let Some(id) = data.equipment_waist_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_waist_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                legs: if let Some(id) = data.equipment_legs_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_legs_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                feet: if let Some(id) = data.equipment_feet_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_feet_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                wrists: if let Some(id) = data.equipment_wrists_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_wrists_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                hands: if let Some(id) = data.equipment_hands_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_hands_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                finger1: if let Some(id) = data.equipment_finger1_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_finger1_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                finger2: if let Some(id) = data.equipment_finger2_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_finger2_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                trinket1: if let Some(id) = data.equipment_trinket1_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_trinket1_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                trinket2: if let Some(id) = data.equipment_trinket2_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_trinket2_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                back: if let Some(id) = data.equipment_back_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_back_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                mainhand: if let Some(id) = data.equipment_mainhand_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_mainhand_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                offhand: if let Some(id) = data.equipment_offhand_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_offhand_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                ranged: if let Some(id) = data.equipment_ranged_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_ranged_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
                tabard: if let Some(id) = data.equipment_tabard_id {
                    match ItemPrototype::load_from_db(&self.db, id as u32).await {
                        Ok(v) => Some(v),
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "[CharacterStartData] equipment_tabard_id is set (id: {id}) but no item prototype found under this id. (race: {race}, class: {class}) This will set someone's item at this slot to None"
                            );
                            None
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    None
                },
            },
            other_equipment: vec![], //TODO
        }))
    }

    pub async fn get_display_id_for_race_gender(
        &self,
        race: ValidRace,
        gender: bool,
    ) -> Result<Option<u32>, sqlx::Error> {
        match sqlx::query_scalar!(
            "SELECT display_id FROM character_display_id WHERE race = ? AND gender = ?",
            race.0,
            gender
        )
        .fetch_one(&self.db)
        .await
        {
            Ok(v) => Ok(Some(v as u32)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn get_item_prototype(&self, id: u32) -> Result<Option<ItemPrototype>, sqlx::Error> {
        match ItemPrototype::load_from_db(&self.db, id).await {
            Ok(v) => Ok(Some(v)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub struct CharacterStartData {
    pub area_id: u32,
    pub map_id: u32,
    pub position: (f32, f32, f32),
    pub orientation: f32,
    pub level: u8,
    pub start_equipment: StartEquipment,

    //Any other equipment that should be put in player's bag
    pub other_equipment: Vec<GearInfo>,
}

#[derive(Debug)]
pub struct StartEquipment {
    pub head: Option<ItemPrototype>,
    pub neck: Option<ItemPrototype>,
    pub shoulders: Option<ItemPrototype>,
    pub body: Option<ItemPrototype>,
    pub chest: Option<ItemPrototype>,
    pub waist: Option<ItemPrototype>,
    pub legs: Option<ItemPrototype>,
    pub feet: Option<ItemPrototype>,
    pub wrists: Option<ItemPrototype>,
    pub hands: Option<ItemPrototype>,
    pub finger1: Option<ItemPrototype>,
    pub finger2: Option<ItemPrototype>,
    pub trinket1: Option<ItemPrototype>,
    pub trinket2: Option<ItemPrototype>,
    pub back: Option<ItemPrototype>,
    pub mainhand: Option<ItemPrototype>,
    pub offhand: Option<ItemPrototype>,
    pub ranged: Option<ItemPrototype>,
    pub tabard: Option<ItemPrototype>,
}

#[derive(Clone, Copy)]
pub struct ValidRace(u8);
impl ValidRace {
    pub fn get(&self) -> u8 {
        self.0
    }
}

impl Display for ValidRace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValidRace({})", self.0)
    }
}

#[derive(Clone, Copy)]
pub struct ValidClass(u8);
impl ValidClass {
    pub fn get(&self) -> u8 {
        self.0
    }
}

impl Display for ValidClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValidClass({})", self.0)
    }
}
