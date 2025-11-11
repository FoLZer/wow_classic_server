use packets::character_info::Equipment;
use sea_orm::DatabaseConnection;

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

    pub async fn validate_race(&self, race: u8) -> Option<ValidRace> {
        todo!();
    }

    pub async fn validate_class(&self, class: u8) -> Option<ValidClass> {
        todo!();
    }

    pub async fn validate_gender(&self, gender: u8) -> Option<ValidGender> {
        todo!();
    }

    // Returns None if this race + class combination is invalid
    pub async fn get_character_start_data(
        &self,
        race: ValidRace,
        class: ValidClass,
    ) -> Option<CharacterStartData> {
        todo!();
    }
}

pub struct CharacterStartData {
    pub area_id: u32,
    pub map_id: u32,
    pub position: (f32, f32, f32),
    pub orientation: f32,
    pub start_equipment: Equipment,
    pub level: u8,
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
