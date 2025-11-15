use std::num::NonZeroU32;

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
use tokio::net::TcpStream;

pub struct Character {
    pub map_id: u32,
    pub position: (f32, f32, f32),
    pub orientation: f32,

    pub account_id: u32,
    pub session_key: [u8; 40],
    pub stream: TcpStream,

    pub object_fields: ObjectFields<guid::Player>,
    pub unit_fields: UnitFields,
    pub player_fields: PlayerFields,
}

impl Character {
    pub fn from_model(
        stream: TcpStream,
        session_key: [u8; 40],
        model: gameserver_entity::character::Model,
    ) -> Self {
        Self {
            map_id: model.map as u32,
            position: (model.position_x, model.position_y, model.position_z),
            orientation: model.orientation,

            account_id: model.account_id as u32,
            session_key,
            stream,

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
                bytes_3: PlayerFieldBytes3::new()
                    .with_gender(model.gender != 0)
                    .into(),
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
                inv_slot_head: [0.into(); 46],
                main_backpack_slots: [0.into(); 32],
                bank_slots: [0.into(); 48],
                bank_bag_slots: [0.into(); 12],
                vendor_buyback_slots: [0.into(); 24],
                keyring_slots: [0.into(); 64],
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
        }
    }
}
