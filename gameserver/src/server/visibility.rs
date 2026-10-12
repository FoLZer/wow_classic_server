use std::collections::HashMap;

use bit_vec::BitVec;
use common::guid::AnyGuid;
use gameobjects::tracked_field::ClientUpdatable;
use log::{error, warn};
use packets::{
    movement_info::{MovementFlags, MovementInfo},
    server::{MAX_SERVER_PACKET_BODY_SIZE, ServerPacket},
    update_data::{
        MovementUpdate, PositionUpdate, PossibleUpdate, UpdateBlocks, UpdateData, ValuesUpdate,
    },
};
use tokio::io::AsyncWriteExt;

use crate::{
    objects::{character::Character, creature::Creature, creature_corpse::CreatureCorpse},
    server::Server,
};

fn build_creature_update_block(
    object_fields: &mut gameobjects::object::ObjectFields<common::guid::Unit>,
    unit_fields: &mut gameobjects::unit::UnitFields,
) -> Option<UpdateData> {
    if !object_fields.has_updates() && !unit_fields.has_updates() {
        return None;
    }

    let mut mask_blocks = BitVec::new();
    let mut values_blocks = Vec::new();

    object_fields.write_update_block(&mut mask_blocks, &mut values_blocks);
    unit_fields.write_update_block(&mut mask_blocks, &mut values_blocks);

    if values_blocks.is_empty() {
        return None;
    }

    object_fields.clear_update_flags();
    unit_fields.clear_update_flags();

    Some(UpdateData::UpdateObject {
        guid: AnyGuid::Unit(*object_fields.guid.get()),
        values: ValuesUpdate {
            mask_blocks,
            values_blocks,
        },
    })
}

impl Server {
    // Players
    pub(super) async fn send_player_updates_to_players(&mut self) {
        let mut update_blocks = HashMap::new();
        let dirty_characters = self.characters.take_dirty();

        for guid in dirty_characters {
            let Some(mut character) = self.characters.get_mut(&guid) else {
                continue;
            };
            let mut mask_blocks = BitVec::new();
            let mut values_blocks = Vec::new();

            character
                .object_fields
                .write_update_block(&mut mask_blocks, &mut values_blocks);
            character
                .unit_fields
                .write_update_block(&mut mask_blocks, &mut values_blocks);
            character
                .player_fields
                .write_update_block(&mut mask_blocks, &mut values_blocks);

            if values_blocks.is_empty() {
                continue;
            }

            character.object_fields.clear_update_flags();
            character.unit_fields.clear_update_flags();
            character.player_fields.clear_update_flags();

            let block = UpdateData::UpdateObject {
                guid: AnyGuid::Player(character.object_fields.guid.get().clone()),
                values: ValuesUpdate {
                    mask_blocks,
                    values_blocks,
                },
            };

            update_blocks.insert(character.object_fields.guid.get().clone(), block);
        }

        for character in self.characters.values() {
            let mut blocks = Vec::new();

            for (guid, block) in &update_blocks {
                if character.sees_player_including_self(*guid) {
                    blocks.push(block.clone());
                }
            }

            if blocks.is_empty() {
                continue;
            }

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks,
                },
            };

            let mut lock = character.stream_tx.lock().await;

