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
            // TODO: this is not what we need to use but I haven't done the research needed to figure these values out yet :-)
            equipment_head_display_id: ActiveValue::Set(0),
            equipment_head_inventory_type: ActiveValue::Set(0),
            equipment_neck_display_id: ActiveValue::Set(0),
            equipment_neck_inventory_type: ActiveValue::Set(0),
            equipment_shoulders_display_id: ActiveValue::Set(0),
            equipment_shoulders_inventory_type: ActiveValue::Set(0),
            equipment_body_display_id: ActiveValue::Set(0),
            equipment_body_inventory_type: ActiveValue::Set(0),
            equipment_chest_display_id: ActiveValue::Set(0),
            equipment_chest_inventory_type: ActiveValue::Set(0),
            equipment_waist_display_id: ActiveValue::Set(0),
            equipment_waist_inventory_type: ActiveValue::Set(0),
            equipment_legs_display_id: ActiveValue::Set(0),
            equipment_legs_inventory_type: ActiveValue::Set(0),
            equipment_feet_display_id: ActiveValue::Set(0),
            equipment_feet_inventory_type: ActiveValue::Set(0),
            equipment_wrists_display_id: ActiveValue::Set(0),
            equipment_wrists_inventory_type: ActiveValue::Set(0),
            equipment_hands_display_id: ActiveValue::Set(0),
            equipment_hands_inventory_type: ActiveValue::Set(0),
            equipment_finger1_display_id: ActiveValue::Set(0),
            equipment_finger1_inventory_type: ActiveValue::Set(0),
            equipment_finger2_display_id: ActiveValue::Set(0),
            equipment_finger2_inventory_type: ActiveValue::Set(0),
            equipment_trinket1_display_id: ActiveValue::Set(0),
            equipment_trinket1_inventory_type: ActiveValue::Set(0),
            equipment_trinket2_display_id: ActiveValue::Set(0),
            equipment_trinket2_inventory_type: ActiveValue::Set(0),
            equipment_back_display_id: ActiveValue::Set(0),
            equipment_back_inventory_type: ActiveValue::Set(0),
            equipment_mainhand_display_id: ActiveValue::Set(0),
            equipment_mainhand_inventory_type: ActiveValue::Set(0),
            equipment_offhand_display_id: ActiveValue::Set(0),
            equipment_offhand_inventory_type: ActiveValue::Set(0),
            equipment_ranged_display_id: ActiveValue::Set(0),
            equipment_ranged_inventory_type: ActiveValue::Set(0),
            equipment_tabard_display_id: ActiveValue::Set(0),
            equipment_tabard_inventory_type: ActiveValue::Set(0),
            other_equipment_table_id: ActiveValue::Set(0),
        }
        .insert(&txn)
        .await?;

        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
