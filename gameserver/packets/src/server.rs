#![allow(non_camel_case_types)]

use std::{ffi::CString, io::Write, num::NonZeroU32};

use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use common::guid::{self, AnyGuid, GossipSourceGuid, Guid, LivingGuid};
use macros::create_server_packets;

use crate::{
    account_result::AccountResult,
    attacker_state::{HitInfo, SubDamages, VictimState},
    bounded::Bounded,
    character_info::{CharacterList, CharacterName},
    chat_message::ServerChatMessage,
    gossip::{GossipMenuItems, GossipQuestMenuItems, NpcTextOption, U32Counted},
    inventory_change_result::InventoryChangeError,
    item_info::{ItemDamage, ItemFlags, ItemSpell, ItemStat},
    movement_info::MovementInfo,
    packed::Packed,
    stabled_pets::StabledPets,
    trainer_list::{TrainerSpell, TrainerType},
    update_data::UpdateBlocks,
    vendor_query::VendorQueryResult,
};

pub trait ServerPacket {
    const PACKET_NAME: &str;
    fn body_size(&self) -> usize;
    fn to_bytes(&self, session_key: Option<[u8; 40]>, encrypt_data: &mut (usize, u8)) -> Vec<u8>;
}

pub const MAX_SHORT_SERVER_PACKET_SIZE: usize = 0x7fff;
pub const MAX_SERVER_PACKET_SIZE: usize = 0x7f_ffff;
pub const MAX_SERVER_PACKET_BODY_SIZE: usize = MAX_SERVER_PACKET_SIZE - size_of::<u16>();

