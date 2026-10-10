use std::time::{Duration, Instant};

use common::guid::LivingGuid;
use gameobjects::unit::UnitFields;
use packets::attacker_state::{DamageSchool, HitHand, HitInfo, SubDamage, SubDamages, VictimState};

use crate::server::Server;

impl Server {
    pub(super) async fn update_player_melee_attacks(&mut self, now: Instant) {
        let mut swings = Vec::new();

        for (guid, character) in &mut self.characters {
            let Some(melee_state) = &mut character.melee_state else {
                continue;
            };
            let Some(victim) = melee_state.victim else {
                continue;
            };

            // TODO: check that the player can attack (not dead, stunned, pacified, casting, etc.)
            // TODO: range check, send SMSG_ATTACKSWING_NOTINRANGE without consuming the swing
            // TODO: facing check, send SMSG_ATTACKSWING_BADFACING without consuming the swing

            let main_attack_time =
                Duration::from_millis(*character.unit_fields.base_attack_time.get() as u64);
            if melee_state
                .last_main_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                melee_state.last_main_hand = Some(now);
                swings.push((LivingGuid::Player(*guid), victim, HitHand::Main));
            }

            /* Requires a check that offhand item is present
            let main_attack_time =
                Duration::from_millis(*character.unit_fields.offhand_attack_time.get() as u64);
            if melee_state
                .last_off_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                melee_state.last_off_hand = Some(now);
                swings.push((LivingGuid::Player(*guid), victim, HitHand::Off));
            }
            */
        }

        for (attacker, victim, hand) in swings {
            self.resolve_melee_swing(attacker, victim, hand).await;
        }
    }

    pub(super) async fn update_creature_melee_attacks(&mut self, now: Instant) {
        let mut swings = Vec::new();

        for (guid, creature) in &mut self.creatures {
            let Some(melee_state) = &mut creature.melee_state else {
                continue;
            };
            let Some(victim) = melee_state.victim else {
                continue;
            };

            // TODO: check that the creature can attack (not dead, stunned, evading, casting, etc.)
            // TODO: range check, the creature should chase instead of swinging
            // TODO: facing check, the creature should turn to the victim instead of swinging

            let main_attack_time =
                Duration::from_millis(*creature.unit_fields.base_attack_time.get() as u64);
            if melee_state
                .last_main_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                melee_state.last_main_hand = Some(now);
                swings.push((LivingGuid::Unit(*guid), victim, HitHand::Main));
            }

            /* Requires a check that offhand item is present
            let main_attack_time =
                Duration::from_millis(*creature.unit_fields.offhand_attack_time.get() as u64);
            if melee_state
                .last_off_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                melee_state.last_off_hand = Some(now);
                swings.push((LivingGuid::Unit(*guid), victim, HitHand::Off));
            }
            */
        }

        for (attacker, victim, hand) in swings {
            self.resolve_melee_swing(attacker, victim, hand).await;
        }
    }

    async fn resolve_melee_swing(
        &mut self,
        attacker: LivingGuid,
        victim: LivingGuid,
        hand: HitHand,
    ) {
        // An earlier swing this tick could have killed the attacker
        if self
            .living_unit_fields(attacker)
            .is_none_or(|fields| *fields.health.get() == 0)
        {
            return;
        }

        let Some(victim_fields) = self.living_unit_fields_mut(victim) else {
            // TODO: victim is gone (logged out, despawned), stop the attack with SMSG_ATTACKSTOP
            return;
        };

        if *victim_fields.health.get() == 0 {
            // TODO: SMSG_ATTACKSWING_DEADTARGET for players, drop the victim for creatures
            return;
        }

        // TODO: check that the victim can be attacked (not immune, not evading, faction, etc.)

        // TODO: roll the hit table (miss/dodge/parry/block/crit/glancing/crushing) and calculate the damage
        let damage = 1;
        let hit_info = HitInfo::new()
            .with_affects_victim(true)
            .with_off_hand(matches!(hand, HitHand::Off));
        let victim_state = VictimState::Normal;

        let response = packets::server::SMSG_ATTACKERSTATEUPDATE {
            hit_info,
            attacker: attacker.into(),
            victim: victim.into(),
            total_damage: damage,
            sub_damages: SubDamages(vec![SubDamage {
                school: DamageSchool::Physical,
                damage: damage as f32,
                int_damage: damage,
                absorbed: 0,
                resisted: 0,
            }]),
            victim_state,
            unkn: 0,
            spell_id: None,
            blocked: 0,
        };

        let _ = victim_fields;

        self.broadcast_packet(&response, |character| {
            !character.sees_including_self(attacker) && !character.sees_including_self(victim)
        })
        .await;

        let Some(victim_fields) = self.living_unit_fields_mut(victim) else {
            // Not really supposed to happen, I guess returning is fine for now
            return;
        };

        let health = *victim_fields.health.get();
        *victim_fields.health.get_mut_using_copy() = health.saturating_sub(damage);
        // TODO: death handling once health reaches 0
    }

    fn living_unit_fields(&self, guid: LivingGuid) -> Option<&UnitFields> {
        match guid {
            LivingGuid::Player(guid) => self.characters.get(&guid).map(|v| &v.unit_fields),
            LivingGuid::Unit(guid) => self.creatures.get(&guid).map(|v| &v.unit_fields),
        }
    }

    fn living_unit_fields_mut(&mut self, guid: LivingGuid) -> Option<&mut UnitFields> {
        match guid {
            LivingGuid::Player(guid) => self.characters.get_mut(&guid).map(|v| &mut v.unit_fields),
            LivingGuid::Unit(guid) => self.creatures.get_mut(&guid).map(|v| &mut v.unit_fields),
        }
    }
}
