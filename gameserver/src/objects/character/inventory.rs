use common::guid::{self, Guid};
use gameobjects::player::VisibleItemFields;
use ordered_float::NotNan;
use packets::{
    inventory_change_result::{InventoryChangeError, InventoryChangeErrorType},
    update_data::UpdateData,
};
use sqlx::{Pool, Sqlite, SqliteExecutor, Transaction};

use crate::{
    game_data::GameDataAccessor,
    objects::{
        character::Character,
        item::Item,
        item_prototype::{InventoryType, ItemClass, ItemPrototype},
    },
    packet_handler::{EquipmentSlot, Slot},
};

#[derive(Debug, Clone, Copy)]
pub enum InventoryOperation {
    Swap {
        src: Slot,
        dst: Slot,
    },
    AutoEquip {
        src: Slot,
    },
    StoreInBackpack {
        src: Slot,
    },
    Split {
        src: Slot,
        dst: Slot,
        count: u32,
    },
    /// An operation that isn't supported uses this operation
    Reject {
        src: Slot,
        error: InventoryChangeErrorType,
    },
}

#[derive(Default)]
pub struct InventoryChanges {
    pub updated: Vec<UpdateData>,
    pub destroyed: Vec<Guid<guid::Item>>,
}

struct SwapPrototypes {
    src: ItemPrototype,
    dst: Option<ItemPrototype>,
    main_hand: Option<ItemPrototype>,
    off_hand: Option<ItemPrototype>,
}

// All moves of a change happen at once: every item is taken out before any is placed
#[derive(Clone, Copy)]
struct ItemMove<'p> {
    from: Slot,
    to: Slot,
    prototype: &'p ItemPrototype,
}

impl ItemMove<'_> {
    fn reversed(self) -> Self {
        Self {
            from: self.to,
            to: self.from,
            ..self
        }
    }
}

enum StackChange {
    Create { slot: Slot, count: u32 },
    SetCount { slot: Slot, count: u32 },
    Destroy { slot: Slot },
}

impl StackChange {
    fn stack_onto(
        src: Slot,
        src_item: &Item,
        dst: Slot,
        dst_item: &Item,
        amount: u32,
    ) -> Vec<Self> {
        let src_change = if amount == src_item.stack_count() {
            Self::Destroy { slot: src }
        } else {
            Self::SetCount {
                slot: src,
                count: src_item.stack_count() - amount,
            }
        };

        vec![
            Self::SetCount {
                slot: dst,
                count: dst_item.stack_count() + amount,
            },
            src_change,
        ]
    }
}

enum InventoryChange<'p> {
    Move(Vec<ItemMove<'p>>),
    Stack(Vec<StackChange>),
}

impl Character {
    fn inventory_error(
        &self,
        error: InventoryChangeErrorType,
        item1_slot: Option<Slot>,
        item2_slot: Option<Slot>,
    ) -> MoveItemError {
        MoveItemError::Inventory(InventoryChangeError {
            error,
            item1: item1_slot.and_then(|slot| self.item_guid(slot)),
            item2: item2_slot.and_then(|slot| self.item_guid(slot)),
        })
    }

    pub fn inventory_operation_error(
        &self,
        operation: InventoryOperation,
        error: InventoryChangeErrorType,
    ) -> InventoryChangeError {
        let (item1_slot, item2_slot) = match operation {
            InventoryOperation::Swap { src, dst } | InventoryOperation::Split { src, dst, .. } => {
                (Some(src), Some(dst))
            }
            InventoryOperation::AutoEquip { src }
            | InventoryOperation::StoreInBackpack { src }
            | InventoryOperation::Reject { src, .. } => (Some(src), None),
        };

        InventoryChangeError {
            error,
            item1: item1_slot.and_then(|slot| self.item_guid(slot)),
            item2: item2_slot.and_then(|slot| self.item_guid(slot)),
        }
    }

    fn item_guid(&self, slot: Slot) -> Option<Guid<guid::Item>> {
        self.items.get(slot).as_ref().map(Item::guid)
    }

