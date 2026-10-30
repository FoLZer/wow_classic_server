use std::{
    ops::{Deref, DerefMut},
    time::{Duration, Instant},
};

use common::guid::{self, Guid, LivingGuid};
use gameobjects::unit::UnitFields;
use packets::attacker_state::{DamageSchool, HitHand, HitInfo, SubDamage, SubDamages, VictimState};
use rand::{RngExt, rng};

use crate::{
    dirty_tracking_map::DirtyTrackingMut,
    map_instance::InstanceId,
    objects::{character::Character, creature::Creature, creature_corpse::CreatureCorpse},
    server::Server,
};

const SWING_ERROR_DELAY: Duration = Duration::from_secs(1);

impl Server {
    pub(super) async fn update_player_melee_attacks(&mut self, now: Instant) {
        let mut swings = Vec::new();

        let character_guids = self.characters.keys().copied().collect::<Vec<_>>();
        for guid in character_guids {
            let Some(mut character) = self.characters.get_mut(&guid) else {
                continue;
            };
            let main_attack_time =
                Duration::from_millis(*character.unit_fields.base_attack_time.get() as u64);
            let Some(melee_state) = &mut character.melee_state else {
                continue;
            };
            let Some(victim) = melee_state.victim else {
                continue;
            };

            // TODO: check that the player can attack (not dead, stunned, pacified, casting, etc.)
            // TODO: facing check, send SMSG_ATTACKSWING_BADFACING without consuming the swing

            if melee_state
                .last_main_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                let report_swing_error = melee_state
                    .last_swing_error
                    .is_none_or(|last| now.duration_since(last) >= SWING_ERROR_DELAY);
                swings.push((
                    LivingGuid::Player(guid),
                    victim,
                    HitHand::Main,
                    report_swing_error,
                ));
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

        for (attacker, victim, hand, report_swing_error) in swings {
            if !self.has_current_melee_victim(attacker, victim) {
                continue;
            }

            let result = self
                .resolve_melee_swing(attacker, victim, hand, report_swing_error)
                .await;
            if let LivingGuid::Player(guid) = attacker {
                let mut character = self
                    .characters
                    .get_mut(&guid)
                    .expect("player attacker checked above");
                let melee_state = character
                    .melee_state
                    .as_mut()
                    .expect("player melee state checked above");

                match result {
                    MeleeSwingResult::Resolved => {
                        match hand {
                            HitHand::Main => melee_state.last_main_hand = Some(now),
                            HitHand::Off => melee_state.last_off_hand = Some(now),
                        }
                        melee_state.last_swing_error = None;
                    }
                    MeleeSwingResult::OutOfRange if report_swing_error => {
                        melee_state.last_swing_error = Some(now);
                    }
                    MeleeSwingResult::OutOfRange | MeleeSwingResult::Failed => {}
                }
            }
        }
    }

    pub(super) async fn update_creature_melee_attacks(&mut self, now: Instant) {
        let mut swings = Vec::new();

        let due_swings = self
            .creatures
            .iter()
            .filter_map(|(guid, creature)| {
                let melee_state = creature.melee_state.as_ref()?;
                let victim = melee_state.victim?;
                let main_attack_time =
                    Duration::from_millis(*creature.unit_fields.base_attack_time.get() as u64);

                melee_state
                    .last_main_hand
                    .is_none_or(|last| now.duration_since(last) >= main_attack_time)
                    .then_some((*guid, victim))
            })
            .collect::<Vec<_>>();

        for (guid, victim) in due_swings {
            // TODO: check that the creature can attack (not dead, stunned, evading, casting, etc.)
            // TODO: range check, the creature should chase instead of swinging
            // TODO: facing check, the creature should turn to the victim instead of swinging

            swings.push((LivingGuid::Unit(guid), victim, HitHand::Main));

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
            if !self.has_current_melee_victim(attacker, victim) {
                continue;
            }

            if self
                .resolve_melee_swing(attacker, victim, hand, false)
                .await
                == MeleeSwingResult::Resolved
                && let LivingGuid::Unit(guid) = attacker
            {
                let mut creature = self
                    .creatures
                    .get_mut(&guid)
                    .expect("creature attacker checked above");
                let melee_state = creature
                    .melee_state
                    .as_mut()
                    .expect("creature melee state checked above");

                match hand {
                    HitHand::Main => melee_state.last_main_hand = Some(now),
                    HitHand::Off => melee_state.last_off_hand = Some(now),
                }
            }
        }
    }

    async fn resolve_melee_swing(
        &mut self,
        attacker: LivingGuid,
        victim: LivingGuid,
        hand: HitHand,
        report_swing_error: bool,
    ) -> MeleeSwingResult {
        // An earlier swing this tick could have killed the attacker
        if self
            .living_unit_fields(attacker)
            .is_none_or(|fields| *fields.health.get() == 0)
        {
            return MeleeSwingResult::Failed;
        }

        let Some(attacker_instance) = self.living_unit_instance(attacker) else {
            return MeleeSwingResult::Failed;
        };
        if self.living_unit_instance(victim) != Some(attacker_instance) {
            return MeleeSwingResult::Failed;
        }

        let Some((attacker_position, attacker_reach)) =
            self.living_unit_position_and_reach(attacker)
        else {
            return MeleeSwingResult::Failed;
        };
        let Some((victim_position, victim_reach)) = self.living_unit_position_and_reach(victim)
        else {
            return MeleeSwingResult::Failed;
        };
        let melee_range = (attacker_reach + victim_reach + 4.0 / 3.0).max(5.0);
        let dx = attacker_position.0 - victim_position.0;
        let dy = attacker_position.1 - victim_position.1;
        let dz = attacker_position.2 - victim_position.2;
        let distance_squared = dx * dx + dy * dy + dz * dz;

        if distance_squared > melee_range * melee_range {
            if report_swing_error && let LivingGuid::Player(attacker_guid) = attacker {
                let response = packets::server::SMSG_ATTACKSWING_NOTINRANGE {};
                self.broadcast_packet(&response, |character| character.guid() != attacker_guid)
                    .await;
            }
            return MeleeSwingResult::OutOfRange;
        }

        let Some(damage) = self.roll_melee_damage(attacker, hand) else {
            return MeleeSwingResult::Failed;
        };

        let Some(victim_fields) = self.living_unit_fields_mut(victim) else {
            // TODO: victim is gone (logged out, despawned), stop the attack with SMSG_ATTACKSTOP
            return MeleeSwingResult::Failed;
        };

        if *victim_fields.health.get() == 0 {
            // TODO: SMSG_ATTACKSWING_DEADTARGET for players, drop the victim for creatures
            return MeleeSwingResult::Failed;
        }

        // TODO: check that the victim can be attacked (not immune, not evading, faction, etc.)

        // TODO: roll the hit table (miss/dodge/parry/block/crit/glancing/crushing)
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

        drop(victim_fields);

        self.broadcast_packet(&response, |character| {
            !character.sees_including_self(attacker) && !character.sees_including_self(victim)
        })
        .await;

        let Some(mut victim_fields) = self.living_unit_fields_mut(victim) else {
            // Not really supposed to happen, I guess returning is fine for now
            return MeleeSwingResult::Failed;
        };

        let health = *victim_fields.health.get();
        *victim_fields.health.get_mut_using_copy() = health.saturating_sub(damage);
        let victim_died = *victim_fields.health.get() == 0;
        drop(victim_fields);

        if victim_died {
            if let LivingGuid::Unit(victim_guid) = victim {
                let Some(victim) = self.creatures.remove(&victim_guid) else {
                    // Not really supposed to happen, I guess returning is fine for now
                    return MeleeSwingResult::Failed;
                };
                if let Some(spawner_index) = victim.spawner_index {
                    self.notify_creature_spawner(victim.instance_id, spawner_index);
                }
                self.creature_corpses.insert(
                    victim_guid,
                    CreatureCorpse {
                        instance_id: victim.instance_id,
                        position: victim.position,
                        orientation: victim.orientation,
                        object_fields: victim.object_fields,
                        unit_fields: victim.unit_fields,
                        died_at: Instant::now(),
                    },
                );
            }
        }
        // TODO: death handling once health reaches 0
        MeleeSwingResult::Resolved
    }

    fn has_current_melee_victim(&self, attacker: LivingGuid, victim: LivingGuid) -> bool {
        match attacker {
            LivingGuid::Player(guid) => self.characters.get(&guid).is_some_and(|character| {
                character
                    .melee_state
                    .as_ref()
                    .is_some_and(|state| state.victim == Some(victim))
            }),
            LivingGuid::Unit(guid) => self.creatures.get(&guid).is_some_and(|creature| {
                creature
                    .melee_state
                    .as_ref()
                    .is_some_and(|state| state.victim == Some(victim))
            }),
        }
    }

    fn living_unit_position_and_reach(&self, guid: LivingGuid) -> Option<((f32, f32, f32), f32)> {
        match guid {
            LivingGuid::Player(guid) => self.characters.get(&guid).map(|character| {
                (
                    character.position,
                    *character.unit_fields.combat_reach.get(),
                )
            }),
            LivingGuid::Unit(guid) => self
                .creatures
                .get(&guid)
                .map(|creature| (creature.position, *creature.unit_fields.combat_reach.get())),
        }
    }

    fn living_unit_instance(&self, guid: LivingGuid) -> Option<InstanceId> {
        match guid {
            LivingGuid::Player(guid) => self
                .characters
                .get(&guid)
                .map(|character| character.instance_id()),
            LivingGuid::Unit(guid) => self
                .creatures
                .get(&guid)
                .map(|creature| creature.instance_id),
        }
    }

    fn roll_melee_damage(&self, attacker: LivingGuid, hand: HitHand) -> Option<u32> {
        let fields = self.living_unit_fields(attacker)?;
        let (min_damage, max_damage) = match hand {
            HitHand::Main => (fields.min_damage.get(), fields.max_damage.get()),
            HitHand::Off => (
                fields.min_offhand_damage.get(),
                fields.max_offhand_damage.get(),
            ),
        };
        let min_damage = min_damage.into_inner() as u32;
        let max_damage = max_damage.into_inner() as u32;

        Some(rng().random_range(min_damage.min(max_damage)..=max_damage))
    }

    fn living_unit_fields(&self, guid: LivingGuid) -> Option<&UnitFields> {
        match guid {
            LivingGuid::Player(guid) => self.characters.get(&guid).map(|v| &v.unit_fields),
            LivingGuid::Unit(guid) => self.creatures.get(&guid).map(|v| &v.unit_fields),
        }
    }

    fn living_unit_fields_mut(&mut self, guid: LivingGuid) -> Option<LivingUnitFieldsMut<'_>> {
        match guid {
            LivingGuid::Player(guid) => self
                .characters
                .get_mut(&guid)
                .map(LivingUnitFieldsMut::Player),
            LivingGuid::Unit(guid) => self
                .creatures
                .get_mut(&guid)
                .map(LivingUnitFieldsMut::Creature),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MeleeSwingResult {
    Resolved,
    OutOfRange,
    Failed,
}

enum LivingUnitFieldsMut<'a> {
    Player(DirtyTrackingMut<'a, Guid<guid::Player>, Character>),
    Creature(DirtyTrackingMut<'a, Guid<guid::Unit>, Creature>),
}

impl Deref for LivingUnitFieldsMut<'_> {
    type Target = UnitFields;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Player(character) => &character.unit_fields,
            Self::Creature(creature) => &creature.unit_fields,
        }
    }
}

impl DerefMut for LivingUnitFieldsMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Player(character) => &mut character.unit_fields,
            Self::Creature(creature) => &mut creature.unit_fields,
        }
    }
}
