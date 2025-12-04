use std::num::NonZeroU32;

use bit_vec::BitVec;
use common::guid::{self, AnyGuid, Guid, GuidType};
use gameobjects::{
    item::ItemFields, object::ObjectFields, player::VisibleItemFields,
    tracked_field::ClientUpdatable,
};
use log::error;
use packets::update_data::{MovementUpdate, PossibleUpdate, UpdateData, ValuesUpdate};
use sqlx::{Pool, Sqlite};

use crate::game_data::GameDataAccessor;

pub struct Item {
    pub object_fields: ObjectFields<guid::Item>,
    pub item_fields: ItemFields,
}

impl Item {
    pub async fn load_from_db(
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        id: u32,
    ) -> Result<Option<Self>, sqlx::Error> {
        let model = match sqlx::query!("SELECT * FROM item WHERE id = ?", id)
            .fetch_one(db)
            .await
        {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Ok(None),
            Err(e) => return Err(e),
        };

        let Some(prototype) = game_data_accessor
            .get_item_prototype(model.prototype_id as u32)
            .await?
        else {
            error!(
                "Tried to load an item without a prototype in the database! (prototype_id: {})",
                model.prototype_id
            );
            return Ok(None);
        };

        Ok(Some(Self {
            object_fields: ObjectFields {
                guid: Guid::from_u32(NonZeroU32::new(model.id as u32).unwrap()).into(),
                object_type: gameobjects::object::TypeBitField::new()
                    .with_object(true)
                    .with_item(true)
                    .into(),
                entry: (model.prototype_id as u32).into(),
                scale_x: 1.0.into(),
                _padding: 0.into(),
            },
            item_fields: ItemFields {
                owner: model
                    .owner
                    .map(|v| NonZeroU32::new(v as u32))
                    .flatten()
                    .map(|v| Guid::from_u32(v))
                    .into(),
                contained_in: None.into(),
                creator: model
                    .creator
                    .map(|v| NonZeroU32::new(v as u32))
                    .flatten()
                    .map(|v| Guid::from_u32(v))
                    .into(),
                gift_creator: model
                    .gift_creator
                    .map(|v| NonZeroU32::new(v as u32))
                    .flatten()
                    .map(|v| Guid::from_u32(v))
                    .into(),
                stack_count: (model.stack_count as u32).into(),
                expires_in: model
                    .expires_in
                    .map(|v| NonZeroU32::new(v as u32))
                    .flatten()
                    .into(),
                spell_charges: [
                    (model.spell1_charges.unwrap_or(0) as u32).into(),
                    (model.spell2_charges.unwrap_or(0) as u32).into(),
                    (model.spell3_charges.unwrap_or(0) as u32).into(),
                    (model.spell4_charges.unwrap_or(0) as u32).into(),
                    (model.spell5_charges.unwrap_or(0) as u32).into(),
                ],
                flags: gameobjects::item::ItemFlags::new().into(),
                enchantments: [
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment1_id as u32,
                        duration: model.enchantment1_duration as u32,
                        charges: model.enchantment1_charges as u32,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment2_id as u32,
                        duration: model.enchantment2_duration as u32,
                        charges: model.enchantment2_charges as u32,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment3_id as u32,
                        duration: model.enchantment3_duration as u32,
                        charges: model.enchantment3_charges as u32,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment4_id as u32,
                        duration: model.enchantment4_duration as u32,
                        charges: model.enchantment4_charges as u32,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment5_id as u32,
                        duration: model.enchantment5_duration as u32,
                        charges: model.enchantment5_charges as u32,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment6_id as u32,
                        duration: model.enchantment6_duration as u32,
                        charges: model.enchantment6_charges as u32,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment7_id as u32,
                        duration: model.enchantment7_duration as u32,
                        charges: model.enchantment7_charges as u32,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment8_id as u32,
                        duration: model.enchantment8_duration as u32,
                        charges: model.enchantment8_charges as u32,
                    }
                    .into(),
                    gameobjects::item::ItemEnchantment {
                        id: model.enchantment9_id as u32,
                        duration: model.enchantment9_duration as u32,
                        charges: model.enchantment9_charges as u32,
                    }
                    .into(),
                ],
                property_seed: 0.into(),
                random_properties_id: (model.random_properties_id as u32).into(),
                item_text_id: (model.item_text_id as u32).into(),
                durability: (model.durability as u32).into(),
                max_durability: prototype.max_durability.into(),
                _padding: 0.into(),
            },
        }))
    }

    pub fn build_create_update_block(&self) -> UpdateData {
        let mut mask_blocks = BitVec::new();
        let mut values_blocks = Vec::new();

        self.object_fields
            .write_full_update_block(&mut mask_blocks, &mut values_blocks);
        self.item_fields
            .write_full_update_block(&mut mask_blocks, &mut values_blocks);

        UpdateData::CreateNewObject {
            guid: AnyGuid::Item(*self.object_fields.guid.get()),
            movement: MovementUpdate {
                is_self_update: false,
                position: None,
                high_guid: Some(guid::Item::get_prefix() as u32),
                is_update_all: true,
                full_guid: PossibleUpdate::NoUpdate,
                transport_time_millis: None,
            },
            values: ValuesUpdate {
                mask_blocks: mask_blocks,
                values_blocks: values_blocks,
            },
        }
    }

    pub fn get_visible_item_fields(&self) -> VisibleItemFields {
        VisibleItemFields {
            creator: self.item_fields.creator.get().clone(),
            item_id: *self.object_fields.entry.get(),
            enchantment_ids: [
                self.item_fields.enchantments[0].get().id,
                self.item_fields.enchantments[1].get().id,
            ],
            unkn: [0; 5],
            random_properties_id: *self.item_fields.random_properties_id.get(),
            property_seed: *self.item_fields.property_seed.get(),
        }
    }
}