    async fn save_inventory(&self, db: impl SqliteExecutor<'_>) -> Result<(), sqlx::Error> {
        use EquipmentSlot::*;

        let item_id = |slot| self.item_guid(slot).map(|guid| guid.get_u32().get());
        let equipment = |slot| item_id(Slot::Equipment(slot));
        let bag = |slot| item_id(Slot::BagItem(slot));
        let backpack = |slot| item_id(Slot::MainBag(slot));

        let result = sqlx::query!(
            "UPDATE character SET
                equipment_head_id = ?, equipment_neck_id = ?, equipment_shoulders_id = ?,
                equipment_body_id = ?, equipment_chest_id = ?, equipment_waist_id = ?,
                equipment_legs_id = ?, equipment_feet_id = ?, equipment_wrists_id = ?,
                equipment_hands_id = ?, equipment_finger1_id = ?, equipment_finger2_id = ?,
                equipment_trinket1_id = ?, equipment_trinket2_id = ?, equipment_back_id = ?,
                equipment_mainhand_id = ?, equipment_offhand_id = ?, equipment_ranged_id = ?,
                equipment_tabard_id = ?,
                bag1_id = ?, bag2_id = ?, bag3_id = ?, bag4_id = ?,
                backpack1_id = ?, backpack2_id = ?, backpack3_id = ?, backpack4_id = ?,
                backpack5_id = ?, backpack6_id = ?, backpack7_id = ?, backpack8_id = ?,
                backpack9_id = ?, backpack10_id = ?, backpack11_id = ?, backpack12_id = ?,
                backpack13_id = ?, backpack14_id = ?, backpack15_id = ?, backpack16_id = ?
            WHERE id = ?",
            equipment(Head),
            equipment(Neck),
            equipment(Shoulders),
            equipment(Body),
            equipment(Chest),
            equipment(Waist),
            equipment(Legs),
            equipment(Feet),
            equipment(Wrists),
            equipment(Hands),
            equipment(Finger1),
            equipment(Finger2),
            equipment(Trinket1),
            equipment(Trinket2),
            equipment(Back),
            equipment(MainHand),
            equipment(OffHand),
            equipment(Ranged),
            equipment(Tabard),
            bag(0),
            bag(1),
            bag(2),
            bag(3),
            backpack(0),
            backpack(1),
            backpack(2),
            backpack(3),
            backpack(4),
            backpack(5),
            backpack(6),
            backpack(7),
            backpack(8),
            backpack(9),
            backpack(10),
            backpack(11),
            backpack(12),
            backpack(13),
            backpack(14),
            backpack(15),
            self.guid().get_u32().get()
        )
        .execute(db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    fn backpack_slots(&self) -> impl Iterator<Item = Slot> {
        (0..self.items.main_backpack.len() as u8).map(Slot::MainBag)
    }

    /// First backpack slot that is empty once the pending moves are done
    fn first_free_backpack_slot(&self, pending_moves: &[ItemMove]) -> Option<Slot> {
        self.backpack_slots().find(|&slot| {
            let filled = pending_moves.iter().any(|item_move| item_move.to == slot);
            let emptied = pending_moves.iter().any(|item_move| item_move.from == slot);
            !filled && (emptied || self.items.get(slot).is_none())
        })
    }

    fn insert_item(&mut self, slot: Slot, item: Item) {
        debug_assert!(self.items.get(slot).is_none());
        *self.items.get_mut(slot) = Some(item);
        self.update_slot_fields(slot);
    }

    fn move_items(&mut self, moves: &[ItemMove]) {
        let items = moves
            .iter()
            .map(|item_move| self.items.get_mut(item_move.from).take())
            .collect::<Vec<_>>();
        for (item_move, item) in moves.iter().zip(items) {
            debug_assert!(self.items.get(item_move.to).is_none());
            *self.items.get_mut(item_move.to) = item;
        }
        for item_move in moves {
            self.update_slot_fields(item_move.from);
            self.update_slot_fields(item_move.to);
        }
    }

    pub async fn swap_items(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        src: Slot,
        dst: Slot,
    ) -> Result<InventoryChanges, MoveItemError> {
        if src == dst {
            return Err(self.inventory_error(
                InventoryChangeErrorType::ItemsCantBeSwapped,
                Some(src),
                None,
            ));
        }

        let prototypes = self
            .load_swap_prototypes(game_data_accessor, src, dst)
            .await?;
        let change = self
            .plan_swap(&prototypes, src, dst)
            .map_err(|e| self.inventory_error(e, Some(src), Some(dst)))?;

        match change {
            InventoryChange::Move(moves) => self.apply_moves(db, &moves).await,
            InventoryChange::Stack(changes) => {
                self.apply_stack_changes(game_data_accessor, db, &prototypes.src, &changes)
                    .await
            }
        }
    }

    async fn load_swap_prototypes(
        &self,
        game_data_accessor: &GameDataAccessor,
        src: Slot,
        dst: Slot,
    ) -> Result<SwapPrototypes, MoveItemError> {
        let load_prototype = async |slot| {
            let Some(item) = self.items.get(slot) else {
                return Ok(None);
            };

            match item.load_prototype(game_data_accessor).await? {
                Some(v) => Ok(Some(v)),
                None => Err(self.inventory_error(
                    InventoryChangeErrorType::ItemNotFound,
                    Some(src),
                    Some(dst),
                )),
            }
        };

        let Some(src_prototype) = load_prototype(src).await? else {
            return Err(self.inventory_error(
                InventoryChangeErrorType::SlotIsEmpty,
                Some(src),
                Some(dst),
            ));
        };

        Ok(SwapPrototypes {
            src: src_prototype,
            dst: load_prototype(dst).await?,
            main_hand: load_prototype(Slot::Equipment(EquipmentSlot::MainHand)).await?,
            off_hand: load_prototype(Slot::Equipment(EquipmentSlot::OffHand)).await?,
        })
    }

    fn plan_swap<'p>(
        &self,
        prototypes: &'p SwapPrototypes,
        src: Slot,
        dst: Slot,
    ) -> Result<InventoryChange<'p>, InventoryChangeErrorType> {
        let main_hand = Slot::Equipment(EquipmentSlot::MainHand);
        let off_hand = Slot::Equipment(EquipmentSlot::OffHand);

        //TODO: Bags
        if matches!(src, Slot::BagItem(_)) || matches!(dst, Slot::BagItem(_)) {
            return Err(InventoryChangeErrorType::ItemDoesntGoToSlot);
        }

        let src_move = ItemMove {
            from: src,
            to: dst,
            prototype: &prototypes.src,
        };
        self.check_move(src_move)?;

        // Only stacks in the backpack can be merged into
        if let (Some(src_item), Some(dst_item)) = (self.items.get(src), self.items.get(dst))
            && matches!(dst, Slot::MainBag(_))
            && let Some(amount) = src_item.stackable_amount_onto(dst_item, prototypes.src.stackable)
        {
            return Ok(InventoryChange::Stack(StackChange::stack_onto(
                src, src_item, dst, dst_item, amount,
            )));
        }

        let mut moves = vec![src_move];
        if let Some(dst_prototype) = &prototypes.dst {
            let dst_move = ItemMove {
                from: dst,
                to: src,
                prototype: dst_prototype,
            };
            self.check_move(dst_move)?;
            moves.push(dst_move);
        }

        let after_swap =
            |slot: Slot, current: &'p Option<ItemPrototype>| -> Option<&'p ItemPrototype> {
                if slot == dst {
                    Some(&prototypes.src)
                } else if slot == src {
                    prototypes.dst.as_ref()
                } else {
                    current.as_ref()
                }
            };

        // A two-handed weapon pushes the off hand item into the backpack
        if let (Some(main_hand_prototype), Some(off_hand_prototype)) = (
            after_swap(main_hand, &prototypes.main_hand),
            after_swap(off_hand, &prototypes.off_hand),
        ) && main_hand_prototype.inventory_type == InventoryType::TwoHandWeapon
        {
            if src == off_hand || dst == off_hand {
                return Err(InventoryChangeErrorType::CantEquipWithTwohanded);
            }

            let Some(free_slot) = self.first_free_backpack_slot(&moves) else {
                return Err(if prototypes.dst.is_some() {
                    InventoryChangeErrorType::ItemsCantBeSwapped
                } else {
                    InventoryChangeErrorType::InventoryFull
                });
            };
            let off_hand_move = ItemMove {
                from: off_hand,
                to: free_slot,
                prototype: off_hand_prototype,
            };
            self.check_move(off_hand_move)?;
            moves.push(off_hand_move);
        }

        Ok(InventoryChange::Move(moves))
    }

