use bitfield_struct::bitfield;
use common::guid::{self, Guid, SelectableGuid};
use macros::tracked;

use crate::tracked_field::{TrackedWriteTrait, UpdateWritable};

#[tracked]
pub struct UnitFields {
    pub charm: Option<Guid<guid::Unit>>,
    pub summon: Option<Guid<guid::Unit>>,
    pub charmed_by: Option<Guid<guid::Unit>>,
    pub summoned_by: Option<Guid<guid::Unit>>,
    pub created_by: Option<Guid<guid::Unit>>,
    pub target: Option<SelectableGuid>,
    pub persuaded: Option<Guid<guid::Unit>>,
    pub channel_object: Option<Guid<guid::Unit>>,
    pub health: u32,
    pub powers: [u32; 5],
    pub max_health: u32,
    pub max_powers: [u32; 5],
    pub level: u32,
    pub faction_template: u32,
    pub bytes_1: UnitFieldBytes1,
    pub virtual_item_slot_displays: [u32; 3],
    pub virtual_item_infos: [VirtualItemInfo; 3],
    pub flags: UnitFlags,
    pub aura: [u32; 48],
    pub aura_flags: [u32; 6],
    pub aura_levels: [u32; 12],
    pub aura_applications: [u32; 12],
    pub aura_state: u32,
    pub base_attack_time: u32,
    pub offhand_attack_time: u32,
    pub ranged_attack_time: u32,
    pub bounding_radius: u32,
    pub combat_reach: f32,
    pub display_id: u32,
    pub native_display_id: u32,
    pub mount_display_id: u32,
    pub min_damage: u32,
    pub max_damage: u32,
    pub min_offhand_damage: u32,
    pub max_offhand_damage: u32,
    pub bytes_2: UnitFieldBytes2,
    pub pet_number: u32,
    pub pet_name_timestamp: u32,
    pub pet_experience: u32,
    pub pet_next_level_exp: u32,
    pub dynamic_flags: u32,
    pub channel_spell: u32,
    pub mod_cast_speed: u32,
    pub created_by_spell: u32,
    pub npc_flags: u32,
    pub npc_emote_state: u32,
    pub training_points: u32,
    pub strength: u32,
    pub agility: u32,
    pub stamina: u32,
    pub intellect: u32,
    pub spirit: u32,
    pub normal_resistance: u32,
    pub holy_resistance: u32,
    pub fire_resistance: u32,
    pub nature_resistance: u32,
    pub frost_resistance: u32,
    pub shadow_resistance: u32,
    pub arcane_resistance: u32,
    pub base_mana: u32,
    pub base_health: u32,
    pub bytes_3: UnitFieldBytes3,
    pub attack_power: u32,
    pub attack_power_mods: u32,
    pub attack_power_multiplier: u32,
    pub ranged_attack_power: u32,
    pub ranged_attack_power_mods: u32,
    pub ranged_attack_power_multiplier: u32,
    pub min_ranged_damage: u32,
    pub max_ranged_damage: u32,
    pub power_cost_modifiers: [u32; 7],
    pub power_cost_multipliers: [u32; 7],
    pub _padding: u32,
}

#[bitfield(u32)]
pub struct UnitFieldBytes1 {
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub power: u8,
}

impl UpdateWritable for UnitFieldBytes1 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}

#[bitfield(u32)]
pub struct UnitFlags {
    _unkn: bool, // never read by the client
    pub not_attackable: bool, // can't be attacked, also overrides the loss of control flags
    pub client_lost_control: bool, // like confused and fleeing, the client can't control the unit
    pub player_controlled: bool, // controlled by a player, also lets the client start swimming
    pub pet_can_rename: bool, // the pet can be renamed
    pub pet_can_abandon: bool, // the pet can be abandoned
    pub plus_mob: bool, // elite
    pub not_attackable_2: bool, // can't be attacked
    pub immune_to_player_controlled: bool, // can't attack or be attacked by player controlled units, also stops inheriting the owner's pvp state
    pub immune_to_npc: bool, // can't attack or be attacked by non player controlled units
    pub is_looting: bool, // shows the looting animation
    pub in_combat_stance: bool, // treated as in combat for animations, also lets the client start swimming
    pub pvp: bool, // flagged for pvp
    pub silenced: bool, // can't cast spells that are prevented by silence
    pub persuaded: bool,
    pub can_swim: bool, // lets the client start swimming
    pub untargetable: bool, // can't be targeted by spells or attacked
    pub is_pacified: bool, // can't cast spells that are prevented by pacify
    pub stunned: bool, // can't move or cast spells
    pub in_combat: bool, // affecting combat, can't cast spells that aren't usable in combat
    pub on_taxi: bool, // riding a taxi
    pub disarmed: bool, // main and off hand weapons are ignored
    pub confused: bool, // can't cast spells while confused
    pub fleeing: bool, // can't cast spells while fleeing
    pub possessed: bool, // controlled by the unit in charmed_by
    pub not_selectable: bool, // only the unit in created_by can interact with it
    pub skinnable: bool,
    pub has_auras_visible: bool, // auras that would otherwise be hidden are shown
    pub loot_without_animation: bool, // suppresses the looting animation, eg. while fishing
    pub no_chat_emotes: bool, // no talk/yell/laugh animations when chatting
    _unkn: bool, // never read by the client
    pub restricted_gameobject_use: bool, // can't use chests and other gameobjects that don't allow it
}

