use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EquipmentSlot {
    Head = 0,
    Neck = 1,
    Shoulders = 2,
    Body = 3,
    Chest = 4,
    Waist = 5,
    Legs = 6,
    Feet = 7,
    Wrists = 8,
    Hands = 9,
    Finger1 = 10,
    Finger2 = 11,
    Trinket1 = 12,
    Trinket2 = 13,
    Back = 14,
    MainHand = 15,
    OffHand = 16,
    Ranged = 17,
    Tabard = 18,
}

impl EquipmentSlot {
    pub const ALL: [EquipmentSlot; 19] = [
        Self::Head,
        Self::Neck,
        Self::Shoulders,
        Self::Body,
        Self::Chest,
        Self::Waist,
        Self::Legs,
        Self::Feet,
        Self::Wrists,
        Self::Hands,
        Self::Finger1,
        Self::Finger2,
        Self::Trinket1,
        Self::Trinket2,
        Self::Back,
        Self::MainHand,
        Self::OffHand,
        Self::Ranged,
        Self::Tabard,
    ];
}

#[derive(Clone)]
pub struct VisibleEquipment {
    pub head: Option<VisibleItem>,
    pub neck: Option<VisibleItem>,
    pub shoulders: Option<VisibleItem>,
    pub body: Option<VisibleItem>,
    pub chest: Option<VisibleItem>,
    pub waist: Option<VisibleItem>,
    pub legs: Option<VisibleItem>,
    pub feet: Option<VisibleItem>,
    pub wrists: Option<VisibleItem>,
    pub hands: Option<VisibleItem>,
    pub finger1: Option<VisibleItem>,
    pub finger2: Option<VisibleItem>,
    pub trinket1: Option<VisibleItem>,
    pub trinket2: Option<VisibleItem>,
    pub back: Option<VisibleItem>,
    pub mainhand: Option<VisibleItem>,
    pub offhand: Option<VisibleItem>,
    pub ranged: Option<VisibleItem>,
    pub tabard: Option<VisibleItem>,
}

#[derive(Clone)]
pub struct VisibleItem {
    pub display_id: NonZeroU32,
    pub inventory_type: u8,
}
