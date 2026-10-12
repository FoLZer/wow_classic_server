use std::time::Instant;

use common::guid::{self, AnyGuid, Guid};
use log::error;

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

        self.remove_creatures(&despawned, now).await;
    }

    pub(super) fn release_quarantined_guids(&mut self, now: Instant) {
        for (guid, e) in self
            .unit_guid_allocator
            .release_quarantined(self.guid_quarantine_time, now)
        {
            error!(
                "Failed to free a quarantined unit guid ({}). Error: {:?}",
                guid.get(),
                e
            );
        }
    }

    async fn remove_creatures(&mut self, creatures: &[Guid<guid::Unit>], now: Instant) {
        let character_guids = self.characters.keys().copied().collect::<Vec<_>>();
        for character_guid in character_guids {
            let Some(mut character) = self.characters.get_mut(&character_guid) else {
                continue;
            };
            for guid in creatures {
                character.visible_creatures.remove(guid);
            }
        }

        for &guid in creatures {
            let response = packets::server::SMSG_DESTROY_OBJECT {
                guid: AnyGuid::Unit(guid),
            };
            self.broadcast_packet(&response, |_| false).await;

            self.unit_guid_allocator.quarantine(guid, now);
        }
    }
}
