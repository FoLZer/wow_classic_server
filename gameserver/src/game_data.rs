use packets::{
    character_info::{Equipment, GearInfo},
    item_info::{ItemDamage, ItemFlags, ItemSpell, ItemStat},
};
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

    pub async fn get_display_id_for_race_gender(
        &self,
        race: ValidRace,
        gender: ValidGender,
    ) -> Result<Option<u32>, sea_orm::DbErr> {
        let data = gameserver_entity::character_display_id::Entity::find_by_id((
            race.0 as i8,
            gender.0 as i8,
        ))
        .one(&self.db)
        .await?;
        Ok(data.map(|v| v.display_id as u32))
    }

    pub async fn get_item_prototype(
        &self,
        item_id: u32,
    ) -> Result<Option<ItemPrototype>, sea_orm::DbErr> {
        let Some(model) = gameserver_entity::item_prototype::Entity::find_by_id(item_id as i32)
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(ItemPrototype {
            item_id,
            class: model.class as u32,
            sub_class: model.sub_class as u32,
            name: model.name,
            description: model.description,
            display_info_id: model.display_id as u32,
            quality: model.quality as u32,
            flags: ItemFlags::new(), //TODO
            buy_price: model.buy_price as u32,
            sell_price: model.sell_price as u32,
            inventory_type: model.inventory_type as u32,
            allowable_class: model.allowable_class as u32,
            allowable_race: model.allowable_race as u32,
            item_level: model.item_level as u32,
            required_level: model.required_level as u32,
            required_skill: model.required_skill as u32,
            required_skill_rank: model.required_skill_rank as u32,
            required_spell: model.required_spell as u32,
            required_honor_rank: model.required_honor_rank as u32,
            required_city_rank: model.required_city_rank as u32,
            required_reputation_faction: model.required_reputation_faction as u32,
            required_reputation_rank: model.required_reputation_rank as u32,
            max_count: model.max_count as u32,
            stackable: model.stackable as u32,
            container_slots: model.container_slots as u32,
            item_stats: [
                ItemStat {
                    ty: model.item_stat1_type as u32,
                    value: model.item_stat1_value as u32,
                },
                ItemStat {
                    ty: model.item_stat2_type as u32,
                    value: model.item_stat2_value as u32,
                },
                ItemStat {
                    ty: model.item_stat3_type as u32,
                    value: model.item_stat3_value as u32,
                },
                ItemStat {
                    ty: model.item_stat4_type as u32,
                    value: model.item_stat4_value as u32,
                },
                ItemStat {
                    ty: model.item_stat5_type as u32,
                    value: model.item_stat5_value as u32,
                },
                ItemStat {
                    ty: model.item_stat6_type as u32,
                    value: model.item_stat6_value as u32,
                },
                ItemStat {
                    ty: model.item_stat7_type as u32,
                    value: model.item_stat7_value as u32,
                },
                ItemStat {
                    ty: model.item_stat8_type as u32,
                    value: model.item_stat8_value as u32,
                },
                ItemStat {
                    ty: model.item_stat9_type as u32,
                    value: model.item_stat9_value as u32,
                },
                ItemStat {
                    ty: model.item_stat10_type as u32,
                    value: model.item_stat10_value as u32,
                },
            ],
            damage: [
                ItemDamage {
                    min: model.item_damage1_min as u32,
                    max: model.item_damage1_max as u32,
                    ty: model.item_damage1_type as u32,
                },
                ItemDamage {
                    min: model.item_damage2_min as u32,
                    max: model.item_damage2_max as u32,
                    ty: model.item_damage2_type as u32,
                },
                ItemDamage {
                    min: model.item_damage3_min as u32,
                    max: model.item_damage3_max as u32,
                    ty: model.item_damage3_type as u32,
                },
                ItemDamage {
                    min: model.item_damage4_min as u32,
                    max: model.item_damage4_max as u32,
                    ty: model.item_damage4_type as u32,
                },
                ItemDamage {
                    min: model.item_damage5_min as u32,
                    max: model.item_damage5_max as u32,
                    ty: model.item_damage5_type as u32,
                },
            ],
            armor: model.armor as u32,
            holy_resistance: model.holy_resistance as u32,
            fire_resistance: model.fire_resistance as u32,
            nature_resistance: model.nature_resistance as u32,
            frost_resistance: model.frost_resistance as u32,
            shadow_resistance: model.shadow_resistance as u32,
            arcane_resistance: model.arcane_resistance as u32,
            delay: model.delay as u32,
            ammo_type: model.ammo_type as u32,
            ranged_mod_range: model.ranged_mod_range,
            spells: [
                if model.spell1_id == 0 {
                    None
                } else {
                    Some(ItemSpell {
                        id: model.spell1_id as u32,
                        trigger: model.spell1_trigger as u32,
                        charges: model.spell1_charges as u32,
                        cooldown: model.spell1_cooldown as u32,
                        category: model.spell1_category as u32,
                        category_cooldown: model.spell1_category_cooldown as u32,
                    })
                },
                if model.spell2_id == 0 {
                    None
                } else {
                    Some(ItemSpell {
                        id: model.spell2_id as u32,
                        trigger: model.spell2_trigger as u32,
                        charges: model.spell2_charges as u32,
                        cooldown: model.spell2_cooldown as u32,
                        category: model.spell2_category as u32,
                        category_cooldown: model.spell2_category_cooldown as u32,
                    })
                },
                if model.spell3_id == 0 {
                    None
                } else {
                    Some(ItemSpell {
                        id: model.spell3_id as u32,
                        trigger: model.spell3_trigger as u32,
                        charges: model.spell3_charges as u32,
                        cooldown: model.spell3_cooldown as u32,
                        category: model.spell3_category as u32,
                        category_cooldown: model.spell3_category_cooldown as u32,
                    })
                },
                if model.spell4_id == 0 {
                    None
                } else {
                    Some(ItemSpell {
                        id: model.spell4_id as u32,
                        trigger: model.spell4_trigger as u32,
                        charges: model.spell4_charges as u32,
                        cooldown: model.spell4_cooldown as u32,
                        category: model.spell4_category as u32,
                        category_cooldown: model.spell4_category_cooldown as u32,
                    })
                },
                if model.spell5_id == 0 {
                    None
                } else {
                    Some(ItemSpell {
                        id: model.spell5_id as u32,
                        trigger: model.spell5_trigger as u32,
                        charges: model.spell5_charges as u32,
                        cooldown: model.spell5_cooldown as u32,
                        category: model.spell5_category as u32,
                        category_cooldown: model.spell5_category_cooldown as u32,
                    })
                },
            ],
            bonding: model.bonding as u32,
            page_text: model.page_text as u32,
            language_id: model.language_id as u32,
            page_material: model.page_material as u32,
            start_quest: model.start_quest as u32,
            lock_id: model.lock_id as u32,
            material: model.material as u32,
            sheath: model.sheath as u32,
            random_property: model.random_property as u32,
            block: model.block as u32,
            item_set: model.item_set as u32,
            max_durability: model.max_durability as u32,
            area: model.area as u32,
            map: model.map as u32,
            bag_family: model.bag_family as u32,
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

pub struct ItemPrototype {
    pub item_id: u32,
    pub class: u32,
    pub sub_class: u32,
    pub name: String,
    pub description: String,
    pub display_info_id: u32,
    pub quality: u32,
    pub flags: ItemFlags,
    pub buy_price: u32,
    pub sell_price: u32,
    pub inventory_type: u32, //TODO
    pub allowable_class: u32,
    pub allowable_race: u32,
    pub item_level: u32,
    pub required_level: u32,
    pub required_skill: u32,
    pub required_skill_rank: u32,
    pub required_spell: u32,
    pub required_honor_rank: u32,
    pub required_city_rank: u32,
    pub required_reputation_faction: u32,
    pub required_reputation_rank: u32,
    pub max_count: u32,
    pub stackable: u32,
    pub container_slots: u32,
    pub item_stats: [ItemStat; 10],
    pub damage: [ItemDamage; 5],

    pub armor: u32,
    pub holy_resistance: u32,
    pub fire_resistance: u32,
    pub nature_resistance: u32,
    pub frost_resistance: u32,
    pub shadow_resistance: u32,
    pub arcane_resistance: u32,

    pub delay: u32,
    pub ammo_type: u32,
    pub ranged_mod_range: f32,

    pub spells: [Option<ItemSpell>; 5],

    pub bonding: u32,
    pub page_text: u32,
    pub language_id: u32,
    pub page_material: u32,
    pub start_quest: u32,
    pub lock_id: u32,
    pub material: u32,
    pub sheath: u32,
    pub random_property: u32,
    pub block: u32,
    pub item_set: u32,
    pub max_durability: u32,
    pub area: u32,
    pub map: u32,
    pub bag_family: u32,
}
