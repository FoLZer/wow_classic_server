use std::{collections::HashSet, num::NonZeroU32, sync::Arc};

use common::guid::{self, Guid, LivingGuid};
use gameobjects::{
    object::{ObjectFields, TypeBitField},
    player::{
        PlayerFieldBytes1, PlayerFieldBytes2, PlayerFieldBytes3, PlayerFields, PlayerFlags,
        QuestLogFields, TutorialFlags, VisibleItemFields,
    },
    unit::{
        NpcFlags, SheathState, StandStateType, UnitFieldBytes1, UnitFieldBytes2,
        UnitFieldBytes2Flags, UnitFieldBytes3, UnitFieldBytes3Flags, UnitFields, UnitFlags,
        VirtualItemInfo,
    },
};
use packets::{
    inventory_change_result::{InventoryChangeError, InventoryChangeErrorType},
    update_data::UpdateData,
};
use sqlx::{Pool, Sqlite};
use tokio::{net::tcp::OwnedWriteHalf, sync::Mutex};

use crate::{
    game_data::GameDataAccessor,
    objects::{
        creature::MeleeState,
        item::Item,
        item_prototype::{InventoryType, ItemPrototype},
    },
    packet_handler::{EquipmentSlot, Slot},
};

pub struct Character {
    pub map_id: u32,
    pub position: (f32, f32, f32),
    pub orientation: f32,

    pub account_id: u32,
    pub session_key: [u8; 40],
    pub stream_tx: Arc<Mutex<OwnedWriteHalf>>,
    pub encrypt_data: Arc<Mutex<(usize, u8)>>,

    items: CharacterItems,

    pub object_fields: ObjectFields<guid::Player>,
    pub unit_fields: UnitFields,
    pub player_fields: PlayerFields,

    pub other_visible_players: HashSet<Guid<guid::Player>>,
    pub visible_creatures: HashSet<Guid<guid::Unit>>,

    pub tutorial_flags: TutorialFlags,

    pub melee_state: Option<MeleeState>,
}

impl Character {
    pub fn guid(&self) -> Guid<guid::Player> {
        *self.object_fields.guid.get()
    }

    pub fn sees(&self, guid: LivingGuid) -> bool {
        match guid {
            LivingGuid::Player(guid) => self.sees_player(guid),
            LivingGuid::Unit(guid) => self.sees_creature(guid),
        }
    }

    pub fn sees_player(&self, guid: Guid<guid::Player>) -> bool {
        self.other_visible_players.contains(&guid)
    }

    pub fn sees_player_including_self(&self, guid: Guid<guid::Player>) -> bool {
        self.guid() == guid || self.other_visible_players.contains(&guid)
    }

    pub fn sees_creature(&self, guid: Guid<guid::Unit>) -> bool {
        self.visible_creatures.contains(&guid)
    }

    pub fn sees_including_self(&self, guid: LivingGuid) -> bool {
        match guid {
            LivingGuid::Player(guid) => self.sees_player_including_self(guid),
            LivingGuid::Unit(guid) => self.sees_creature(guid),
        }
    }

    pub fn distance_to_squared(&self, position: (f32, f32, f32)) -> f32 {
        let dx = self.position.0 - position.0;
        let dy = self.position.1 - position.1;
        let dz = self.position.2 - position.2;
        dx * dx + dy * dy + dz * dz
    }

