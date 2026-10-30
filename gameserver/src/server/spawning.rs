use common::guid::{self, Guid};
use gameobjects::{
    object::{ObjectFields, TypeBitField},
    unit::{
        AttackPowerMods, NpcFlags, SheathState, StandStateType, UnitFieldBytes1, UnitFieldBytes2,
        UnitFieldBytes2Flags, UnitFieldBytes3, UnitFieldBytes3Flags, UnitFields, UnitFlags,
        VirtualItemInfo,
    },
};
use log::error;
use ordered_float::NotNan;
use rand::{RngExt, rng};

use crate::{
    creature_spawner::{CreatureSpawnInfo, CreatureSpawner},
    dirty_tracking_map::DirtyTrackingMap,
    game_data::GameDataAccessor,
    guid_allocator::GuidAllocator,
    map_instance::InstanceId,
    objects::{creature::Creature, creature_prototype::CreaturePrototype},
    server::Server,
    sparse_set::SparseSet,
};

impl Server {
    pub(super) async fn do_initial_spawns(
        instance_id: InstanceId,
        creature_spawners: &mut SparseSet<CreatureSpawner>,
        game_data_accessor: &GameDataAccessor,
        unit_guid_allocator: &mut GuidAllocator<guid::Unit>,
        creatures: &mut DirtyTrackingMap<Guid<guid::Unit>, Creature>,
    ) {
        let mut to_deactivate = Vec::new();

        creature_spawners.for_each_async(async |index, spawner| {
            let spawner_result = spawner.force_first_spawn();

            for spawn_info in spawner_result.creatures_to_spawn {
                // TODO: prefetch for faster loading time
                let prototype = match game_data_accessor
                    .get_creature_prototype(spawn_info.spawn_creature_id)
                    .await
                {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        error!(
                            "Tried to spawn a creature with a missing prototype (id: {})",
                            spawn_info.spawn_creature_id
                        );
                        continue;
                    }
                    Err(e) => {
                        error!(
                            "Failed to query creature prototype due to a DB error (id: {}). Error: {}",
                            spawn_info.spawn_creature_id, e
                        );
                        continue;
                    }
                };

                Self::spawn_creature(
                    unit_guid_allocator,
                    prototype,
                    spawn_info,
                    creatures,
                    instance_id,
                    Some(index),
                );
            }

            if !spawner_result.keep_active {
                to_deactivate.push(index);
            }
        }).await;

