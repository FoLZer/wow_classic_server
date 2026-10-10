use std::time::Instant;

use common::guid::{self, AnyGuid, Guid};
use log::error;
use packets::{
    server::ServerPacket,
    update_data::{UpdateBlocks, UpdateData},
};
use tokio::io::AsyncWriteExt;

use crate::server::Server;

impl Server {
    pub(super) async fn despawn_corpses(&mut self, now: Instant) {
        let despawned: Vec<_> = self
            .creature_corpses
            .extract_if(|_, corpse| (now - corpse.died_at) >= self.corpse_despawn_time)
            .map(|(guid, _)| guid)
            .collect();

        if despawned.is_empty() {
            return;
        }

        self.remove_creatures(&despawned).await;
    }

    async fn remove_creatures(&mut self, creatures: &[Guid<guid::Unit>]) {
        for character in self.characters.values_mut() {
            let guids: Vec<_> = creatures
                .iter()
                .filter(|guid| character.visible_creatures.remove(guid))
                .map(|guid| AnyGuid::Unit(*guid))
                .collect();

            if guids.is_empty() {
                continue;
            }

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks: vec![UpdateData::OutOfRangeDestroyObject { guids }],
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
}