    pub async fn load_from_db(
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        guid: Guid<guid::Player>,
        account_id: u32,
        stream_tx: OwnedWriteHalf,
        session_key: [u8; 40],
        encrypt_data: (usize, u8),
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

        let items = CharacterItems {
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
            encrypt_data: Arc::new(Mutex::new(encrypt_data)),

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
                level: (model.level as u32).into(),
                faction_template: 1.into(),
                bytes_1: UnitFieldBytes1::new()
                    .with_race(model.race as u8)
                    .with_class(model.class as u8)
                    .with_gender(model.gender as u8)
                    .with_power(0)
                    .into(),
                virtual_item_slot_displays: [0.into(); 3],
                virtual_item_infos: [VirtualItemInfo::new().into(); 3],
                flags: UnitFlags::new().with_player_controlled(true).into(),
                aura: [0.into(); 48],
                aura_flags: [0.into(); 6],
                aura_levels: [0.into(); 12],
                aura_applications: [0.into(); 12],
                aura_state: 0.into(),
                base_attack_time: 2000.into(),
                offhand_attack_time: 0.into(),
                ranged_attack_time: 0.into(),
                bounding_radius: 4.into(),
                combat_reach: 1.5.into(),
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
                npc_flags: NpcFlags::new().into(),
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
                flags: PlayerFlags::new().into(),
                guild_id: 0.into(),
                guild_rank: 0.into(),
                bytes_1: PlayerFieldBytes1::new()
                    .with_skin(model.skin as u8)
                    .with_face(model.face as u8)
                    .with_hair_style(model.hair_style as u8)
                    .with_hair_color(model.hair_color as u8)
                    .into(),
                bytes_2: PlayerFieldBytes2::new()
                    .with_facial_hair(model.facial_hair as u8)
                    .with_bank_bag_slots(0)
                    .with_rested_state(2) // Exhaustion.dbc: 1 - Rested (200%), 2 - Normal (100%)
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

                visible_items: [
                    if let Some(item) = &items.equipment.head {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.neck {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.shoulders {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.body {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.chest {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.waist {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.legs {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.feet {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.wrists {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.hands {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.finger1 {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.finger2 {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.trinket1 {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.trinket2 {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.back {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.mainhand {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.offhand {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.ranged {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                    if let Some(item) = &items.equipment.tabard {
                        item.get_visible_item_fields().into()
                    } else {
                        VisibleItemFields::default().into()
                    },
                ],

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
            other_visible_players: HashSet::new(),
            visible_creatures: HashSet::new(),

            tutorial_flags: TutorialFlags::from_bits(model.tutorial_flags as u64),

            items,

            melee_state: None,
        })
    }

    pub fn is_alive(&self) -> bool {
        *self.unit_fields.health.get() != 0
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

    pub async fn swap_items(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        src: Slot,
        dst: Slot,
    ) -> Result<(), MoveItemError> {
        let error = |error| {
            MoveItemError::Inventory(InventoryChangeError {
                error,
                item1: self
                    .items
                    .get(src)
                    .as_ref()
                    .map(|v| *v.object_fields.guid.get()),
                item2: self
                    .items
                    .get(dst)
                    .as_ref()
                    .map(|v| *v.object_fields.guid.get()),
            })
        };

        // Which slot's item ends up in the given slot after the swap
        let after_swap = |slot: Slot| {
            if slot == dst {
                src
            } else if slot == src {
                dst
            } else {
                slot
            }
        };

        let load_prototype = async |slot| {
            let Some(item) = self.items.get(slot) else {
                return Ok(None);
            };

            match item.load_prototype(game_data_accessor).await? {
                Some(v) => Ok(Some(v)),
                None => Err(error(InventoryChangeErrorType::ItemNotFound)),
            }
        };

        let Some(src_prototype) = load_prototype(src).await? else {
            return Err(error(InventoryChangeErrorType::SlotIsEmpty));
        };
        let dst_prototype = load_prototype(dst).await?;
        let main_hand_prototype =
            load_prototype(after_swap(Slot::Equipment(EquipmentSlot::MainHand))).await?;
        let off_hand_prototype =
            load_prototype(after_swap(Slot::Equipment(EquipmentSlot::OffHand))).await?;

        if src == dst {
            return Ok(());
        }

        if !self.is_alive() {
            return Err(error(InventoryChangeErrorType::YouAreDead));
        }

        if let Err(e) = self.can_move_item(&src_prototype, src, dst) {
            return Err(error(e));
        }
        if let Some(dst_prototype) = &dst_prototype
            && let Err(e) = self.can_move_item(dst_prototype, dst, src)
        {
            return Err(error(e));
        }

        // An off hand item that has to be moved to the backpack to make room for a two-handed weapon
        let mut displaced_off_hand = None;
        if let (Some(main_hand_prototype), Some(off_hand_prototype)) =
            (&main_hand_prototype, &off_hand_prototype)
            && main_hand_prototype.inventory_type == InventoryType::TwoHandWeapon
        {
            if matches!(src, Slot::Equipment(EquipmentSlot::OffHand))
                || matches!(dst, Slot::Equipment(EquipmentSlot::OffHand))
            {
                return Err(error(InventoryChangeErrorType::CantEquipWithTwohanded));
            }

            if let Err(e) = self.can_move_item(
                off_hand_prototype,
                Slot::Equipment(EquipmentSlot::OffHand),
                Slot::MainBag(0),
            ) {
                return Err(error(e));
            }

            let free_slot = (0..self.items.main_backpack.len() as u8)
                .map(Slot::MainBag)
                .find(|&slot| self.items.get(after_swap(slot)).is_none());
            let Some(free_slot) = free_slot else {
                return Err(error(if dst_prototype.is_some() {
                    InventoryChangeErrorType::ItemsCantBeSwapped
                } else {
                    InventoryChangeErrorType::InventoryFull
                }));
            };
            displaced_off_hand = Some(free_slot);
        }

        let src_item = self.items.get_mut(src).take();
        let dst_item = self.items.get_mut(dst).take();
        *self.items.get_mut(dst) = src_item;
        *self.items.get_mut(src) = dst_item;
        self.update_slot_fields(src);
        self.update_slot_fields(dst);

        if let Some(free_slot) = displaced_off_hand {
            *self.items.get_mut(free_slot) = self
                .items
                .get_mut(Slot::Equipment(EquipmentSlot::OffHand))
                .take();
            self.update_slot_fields(Slot::Equipment(EquipmentSlot::OffHand));
            self.update_slot_fields(free_slot);
        }

        Ok(())
    }

    pub async fn auto_equip_item(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        src: Slot,
    ) -> Result<(), MoveItemError> {
        let Some(item) = self.items.get(src) else {
            return Err(MoveItemError::Inventory(InventoryChangeError {
                error: InventoryChangeErrorType::SlotIsEmpty,
                item1: None,
                item2: None,
            }));
        };
        let Some(prototype) = item.load_prototype(game_data_accessor).await? else {
            return Err(MoveItemError::Inventory(InventoryChangeError {
                error: InventoryChangeErrorType::ItemNotFound,
                item1: Some(*item.object_fields.guid.get()),
                item2: None,
            }));
        };

        let Some(dst) = self.find_equip_slot(&prototype) else {
            return Err(MoveItemError::Inventory(InventoryChangeError {
                error: InventoryChangeErrorType::ItemCantBeEquipped,
                item1: self
                    .items
                    .get(src)
                    .as_ref()
                    .map(|v| *v.object_fields.guid.get()),
                item2: None,
            }));
        };

        self.swap_items(game_data_accessor, src, dst).await
    }

    pub async fn store_item_in_backpack(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        src: Slot,
    ) -> Result<(), MoveItemError> {
        let error = |error| {
            MoveItemError::Inventory(InventoryChangeError {
                error,
                item1: self
                    .items
                    .get(src)
                    .as_ref()
                    .map(|v| *v.object_fields.guid.get()),
                item2: None,
            })
        };

        if self.items.get(src).is_none() {
            return Err(error(InventoryChangeErrorType::SlotIsEmpty));
        }

        if matches!(src, Slot::MainBag(_)) {
            return Err(error(InventoryChangeErrorType::None));
        }

        let Some(dst) = (0..self.items.main_backpack.len() as u8)
            .map(Slot::MainBag)
            .find(|&slot| self.items.get(slot).is_none())
        else {
            return Err(error(InventoryChangeErrorType::BagFull));
        };

        self.swap_items(game_data_accessor, src, dst).await
    }

    /// Picks an empty slot out of the ones the item fits into, or the first one if all of them are taken
    fn find_equip_slot(&self, prototype: &ItemPrototype) -> Option<Slot> {
        use EquipmentSlot::*;

        let slots: &[Slot] = match prototype.inventory_type {
            InventoryType::Head => &[Slot::Equipment(Head)],
            InventoryType::Neck => &[Slot::Equipment(Neck)],
            InventoryType::Shoulders => &[Slot::Equipment(Shoulders)],
            InventoryType::Body => &[Slot::Equipment(Body)],
            InventoryType::Chest | InventoryType::Robe => &[Slot::Equipment(Chest)],
            InventoryType::Waist => &[Slot::Equipment(Waist)],
            InventoryType::Legs => &[Slot::Equipment(Legs)],
            InventoryType::Feet => &[Slot::Equipment(Feet)],
            InventoryType::Wrists => &[Slot::Equipment(Wrists)],
            InventoryType::Hands => &[Slot::Equipment(Hands)],
            InventoryType::Finger => &[Slot::Equipment(Finger1), Slot::Equipment(Finger2)],
            InventoryType::Trinket => &[Slot::Equipment(Trinket1), Slot::Equipment(Trinket2)],
            InventoryType::Cloak => &[Slot::Equipment(Back)],
            //TODO: Should also try OffHand if the player can dual wield
            InventoryType::Weapon
            | InventoryType::TwoHandWeapon
            | InventoryType::WeaponMainHand => &[Slot::Equipment(MainHand)],
            InventoryType::Shield | InventoryType::WeaponOffHand | InventoryType::Holdable => {
                &[Slot::Equipment(OffHand)]
            }
            InventoryType::Ranged
            | InventoryType::Thrown
            | InventoryType::RangedRight
            | InventoryType::Relic => &[Slot::Equipment(Ranged)],
            InventoryType::Tabard => &[Slot::Equipment(Tabard)],
            InventoryType::Bag | InventoryType::Quiver => &[
                Slot::BagItem(0),
                Slot::BagItem(1),
                Slot::BagItem(2),
                Slot::BagItem(3),
            ],
            _ => return None,
        };

        slots
            .iter()
            .copied()
            .find(|&slot| self.items.get(slot).is_none())
            .or(slots.first().copied())
    }

    fn can_move_item(
        &self,
        prototype: &ItemPrototype,
        from: Slot,
        to: Slot,
    ) -> Result<(), InventoryChangeErrorType> {
        if !Slot::is_equipable(&to) && !Slot::is_equipable(&from) {
            return Ok(());
        }

        if (matches!(to, Slot::Equipment(_)) || matches!(from, Slot::Equipment(_)))
            && self.unit_fields.flags.get().in_combat()
            && !prototype.equipable_in_combat()
        {
            return Err(InventoryChangeErrorType::NotInCombat);
        }

        let fits = match to {
            Slot::Equipment(slot) => {
                use EquipmentSlot::*;

                match prototype.inventory_type {
                    InventoryType::Head => slot == Head,
                    InventoryType::Neck => slot == Neck,
                    InventoryType::Shoulders => slot == Shoulders,
                    InventoryType::Body => slot == Body,
                    InventoryType::Chest | InventoryType::Robe => slot == Chest,
                    InventoryType::Waist => slot == Waist,
                    InventoryType::Legs => slot == Legs,
                    InventoryType::Feet => slot == Feet,
                    InventoryType::Wrists => slot == Wrists,
                    InventoryType::Hands => slot == Hands,
                    InventoryType::Finger => matches!(slot, Finger1 | Finger2),
                    InventoryType::Trinket => matches!(slot, Trinket1 | Trinket2),
                    InventoryType::Cloak => slot == Back,
                    //TODO: OffHand requires the dual wield skill
                    InventoryType::Weapon => matches!(slot, MainHand | OffHand),
                    InventoryType::TwoHandWeapon | InventoryType::WeaponMainHand => {
                        slot == MainHand
                    }
                    InventoryType::Shield
                    | InventoryType::WeaponOffHand
                    | InventoryType::Holdable => slot == OffHand,
                    InventoryType::Ranged
                    | InventoryType::Thrown
                    | InventoryType::RangedRight
                    | InventoryType::Relic => slot == Ranged,
                    InventoryType::Tabard => slot == Tabard,
                    _ => return Err(InventoryChangeErrorType::ItemCantBeEquipped),
                }
            }
            Slot::BagItem(_) => matches!(
                prototype.inventory_type,
                InventoryType::Bag | InventoryType::Quiver
            ),
            Slot::MainBag(_) => true,
        };
        if !fits {
            return Err(InventoryChangeErrorType::ItemDoesntGoToSlot);
        }

        if Slot::is_equipable(&to) {
            self.can_equip_item(prototype)?;
        }

        Ok(())
    }

    //TODO: Skill, spell, honor rank and reputation requirements
    fn can_equip_item(&self, prototype: &ItemPrototype) -> Result<(), InventoryChangeErrorType> {
        let bytes_1 = self.unit_fields.bytes_1.get();
        let class_mask = 1 << (bytes_1.class() - 1);
        let race_mask = 1 << (bytes_1.race() - 1);
        if prototype.allowable_class & class_mask == 0 || prototype.allowable_race & race_mask == 0
        {
            return Err(InventoryChangeErrorType::YouCanNeverUseThatItem);
        }

        if *self.unit_fields.level.get() < prototype.required_level {
            return Err(InventoryChangeErrorType::CantEquipLevel {
                required_level: prototype.required_level,
            });
        }

        Ok(())
    }

    // Copies the item in a slot to the player fields that mirror it
    fn update_slot_fields(&mut self, slot: Slot) {
        let item = self.items.get(slot).as_ref();
        let guid = item.map(|v| *v.object_fields.guid.get());
        let player_fields = &mut self.player_fields;
        match slot {
            Slot::Equipment(slot) => {
                *player_fields.equipment_slots[slot as usize].get_mut_using_copy() = guid;
                *player_fields.visible_items[slot as usize].get_mut_using_copy() =
                    item.map_or_else(VisibleItemFields::default, |v| v.get_visible_item_fields());
            }
            Slot::BagItem(slot) => {
                *player_fields.bag_slots[slot as usize].get_mut_using_copy() = guid;
            }
            Slot::MainBag(slot) => {
                *player_fields.main_backpack_slots[slot as usize].get_mut_using_copy() = guid;
            }
        }
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

impl CharacterItems {
    fn get(&self, slot: Slot) -> &Option<Item> {
        match slot {
            Slot::Equipment(slot) => self.equipment.get(slot),
            Slot::BagItem(slot) => &self.bags[slot as usize],
            Slot::MainBag(slot) => &self.main_backpack[slot as usize],
        }
    }

    fn get_mut(&mut self, slot: Slot) -> &mut Option<Item> {
        match slot {
            Slot::Equipment(slot) => self.equipment.get_mut(slot),
            Slot::BagItem(slot) => &mut self.bags[slot as usize],
            Slot::MainBag(slot) => &mut self.main_backpack[slot as usize],
        }
    }
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

impl EquipmentItems {
    fn get(&self, slot: EquipmentSlot) -> &Option<Item> {
        match slot {
            EquipmentSlot::Head => &self.head,
            EquipmentSlot::Neck => &self.neck,
            EquipmentSlot::Shoulders => &self.shoulders,
            EquipmentSlot::Body => &self.body,
            EquipmentSlot::Chest => &self.chest,
            EquipmentSlot::Waist => &self.waist,
            EquipmentSlot::Legs => &self.legs,
            EquipmentSlot::Feet => &self.feet,
            EquipmentSlot::Wrists => &self.wrists,
            EquipmentSlot::Hands => &self.hands,
            EquipmentSlot::Finger1 => &self.finger1,
            EquipmentSlot::Finger2 => &self.finger2,
            EquipmentSlot::Trinket1 => &self.trinket1,
            EquipmentSlot::Trinket2 => &self.trinket2,
            EquipmentSlot::Back => &self.back,
            EquipmentSlot::MainHand => &self.mainhand,
            EquipmentSlot::OffHand => &self.offhand,
            EquipmentSlot::Ranged => &self.ranged,
            EquipmentSlot::Tabard => &self.tabard,
        }
    }

    fn get_mut(&mut self, slot: EquipmentSlot) -> &mut Option<Item> {
        match slot {
            EquipmentSlot::Head => &mut self.head,
            EquipmentSlot::Neck => &mut self.neck,
            EquipmentSlot::Shoulders => &mut self.shoulders,
            EquipmentSlot::Body => &mut self.body,
            EquipmentSlot::Chest => &mut self.chest,
            EquipmentSlot::Waist => &mut self.waist,
            EquipmentSlot::Legs => &mut self.legs,
            EquipmentSlot::Feet => &mut self.feet,
            EquipmentSlot::Wrists => &mut self.wrists,
            EquipmentSlot::Hands => &mut self.hands,
            EquipmentSlot::Finger1 => &mut self.finger1,
            EquipmentSlot::Finger2 => &mut self.finger2,
            EquipmentSlot::Trinket1 => &mut self.trinket1,
            EquipmentSlot::Trinket2 => &mut self.trinket2,
            EquipmentSlot::Back => &mut self.back,
            EquipmentSlot::MainHand => &mut self.mainhand,
            EquipmentSlot::OffHand => &mut self.offhand,
            EquipmentSlot::Ranged => &mut self.ranged,
            EquipmentSlot::Tabard => &mut self.tabard,
        }
    }
}

pub enum MoveItemError {
    // Sent to the client
    Inventory(InventoryChangeError),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for MoveItemError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

pub enum CharacterCreateError {
    InvalidRace,
    InvalidClass,
    InvalidGender,
    InvalidRaceClassCombination,
    DisplayIdNotFound,
    Database(sqlx::Error),
}
