use std::{ffi::CString, io::ErrorKind, sync::Arc};

use chrono::Local;
use common::guid::{self, Guid};
use concurrent_queue::ConcurrentQueue;
use gameobjects::unit::StandStateType;
use log::{error, warn};
use packets::{
    client::{ClientPacket, ParseError},
    item_info::{ItemDamage, ItemFlags, ItemStat},
    movement_info::MovementInfo,
};
use tokio::{
    io::AsyncWriteExt,
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::Mutex,
};

use crate::game_data::GameDataAccessor;

pub async fn packet_handler(
    mut rx: OwnedReadHalf,
    tx: Arc<Mutex<OwnedWriteHalf>>,
    session_key: [u8; 40],
    character_id: Guid<guid::Player>,
    player_update_queue: Arc<ConcurrentQueue<PlayerUpdate>>,
    game_data_accessor: GameDataAccessor,
) {
    loop {
        let packet = match packets::client::read_packet(&mut rx, session_key).await {
            Ok(v) => v,
            Err(ParseError::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                let _ = player_update_queue.push(PlayerUpdate {
                    character_id: character_id,
                    data: PlayerUpdateData::ForceKick,
                });
                return;
            }
            Err(e) => {
                warn!(
                    "Failed to parse a packet from a client (character_id: {}). Error: {:?}",
                    character_id.get(),
                    e
                );
                continue;
            }
        };

        match packet {
            // Movement
            ClientPacket::MSG_MOVE_START_FORWARD(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_BACKWARD(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_STRAFE_LEFT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_STRAFE_RIGHT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP_STRAFE(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_JUMP(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_TURN_LEFT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_TURN_RIGHT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP_TURN(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_PITCH_UP(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_PITCH_DOWN(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP_PITCH(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_SET_RUN_MODE(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_SET_WALK_MODE(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_FALL_LAND(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_SWIM(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP_SWIM(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_SET_FACING(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_SET_PITCH(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_HEARTBEAT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement(packet.movement_info),
                }) {
                    return;
                };
            }
            // Movement processing end
            ClientPacket::CMSG_STANDSTATECHANGE(packet) => {
                let state = StandStateType::from_bits_non_const(packet.anim_state as u8);

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::SetAnimationState { state: state },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_SWAP_INV_ITEM(packet) => {
                let src_slot = parse_slot(packet.src_slot);
                let dst_slot = parse_slot(packet.dst_slot);

                dbg!(&src_slot, &dst_slot);

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::SwapInventoryItem {
                        src: src_slot,
                        dst: dst_slot,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_ITEM_QUERY_SINGLE(packet) => {
                let item_id = packet.item_id;

                let item_prototype = match game_data_accessor.get_item_prototype(item_id).await {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        continue;
                    }
                    Err(e) => {
                        error!(
                            "Failed to get query item prototype due to a DB error (item_id: {}, requesting character_id: {}). Error: {}",
                            item_id,
                            character_id.get(),
                            e
                        );
                        continue;
                    }
                };

                let response = packets::server::SMSG_ITEM_QUERY_SINGLE_RESPONSE {
                    item_id,
                    class: item_prototype.class,
                    sub_class: item_prototype.sub_class,
                    name_1: CString::new(item_prototype.name).unwrap(),
                    name_2: CString::new("").unwrap(),
                    name_3: CString::new("").unwrap(),
                    name_4: CString::new("").unwrap(),
                    display_info_id: item_prototype.display_info_id,
                    quality: item_prototype.quality,
                    flags: item_prototype.flags,
                    buy_price: item_prototype.buy_price,
                    sell_price: item_prototype.sell_price,
                    inventory_type: item_prototype.inventory_type,
                    allowable_class: item_prototype.allowable_class,
                    allowable_race: item_prototype.allowable_race,
                    item_level: item_prototype.item_level,
                    required_level: item_prototype.required_level,
                    required_skill: item_prototype.required_skill,
                    required_skill_rank: item_prototype.required_skill_rank,
                    required_spell: item_prototype.required_spell,
                    required_honor_rank: item_prototype.required_honor_rank,
                    required_city_rank: item_prototype.required_city_rank,
                    required_reputation_faction: item_prototype.required_reputation_faction,
                    required_reputation_rank: item_prototype.required_reputation_rank,
                    max_count: item_prototype.max_count,
                    stackable: item_prototype.stackable,
                    container_slots: item_prototype.container_slots,
                    item_stats: item_prototype.item_stats,
                    damage: item_prototype.damage,
                    armor: item_prototype.armor,
                    holy_resistance: item_prototype.holy_resistance,
                    fire_resistance: item_prototype.fire_resistance,
                    nature_resistance: item_prototype.nature_resistance,
                    frost_resistance: item_prototype.frost_resistance,
                    shadow_resistance: item_prototype.shadow_resistance,
                    arcane_resistance: item_prototype.arcane_resistance,
                    delay: item_prototype.delay,
                    ammo_type: item_prototype.ammo_type,
                    ranged_mod_range: item_prototype.ranged_mod_range,
                    spells: item_prototype.spells,
                    bonding: item_prototype.bonding,
                    description: CString::new(item_prototype.description).unwrap(),
                    page_text: item_prototype.page_text,
                    language_id: item_prototype.language_id,
                    page_material: item_prototype.page_material,
                    start_quest: item_prototype.start_quest,
                    lock_id: item_prototype.lock_id,
                    material: item_prototype.material,
                    sheath: item_prototype.sheath,
                    random_property: item_prototype.random_property,
                    block: item_prototype.block,
                    item_set: item_prototype.item_set,
                    max_durability: item_prototype.max_durability,
                    area: item_prototype.area,
                    map: item_prototype.map,
                    bag_family: item_prototype.bag_family,
                };

                let mut lock = tx.lock().await;

                if let Err(e) = lock.write_all(&response.to_bytes(Some(session_key))).await {
                    warn!(
                        "Failed to send SMSG_ITEM_QUERY_SINGLE_RESPONSE to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };
            }
            ClientPacket::CMSG_QUERY_TIME(_) => {
                let response = packets::server::SMSG_QUERY_TIME_RESPONSE { time: Local::now() };

                let mut lock = tx.lock().await;

                if let Err(e) = lock.write_all(&response.to_bytes(Some(session_key))).await {
                    warn!(
                        "Failed to send SMSG_PONG to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };
            }
            ClientPacket::CMSG_PING(packet) => {
                let response = packets::server::SMSG_PONG {
                    sequence_id: packet.sequence_id,
                };

                let mut lock = tx.lock().await;

                if let Err(e) = lock.write_all(&response.to_bytes(Some(session_key))).await {
                    warn!(
                        "Failed to send SMSG_PONG to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };
            }
            _ => {
                warn!(
                    "Client (character_id: {}) tried to send a packet in a wrong state (current state: game world). Packet: {:?}",
                    character_id.get(),
                    packet
                );
            }
        }
    }
}

pub struct PlayerUpdate {
    pub character_id: Guid<guid::Player>,
    pub data: PlayerUpdateData,
}

pub enum PlayerUpdateData {
    Movement(MovementInfo),
    SwapInventoryItem { src: Slot, dst: Slot },
    SetAnimationState { state: StandStateType },
    // Kicks the client by abruptly dropping their connection, usually due to an error in reading client's packets
    ForceKick,
}

fn parse_slot(slot: u8) -> Slot {
    dbg!(slot);
    match slot {
        23..=38 => Slot::MainBag(slot - 23),
        _ => todo!(),
    }
}

#[derive(Debug)]
pub enum Slot {
    MainBag(u8),
}
