use bitfield_struct::bitfield;
use common::guid::{self, Guid};
use macros::tracked;

use crate::tracked_field::{TrackedWriteTrait, UpdateWritable};

#[tracked]
pub struct PlayerFields {
    pub duel_arbiter: Option<Guid<guid::GameObject>>,
    pub flags: u32,
    pub guild_id: u32,
    pub guild_rank: u32,
    pub _unknown_bytes_1: u32,
    pub bytes_2: PlayerFieldBytes2,
    pub bytes_3: PlayerFieldBytes3,
    pub duel_team: u32,
    pub guild_timestamp: u32,
    pub quest_log: [QuestLogFields; 20],
    pub visible_items: [VisibleItemFields; 19],
    pub inv_slot_head: [u32; 46],
    pub main_backpack_slots: [u32; 32],
    pub bank_slots: [u32; 48],
    pub bank_bag_slots: [u32; 12],
    pub vendor_buyback_slots: [u32; 24],
    pub keyring_slots: [u32; 64],
    pub far_sight: Option<Guid<guid::DynamicObject>>,
    pub field_combo_target: Option<Guid<guid::GameObject>>, //unknown
    pub xp: u32,
    pub next_level_xp: u32,
    pub skill_infos: [u32; 384],
    pub character_points_1: u32,
    pub character_points_2: u32,
    pub track_creatures: u32,
    pub track_resources: u32,
    pub block_percentage: u32,
    pub dodge_percentage: u32,
    pub parry_percentage: u32,
    pub crit_percentage: u32,
    pub ranged_crit_percentage: u32,
    pub explored_zones: [u32; 64],
    pub rest_state_experience: u32,
    pub coinage: u32,
    pub pos_stats: [u32; 5],
    pub neg_stats: [u32; 5],
    pub resistance_buff_mods_positive: [u32; 7],
    pub resistance_buff_mods_negative: [u32; 7],
    pub mod_damage_done_pos: [u32; 7],
    pub mod_damage_done_neg: [u32; 7],
    pub mod_damage_done_pct: [u32; 7],
    pub _unknown_bytes_4: u32,
    pub ammo_id: u32,
    pub self_res_spell: u32,
    pub pvp_medals: u32,
    pub buyback_prices: [u32; 12],
    pub buyback_timestamps: [u32; 12],
    pub session_kills: u32,
    pub yesterday_kills: u32,
    pub last_week_kills: u32,
    pub this_week_kills: u32,
    pub this_week_contribution: u32,
    pub lifetime_honorable_kills: u32,
    pub lifetime_dishonorable_kills: u32,
    pub yesterday_contribution: u32,
    pub last_week_contribution: u32,
    pub last_week_rank: u32,
    pub _unknown_bytes_5: u32,
    pub watched_faction_index: u32,
    pub combat_ratings: [u32; 20],
}

#[derive(Clone, Copy)]
pub struct QuestLogFields {
    pub log_1: u32,
    pub log_2: u32,
    pub log_3: u32,
}

impl UpdateWritable for QuestLogFields {
    fn get_mask_bits_count() -> usize {
        3
    }

    fn get_update_blocks_count() -> usize {
        3
    }

    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.log_1;
        blocks[1] = self.log_2;
        blocks[2] = self.log_3;
    }
}

pub struct VisibleItemFields {
    pub creator: Option<Guid<guid::Player>>,
    pub unkn: [u32; 8],
    pub properties: u32,
    pub _padding: u32,
}

impl UpdateWritable for VisibleItemFields {
    fn get_mask_bits_count() -> usize {
        12
    }

    fn get_update_blocks_count() -> usize {
        12
    }

    fn write(&self, blocks: &mut [u32]) {
        self.creator.write(&mut blocks[0..=1]);
        for (i, v) in self.unkn.iter().enumerate() {
            let i = 2 + i;
            v.write(&mut blocks[i..=i]);
        }
        blocks[10] = self.properties;
        blocks[11] = self._padding;
    }
}

#[bitfield(u32)]
pub struct PlayerFieldBytes2 {
    pub facial_hair: u8,
    pub _unknown: u8,
    pub bank_bag_slots: u8,
    pub rested_state: u8,
}

impl UpdateWritable for PlayerFieldBytes2 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}

#[bitfield(u32)]
pub struct PlayerFieldBytes3 {
    pub gender: bool,
    #[bits(15)]
    pub drunk_value: u16,
    pub _unknown: u8,
    pub honor_rank: u8,
}

impl UpdateWritable for PlayerFieldBytes3 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}
