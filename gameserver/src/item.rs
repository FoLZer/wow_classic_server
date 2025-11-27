use std::num::NonZeroU32;

use common::guid::{self, Guid};
use gameobjects::{item::ItemFields, object::ObjectFields};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection};

use crate::game_data::ItemPrototype;

pub struct Item {
    pub object_fields: ObjectFields<guid::Item>,
    pub item_fields: ItemFields,

    pub display_id: u32,
    pub inventory_type: u8
}

impl Item {
    /// Model must have prototype loaded
    pub fn from_model(model: &gameserver_entity::item::ModelEx) -> Self {
        let prototype = model.prototype.as_ref().unwrap();

        Self {
            object_fields: ObjectFields {
                guid: Guid::from_u32(NonZeroU32::new(model.id).unwrap()).into(),
                object_type: gameobjects::object::TypeBitField::new()
                    .with_object(true)
                    .with_item(true)
                    .into(),
                entry: model.item_prototype_id.into(),
                scale_x: 1.0.into(),
                _padding: 0.into(),
            },
            item_fields: ItemFields {
                owner: NonZeroU32::new(model.owner_id)
                    .map(|v| Guid::from_u32(v))
                    .into(),
                contained_in: None.into(),
                creator: model
                    .creator_id
                    .map(|v| NonZeroU32::new(v))
                    .flatten()
                    .map(|v| Guid::from_u32(v))
                    .into(),
                gift_creator: model
                    .gift_creator_id
                    .map(|v| NonZeroU32::new(v))
                    .flatten()
                    .map(|v| Guid::from_u32(v))
                    .into(),
                stack_count: model.stack_count.into(),
                expires_in: model
                    .expires_in
                    .map(|v| NonZeroU32::new(v))
                    .flatten()
                    .into(),
                spell_charges: [
                    model.spell1_charges.unwrap_or(0).into(),
                    model.spell2_charges.unwrap_or(0).into(),
                    model.spell3_charges.unwrap_or(0).into(),
                    model.spell4_charges.unwrap_or(0).into(),
                    model.spell5_charges.unwrap_or(0).into(),
                ],
                flags: gameobjects::item::ItemFlags::new().into(),
                enchantments: [
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment1_id,
                        duration: model.enchantment1_duration,
                        charges: model.enchantment1_charges,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment2_id,
                        duration: model.enchantment2_duration,
                        charges: model.enchantment2_charges,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment3_id,
                        duration: model.enchantment3_duration,
                        charges: model.enchantment3_charges,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment4_id,
                        duration: model.enchantment4_duration,
                        charges: model.enchantment4_charges,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment5_id,
                        duration: model.enchantment5_duration,
                        charges: model.enchantment5_charges,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment6_id,
                        duration: model.enchantment6_duration,
                        charges: model.enchantment6_charges,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment7_id,
                        duration: model.enchantment7_duration,
                        charges: model.enchantment7_charges,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment8_id,
                        duration: model.enchantment8_duration,
                        charges: model.enchantment8_charges,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment9_id,
                        duration: model.enchantment9_duration,
                        charges: model.enchantment9_charges,
                    }
                    .into(),
                ],
                property_seed: 0.into(),
                random_properties_id: model.random_properties_id.into(),
                item_text_id: model.item_text_id.into(),
                durability: model.durability.into(),
                max_durability: prototype.max_durability.into(),
                _padding: 0.into(),
            },

            display_id: prototype.display_id,
            inventory_type: prototype.inventory_type
        }
    }

    pub async fn create_for_new_character(
        db: &DatabaseConnection,
        prototype: &ItemPrototype,
        owner_id: u32,
    ) -> Result<u32, sea_orm::DbErr> {
        gameserver_entity::item::ActiveModel {
            id: ActiveValue::NotSet,
            item_prototype_id: ActiveValue::Set(prototype.item_id),
            owner_id: ActiveValue::Set(owner_id),
            creator_id: ActiveValue::Set(None),
            gift_creator_id: ActiveValue::Set(None),
            stack_count: ActiveValue::Set(1),
            expires_in: ActiveValue::Set(None), //TODO
            spell1_charges: ActiveValue::Set(prototype.spells[0].as_ref().map(|v| v.charges)),
            spell2_charges: ActiveValue::Set(prototype.spells[1].as_ref().map(|v| v.charges)),
            spell3_charges: ActiveValue::Set(prototype.spells[2].as_ref().map(|v| v.charges)),
            spell4_charges: ActiveValue::Set(prototype.spells[3].as_ref().map(|v| v.charges)),
            spell5_charges: ActiveValue::Set(prototype.spells[4].as_ref().map(|v| v.charges)),
            is_binded: ActiveValue::Set(false),
            is_unlocked: ActiveValue::Set(false),
            is_wrapped: ActiveValue::Set(false),
            is_readable: ActiveValue::Set(false),
            random_properties_id: ActiveValue::Set(0),
            item_text_id: ActiveValue::Set(0),
            durability: ActiveValue::Set(prototype.max_durability),
            enchantment1_id: ActiveValue::Set(0),
            enchantment1_duration: ActiveValue::Set(0),
            enchantment1_charges: ActiveValue::Set(0),
            enchantment2_id: ActiveValue::Set(0),
            enchantment2_duration: ActiveValue::Set(0),
            enchantment2_charges: ActiveValue::Set(0),
            enchantment3_id: ActiveValue::Set(0),
            enchantment3_duration: ActiveValue::Set(0),
            enchantment3_charges: ActiveValue::Set(0),
            enchantment4_id: ActiveValue::Set(0),
            enchantment4_duration: ActiveValue::Set(0),
            enchantment4_charges: ActiveValue::Set(0),
            enchantment5_id: ActiveValue::Set(0),
            enchantment5_duration: ActiveValue::Set(0),
            enchantment5_charges: ActiveValue::Set(0),
            enchantment6_id: ActiveValue::Set(0),
            enchantment6_duration: ActiveValue::Set(0),
            enchantment6_charges: ActiveValue::Set(0),
            enchantment7_id: ActiveValue::Set(0),
            enchantment7_duration: ActiveValue::Set(0),
            enchantment7_charges: ActiveValue::Set(0),
            enchantment8_id: ActiveValue::Set(0),
            enchantment8_duration: ActiveValue::Set(0),
            enchantment8_charges: ActiveValue::Set(0),
            enchantment9_id: ActiveValue::Set(0),
            enchantment9_duration: ActiveValue::Set(0),
            enchantment9_charges: ActiveValue::Set(0),
        }
        .insert(db)
        .await
        .map(|v| v.id)
    }
}
