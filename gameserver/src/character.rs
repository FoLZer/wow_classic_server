use std::{num::NonZeroU32, sync::Arc};

use common::guid::{self, Guid};
use gameobjects::{
    object::{ObjectFields, TypeBitField},
    player::{
        PlayerFieldBytes2, PlayerFieldBytes3, PlayerFields, QuestLogFields, VisibleItemFields,
    },
    unit::{
        SheathState, StandStateType, UnitFieldBytes1, UnitFieldBytes2, UnitFieldBytes2Flags,
        UnitFieldBytes3, UnitFieldBytes3Flags, UnitFields,
    },
};
use packets::update_data::UpdateData;
use sqlx::{Pool, Sqlite};
use tokio::{net::tcp::OwnedWriteHalf, sync::Mutex};

use crate::{game_data::GameDataAccessor, item::Item};

pub struct Character {
    pub map_id: u32,
    pub position: (f32, f32, f32),
    pub orientation: f32,

    pub account_id: u32,
    pub session_key: [u8; 40],
    pub stream_tx: Arc<Mutex<OwnedWriteHalf>>,

    items: CharacterItems,

    pub object_fields: ObjectFields<guid::Player>,
    pub unit_fields: UnitFields,
    pub player_fields: PlayerFields,
}

impl Character {
    pub async fn load_from_db(
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        guid: Guid<guid::Player>,
        account_id: u32,
        stream_tx: OwnedWriteHalf,
        session_key: [u8; 40],
    ) -> Result<Self, (OwnedWriteHalf, sqlx::Error)> {
        let character_id = guid.get_u32();
        let model = match sqlx::query!(
            "SELECT * FROM character WHERE id = ? AND account_id = ?",
            character_id,
            account_id
        )
        .fetch_one(db)
        .await
        {
            Ok(v) => v,
            Err(e) => return Err((stream_tx, e)),
        };

        Ok(Self {
            map_id: model.map as u32,
            position: (
                model.position_x as f32,
                model.position_y as f32,
                model.position_z as f32,
            ),
            orientation: model.orientation as f32,

            account_id: model.account_id as u32,
            session_key,

            items: CharacterItems {
                equipment: EquipmentItems {
                    head: if let Some(id) = model.equipment_head_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    neck: if let Some(id) = model.equipment_neck_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    shoulders: if let Some(id) = model.equipment_shoulders_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    body: if let Some(id) = model.equipment_body_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    chest: if let Some(id) = model.equipment_chest_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    waist: if let Some(id) = model.equipment_waist_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    legs: if let Some(id) = model.equipment_legs_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    feet: if let Some(id) = model.equipment_feet_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    wrists: if let Some(id) = model.equipment_wrists_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    hands: if let Some(id) = model.equipment_hands_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    finger1: if let Some(id) = model.equipment_finger1_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    finger2: if let Some(id) = model.equipment_finger2_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    trinket1: if let Some(id) = model.equipment_trinket1_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    trinket2: if let Some(id) = model.equipment_trinket2_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    back: if let Some(id) = model.equipment_back_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    mainhand: if let Some(id) = model.equipment_mainhand_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    offhand: if let Some(id) = model.equipment_offhand_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    ranged: if let Some(id) = model.equipment_ranged_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                    tabard: if let Some(id) = model.equipment_tabard_id {
                        match Item::load_from_db(game_data_accessor, db, id as u32).await {
                            Ok(v) => v,
                            Err(e) => return Err((stream_tx, e)),
                        }
                    } else {
                        None
                    },
                },
                bags: [const { None }; 4],
                main_backpack: [const { None }; 16],
                bank: [const { None }; 28],
                bank_bags: [const { None }; 7],
                vendor_buyback: [const { None }; 12],
                keyring: [const { None }; 12],
            },

            stream_tx: Arc::new(Mutex::new(stream_tx)),

            //TODO: fill in these values, obviously
            object_fields: ObjectFields {
                guid: Guid::from_u32(NonZeroU32::new(model.id as u32).unwrap()).into(), // Theoretically this should never be a problem but someone might be able to add a character with id 0 into the database,
                object_type: TypeBitField::new()
                    .with_player(true)
                    .with_unit(true)
                    .with_object(true)
                    .into(),
                entry: 0.into(),
                scale_x: 1.0.into(),
                _padding: 0.into(),
            },
            unit_fields: UnitFields {
                charm: None.into(),
                summon: None.into(),
                charmed_by: None.into(),
                summoned_by: None.into(),
                created_by: None.into(),
                target: None.into(),
                persuaded: None.into(),
                channel_object: None.into(),
                health: 100.into(),
                powers: [0.into(); 5],
                max_health: 500.into(),
                max_powers: [0.into(); 5],
                level: 1.into(),
                faction_template: 1.into(),
                bytes_1: UnitFieldBytes1::new()
                    .with_race(model.race as u8)
                    .with_class(model.class as u8)
                    .with_gender(model.gender as u8)
                    .with_power(0)
                    .into(),
                virtual_item_slot_displays: [0.into(); 3],
                virtual_item_infos: [0.into(); 6],
                flags: 0.into(),
                aura: [0.into(); 48],
                aura_flags: [0.into(); 6],
                aura_levels: [0.into(); 12],
                aura_applications: [0.into(); 12],
                aura_state: 0.into(),
                base_attack_time: 1.into(),
                offhand_attack_time: 2.into(),
                ranged_attack_time: 3.into(),
                bounding_radius: 4.into(),
                combat_reach: 5.into(),
                display_id: (model.display_id as u32).into(),
                native_display_id: 0.into(),
                mount_display_id: 0.into(),
                min_damage: 70.into(),
                max_damage: 80.into(),
                min_offhand_damage: 90.into(),
                max_offhand_damage: 130.into(),
                bytes_2: UnitFieldBytes2::new()
                    .with_stand_state(StandStateType::Stand)
                    .with_loyalty_level(0)
                    .with_free_talent_points(0)
                    .with_flags(UnitFieldBytes2Flags::new())
                    .into(),
                pet_number: 0.into(),
                pet_name_timestamp: 0.into(),
                pet_experience: 0.into(),
                pet_next_level_exp: 0.into(),
                dynamic_flags: 0.into(),
                channel_spell: 0.into(),
                mod_cast_speed: 1.into(),
                created_by_spell: 0.into(),
                npc_flags: 0.into(),
                npc_emote_state: 0.into(),
                training_points: 0.into(),
                strength: 1.into(),
                agility: 1.into(),
                stamina: 1.into(),
                intellect: 1.into(),
                spirit: 1.into(),
                normal_resistance: 0.into(),
                holy_resistance: 0.into(),
                fire_resistance: 0.into(),
                nature_resistance: 0.into(),
                frost_resistance: 0.into(),
                shadow_resistance: 0.into(),
                arcane_resistance: 0.into(),
                base_mana: 100.into(),
                base_health: 100.into(),
                bytes_3: UnitFieldBytes3::new()
                    .with_sheath_state(SheathState::Unarmed)
                    .with_flags(UnitFieldBytes3Flags::new())
                    .into(),
                attack_power: 1.into(),
                attack_power_mods: 0.into(),
                attack_power_multiplier: 1.into(),
                ranged_attack_power: 0.into(),
                ranged_attack_power_mods: 0.into(),
                ranged_attack_power_multiplier: 0.into(),
                min_ranged_damage: 0.into(),
                max_ranged_damage: 0.into(),
                power_cost_modifiers: [0.into(); 7],
                power_cost_multipliers: [1.into(); 7],
                _padding: 0.into(),
            },
            player_fields: PlayerFields {
                duel_arbiter: None.into(),
                flags: 0.into(),
                guild_id: 0.into(),
                guild_rank: 0.into(),
                _unknown_bytes_1: 0.into(),
                bytes_2: PlayerFieldBytes2::new()
                    .with_facial_hair(model.facial_hair as u8)
                    .with_bank_bag_slots(0)
                    .with_rested_state(0)
                    .into(),
                bytes_3: PlayerFieldBytes3::new().with_gender(model.gender).into(),
                duel_team: 0.into(),
                guild_timestamp: 0.into(),
                quest_log: [QuestLogFields {
                    log_1: 0,
                    log_2: 0,
                    log_3: 0,
                }
                .into(); 20],
                visible_items: std::array::from_fn(|_| {
                    VisibleItemFields {
                        creator: None,
                        unkn: [0; 8],
                        properties: 0,
                        _padding: 0,
                    }
                    .into()
                }),

                equipment_slots: [
                    model
                        .equipment_head_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_neck_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_shoulders_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_body_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_chest_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_waist_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_legs_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_feet_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_wrists_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_hands_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_finger1_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_finger2_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_trinket1_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_trinket2_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_back_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_mainhand_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_offhand_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_ranged_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                    model
                        .equipment_tabard_id
                        .map(|v| Guid::from_u32(NonZeroU32::new(v as u32).unwrap()))
                        .into(),
                ],
                bag_slots: [None.into(); 4],
                main_backpack_slots: [None.into(); 16],
                bank_slots: [None.into(); 28],
                bank_bag_slots: [None.into(); 7],
                vendor_buyback_slots: [None.into(); 12],
                keyring_slots: [None.into(); 12],
                _unkn: [None.into(); 15],

                far_sight: None.into(),
                field_combo_target: None.into(),
                xp: 0.into(),
                next_level_xp: 100.into(),
                skill_infos: [0.into(); 384],
                character_points_1: 0.into(),
                character_points_2: 0.into(),
                track_creatures: 0.into(),
                track_resources: 0.into(),
                block_percentage: 0.into(),
                dodge_percentage: 0.into(),
                parry_percentage: 0.into(),
                crit_percentage: 0.into(),
                ranged_crit_percentage: 0.into(),
                explored_zones: [0.into(); 64],
                rest_state_experience: 0.into(),
                coinage: 2.into(),
                pos_stats: [0.into(); 5],
                neg_stats: [0.into(); 5],
                resistance_buff_mods_positive: [0.into(); 7],
                resistance_buff_mods_negative: [0.into(); 7],
                mod_damage_done_pos: [0.into(); 7],
                mod_damage_done_neg: [0.into(); 7],
                mod_damage_done_pct: [0.into(); 7],
                _unknown_bytes_4: 0.into(),
                ammo_id: 0.into(),
                self_res_spell: 0.into(),
                pvp_medals: 0.into(),
                buyback_prices: [0.into(); 12],
                buyback_timestamps: [0.into(); 12],
                session_kills: 0.into(),
                yesterday_kills: 0.into(),
                last_week_kills: 0.into(),
                this_week_kills: 0.into(),
                this_week_contribution: 0.into(),
                lifetime_honorable_kills: 0.into(),
                lifetime_dishonorable_kills: 0.into(),
                yesterday_contribution: 0.into(),
                last_week_contribution: 0.into(),
                last_week_rank: 0.into(),
                _unknown_bytes_5: 0.into(),
                watched_faction_index: u32::MAX.into(),
                combat_ratings: [0.into(); 20],
            },
        })
    }

