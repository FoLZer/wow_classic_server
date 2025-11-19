use sea_orm_migration::{prelude::*, schema::pk_auto};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Item::Table)
                    .if_not_exists()
                    .col(pk_auto(Item::Id))
                    .col(ColumnDef::new(Item::ItemPrototypeId).integer().not_null())
                    .col(ColumnDef::new(Item::Owner).integer().not_null())
                    .col(ColumnDef::new(Item::Creator).integer())
                    .col(ColumnDef::new(Item::GiftCreator).integer())
                    .col(ColumnDef::new(Item::StackCount).integer().not_null())
                    .col(ColumnDef::new(Item::ExpiresIn).integer())
                    .col(ColumnDef::new(Item::Spell1Charges).integer())
                    .col(ColumnDef::new(Item::Spell2Charges).integer())
                    .col(ColumnDef::new(Item::Spell3Charges).integer())
                    .col(ColumnDef::new(Item::Spell4Charges).integer())
                    .col(ColumnDef::new(Item::Spell5Charges).integer())
                    .col(ColumnDef::new(Item::IsBinded).boolean().not_null())
                    .col(ColumnDef::new(Item::IsUnlocked).boolean().not_null())
                    .col(ColumnDef::new(Item::IsWrapped).boolean().not_null())
                    .col(ColumnDef::new(Item::IsReadable).boolean().not_null())
                    .col(
                        ColumnDef::new(Item::RandomPropertiesId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::ItemTextId).integer().not_null())
                    .col(ColumnDef::new(Item::Durability).integer().not_null())
                    .col(ColumnDef::new(Item::Enchantment1Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment1Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment1Charges)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::Enchantment2Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment2Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment2Charges)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::Enchantment3Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment3Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment3Charges)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::Enchantment4Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment4Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment4Charges)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::Enchantment5Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment5Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment5Charges)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::Enchantment6Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment6Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment6Charges)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::Enchantment7Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment7Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment7Charges)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::Enchantment8Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment8Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment8Charges)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Item::Enchantment9Id).integer().not_null())
                    .col(
                        ColumnDef::new(Item::Enchantment9Duration)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Item::Enchantment9Charges)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Item::Table, Item::Owner)
                            .to(Character::Table, Character::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Item::Table, Item::Creator)
                            .to(Character::Table, Character::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Item::Table, Item::GiftCreator)
                            .to(Character::Table, Character::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Character::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Character::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Character::AccountId).integer().not_null())
                    .col(ColumnDef::new(Character::Name).text().not_null())
                    .col(ColumnDef::new(Character::Race).tiny_integer().not_null())
                    .col(ColumnDef::new(Character::Class).tiny_integer().not_null())
                    .col(ColumnDef::new(Character::Gender).tiny_integer().not_null())
                    .col(ColumnDef::new(Character::Skin).tiny_integer().not_null())
                    .col(ColumnDef::new(Character::Face).tiny_integer().not_null())
                    .col(
                        ColumnDef::new(Character::HairStyle)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::HairColor)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::FacialHair)
                            .tiny_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Character::Level).tiny_integer().not_null())
                    .col(ColumnDef::new(Character::Area).integer().not_null())
                    .col(ColumnDef::new(Character::Map).integer().not_null())
                    .col(ColumnDef::new(Character::PositionX).float().not_null())
                    .col(ColumnDef::new(Character::PositionY).float().not_null())
                    .col(ColumnDef::new(Character::PositionZ).float().not_null())
                    .col(ColumnDef::new(Character::Orientation).float().not_null())
                    .col(ColumnDef::new(Character::GuildId).integer().not_null())
                    .col(ColumnDef::new(Character::Flags).integer().not_null())
                    .col(ColumnDef::new(Character::FirstLogin).boolean().not_null())
                    .col(ColumnDef::new(Character::DisplayId).integer().not_null())
                    .col(
                        ColumnDef::new(Character::EquipmentHead)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentNeck)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentShoulders)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentBody)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentChest)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentWaist)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentLegs)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentFeet)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentWrists)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentHands)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentFinger1)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentFinger2)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentTrinket1)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentTrinket2)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentBack)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentMainhand)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentOffhand)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentRanged)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::EquipmentTabard)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Character::Bag1).integer().not_null())
                    .col(ColumnDef::new(Character::Bag2).integer().not_null())
                    .col(ColumnDef::new(Character::Bag3).integer().not_null())
                    .col(ColumnDef::new(Character::Bag4).integer().not_null())
                    .col(
                        ColumnDef::new(Character::MainBackpack1)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack2)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack3)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack4)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack5)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack6)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack7)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack8)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack9)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack10)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack11)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack12)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack13)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack14)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack15)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::MainBackpack16)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Character::Bank1).integer().not_null())
                    .col(ColumnDef::new(Character::Bank2).integer().not_null())
                    .col(ColumnDef::new(Character::Bank3).integer().not_null())
                    .col(ColumnDef::new(Character::Bank4).integer().not_null())
                    .col(ColumnDef::new(Character::Bank5).integer().not_null())
                    .col(ColumnDef::new(Character::Bank6).integer().not_null())
                    .col(ColumnDef::new(Character::Bank7).integer().not_null())
                    .col(ColumnDef::new(Character::Bank8).integer().not_null())
                    .col(ColumnDef::new(Character::Bank9).integer().not_null())
                    .col(ColumnDef::new(Character::Bank10).integer().not_null())
                    .col(ColumnDef::new(Character::Bank11).integer().not_null())
                    .col(ColumnDef::new(Character::Bank12).integer().not_null())
                    .col(ColumnDef::new(Character::Bank13).integer().not_null())
                    .col(ColumnDef::new(Character::Bank14).integer().not_null())
                    .col(ColumnDef::new(Character::Bank15).integer().not_null())
                    .col(ColumnDef::new(Character::Bank16).integer().not_null())
                    .col(ColumnDef::new(Character::Bank17).integer().not_null())
                    .col(ColumnDef::new(Character::Bank18).integer().not_null())
                    .col(ColumnDef::new(Character::Bank19).integer().not_null())
                    .col(ColumnDef::new(Character::Bank20).integer().not_null())
                    .col(ColumnDef::new(Character::Bank21).integer().not_null())
                    .col(ColumnDef::new(Character::Bank22).integer().not_null())
                    .col(ColumnDef::new(Character::Bank23).integer().not_null())
                    .col(ColumnDef::new(Character::Bank24).integer().not_null())
                    .col(ColumnDef::new(Character::Bank25).integer().not_null())
                    .col(ColumnDef::new(Character::Bank26).integer().not_null())
                    .col(ColumnDef::new(Character::Bank27).integer().not_null())
                    .col(ColumnDef::new(Character::Bank28).integer().not_null())
                    .col(ColumnDef::new(Character::BankBag1).integer().not_null())
                    .col(ColumnDef::new(Character::BankBag2).integer().not_null())
                    .col(ColumnDef::new(Character::BankBag3).integer().not_null())
                    .col(ColumnDef::new(Character::BankBag4).integer().not_null())
                    .col(ColumnDef::new(Character::BankBag5).integer().not_null())
                    .col(ColumnDef::new(Character::BankBag6).integer().not_null())
                    .col(ColumnDef::new(Character::BankBag7).integer().not_null())
                    .col(
                        ColumnDef::new(Character::VendorBuyback1)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback2)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback3)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback4)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback5)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback6)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback7)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback8)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback9)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback10)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback11)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Character::VendorBuyback12)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Character::Keyring1).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring2).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring3).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring4).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring5).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring6).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring7).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring8).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring9).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring10).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring11).integer().not_null())
                    .col(ColumnDef::new(Character::Keyring12).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentHead)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentNeck)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentShoulders)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentBody)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentChest)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentWaist)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentLegs)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentFeet)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentWrists)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentHands)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentFinger1)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentFinger2)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentTrinket1)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentTrinket2)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentBack)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentMainhand)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentOffhand)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentRanged)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::EquipmentTabard)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bag1)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bag2)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bag3)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bag4)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack1)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack2)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack3)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack4)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack5)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack6)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack7)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack8)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack9)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack10)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack11)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack12)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack13)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack14)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack15)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::MainBackpack16)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank1)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank2)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank3)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank4)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank5)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank6)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank7)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank8)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank9)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank10)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank11)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank12)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank13)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank14)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank15)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank16)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank17)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank18)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank19)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank20)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank21)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank22)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank23)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank24)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank25)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank26)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank27)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Bank28)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::BankBag1)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::BankBag2)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::BankBag3)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::BankBag4)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::BankBag5)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::BankBag6)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::BankBag7)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback1)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback2)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback3)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback4)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback5)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback6)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback7)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback8)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback9)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback10)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback11)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::VendorBuyback12)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring1)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring2)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring3)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring4)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring5)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring6)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring7)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring8)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring9)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring10)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring11)
                            .to(Item::Table, Item::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Character::Table, Character::Keyring12)
                            .to(Item::Table, Item::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Item::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Character::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Character {
    Table,

    Id,
    AccountId,
    Name,
    Race,
    Class,
    Gender,
    Skin,
    Face,
    HairStyle,
    HairColor,
    FacialHair,
    Level,
    Area,
    Map,
    PositionX,
    PositionY,
    PositionZ,
    Orientation,
    GuildId,
    Flags,
    FirstLogin,
    DisplayId,

    EquipmentHead,
    EquipmentNeck,
    EquipmentShoulders,
    EquipmentBody,
    EquipmentChest,
    EquipmentWaist,
    EquipmentLegs,
    EquipmentFeet,
    EquipmentWrists,
    EquipmentHands,
    EquipmentFinger1,
    EquipmentFinger2,
    EquipmentTrinket1,
    EquipmentTrinket2,
    EquipmentBack,
    EquipmentMainhand,
    EquipmentOffhand,
    EquipmentRanged,
    EquipmentTabard,

    Bag1,
    Bag2,
    Bag3,
    Bag4,

    MainBackpack1,
    MainBackpack2,
    MainBackpack3,
    MainBackpack4,
    MainBackpack5,
    MainBackpack6,
    MainBackpack7,
    MainBackpack8,
    MainBackpack9,
    MainBackpack10,
    MainBackpack11,
    MainBackpack12,
    MainBackpack13,
    MainBackpack14,
    MainBackpack15,
    MainBackpack16,

    Bank1,
    Bank2,
    Bank3,
    Bank4,
    Bank5,
    Bank6,
    Bank7,
    Bank8,
    Bank9,
    Bank10,
    Bank11,
    Bank12,
    Bank13,
    Bank14,
    Bank15,
    Bank16,
    Bank17,
    Bank18,
    Bank19,
    Bank20,
    Bank21,
    Bank22,
    Bank23,
    Bank24,
    Bank25,
    Bank26,
    Bank27,
    Bank28,

    BankBag1,
    BankBag2,
    BankBag3,
    BankBag4,
    BankBag5,
    BankBag6,
    BankBag7,

    VendorBuyback1,
    VendorBuyback2,
    VendorBuyback3,
    VendorBuyback4,
    VendorBuyback5,
    VendorBuyback6,
    VendorBuyback7,
    VendorBuyback8,
    VendorBuyback9,
    VendorBuyback10,
    VendorBuyback11,
    VendorBuyback12,

    Keyring1,
    Keyring2,
    Keyring3,
    Keyring4,
    Keyring5,
    Keyring6,
    Keyring7,
    Keyring8,
    Keyring9,
    Keyring10,
    Keyring11,
    Keyring12,
}

#[derive(DeriveIden)]
enum Item {
    Table,

    Id,

    ItemPrototypeId,

    Owner,
    Creator,
    GiftCreator,
    StackCount,
    ExpiresIn,

    Spell1Charges,
    Spell2Charges,
    Spell3Charges,
    Spell4Charges,
    Spell5Charges,

    // Flags
    IsBinded,
    IsUnlocked,
    IsWrapped,
    IsReadable,

    RandomPropertiesId,
    ItemTextId,
    Durability,

    Enchantment1Id,
    Enchantment1Duration,
    Enchantment1Charges,
    Enchantment2Id,
    Enchantment2Duration,
    Enchantment2Charges,
    Enchantment3Id,
    Enchantment3Duration,
    Enchantment3Charges,
    Enchantment4Id,
    Enchantment4Duration,
    Enchantment4Charges,
    Enchantment5Id,
    Enchantment5Duration,
    Enchantment5Charges,
    Enchantment6Id,
    Enchantment6Duration,
    Enchantment6Charges,
    Enchantment7Id,
    Enchantment7Duration,
    Enchantment7Charges,
    Enchantment8Id,
    Enchantment8Duration,
    Enchantment8Charges,
    Enchantment9Id,
    Enchantment9Duration,
    Enchantment9Charges,
}