            if let Err(e) = lock
                .write_all(&response.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                error!(
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };
        }
    }

    pub(super) async fn remove_invisible_players_for_other_players(&mut self) {
        let mut guids_to_remove_for_character = HashMap::new();

        for character in self.characters.values() {
            let mut guids_to_remove = Vec::new();

            for guid in &character.other_visible_players {
                let Some(other_character) = self.characters.get(guid) else {
                    warn!(
                        "Character got removed but was not cleaned up, removing. (character still in view of other characters, from: {}, to: {})",
                        character.object_fields.guid.get().get_u32(),
                        guid.get_u32()
                    );
                    guids_to_remove.push(*guid);
                    continue;
                };

                if !self.is_character_visible(&character, other_character) {
                    guids_to_remove.push(*guid);
                }
            }

            if !guids_to_remove.is_empty() {
                guids_to_remove_for_character
                    .insert(*character.object_fields.guid.get(), guids_to_remove);
            }
        }

        for (guid, guids_to_remove) in guids_to_remove_for_character {
            let Some(mut character) = self.characters.get_mut(&guid) else {
                // Not possible due to how this map got constructed above
                unreachable!();
            };

            let block = UpdateData::OutOfRangeDestroyObject {
                guids: guids_to_remove
                    .iter()
                    .map(|v| AnyGuid::Player(*v))
                    .collect(),
            };

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks: vec![block],
                },
            };

            {
                let mut lock = character.stream_tx.lock().await;

                if let Err(e) = lock
                    .write_all(&response.to_bytes(
                        Some(character.session_key),
                        &mut *character.encrypt_data.lock().await,
                    ))
                    .await
                {
                    error!(
                        "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                        character.account_id, e
                    );
                    return;
                };
            }

            character
                .other_visible_players
                .retain(|v| !guids_to_remove.contains(v));
        }
    }

    pub(super) async fn create_new_players_for_players(&mut self) {
        let mut new_visible_characters_for_characters = HashMap::new();
        for (guid, character) in &self.characters {
            let mut new_visible_characters = Vec::new();

            for (guid, potentially_new_character) in &self.characters {
                if !character.sees_player_including_self(*guid)
                    && self.is_character_visible(character, potentially_new_character)
                {
                    new_visible_characters.push(*guid);
                }
            }

            new_visible_characters_for_characters.insert(*guid, new_visible_characters);
        }

        // Potential for a cache here but I don't think that many people are going to become visible in a single tick
        // It is a possibility but will depend on is_character_visible() function being split on maps or not

        for (guid, new_visible_characters) in new_visible_characters_for_characters {
            let mut blocks = Vec::new();

            for guid in &new_visible_characters {
                let Some(character) = self.characters.get(&guid) else {
                    // Not possible due to how this vec got constructed above
                    unreachable!();
                };

                let mut mask_blocks = BitVec::new();
                let mut values_blocks = Vec::new();

                character
                    .object_fields
                    .write_full_update_block(&mut mask_blocks, &mut values_blocks);
                character
                    .unit_fields
                    .write_full_update_block(&mut mask_blocks, &mut values_blocks);
                character
                    .player_fields
                    .write_full_update_block(&mut mask_blocks, &mut values_blocks);

                blocks.push(UpdateData::CreateNewObject {
                    guid: AnyGuid::Player(character.object_fields.guid.get().clone()),
                    movement: MovementUpdate {
                        is_self_update: false,
                        position: Some(PositionUpdate::Living {
                            movement_info: MovementInfo {
                                movement_flags: MovementFlags::new(),
                                timestamp: 0,
                                pos_x: character.position.0,
                                pos_y: character.position.1,
                                pos_z: character.position.2,
                                orientation: character.orientation,
                                on_transport_data: None,
                                swimming_pitch: None,
                                fall_time: 0,
                                falling_data: None,
                                spline_elevation: None,
                            },
                            walk_speed: 1.0,
                            run_speed: 80.0,
                            run_backwards_speed: 4.5,
                            swim_speed: 4.722222,
                            swim_backwards_speed: 2.5,
                            turn_speed: std::f32::consts::PI,
                        }),
                        high_guid: None,
                        is_update_all: true,
                        full_guid: PossibleUpdate::NoUpdate,
                        transport_time_millis: None,
                    },
                    values: ValuesUpdate {
                        mask_blocks,
                        values_blocks,
                    },
                });
            }

            if blocks.is_empty() {
                continue;
            }

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks,
                },
            };

            let Some(mut character) = self.characters.get_mut(&guid) else {
                // Not possible due to how this map got constructed above
                unreachable!();
            };

            {
                let mut lock = character.stream_tx.lock().await;

                if let Err(e) = lock
                    .write_all(&response.to_bytes(
                        Some(character.session_key),
                        &mut *character.encrypt_data.lock().await,
                    ))
                    .await
                {
                    error!(
                        "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                        character.account_id, e
                    );
                    return;
                };
            }

            for guid in new_visible_characters {
                character.other_visible_players.insert(guid);
            }
        }
    }

    // Creatures
    pub(super) async fn send_creature_updates_to_players(&mut self) {
        if self.characters.is_empty() {
            return;
        }

        let mut update_blocks = HashMap::new();
        let mut dirty_creatures = self.creatures.take_dirty();
        dirty_creatures.extend(self.creature_corpses.take_dirty());

        for guid in dirty_creatures {
            let block = if let Some(mut creature) = self.creatures.get_mut(&guid) {
                let creature = &mut *creature;
                build_creature_update_block(&mut creature.object_fields, &mut creature.unit_fields)
            } else if let Some(mut corpse) = self.creature_corpses.get_mut(&guid) {
                let corpse = &mut *corpse;
                build_creature_update_block(&mut corpse.object_fields, &mut corpse.unit_fields)
            } else {
                continue;
            };

            let Some(block) = block else {
                continue;
            };

            update_blocks.insert(guid, block);
        }

        for character in self.characters.values() {
            let blocks = character
                .visible_creatures
                .iter()
                .filter_map(|guid| update_blocks.get(guid).cloned())
                .collect::<Vec<_>>();

            if blocks.is_empty() {
                continue;
            }

            let batches = UpdateBlocks {
                has_transport: false,
                blocks,
            }
            .into_sized_batches(MAX_SERVER_PACKET_BODY_SIZE);

            let mut lock = character.stream_tx.lock().await;

            for update_data in batches {
                let response = packets::server::SMSG_UPDATE_OBJECT { update_data };

                if let Err(e) = lock
                    .write_all(&response.to_bytes(
                        Some(character.session_key),
                        &mut *character.encrypt_data.lock().await,
                    ))
                    .await
                {
                    error!(
                        "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                        character.account_id, e
                    );
                    break;
                };
            }
        }
    }

    pub(super) async fn remove_invisible_creatures_for_players(&mut self) {
        let mut guids_to_remove_for_character = HashMap::new();

        for character in self.characters.values() {
            let mut guids_to_remove = Vec::new();

            for guid in &character.visible_creatures {
                let is_visible = if let Some(creature) = self.creatures.get(guid) {
                    self.is_creature_visible_to_player(&character, creature)
                } else if let Some(corpse) = self.creature_corpses.get(guid) {
                    self.is_corpse_visible_to_player(&character, corpse)
                } else {
                    warn!(
                        "Creature got removed but was not cleaned up, removing. (creature still in view of other characters, from: {}, to: {})",
                        character.object_fields.guid.get().get_u32(),
                        guid.get_u32()
                    );
                    guids_to_remove.push(*guid);
                    continue;
                };

                if !is_visible {
                    guids_to_remove.push(*guid);
                }
            }

            if !guids_to_remove.is_empty() {
                guids_to_remove_for_character
                    .insert(*character.object_fields.guid.get(), guids_to_remove);
            }
        }

        for (guid, guids_to_remove) in guids_to_remove_for_character {
            let Some(mut character) = self.characters.get_mut(&guid) else {
                // Not possible due to how this map got constructed above
                unreachable!();
            };

            let block = UpdateData::OutOfRangeDestroyObject {
                guids: guids_to_remove.iter().map(|v| AnyGuid::Unit(*v)).collect(),
            };

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks: vec![block],
                },
            };

            {
                let mut lock = character.stream_tx.lock().await;

                if let Err(e) = lock
                    .write_all(&response.to_bytes(
                        Some(character.session_key),
                        &mut *character.encrypt_data.lock().await,
                    ))
                    .await
                {
                    error!(
                        "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                        character.account_id, e
                    );
                    return;
                };
            }

            character
                .visible_creatures
                .retain(|v| !guids_to_remove.contains(v));
        }
    }

    pub(super) async fn create_new_creatures_for_players(&mut self) {
        let mut new_visible_creatures_for_characters = HashMap::new();
        for (guid, character) in &self.characters {
            let mut new_visible_characters = Vec::new();

            for (guid, potentially_new_creature) in self.creatures.iter() {
                if !character.sees_creature(*guid)
                    && self.is_creature_visible_to_player(character, potentially_new_creature)
                {
                    new_visible_characters.push(*guid);
                }
            }

            for (guid, potentially_new_corpse) in self.creature_corpses.iter() {
                if !character.sees_creature(*guid)
                    && self.is_corpse_visible_to_player(character, potentially_new_corpse)
                {
                    new_visible_characters.push(*guid);
                }
            }

            new_visible_creatures_for_characters.insert(*guid, new_visible_characters);
        }

        // Potential for a cache here but I don't think that many people are going to become visible in a single tick
        // It is a possibility but will depend on is_character_visible() function being split on maps or not

        for (guid, new_visible_creatures) in new_visible_creatures_for_characters {
            let mut blocks = Vec::new();

            for guid in &new_visible_creatures {
                let mut mask_blocks = BitVec::new();
                let mut values_blocks = Vec::new();

                let (position, orientation, object_guid) =
                    if let Some(mut creature) = self.creatures.get_mut(&guid) {
                        let creature = &mut *creature;
                        creature
                            .object_fields
                            .write_full_update_block(&mut mask_blocks, &mut values_blocks);
                        creature
                            .unit_fields
                            .write_full_update_block(&mut mask_blocks, &mut values_blocks);
                        (
                            creature.position,
                            creature.orientation,
                            *creature.object_fields.guid.get(),
                        )
                    } else if let Some(mut corpse) = self.creature_corpses.get_mut(&guid) {
                        let corpse = &mut *corpse;
                        corpse
                            .object_fields
                            .write_full_update_block(&mut mask_blocks, &mut values_blocks);
                        corpse
                            .unit_fields
                            .write_full_update_block(&mut mask_blocks, &mut values_blocks);
                        (
                            corpse.position,
                            corpse.orientation,
                            *corpse.object_fields.guid.get(),
                        )
                    } else {
                        // Not possible due to how this vec got constructed above
                        unreachable!();
                    };

                blocks.push(UpdateData::CreateNewObject {
                    guid: AnyGuid::Unit(object_guid),
                    movement: MovementUpdate {
                        is_self_update: false,
                        position: Some(PositionUpdate::Living {
                            movement_info: MovementInfo {
                                movement_flags: MovementFlags::new(),
                                timestamp: 0,
                                pos_x: position.0,
                                pos_y: position.1,
                                pos_z: position.2,
                                orientation,
                                on_transport_data: None,
                                swimming_pitch: None,
                                fall_time: 0,
                                falling_data: None,
                                spline_elevation: None,
                            },
                            walk_speed: 1.0,
                            run_speed: 80.0,
                            run_backwards_speed: 4.5,
                            swim_speed: 4.722222,
                            swim_backwards_speed: 2.5,
                            turn_speed: std::f32::consts::PI,
                        }),
                        high_guid: None,
                        is_update_all: true,
                        full_guid: PossibleUpdate::NoUpdate,
                        transport_time_millis: None,
                    },
                    values: ValuesUpdate {
                        mask_blocks,
                        values_blocks,
                    },
                });
            }

            let batches = UpdateBlocks {
                has_transport: false,
                blocks,
            }
            .into_sized_batches(MAX_SERVER_PACKET_BODY_SIZE);

            let Some(mut character) = self.characters.get_mut(&guid) else {
                // Not possible due to how this map got constructed above
                unreachable!();
            };

            {
                let mut lock = character.stream_tx.lock().await;

                for update_data in batches {
                    let response = packets::server::SMSG_UPDATE_OBJECT { update_data };

                    if let Err(e) = lock
                        .write_all(&response.to_bytes(
                            Some(character.session_key),
                            &mut *character.encrypt_data.lock().await,
                        ))
                        .await
                    {
                        error!(
                            "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                            character.account_id, e
                        );
                        return;
                    };
                }
            }

            for guid in new_visible_creatures {
                character.visible_creatures.insert(guid);
            }
        }
    }

    fn is_character_visible(&self, from: &Character, to: &Character) -> bool {
        true // TODO
    }

    fn is_creature_visible_to_player(&self, from: &Character, to: &Creature) -> bool {
        Self::is_in_visibility_distance(from.position, to.position)
        // TODO
    }

    fn is_corpse_visible_to_player(&self, from: &Character, to: &CreatureCorpse) -> bool {
        Self::is_in_visibility_distance(from.position, to.position)
        // TODO
    }

    fn is_in_visibility_distance(from: (f32, f32, f32), to: (f32, f32, f32)) -> bool {
        let dist = (from.0 - to.0).powi(2) + (from.1 - to.1).powi(2) + (from.2 - to.2).powi(2);

        dist < 20000.0
    }
}