    pub async fn create_new_character(
        packet: &packets::client::CMSG_CHAR_CREATE,
        account_id: u32,
        db: &Pool<Sqlite>,
        game_data_accessor: &GameDataAccessor,
    ) -> Result<(), CharacterCreateError> {
        let race = match game_data_accessor.validate_race(packet.race).await {
            Ok(Some(v)) => v,
            Ok(None) => return Err(CharacterCreateError::InvalidRace),
            Err(e) => return Err(CharacterCreateError::Database(e)),
        };
        let class = match game_data_accessor.validate_class(packet.class).await {
            Ok(Some(v)) => v,
            Ok(None) => return Err(CharacterCreateError::InvalidClass),
            Err(e) => return Err(CharacterCreateError::Database(e)),
        };
        let gender = match packet.gender {
            0 => false,
            1 => true,
            _ => return Err(CharacterCreateError::InvalidGender),
        };
        //TODO: all the validate_ function calls must be joined and done in parallel
        //TODO: validate name
        let name = packet.character_name.to_string_lossy().to_string();
        //TODO: validate skin, face, hairstyle, etc.

        let start_char_info = match game_data_accessor
            .get_character_start_data(race, class)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => return Err(CharacterCreateError::InvalidRaceClassCombination),
            Err(e) => return Err(CharacterCreateError::Database(e)),
        };