        for index in to_deactivate {
            creature_spawners.deactivate(index);
        }
    }

    pub(super) fn notify_creature_spawner(
        &mut self,
        instance_id: InstanceId,
        spawner_index: usize,
    ) {
        let Some(instance) = self.instances.get_mut(&instance_id) else {
            return;
        };
        let Some(spawner) = instance.creature_spawners.get_mut(spawner_index) else {
            return;
        };
        spawner.notify_creature_died();
        instance.creature_spawners.activate(spawner_index);
    }

    pub(super) async fn process_queued_creature_spawners(&mut self) {
        for (&instance_id, instance) in &mut self.instances {
            let mut to_deactivate = Vec::new();

            instance.creature_spawners.for_each_async(async |index, spawner| {
                let spawner_result = spawner.get_creatures_to_spawn();

                for spawn_info in spawner_result.creatures_to_spawn {
                    // TODO: prefetch, a tick should not be stuck waiting for db access here, especially sequentially like that
                    let prototype = match self
                        .game_data_accessor
                        .get_creature_prototype(spawn_info.spawn_creature_id)
                        .await
                    {
                        Ok(Some(v)) => v,
                        Ok(None) => {
                            error!(
                                "Tried to spawn a creature with a missing prototype (id: {})",
                                spawn_info.spawn_creature_id
                            );
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "Failed to query creature prototype due to a DB error (id: {}). Error: {}",
                                spawn_info.spawn_creature_id, e
                            );
                            continue;
                        }
                    };

                    Self::spawn_creature(
                        &mut self.unit_guid_allocator,
                        prototype,
                        spawn_info,
                        &mut self.creatures,
                        instance_id,
                        Some(index),
                    );
                }

                if !spawner_result.keep_active {
                    to_deactivate.push(index);
                }
            }).await;

            for index in to_deactivate {
                instance.creature_spawners.deactivate(index);
            }
        }
    }

    pub(super) fn spawn_creature(
        unit_guid_allocator: &mut GuidAllocator<guid::Unit>,
        prototype: CreaturePrototype,
        spawn_info: CreatureSpawnInfo,
        creatures_out: &mut DirtyTrackingMap<Guid<guid::Unit>, Creature>,
        instance_id: InstanceId,
        spawner_index: Option<usize>,
    ) {
        let guid = unit_guid_allocator
            .allocate()
            .expect("No more available guids left");

        creatures_out.insert(
            guid,
            Creature {
                instance_id,
                position: spawn_info.position,
                orientation: spawn_info.orientation,
                object_fields: ObjectFields {
                    guid: guid.into(),
                    object_type: TypeBitField::new().with_unit(true).with_object(true).into(),
                    entry: spawn_info.spawn_creature_id.into(),
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
                    health: (prototype.health.unwrap_or(prototype.max_health)).into(),
                    powers: [0.into(); 5],
                    max_health: prototype.max_health.into(),
                    max_powers: [0.into(); 5],
                    level: (rng().random_range(prototype.level)).into(),
                    faction_template: prototype.faction.into(),
                    bytes_1: UnitFieldBytes1::new()
                        .with_race(prototype.race)
                        .with_class(prototype.class)
                        .with_gender(prototype.gender)
                        .with_power(prototype.power)
                        .into(),
                    virtual_item_slot_displays: [
                        prototype
                            .equipment_mainhand
                            .as_ref()
                            .map(|v| v.display_info_id)
                            .unwrap_or(0)
                            .into(),
                        prototype
                            .equipment_offhand
                            .as_ref()
                            .map(|v| v.display_info_id)
                            .unwrap_or(0)
                            .into(),
                        prototype
                            .equipment_ranged
                            .as_ref()
                            .map(|v| v.display_info_id)
                            .unwrap_or(0)
                            .into(),
                    ],
                    virtual_item_infos: [
                        prototype
                            .equipment_mainhand
                            .map(|v| {
                                VirtualItemInfo::new()
                                    .with_class(v.class as u8)
                                    .with_sub_class(v.sub_class as u8)
                                    .with_material(v.material as u8)
                                    .with_inventory_type(v.inventory_type as u8)
                                    .with_sheath(v.sheath as u8)
                            })
                            .unwrap_or_default()
                            .into(),
                        prototype
                            .equipment_offhand
                            .map(|v| {
                                VirtualItemInfo::new()
                                    .with_class(v.class as u8)
                                    .with_sub_class(v.sub_class as u8)
                                    .with_material(v.material as u8)
                                    .with_inventory_type(v.inventory_type as u8)
                                    .with_sheath(v.sheath as u8)
                            })
                            .unwrap_or_default()
                            .into(),
                        prototype
                            .equipment_ranged
                            .map(|v| {
                                VirtualItemInfo::new()
                                    .with_class(v.class as u8)
                                    .with_sub_class(v.sub_class as u8)
                                    .with_material(v.material as u8)
                                    .with_inventory_type(v.inventory_type as u8)
                                    .with_sheath(v.sheath as u8)
                            })
                            .unwrap_or_default()
                            .into(),
                    ],
                    flags: UnitFlags::new().into(),
                    aura: [0.into(); 48],
                    aura_flags: [0.into(); 6],
                    aura_levels: [0.into(); 12],
                    aura_applications: [0.into(); 12],
                    aura_state: 0.into(),
                    base_attack_time: prototype.base_attack_time.into(),
                    offhand_attack_time: prototype.offhand_attack_time.into(),
                    ranged_attack_time: prototype.ranged_attack_time.into(),
                    bounding_radius: 4.into(),
                    combat_reach: prototype.combat_reach.into(),
                    display_id: prototype.display_id.into(),
                    native_display_id: prototype.native_display_id.into(),
                    mount_display_id: prototype.initial_mount_display_id.unwrap_or(0).into(),
                    //TODO: Damage from the creature prototype
                    min_damage: NotNan::<f32>::from(1u8).into(),
                    max_damage: NotNan::<f32>::from(2u8).into(),
                    min_offhand_damage: NotNan::<f32>::default().into(),
                    max_offhand_damage: NotNan::<f32>::default().into(),
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
                    npc_flags: NpcFlags::from_bits(prototype.npc_flags as u32)
                        .with_vendor(prototype.vendor_menu.is_some())
                        .with_gossip(prototype.gossip_menu.is_some())
                        .into(),
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
                    attack_power_mods: AttackPowerMods::new().into(),
                    attack_power_multiplier: NotNan::<f32>::new(0.0).unwrap().into(),
                    ranged_attack_power: 0.into(),
                    ranged_attack_power_mods: AttackPowerMods::new().into(),
                    ranged_attack_power_multiplier: NotNan::<f32>::new(0.0).unwrap().into(),
                    min_ranged_damage: NotNan::<f32>::default().into(),
                    max_ranged_damage: NotNan::<f32>::default().into(),
                    power_cost_modifiers: [0.into(); 7],
                    power_cost_multipliers: [1.into(); 7],
                    _padding: 0.into(),
                },
                spawner_index,
                melee_state: None,
                vendor_menu: prototype.vendor_menu,
                gossip_menu: prototype.gossip_menu,
            },
        );
    }
}
