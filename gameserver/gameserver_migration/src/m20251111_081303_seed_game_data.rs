use sea_orm_migration::{
    prelude::*,
    schema::*,
    sea_orm::{ActiveModelTrait, ActiveValue, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let txn = db.begin().await?;

        // Races
        gameserver_entity::race::ActiveModel {
            id: ActiveValue::Set(1),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::race::ActiveModel {
            id: ActiveValue::Set(2),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::race::ActiveModel {
            id: ActiveValue::Set(3),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::race::ActiveModel {
            id: ActiveValue::Set(4),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::race::ActiveModel {
            id: ActiveValue::Set(5),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::race::ActiveModel {
            id: ActiveValue::Set(6),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::race::ActiveModel {
            id: ActiveValue::Set(7),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::race::ActiveModel {
            id: ActiveValue::Set(8),
        }
        .insert(&txn)
        .await?;

        // Classes
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(1),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(2),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(3),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(4),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(5),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(6),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(7),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(8),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(9),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(10),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::class::ActiveModel {
            id: ActiveValue::Set(11),
        }
        .insert(&txn)
        .await?;

        // Genders
        gameserver_entity::gender::ActiveModel {
            id: ActiveValue::Set(0),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::gender::ActiveModel {
            id: ActiveValue::Set(1),
        }
        .insert(&txn)
        .await?;

        // Player Start Data
        gameserver_entity::player_start_data::ActiveModel {
            race: ActiveValue::Set(1),
            class: ActiveValue::Set(1),
            area_id: ActiveValue::Set(12),
            map_id: ActiveValue::Set(0),
            position_x: ActiveValue::Set(-8949.95),
            position_y: ActiveValue::Set(-132.493),
            position_z: ActiveValue::Set(83.5312),
            orientation: ActiveValue::Set(0.0),
            level: ActiveValue::Set(1),
            equipment_head_id: ActiveValue::Set(None),
            equipment_neck_id: ActiveValue::Set(None),
            equipment_shoulders_id: ActiveValue::Set(None),
            equipment_body_id: ActiveValue::Set(None),
            equipment_chest_id: ActiveValue::Set(None),
            equipment_waist_id: ActiveValue::Set(None),
            equipment_legs_id: ActiveValue::Set(None),
            equipment_feet_id: ActiveValue::Set(None),
            equipment_wrists_id: ActiveValue::Set(None),
            equipment_hands_id: ActiveValue::Set(None),
            equipment_finger1_id: ActiveValue::Set(None),
            equipment_finger2_id: ActiveValue::Set(None),
            equipment_trinket1_id: ActiveValue::Set(None),
            equipment_trinket2_id: ActiveValue::Set(None),
            equipment_back_id: ActiveValue::Set(None),
            equipment_mainhand_id: ActiveValue::Set(None),
            equipment_offhand_id: ActiveValue::Set(None),
            equipment_ranged_id: ActiveValue::Set(None),
            equipment_tabard_id: ActiveValue::Set(None),
            other_equipment_table_id: ActiveValue::Set(0),
        }
        .insert(&txn)
        .await?;

        // Character Display Ids
        gameserver_entity::character_display_id::ActiveModel {
            race: ActiveValue::Set(1),
            gender: ActiveValue::Set(0),
            display_id: ActiveValue::Set(49),
        }
        .insert(&txn)
        .await?;
        gameserver_entity::character_display_id::ActiveModel {
            race: ActiveValue::Set(1),
            gender: ActiveValue::Set(1),
            display_id: ActiveValue::Set(50),
        }
        .insert(&txn)
        .await?;

        // Item Prototypes
        gameserver_entity::item_prototype::ActiveModel {
            id: ActiveValue::Set(789),
            class: ActiveValue::Set(2),
            sub_class: ActiveValue::Set(4),
            name: ActiveValue::Set("Stout Battlehammer".to_string()),
            description: ActiveValue::Set("".to_string()),
            display_id: ActiveValue::Set(19699),
            quality: ActiveValue::Set(2),
            buy_price: ActiveValue::Set(9847),
            sell_price: ActiveValue::Set(1969),
            inventory_type: ActiveValue::Set(21),
            allowable_class: ActiveValue::Set(u32::MAX as i32),
            allowable_race: ActiveValue::Set(u32::MAX as i32),
            item_level: ActiveValue::Set(22),
            required_level: ActiveValue::Set(17),
            required_skill: ActiveValue::Set(0),
            required_skill_rank: ActiveValue::Set(0),
            required_spell: ActiveValue::Set(0),
            required_honor_rank: ActiveValue::Set(0),
            required_city_rank: ActiveValue::Set(0),
            required_reputation_faction: ActiveValue::Set(0),
            required_reputation_rank: ActiveValue::Set(0),
            max_count: ActiveValue::Set(0),
            stackable: ActiveValue::Set(1),
            container_slots: ActiveValue::Set(0),
            item_stat1_type: ActiveValue::Set(0),
            item_stat1_value: ActiveValue::Set(0),
            item_stat2_type: ActiveValue::Set(0),
            item_stat2_value: ActiveValue::Set(0),
            item_stat3_type: ActiveValue::Set(0),
            item_stat3_value: ActiveValue::Set(0),
            item_stat4_type: ActiveValue::Set(0),
            item_stat4_value: ActiveValue::Set(0),
            item_stat5_type: ActiveValue::Set(0),
            item_stat5_value: ActiveValue::Set(0),
            item_stat6_type: ActiveValue::Set(0),
            item_stat6_value: ActiveValue::Set(0),
            item_stat7_type: ActiveValue::Set(0),
            item_stat7_value: ActiveValue::Set(0),
            item_stat8_type: ActiveValue::Set(0),
            item_stat8_value: ActiveValue::Set(0),
            item_stat9_type: ActiveValue::Set(0),
            item_stat9_value: ActiveValue::Set(0),
            item_stat10_type: ActiveValue::Set(0),
            item_stat10_value: ActiveValue::Set(0),
            item_damage1_min: ActiveValue::Set(17),
            item_damage1_max: ActiveValue::Set(33),
            item_damage1_type: ActiveValue::Set(0),
            item_damage2_min: ActiveValue::Set(0),
            item_damage2_max: ActiveValue::Set(0),
            item_damage2_type: ActiveValue::Set(0),
            item_damage3_min: ActiveValue::Set(0),
            item_damage3_max: ActiveValue::Set(0),
            item_damage3_type: ActiveValue::Set(0),
            item_damage4_min: ActiveValue::Set(0),
            item_damage4_max: ActiveValue::Set(0),
            item_damage4_type: ActiveValue::Set(0),
            item_damage5_min: ActiveValue::Set(0),
            item_damage5_max: ActiveValue::Set(0),
            item_damage5_type: ActiveValue::Set(0),
            armor: ActiveValue::Set(0),
            holy_resistance: ActiveValue::Set(0),
            fire_resistance: ActiveValue::Set(0),
            nature_resistance: ActiveValue::Set(0),
            frost_resistance: ActiveValue::Set(0),
            shadow_resistance: ActiveValue::Set(0),
            arcane_resistance: ActiveValue::Set(0),
            delay: ActiveValue::Set(2300),
            ammo_type: ActiveValue::Set(0),
            ranged_mod_range: ActiveValue::Set(0.0),
            spell1_id: ActiveValue::Set(0),
            spell1_trigger: ActiveValue::Set(0),
            spell1_charges: ActiveValue::Set(0),
            spell1_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell1_category: ActiveValue::Set(0),
            spell1_category_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell2_id: ActiveValue::Set(0),
            spell2_trigger: ActiveValue::Set(0),
            spell2_charges: ActiveValue::Set(0),
            spell2_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell2_category: ActiveValue::Set(0),
            spell2_category_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell3_id: ActiveValue::Set(0),
            spell3_trigger: ActiveValue::Set(0),
            spell3_charges: ActiveValue::Set(0),
            spell3_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell3_category: ActiveValue::Set(0),
            spell3_category_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell4_id: ActiveValue::Set(0),
            spell4_trigger: ActiveValue::Set(0),
            spell4_charges: ActiveValue::Set(0),
            spell4_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell4_category: ActiveValue::Set(0),
            spell4_category_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell5_id: ActiveValue::Set(0),
            spell5_trigger: ActiveValue::Set(0),
            spell5_charges: ActiveValue::Set(0),
            spell5_cooldown: ActiveValue::Set(u32::MAX as i32),
            spell5_category: ActiveValue::Set(0),
            spell5_category_cooldown: ActiveValue::Set(u32::MAX as i32),
            bonding: ActiveValue::Set(2),
            page_text: ActiveValue::Set(0),
            language_id: ActiveValue::Set(0),
            page_material: ActiveValue::Set(0),
            start_quest: ActiveValue::Set(0),
            lock_id: ActiveValue::Set(0),
            material: ActiveValue::Set(2),
            sheath: ActiveValue::Set(3),
            random_property: ActiveValue::Set(5197),
            block: ActiveValue::Set(0),
            item_set: ActiveValue::Set(0),
            max_durability: ActiveValue::Set(60),
            area: ActiveValue::Set(0),
            map: ActiveValue::Set(0),
            bag_family: ActiveValue::Set(0),
        }
        .insert(&txn)
        .await?;

        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