        let display_id = match game_data_accessor
            .get_display_id_for_race_gender(race, gender)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => return Err(CharacterCreateError::DisplayIdNotFound),
            Err(e) => return Err(CharacterCreateError::Database(e)),
        };

        let equipment_head_id = if let Some(proto) = start_char_info.start_equipment.head {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_neck_id = if let Some(proto) = start_char_info.start_equipment.neck {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_shoulders_id = if let Some(proto) = start_char_info.start_equipment.shoulders
        {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_body_id = if let Some(proto) = start_char_info.start_equipment.body {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_chest_id = if let Some(proto) = start_char_info.start_equipment.chest {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_waist_id = if let Some(proto) = start_char_info.start_equipment.waist {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_legs_id = if let Some(proto) = start_char_info.start_equipment.legs {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_feet_id = if let Some(proto) = start_char_info.start_equipment.feet {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_wrists_id = if let Some(proto) = start_char_info.start_equipment.wrists {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_hands_id = if let Some(proto) = start_char_info.start_equipment.hands {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_finger1_id = if let Some(proto) = start_char_info.start_equipment.finger1 {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_finger2_id = if let Some(proto) = start_char_info.start_equipment.finger2 {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_trinket1_id = if let Some(proto) = start_char_info.start_equipment.trinket1 {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_trinket2_id = if let Some(proto) = start_char_info.start_equipment.trinket2 {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_back_id = if let Some(proto) = start_char_info.start_equipment.back {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_mainhand_id = if let Some(proto) = start_char_info.start_equipment.mainhand {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_offhand_id = if let Some(proto) = start_char_info.start_equipment.offhand {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_ranged_id = if let Some(proto) = start_char_info.start_equipment.ranged {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };
        let equipment_tabard_id = if let Some(proto) = start_char_info.start_equipment.tabard {
            Some(
                proto
                    .create_item(db, None, None, None, proto.max_count)
                    .await
                    .map_err(|e| CharacterCreateError::Database(e))?,
            )
        } else {
            None
        };

        let race = race.get();
        let class = class.get();
        sqlx::query!(
            "INSERT INTO character(
                account_id, name, race, class, gender, skin,
                face, hair_style, hair_color, facial_hair, level,
                area, map, position_x, position_y, position_z,
                orientation, first_login, display_id,
                equipment_head_id, equipment_neck_id, equipment_shoulders_id,
                equipment_body_id, equipment_chest_id, equipment_waist_id,
                equipment_legs_id, equipment_feet_id, equipment_wrists_id,
                equipment_hands_id, equipment_finger1_id, equipment_finger2_id,
                equipment_trinket1_id, equipment_trinket2_id, equipment_back_id,
                equipment_mainhand_id, equipment_offhand_id, equipment_ranged_id,
                equipment_tabard_id
            )
            VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            account_id,
            name,
            race,
            class,
            gender,
            packet.skin,
            packet.face,
            packet.hairstyle,
            packet.haircolor,
            packet.facialhair,
            start_char_info.level,
            start_char_info.area_id,
            start_char_info.map_id,
            start_char_info.position.0,
            start_char_info.position.1,
            start_char_info.position.2,
            start_char_info.orientation,
            true,
            display_id,
            equipment_head_id,
            equipment_neck_id,
            equipment_shoulders_id,
            equipment_body_id,
            equipment_chest_id,
            equipment_waist_id,
            equipment_legs_id,
            equipment_feet_id,
            equipment_wrists_id,
            equipment_hands_id,
            equipment_finger1_id,
            equipment_finger2_id,
            equipment_trinket1_id,
            equipment_trinket2_id,
            equipment_back_id,
            equipment_mainhand_id,
            equipment_offhand_id,
            equipment_ranged_id,
            equipment_tabard_id
        )
        .execute(db)
        .await
        .map_err(|e| CharacterCreateError::Database(e))?;

        Ok(())
    }

    pub fn build_item_full_update_blocks(&self) -> Vec<UpdateData> {
        let mut r = Vec::with_capacity(130); // 113 fields for items
        if let Some(item) = &self.items.equipment.head {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.neck {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.shoulders {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.body {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.chest {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.waist {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.legs {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.feet {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.wrists {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.hands {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.finger1 {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.finger2 {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.trinket1 {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.trinket2 {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.back {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.mainhand {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.offhand {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.ranged {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.tabard {
            r.push(item.build_create_update_block());
        }
        for item in &self.items.bags {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.main_backpack {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.bank {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.bank_bags {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.vendor_buyback {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.keyring {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }

        r
    }
}

struct CharacterItems {
    pub equipment: EquipmentItems,
    pub bags: [Option<Item>; 4],
    pub main_backpack: [Option<Item>; 16],
    pub bank: [Option<Item>; 28],
    pub bank_bags: [Option<Item>; 7],
    pub vendor_buyback: [Option<Item>; 12],
    pub keyring: [Option<Item>; 12],
    // unknown: [Item; 15]
}

pub struct EquipmentItems {
    pub head: Option<Item>,
    pub neck: Option<Item>,
    pub shoulders: Option<Item>,
    pub body: Option<Item>,
    pub chest: Option<Item>,
    pub waist: Option<Item>,
    pub legs: Option<Item>,
    pub feet: Option<Item>,
    pub wrists: Option<Item>,
    pub hands: Option<Item>,
    pub finger1: Option<Item>,
    pub finger2: Option<Item>,
    pub trinket1: Option<Item>,
    pub trinket2: Option<Item>,
    pub back: Option<Item>,
    pub mainhand: Option<Item>,
    pub offhand: Option<Item>,
    pub ranged: Option<Item>,
    pub tabard: Option<Item>,
}

pub enum CharacterCreateError {
    InvalidRace,
    InvalidClass,
    InvalidGender,
    InvalidRaceClassCombination,
    DisplayIdNotFound,
    Database(sqlx::Error),
}