impl UpdateWritable for UnitFlags {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct UnitFieldBytes2 {
    #[bits(8)]
    pub stand_state: StandStateType,
    pub loyalty_level: u8,
    pub free_talent_points: u8,
    #[bits(8)]
    pub flags: UnitFieldBytes2Flags,
}

impl UpdateWritable for UnitFieldBytes2 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}

#[bitfield(u8)]
pub struct UnitFieldBytes2Flags {
    pub always_stand: bool,
    pub creep: bool,
    pub untrackable: bool,
    pub _unknown_0: bool,
    pub _unknown_1: bool,
    pub _unknown_2: bool,
    pub _unknown_3: bool,
    pub _unknown_4: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum StandStateType {
    Stand = 0,
    Sit = 1,
    SitChair = 2,
    Sleep = 3,
    SitLowChair = 4,
    SitMediumChair = 5,
    SitHighChair = 6,
    Dead = 7,
    Kneel = 8,
}

impl StandStateType {
    pub const fn into_bits(self) -> u8 {
        self as _
    }
    pub const fn from_bits(value: u8) -> Self {
        let Ok(value) = Self::try_from_bits(value) else {
            panic!();
        };
        value
    }
    pub const fn try_from_bits(value: u8) -> Result<Self, u8> {
        Ok(match value {
            0 => Self::Stand,
            1 => Self::Sit,
            2 => Self::SitChair,
            3 => Self::Sleep,
            4 => Self::SitLowChair,
            5 => Self::SitMediumChair,
            6 => Self::SitHighChair,
            7 => Self::Dead,
            8 => Self::Kneel,
            _ => return Err(value),
        })
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq)]
pub struct UnitFieldBytes3 {
    #[bits(8)]
    pub sheath_state: SheathState,
    #[bits(8)]
    pub flags: UnitFieldBytes3Flags,
    pub _unknown_0: u8,
    pub _unknown_1: u8,
}

impl UpdateWritable for UnitFieldBytes3 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SheathState {
    Unarmed = 0,
    Melee = 1,
    Ranged = 2,
}

impl SheathState {
    const fn into_bits(self) -> u8 {
        self as _
    }
    const fn from_bits(value: u8) -> Self {
        let Ok(value) = Self::try_from_bits(value) else {
            panic!();
        };
        value
    }

    pub const fn try_from_bits(value: u8) -> Result<Self, u8> {
        Ok(match value {
            0 => Self::Unarmed,
            1 => Self::Melee,
            2 => Self::Ranged,
            _ => return Err(value),
        })
    }
}

#[bitfield(u8)]
pub struct UnitFieldBytes3Flags {
    unk0: bool,
    unk1: bool,
    unk2: bool,
    unk3: bool,
    auras: bool, // show positive auras as positive, and allow its dispel
    unk5: bool,
    unk6: bool,
    unk7: bool,
}

#[bitfield(u64)]
pub struct VirtualItemInfo {
    pub class: u8,
    pub sub_class: u8,
    pub material: u8,
    pub inventory_type: u8,
    pub sheath: u8,
    pub _unknown_0: u8,
    pub _unknown_1: u8,
    pub _unknown_2: u8,
}

impl UpdateWritable for VirtualItemInfo {
    fn get_update_blocks_count() -> usize {
        2
    }

    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0 as u32;
        blocks[1] = (self.0 >> 32) as u32;
    }
}