create_server_packets!(
SMSG_CHAR_CREATE 0x03A {
    result: AccountResult: LittleEndian,
},
SMSG_CHAR_DELETE 0x03C {
    result: AccountResult: LittleEndian,
},
SMSG_CHAR_ENUM 0x03B {
    characters: CharacterList: LittleEndian,
},
SMSG_CHAR_LOGIN_FAILED 0x041 {
    result: AccountResult: LittleEndian,
},
SMSG_LOGIN_SETTIMESPEED 0x042 {
    game_time: DateTime<Local>: LittleEndian,
    game_speed: f32: LittleEndian,
},
SMSG_LOGOUT_RESPONSE 0x04C {
    failed: u32: LittleEndian,
    instant: u8: LittleEndian,
},
SMSG_LOGOUT_COMPLETE 0x04D {},
SMSG_NAME_QUERY_RESPONSE 0x051 {
    guid: Guid<guid::Player>: LittleEndian,
    name: Option<CharacterName>: LittleEndian,
    realm_name: Option<Bounded<CString, 256>>: LittleEndian,
    race: u32: LittleEndian,
    gender: u32: LittleEndian,
    class: u32: LittleEndian,
},
SMSG_ITEM_QUERY_SINGLE_RESPONSE 0x058 {
    item_id: u32: LittleEndian,
    class: u32: LittleEndian,
    sub_class: u32: LittleEndian,
    name_1: Bounded<CString, 256>: LittleEndian,
    name_2: Bounded<CString, 256>: LittleEndian,
    name_3: Bounded<CString, 256>: LittleEndian,
    name_4: Bounded<CString, 256>: LittleEndian,
    display_info_id: u32: LittleEndian,
    quality: u32: LittleEndian,
    flags: ItemFlags: LittleEndian,
    buy_price: u32: LittleEndian,
    sell_price: u32: LittleEndian,
    inventory_type: u32: LittleEndian, //TODO
    allowable_class: u32: LittleEndian,
    allowable_race: u32: LittleEndian,
    item_level: u32: LittleEndian,
    required_level: u32: LittleEndian,
    required_skill: u32: LittleEndian,
    required_skill_rank: u32: LittleEndian,
    required_spell: u32: LittleEndian,
    required_honor_rank: u32: LittleEndian,
    required_city_rank: u32: LittleEndian,
    required_reputation_faction: u32: LittleEndian,
    required_reputation_rank: u32: LittleEndian,
    max_count: u32: LittleEndian,
    stackable: u32: LittleEndian,
    container_slots: u32: LittleEndian,
    item_stats: [ItemStat; 10]: LittleEndian,
    damage: [ItemDamage; 5]: LittleEndian,

    armor: u32: LittleEndian,
    holy_resistance: u32: LittleEndian,
    fire_resistance: u32: LittleEndian,
    nature_resistance: u32: LittleEndian,
    frost_resistance: u32: LittleEndian,
    shadow_resistance: u32: LittleEndian,
    arcane_resistance: u32: LittleEndian,

    delay: u32: LittleEndian,
    ammo_type: u32: LittleEndian,
    ranged_mod_range: f32: LittleEndian,

    spells: [Option<ItemSpell>; 5]: LittleEndian,

    bonding: u32: LittleEndian,
    description: Bounded<CString, 1024>: LittleEndian,
    page_text: u32: LittleEndian,
    language_id: u32: LittleEndian,
    page_material: u32: LittleEndian,
    start_quest: u32: LittleEndian,
    lock_id: u32: LittleEndian,
    material: u32: LittleEndian,
    sheath: u32: LittleEndian,
    random_property: u32: LittleEndian,
    block: u32: LittleEndian,
    item_set: u32: LittleEndian,
    max_durability: u32: LittleEndian,
    area: u32: LittleEndian,
    map: u32: LittleEndian,
    bag_family: u32: LittleEndian,
},
SMSG_CREATURE_QUERY_RESPONSE 0x061 {
    creature_id: u32: LittleEndian,
    name_1: Bounded<CString, 256>: LittleEndian,
    name_2: Bounded<CString, 256>: LittleEndian,
    name_3: Bounded<CString, 256>: LittleEndian,
    name_4: Bounded<CString, 256>: LittleEndian,
    sub_name: Bounded<CString, 256>: LittleEndian,
    flags: u32: LittleEndian,
    creature_type: u32: LittleEndian,
    family: Option<NonZeroU32>: LittleEndian,
    rank: u32: LittleEndian,
    unkn: u32: LittleEndian,
    pet_spell_data_id: u32: LittleEndian,
    display_id: u32: LittleEndian,
    civilian: u16: LittleEndian, //mask?
},
SMSG_MESSAGECHAT 0x096 {
    chat: ServerChatMessage: LittleEndian,
},
SMSG_UPDATE_OBJECT 0x0A9 {
    update_data: UpdateBlocks: LittleEndian,
},
SMSG_DESTROY_OBJECT 0x0AA {
    guid: AnyGuid: LittleEndian,
},
MSG_MOVE_START_FORWARD 0x0B5 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_BACKWARD 0x0B6 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP 0x0B7 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_STRAFE_LEFT 0x0B8 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_STRAFE_RIGHT 0x0B9 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP_STRAFE 0x0BA {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_JUMP 0x0BB {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_TURN_LEFT 0x0BC {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_TURN_RIGHT 0x0BD {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP_TURN 0x0BE {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_PITCH_UP 0x0BF {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_PITCH_DOWN 0x0C0 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP_PITCH 0x0C1 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_SET_RUN_MODE 0x0C2 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_SET_WALK_MODE 0x0C3 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_FALL_LAND 0x0C9 {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_SWIM 0x0CA {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP_SWIM 0x0CB {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_SET_FACING 0x0DA {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_SET_PITCH 0x0DB {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_HEARTBEAT 0x0EE {
    mover: Packed<Guid<guid::Player>>: LittleEndian,
    movement_info: MovementInfo: LittleEndian,
},
SMSG_TUTORIAL_FLAGS 0x0FD {
    tutorial_data0: u32: LittleEndian,
    tutorial_data1: u32: LittleEndian,
    tutorial_data2: u32: LittleEndian,
    tutorial_data3: u32: LittleEndian,
    tutorial_data4: u32: LittleEndian,
    tutorial_data5: u32: LittleEndian,
    tutorial_data6: u32: LittleEndian,
    tutorial_data7: u32: LittleEndian,
},
SMSG_INVENTORY_CHANGE_FAILURE 0x112 {
    result: InventoryChangeError: LittleEndian
},
SMSG_ATTACKSTART 0x143 {
    attacker: LivingGuid: LittleEndian,
    victim: LivingGuid: LittleEndian,
},
SMSG_ATTACKSTOP 0x144 {
    attacker: Packed<LivingGuid>: LittleEndian,
    victim: Option<Packed<LivingGuid>>: LittleEndian,
    unkn: u32: LittleEndian, // when non-zero the client stores the attacker-victim distance, purpose unknown
},
SMSG_ATTACKSWING_NOTINRANGE 0x145 {},
SMSG_ATTACKSWING_BADFACING 0x146 {},
SMSG_ATTACKSWING_DEADTARGET 0x148 {},
SMSG_ATTACKSWING_CANT_ATTACK 0x149 {},
SMSG_ATTACKERSTATEUPDATE 0x14A {
    hit_info: HitInfo: LittleEndian,
    attacker: Packed<LivingGuid>: LittleEndian,
    victim: Packed<LivingGuid>: LittleEndian,
    total_damage: u32: LittleEndian,
    sub_damages: SubDamages: LittleEndian,
    victim_state: VictimState: LittleEndian,
    unkn: u32: LittleEndian,
    spell_id: Option<NonZeroU32>: LittleEndian,
    blocked: u32: LittleEndian,
},
SMSG_BINDPOINTUPDATE 0x155 {
    homebind_x: f32: LittleEndian,
    homebind_y: f32: LittleEndian,
    homebind_z: f32: LittleEndian,
    homebind_map_id: u32: LittleEndian,
    homebind_area_id: u32: LittleEndian,
},
SMSG_GOSSIP_MESSAGE 0x17D {
    source: GossipSourceGuid: LittleEndian,
    title_id: u32: LittleEndian,
    menu_items: GossipMenuItems: LittleEndian,
    quest_menu_items: GossipQuestMenuItems: LittleEndian
},
SMSG_NPC_TEXT_UPDATE 0x180 {
    text_id: u32: LittleEndian,
    options: [NpcTextOption; 8]: LittleEndian
},
SMSG_QUESTGIVER_STATUS 0x183 {
    questgiver_guid: GossipSourceGuid: LittleEndian,
    status: Option<QuestStatus>: LittleEndian
},
SMSG_LIST_INVENTORY 0x19F {
    source: Guid<guid::Unit>: LittleEndian,
    query_result: VendorQueryResult: LittleEndian,
},
SMSG_BUY_ITEM 0x1A4 {
    vendor_guid: Guid<guid::Unit>: LittleEndian,
    vendor_slot: u32: LittleEndian,
    new_count: u32: LittleEndian,
    purchase_count: u32: LittleEndian,
},
SMSG_BUY_FAILED 0x1A5 {
    vendor_guid: Guid<guid::Unit>: LittleEndian,
    item_id: u32: LittleEndian,
    reason: u8: LittleEndian,
},
SMSG_TAXINODE_STATUS 0x1AB {
    npc_guid: Guid<guid::Unit>: LittleEndian,
    is_known: U8Bool: LittleEndian,
},
SMSG_TRAINER_LIST 0x1B1 {
    trainer_guid: Guid<guid::Unit>: LittleEndian,
    trainer_type: TrainerType: LittleEndian,
    // 1024 is arbitrary safe bound here, it can be increased up to ~20k but such high values are unreliable due to stack limits
    spells: Bounded<U32Counted<TrainerSpell>, 1024>: LittleEndian,
    greeting: Bounded<CString, 512>: LittleEndian,
},
SMSG_SHOW_BANK 0x1B8 {
    npc_guid: Guid<guid::Unit>: LittleEndian
},
SMSG_QUERY_TIME_RESPONSE 0x1CF {
    time: DateTime<Local>: LittleEndian
},
SMSG_PONG 0x1DD {
    sequence_id: u32: LittleEndian,
},
SMSG_AUTH_CHALLENGE 0x1EC {
    server_seed: u32: LittleEndian,
},
SMSG_AUTH_RESPONSE 0x1EE {
    result: AccountResult: LittleEndian,
},
MSG_TABARDVENDOR_ACTIVATE 0x1F2 {
    npc_guid: Guid<guid::Unit>: LittleEndian
},
SMSG_ACCOUNT_DATA_TIMES 0x209 {
    unkn: [u32; 32]: LittleEndian
},
SMSG_SET_REST_START 0x21E {
    unkn: u32: LittleEndian
},
SMSG_LOGIN_VERIFY_WORLD 0x236 {
    map: u32: LittleEndian,
    position_x: f32: LittleEndian,
    position_y: f32: LittleEndian,
    position_z: f32: LittleEndian,
    orientation: f32: LittleEndian,
},
SMSG_BATTLEFIELD_LIST 0x23D {
    battlemaster_guid: Guid<guid::Unit>: LittleEndian,
    map_id: u32: LittleEndian,
    bracket: u8: LittleEndian,
    instance_ids: U32Counted<u32>: LittleEndian,
},
MSG_AUCTION_HELLO 0x255 {
    auctioneer_guid: Guid<guid::Unit>: LittleEndian,
    auction_house_id: u32: LittleEndian,
},
MSG_LIST_STABLED_PETS 0x26F {
    stable_master_guid: Guid<guid::Unit>: LittleEndian,
    stabled_pets: StabledPets: LittleEndian,
},
);

pub trait OrderedWrite<T: ByteOrder> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()>
    where
        Self: Sized;

    fn serialized_size(&self) -> usize
    where
        Self: Sized,
    {
        let mut serialized = Vec::new();
        self.write(&mut serialized).unwrap();
        serialized.len()
    }
}

impl<T: ByteOrder> OrderedWrite<T> for u8 {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u8(*self)
    }

    fn serialized_size(&self) -> usize {
        size_of::<Self>()
    }
}

/// A bool sent as a single byte
pub struct U8Bool(pub bool);

impl From<bool> for U8Bool {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl<T: ByteOrder> OrderedWrite<T> for U8Bool {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u8(self.0 as u8)
    }

    fn serialized_size(&self) -> usize {
        size_of::<u8>()
    }
}

impl<T: ByteOrder> OrderedWrite<T> for u16 {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u16::<T>(*self)
    }

    fn serialized_size(&self) -> usize {
        size_of::<Self>()
    }
}

impl<T: ByteOrder> OrderedWrite<T> for u32 {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<T>(*self)
    }

    fn serialized_size(&self) -> usize {
        size_of::<Self>()
    }
}

impl<T: ByteOrder> OrderedWrite<T> for Option<NonZeroU32> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<T>(self.map_or(0, |v| v.get()))
    }

    fn serialized_size(&self) -> usize {
        size_of::<u32>()
    }
}

impl<T: ByteOrder> OrderedWrite<T> for f32 {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_f32::<T>(*self)
    }

    fn serialized_size(&self) -> usize {
        size_of::<Self>()
    }
}

impl<B: ByteOrder, T: OrderedWrite<B>, const N: usize> OrderedWrite<B> for [T; N] {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        for v in self {
            v.write(writer)?;
        }

        Ok(())
    }

    fn serialized_size(&self) -> usize {
        self.iter().map(OrderedWrite::<B>::serialized_size).sum()
    }
}

impl<T: ByteOrder, Tz: TimeZone> OrderedWrite<T> for DateTime<Tz> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        let year = self.year() as u32 - 2000;
        let v = year << 24
            | self.month() << 20
            | (self.day() - 1) << 14
            | self.weekday().num_days_from_sunday()
            | self.hour() << 6
            | self.minute();

        writer.write_u32::<T>(v)
    }

    fn serialized_size(&self) -> usize {
        size_of::<u32>()
    }
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum QuestStatus {
    Unavailable = 1,
    Chat = 2,
    Incomplete = 3,
    RewardRep = 4,
    Available = 5,
    Reward = 6,
    Reward2 = 7,
}

impl<T: ByteOrder> OrderedWrite<T> for QuestStatus {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<T>(*self as u32)
    }

    fn serialized_size(&self) -> usize {
        size_of::<u32>()
    }
}

impl<T: ByteOrder> OrderedWrite<T> for Option<QuestStatus> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        match self {
            Some(v) => <QuestStatus as OrderedWrite<T>>::write(v, writer),
            None => writer.write_u32::<T>(0),
        }
    }

    fn serialized_size(&self) -> usize {
        size_of::<u32>()
    }
}
