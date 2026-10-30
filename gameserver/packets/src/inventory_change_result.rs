use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use common::guid::{self, Guid};

use crate::server::OrderedWrite;

pub struct InventoryChangeError {
    pub error: InventoryChangeErrorType,
    pub item1: Option<Guid<guid::Item>>,
    pub item2: Option<Guid<guid::Item>>,
}

impl<T: ByteOrder> OrderedWrite<T> for InventoryChangeError {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u8(self.error.id())?;
        if let InventoryChangeErrorType::CantEquipLevel { required_level } = self.error {
            writer.write_u32::<LittleEndian>(required_level)?;
        }
        writer.write_u64::<LittleEndian>(self.item1.map_or(0, |v| v.get().get()))?;
        writer.write_u64::<LittleEndian>(self.item2.map_or(0, |v| v.get().get()))?;
        let bag_slot = match self.error {
            InventoryChangeErrorType::ItemDoesntGoIntoBag2 { bag_slot } => bag_slot.unwrap_or(0xff),
            _ => 0,
        };
        writer.write_u8(bag_slot)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InventoryChangeErrorType {
    CantEquipLevel { required_level: u32 },
    CantEquipSkill,
    ItemDoesntGoToSlot,
    BagFull,
    NonemptyBagOverOtherBag,
    CantTradeEquipBags,
    OnlyAmmoCanGoHere,
    NoRequiredProficiency,
    NoEquipmentSlotAvailable,
    YouCanNeverUseThatItem,
    YouCanNeverUseThatItem2,
    NoEquipmentSlotAvailable2,
    CantEquipWithTwohanded,
    CantDualWield,
    ItemDoesntGoIntoBag,
    ItemDoesntGoIntoBag2 { bag_slot: Option<u8> },
    CantCarryMoreOfThis,
    NoEquipmentSlotAvailable3,
    ItemCantStack,
    ItemCantBeEquipped,
    ItemsCantBeSwapped,
    SlotIsEmpty,
    ItemNotFound,
    CantDropSoulbound,
    OutOfRange,
    TooFewToSplit,
    CouldntSplitItems,
    MissingReagent,
    NotEnoughMoney,
    NotABag,
    CanOnlyDoWithEmptyBags,
    DontOwnThatItem,
    CanEquipOnly1Quiver,
    MustPurchaseThatBagSlot,
    TooFarAwayFromBank,
    ItemLocked,
    YouAreStunned,
    YouAreDead,
    CantDoRightNow,
    IntBagError,
    CanEquipOnly1Bolt,
    CanEquipOnly1Ammopouch,
    StackableCantBeWrapped,
    EquippedCantBeWrapped,
    WrappedCantBeWrapped,
    BoundCantBeWrapped,
    UniqueCantBeWrapped,
    BagsCantBeWrapped,
    AlreadyLooted,
    InventoryFull,
    BankFull,
    ItemIsCurrentlySoldOut,
    BagFull3,
    ItemNotFound2,
    ItemCantStack2,
    BagFull4,
    ItemSoldOut,
    ObjectIsBusy,
    None,
    NotInCombat,
    NotWhileDisarmed,
    BagFull6,
    CantEquipRank,
    CantEquipReputation,
    TooManySpecialBags,
    LootCantLootThatNow,
}

impl InventoryChangeErrorType {
    pub fn id(&self) -> u8 {
        match self {
            InventoryChangeErrorType::CantEquipLevel { .. } => 1,
            InventoryChangeErrorType::CantEquipSkill => 2,
            InventoryChangeErrorType::ItemDoesntGoToSlot => 3,
            InventoryChangeErrorType::BagFull => 4,
            InventoryChangeErrorType::NonemptyBagOverOtherBag => 5,
            InventoryChangeErrorType::CantTradeEquipBags => 6,
            InventoryChangeErrorType::OnlyAmmoCanGoHere => 7,
            InventoryChangeErrorType::NoRequiredProficiency => 8,
            InventoryChangeErrorType::NoEquipmentSlotAvailable => 9,
            InventoryChangeErrorType::YouCanNeverUseThatItem => 10,
            InventoryChangeErrorType::YouCanNeverUseThatItem2 => 11,
            InventoryChangeErrorType::NoEquipmentSlotAvailable2 => 12,
            InventoryChangeErrorType::CantEquipWithTwohanded => 13,
            InventoryChangeErrorType::CantDualWield => 14,
            InventoryChangeErrorType::ItemDoesntGoIntoBag => 15,
            InventoryChangeErrorType::ItemDoesntGoIntoBag2 { .. } => 16,
            InventoryChangeErrorType::CantCarryMoreOfThis => 17,
            InventoryChangeErrorType::NoEquipmentSlotAvailable3 => 18,
            InventoryChangeErrorType::ItemCantStack => 19,
            InventoryChangeErrorType::ItemCantBeEquipped => 20,
            InventoryChangeErrorType::ItemsCantBeSwapped => 21,
            InventoryChangeErrorType::SlotIsEmpty => 22,
            InventoryChangeErrorType::ItemNotFound => 23,
            InventoryChangeErrorType::CantDropSoulbound => 24,
            InventoryChangeErrorType::OutOfRange => 25,
            InventoryChangeErrorType::TooFewToSplit => 26,
            InventoryChangeErrorType::CouldntSplitItems => 27,
            InventoryChangeErrorType::MissingReagent => 28,
            InventoryChangeErrorType::NotEnoughMoney => 29,
            InventoryChangeErrorType::NotABag => 30,
            InventoryChangeErrorType::CanOnlyDoWithEmptyBags => 31,
            InventoryChangeErrorType::DontOwnThatItem => 32,
            InventoryChangeErrorType::CanEquipOnly1Quiver => 33,
            InventoryChangeErrorType::MustPurchaseThatBagSlot => 34,
            InventoryChangeErrorType::TooFarAwayFromBank => 35,
            InventoryChangeErrorType::ItemLocked => 36,
            InventoryChangeErrorType::YouAreStunned => 37,
            InventoryChangeErrorType::YouAreDead => 38,
            InventoryChangeErrorType::CantDoRightNow => 39,
            InventoryChangeErrorType::IntBagError => 40,
            InventoryChangeErrorType::CanEquipOnly1Bolt => 41,
            InventoryChangeErrorType::CanEquipOnly1Ammopouch => 42,
            InventoryChangeErrorType::StackableCantBeWrapped => 43,
            InventoryChangeErrorType::EquippedCantBeWrapped => 44,
            InventoryChangeErrorType::WrappedCantBeWrapped => 45,
            InventoryChangeErrorType::BoundCantBeWrapped => 46,
            InventoryChangeErrorType::UniqueCantBeWrapped => 47,
            InventoryChangeErrorType::BagsCantBeWrapped => 48,
            InventoryChangeErrorType::AlreadyLooted => 49,
            InventoryChangeErrorType::InventoryFull => 50,
            InventoryChangeErrorType::BankFull => 51,
            InventoryChangeErrorType::ItemIsCurrentlySoldOut => 52,
            InventoryChangeErrorType::BagFull3 => 53,
            InventoryChangeErrorType::ItemNotFound2 => 54,
            InventoryChangeErrorType::ItemCantStack2 => 55,
            InventoryChangeErrorType::BagFull4 => 56,
            InventoryChangeErrorType::ItemSoldOut => 57,
            InventoryChangeErrorType::ObjectIsBusy => 58,
            InventoryChangeErrorType::None => 59,
            InventoryChangeErrorType::NotInCombat => 60,
            InventoryChangeErrorType::NotWhileDisarmed => 61,
            InventoryChangeErrorType::BagFull6 => 62,
            InventoryChangeErrorType::CantEquipRank => 63,
            InventoryChangeErrorType::CantEquipReputation => 64,
            InventoryChangeErrorType::TooManySpecialBags => 65,
            InventoryChangeErrorType::LootCantLootThatNow => 66,
        }
    }
}