    fn check_move(&self, item_move: ItemMove) -> Result<(), InventoryChangeErrorType> {
        let prototype = item_move.prototype;
        let changes_equipment = matches!(item_move.from, Slot::Equipment(_))
            || matches!(item_move.to, Slot::Equipment(_));
        if changes_equipment && self.in_combat() && !prototype.equipable_in_combat() {
            return Err(InventoryChangeErrorType::NotInCombat);
        }

        let Slot::Equipment(slot) = item_move.to else {
            return Ok(());
        };

        if !prototype
            .inventory_type
            .equip_slots(self.can_dual_wield())
            .contains(&slot)
        {
            return Err(InventoryChangeErrorType::ItemDoesntGoToSlot);
        }

        self.can_equip_item(prototype)?;

        if prototype.inventory_type == InventoryType::WeaponOffHand && !self.can_dual_wield() {
            return Err(InventoryChangeErrorType::CantDualWield);
        }

        Ok(())
    }

    async fn apply_moves(
        &mut self,
        db: &Pool<Sqlite>,
        moves: &[ItemMove<'_>],
    ) -> Result<InventoryChanges, MoveItemError> {
        self.move_items(moves);

        if let Err(e) = self.save_inventory(db).await {
            let reversed_moves = moves.iter().map(|m| m.reversed()).collect::<Vec<_>>();
            self.move_items(&reversed_moves);
            return Err(e.into());
        }

        for item_move in moves {
            if let Slot::Equipment(slot) = item_move.from {
                self.remove_item_stats(slot, item_move.prototype);
            }
        }
        for item_move in moves {
            if let Slot::Equipment(slot) = item_move.to {
                self.add_item_stats(slot, item_move.prototype);
            }
        }

        Ok(InventoryChanges::default())
    }

    async fn apply_stack_changes(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        prototype: &ItemPrototype,
        changes: &[StackChange],
    ) -> Result<InventoryChanges, MoveItemError> {
        let mut tx = db.begin().await?;

        let mut created_items = Vec::new();
        for change in changes {
            let &StackChange::Create { slot, count } = change else {
                continue;
            };
            let item_id = prototype
                .create_item(&mut *tx, Some(self.guid()), None, None, count)
                .await?;
            let Some(item) = Item::load_from_db(game_data_accessor, &mut *tx, item_id).await?
            else {
                return Err(self.inventory_error(
                    InventoryChangeErrorType::ItemNotFound,
                    None,
                    None,
                ));
            };
            created_items.push((slot, item));
        }

        let previous_counts = changes
            .iter()
            .filter_map(|change| match *change {
                StackChange::SetCount { slot, .. } => self
                    .items
                    .get(slot)
                    .as_ref()
                    .map(|item| (slot, item.stack_count())),
                _ => None,
            })
            .collect::<Vec<_>>();

        for (slot, item) in created_items {
            self.insert_item(slot, item);
        }
        let mut destroyed_items = Vec::new();
        for change in changes {
            match *change {
                StackChange::Create { .. } => {}
                StackChange::SetCount { slot, count } => {
                    if let Some(item) = self.items.get_mut(slot) {
                        item.set_stack_count(count);
                    }
                }
                StackChange::Destroy { slot } => {
                    destroyed_items
                        .extend(self.items.get_mut(slot).take().map(|item| (slot, item)));
                    self.update_slot_fields(slot);
                }
            }
        }

        if let Err(e) = self.save_stack_changes(tx, changes, &destroyed_items).await {
            for change in changes {
                if let StackChange::Create { slot, .. } = *change {
                    self.items.get_mut(slot).take();
                    self.update_slot_fields(slot);
                }
            }
            for (slot, count) in previous_counts {
                if let Some(item) = self.items.get_mut(slot) {
                    item.set_stack_count(count);
                }
            }
            for (slot, item) in destroyed_items {
                self.insert_item(slot, item);
            }
            return Err(e.into());
        }

        for change in changes {
            match *change {
                StackChange::Create {
                    slot: Slot::Equipment(slot),
                    ..
                } => self.add_item_stats(slot, prototype),
                StackChange::Destroy {
                    slot: Slot::Equipment(slot),
                } => self.remove_item_stats(slot, prototype),
                _ => {}
            }
        }

        let mut inventory_changes = InventoryChanges::default();
        for change in changes {
            if let StackChange::Create { slot, .. } | StackChange::SetCount { slot, .. } = *change
                && let Some(item) = self.items.get(slot)
            {
                inventory_changes
                    .updated
                    .push(item.build_create_update_block());
            }
        }
        inventory_changes.destroyed = destroyed_items
            .iter()
            .map(|(_, item)| item.guid())
            .collect();

        Ok(inventory_changes)
    }

    async fn save_stack_changes(
        &self,
        mut tx: Transaction<'_, Sqlite>,
        changes: &[StackChange],
        destroyed_items: &[(Slot, Item)],
    ) -> Result<(), sqlx::Error> {
        // Has to happen before destroyed items are deleted since the inventory references them
        if changes
            .iter()
            .any(|change| !matches!(change, StackChange::SetCount { .. }))
        {
            self.save_inventory(&mut *tx).await?;
        }
        for (_, item) in destroyed_items {
            item.delete_from_db(&mut *tx).await?;
        }
        for change in changes {
            if let StackChange::SetCount { slot, .. } = *change
                && let Some(item) = self.items.get(slot)
            {
                item.save_stack_count(&mut *tx).await?;
            }
        }

        tx.commit().await
    }

    pub async fn auto_equip_item(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        src: Slot,
    ) -> Result<InventoryChanges, MoveItemError> {
        let Some(item) = self.items.get(src) else {
            return Err(self.inventory_error(
                InventoryChangeErrorType::SlotIsEmpty,
                Some(src),
                None,
            ));
        };
        let Some(prototype) = item.load_prototype(game_data_accessor).await? else {
            return Err(self.inventory_error(
                InventoryChangeErrorType::ItemNotFound,
                Some(src),
                None,
            ));
        };

        let Some(dst) = self.find_equip_slot(&prototype) else {
            return Err(self.inventory_error(
                InventoryChangeErrorType::ItemCantBeEquipped,
                Some(src),
                None,
            ));
        };

        self.swap_items(game_data_accessor, db, src, dst).await
    }

    pub async fn store_item_in_backpack(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        src: Slot,
    ) -> Result<InventoryChanges, MoveItemError> {
        if self.items.get(src).is_none() {
            return Err(self.inventory_error(
                InventoryChangeErrorType::SlotIsEmpty,
                Some(src),
                None,
            ));
        }

        if matches!(src, Slot::MainBag(_)) {
            return Err(self.inventory_error(InventoryChangeErrorType::None, Some(src), None));
        }

        let Some(dst) = self.first_free_backpack_slot(&[]) else {
            return Err(self.inventory_error(InventoryChangeErrorType::BagFull, Some(src), None));
        };

        self.swap_items(game_data_accessor, db, src, dst).await
    }

    pub async fn split_item(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        src: Slot,
        dst: Slot,
        count: u32,
    ) -> Result<InventoryChanges, MoveItemError> {
        if src == dst || count == 0 {
            return Err(self.inventory_error(
                InventoryChangeErrorType::CouldntSplitItems,
                Some(src),
                None,
            ));
        }

        let Some(item) = self.items.get(src) else {
            return Err(self.inventory_error(
                InventoryChangeErrorType::ItemNotFound,
                Some(src),
                None,
            ));
        };
        let Some(prototype) = item.load_prototype(game_data_accessor).await? else {
            return Err(self.inventory_error(
                InventoryChangeErrorType::ItemNotFound,
                Some(src),
                None,
            ));
        };

        let changes = self
            .plan_split(&prototype, src, dst, count)
            .map_err(|e| self.inventory_error(e, Some(src), None))?;
        self.apply_stack_changes(game_data_accessor, db, &prototype, &changes)
            .await
    }

    fn plan_split(
        &self,
        prototype: &ItemPrototype,
        src: Slot,
        dst: Slot,
        count: u32,
    ) -> Result<Vec<StackChange>, InventoryChangeErrorType> {
        let Some(src_item) = self.items.get(src) else {
            return Err(InventoryChangeErrorType::ItemNotFound);
        };
        let src_count = src_item.stack_count();
        if count == src_count {
            return Err(InventoryChangeErrorType::CouldntSplitItems);
        }
        if count > src_count {
            return Err(InventoryChangeErrorType::TooFewToSplit);
        }

        //TODO: Bags
        if matches!(src, Slot::BagItem(_)) || matches!(dst, Slot::BagItem(_)) {
            return Err(InventoryChangeErrorType::ItemDoesntGoToSlot);
        }

        self.check_move(ItemMove {
            from: src,
            to: dst,
            prototype,
        })?;

        let Some(dst_item) = self.items.get(dst) else {
            return Ok(vec![
                StackChange::Create { slot: dst, count },
                StackChange::SetCount {
                    slot: src,
                    count: src_count - count,
                },
            ]);
        };

        match src_item.stackable_amount_onto(dst_item, prototype.stackable) {
            Some(amount) if matches!(dst, Slot::MainBag(_)) && amount >= count => {
                Ok(StackChange::stack_onto(src, src_item, dst, dst_item, count))
            }
            _ => Err(InventoryChangeErrorType::ItemCantStack),
        }
    }

    pub async fn store_new_items(
        &mut self,
        game_data_accessor: &GameDataAccessor,
        db: &Pool<Sqlite>,
        prototype: &ItemPrototype,
        count: u32,
        requested_slot: Option<Slot>,
    ) -> Result<InventoryChanges, MoveItemError> {
        let changes = self
            .plan_new_items(prototype, count, requested_slot)
            .map_err(|e| self.inventory_error(e, None, None))?;
        self.apply_stack_changes(game_data_accessor, db, prototype, &changes)
            .await
    }

    fn plan_new_items(
        &self,
        prototype: &ItemPrototype,
        count: u32,
        requested_slot: Option<Slot>,
    ) -> Result<Vec<StackChange>, InventoryChangeErrorType> {
        let max_stack = prototype.stackable.max(1);

        let fill = |slot: Slot, remaining: u32| match self.items.get(slot) {
            None => {
                let amount = max_stack.min(remaining);
                Some((
                    StackChange::Create {
                        slot,
                        count: amount,
                    },
                    amount,
                ))
            }
            Some(item) if item.entry() == prototype.item_id => {
                let count = item.stack_count();
                let amount = max_stack.saturating_sub(count).min(remaining);
                (amount > 0).then_some((
                    StackChange::SetCount {
                        slot,
                        count: count + amount,
                    },
                    amount,
                ))
            }
            Some(_) => None,
        };

        let mut changes = Vec::new();
        let mut remaining = count;

        if let Some(slot) = requested_slot {
            //TODO: Bags and equipment
            if !matches!(slot, Slot::MainBag(_)) {
                return Err(InventoryChangeErrorType::ItemDoesntGoToSlot);
            }

            let Some((change, amount)) = fill(slot, remaining) else {
                return Err(InventoryChangeErrorType::ItemCantStack);
            };
            changes.push(change);
            remaining -= amount;
        }

        let stacks = self
            .backpack_slots()
            .filter(|&slot| self.items.get(slot).is_some());
        let free_slots = self
            .backpack_slots()
            .filter(|&slot| self.items.get(slot).is_none());
        for slot in stacks.chain(free_slots) {
            if remaining == 0 {
                break;
            }
            if Some(slot) == requested_slot {
                continue;
            }

            if let Some((change, amount)) = fill(slot, remaining) {
                changes.push(change);
                remaining -= amount;
            }
        }

        if remaining > 0 {
            return Err(InventoryChangeErrorType::InventoryFull);
        }

        Ok(changes)
    }

    /// Picks an empty slot out of the ones the item fits into, or the first one if all of them are taken
    fn find_equip_slot(&self, prototype: &ItemPrototype) -> Option<Slot> {
        let slots = prototype.inventory_type.equip_slots(self.can_dual_wield());

        slots
            .iter()
            .map(|&slot| Slot::Equipment(slot))
            .find(|&slot| self.items.get(slot).is_none())
            .or(slots.first().map(|&slot| Slot::Equipment(slot)))
    }

    //TODO: Requires the Dual Wield spell, which starting characters don't have
    fn can_dual_wield(&self) -> bool {
        false
    }

    //TODO: Skill, spell, honor rank and reputation requirements
    fn can_equip_item(&self, prototype: &ItemPrototype) -> Result<(), InventoryChangeErrorType> {
        let bytes_1 = self.unit_fields.bytes_1.get();
        let class_mask = 1 << (bytes_1.class() - 1);
        let race_mask = 1 << (bytes_1.race() - 1);
        if prototype.allowable_class & class_mask == 0 || prototype.allowable_race & race_mask == 0
        {
            return Err(InventoryChangeErrorType::YouCanNeverUseThatItem);
        }

        if *self.unit_fields.level.get() < prototype.required_level {
            return Err(InventoryChangeErrorType::CantEquipLevel {
                required_level: prototype.required_level,
            });
        }

        Ok(())
    }

    // Copies the item in a slot to the player fields that mirror it
    fn update_slot_fields(&mut self, slot: Slot) {
        let item = self.items.get(slot).as_ref();
        let guid = item.map(Item::guid);
        let player_fields = &mut self.player_fields;
        match slot {
            Slot::Equipment(slot) => {
                *player_fields.equipment_slots[slot as usize].get_mut_using_copy() = guid;
                *player_fields.visible_items[slot as usize].get_mut_using_copy() =
                    item.map_or_else(VisibleItemFields::default, |v| v.get_visible_item_fields());
            }
            Slot::BagItem(slot) => {
                *player_fields.bag_slots[slot as usize].get_mut_using_copy() = guid;
            }
            Slot::MainBag(slot) => {
                *player_fields.main_backpack_slots[slot as usize].get_mut_using_copy() = guid;
            }
        }
    }

    pub fn build_item_full_update_blocks(&self) -> Vec<UpdateData> {
        let mut r = Vec::with_capacity(130); // 113 fields for items
        if let Some(item) = &self.items.equipment.head {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.neck {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.shoulders {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.body {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.chest {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.waist {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.legs {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.feet {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.wrists {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.hands {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.finger1 {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.finger2 {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.trinket1 {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.trinket2 {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.back {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.mainhand {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.offhand {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.ranged {
            r.push(item.build_create_update_block());
        }
        if let Some(item) = &self.items.equipment.tabard {
            r.push(item.build_create_update_block());
        }
        for item in &self.items.bags {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.main_backpack {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.bank {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.bank_bags {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.vendor_buyback {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }
        for item in &self.items.keyring {
            if let Some(item) = item {
                r.push(item.build_create_update_block());
            }
        }

        r
    }
}

pub(super) struct WeaponStats {
    pub min_damage: NotNan<f32>,
    pub max_damage: NotNan<f32>,
    pub attack_time: u32,
}

impl Default for WeaponStats {
    // Unarmed
    fn default() -> Self {
        Self {
            min_damage: NotNan::from(1u8),
            max_damage: NotNan::from(2u8),
            attack_time: 2000,
        }
    }
}

impl WeaponStats {
    fn of_weapon(prototype: &ItemPrototype) -> Option<Self> {
        (prototype.class == ItemClass::Weapon).then(|| Self {
            min_damage: NotNan::new(prototype.damage[0].min as f32)
                .expect("an integer is never NaN"),
            max_damage: NotNan::new(prototype.damage[0].max as f32)
                .expect("an integer is never NaN"),
            attack_time: prototype.delay,
        })
    }
}

impl<S> Character<S> {
    //TODO: Armor from agility, damage from attack power
    pub(super) fn add_item_stats(&mut self, slot: EquipmentSlot, prototype: &ItemPrototype) {
        *self.unit_fields.normal_resistance.get_mut_using_copy() += prototype.armor;
        if let Some(weapon) = WeaponStats::of_weapon(prototype) {
            self.set_weapon_stats(slot, weapon);
        }
    }

    fn remove_item_stats(&mut self, slot: EquipmentSlot, prototype: &ItemPrototype) {
        let mut armor = self.unit_fields.normal_resistance.get_mut_using_copy();
        *armor = armor.saturating_sub(prototype.armor);
        drop(armor);

        if WeaponStats::of_weapon(prototype).is_some() {
            self.set_weapon_stats(slot, WeaponStats::default());
        }
    }

    fn set_weapon_stats(&mut self, slot: EquipmentSlot, weapon: WeaponStats) {
        let unit_fields = &mut self.unit_fields;
        let (attack_time, min_damage, max_damage) = match slot {
            EquipmentSlot::MainHand => (
                &mut unit_fields.base_attack_time,
                &mut unit_fields.min_damage,
                &mut unit_fields.max_damage,
            ),
            EquipmentSlot::OffHand => (
                &mut unit_fields.offhand_attack_time,
                &mut unit_fields.min_offhand_damage,
                &mut unit_fields.max_offhand_damage,
            ),
            EquipmentSlot::Ranged => (
                &mut unit_fields.ranged_attack_time,
                &mut unit_fields.min_ranged_damage,
                &mut unit_fields.max_ranged_damage,
            ),
            _ => return,
        };

        *attack_time.get_mut_using_copy() = weapon.attack_time;
        *min_damage.get_mut_using_copy() = weapon.min_damage;
        *max_damage.get_mut_using_copy() = weapon.max_damage;
    }
}

pub(super) struct CharacterItems {
    pub equipment: EquipmentItems,
    pub bags: [Option<Item>; 4],
    pub main_backpack: [Option<Item>; 16],
    pub bank: [Option<Item>; 28],
    pub bank_bags: [Option<Item>; 7],
    pub vendor_buyback: [Option<Item>; 12],
    pub keyring: [Option<Item>; 12],
    // unknown: [Item; 15]
}

impl CharacterItems {
    pub(super) async fn load_equipped_prototypes(
        &self,
        game_data_accessor: &GameDataAccessor,
    ) -> Result<Vec<(EquipmentSlot, ItemPrototype)>, sqlx::Error> {
        let mut prototypes = Vec::new();
        for slot in EquipmentSlot::ALL {
            let Some(item) = self.equipment.get(slot) else {
                continue;
            };
            if let Some(prototype) = item.load_prototype(game_data_accessor).await? {
                prototypes.push((slot, prototype));
            }
        }

        Ok(prototypes)
    }

    fn get(&self, slot: Slot) -> &Option<Item> {
        match slot {
            Slot::Equipment(slot) => self.equipment.get(slot),
            Slot::BagItem(slot) => &self.bags[slot as usize],
            Slot::MainBag(slot) => &self.main_backpack[slot as usize],
        }
    }

    fn get_mut(&mut self, slot: Slot) -> &mut Option<Item> {
        match slot {
            Slot::Equipment(slot) => self.equipment.get_mut(slot),
            Slot::BagItem(slot) => &mut self.bags[slot as usize],
            Slot::MainBag(slot) => &mut self.main_backpack[slot as usize],
        }
    }
}

pub(super) struct EquipmentItems {
    pub head: Option<Item>,
    pub neck: Option<Item>,
    pub shoulders: Option<Item>,
    pub body: Option<Item>,
    pub chest: Option<Item>,
    pub waist: Option<Item>,
    pub legs: Option<Item>,
    pub feet: Option<Item>,
    pub wrists: Option<Item>,
    pub hands: Option<Item>,
    pub finger1: Option<Item>,
    pub finger2: Option<Item>,
    pub trinket1: Option<Item>,
    pub trinket2: Option<Item>,
    pub back: Option<Item>,
    pub mainhand: Option<Item>,
    pub offhand: Option<Item>,
    pub ranged: Option<Item>,
    pub tabard: Option<Item>,
}

impl EquipmentItems {
    fn get(&self, slot: EquipmentSlot) -> &Option<Item> {
        match slot {
            EquipmentSlot::Head => &self.head,
            EquipmentSlot::Neck => &self.neck,
            EquipmentSlot::Shoulders => &self.shoulders,
            EquipmentSlot::Body => &self.body,
            EquipmentSlot::Chest => &self.chest,
            EquipmentSlot::Waist => &self.waist,
            EquipmentSlot::Legs => &self.legs,
            EquipmentSlot::Feet => &self.feet,
            EquipmentSlot::Wrists => &self.wrists,
            EquipmentSlot::Hands => &self.hands,
            EquipmentSlot::Finger1 => &self.finger1,
            EquipmentSlot::Finger2 => &self.finger2,
            EquipmentSlot::Trinket1 => &self.trinket1,
            EquipmentSlot::Trinket2 => &self.trinket2,
            EquipmentSlot::Back => &self.back,
            EquipmentSlot::MainHand => &self.mainhand,
            EquipmentSlot::OffHand => &self.offhand,
            EquipmentSlot::Ranged => &self.ranged,
            EquipmentSlot::Tabard => &self.tabard,
        }
    }

    fn get_mut(&mut self, slot: EquipmentSlot) -> &mut Option<Item> {
        match slot {
            EquipmentSlot::Head => &mut self.head,
            EquipmentSlot::Neck => &mut self.neck,
            EquipmentSlot::Shoulders => &mut self.shoulders,
            EquipmentSlot::Body => &mut self.body,
            EquipmentSlot::Chest => &mut self.chest,
            EquipmentSlot::Waist => &mut self.waist,
            EquipmentSlot::Legs => &mut self.legs,
            EquipmentSlot::Feet => &mut self.feet,
            EquipmentSlot::Wrists => &mut self.wrists,
            EquipmentSlot::Hands => &mut self.hands,
            EquipmentSlot::Finger1 => &mut self.finger1,
            EquipmentSlot::Finger2 => &mut self.finger2,
            EquipmentSlot::Trinket1 => &mut self.trinket1,
            EquipmentSlot::Trinket2 => &mut self.trinket2,
            EquipmentSlot::Back => &mut self.back,
            EquipmentSlot::MainHand => &mut self.mainhand,
            EquipmentSlot::OffHand => &mut self.offhand,
            EquipmentSlot::Ranged => &mut self.ranged,
            EquipmentSlot::Tabard => &mut self.tabard,
        }
    }
}

pub enum MoveItemError {
    // Sent to the client
    Inventory(InventoryChangeError),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for MoveItemError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}
