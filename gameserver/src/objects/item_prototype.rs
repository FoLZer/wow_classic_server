use common::guid::{self, Guid};
use packets::item_info::{ItemDamage, ItemFlags, ItemSpell, ItemStat};
use sqlx::{Pool, Sqlite};

#[derive(Debug)]
pub struct ItemPrototype {
    pub item_id: u32,
    pub class: u32,
    pub sub_class: u32,
    pub name: String,
    pub description: String,
    pub display_info_id: u32,
    pub quality: u32,
    pub flags: ItemFlags,
    pub buy_price: u32,
    pub sell_price: u32,
    pub inventory_type: u8, //TODO
    pub allowable_class: u32,
    pub allowable_race: u32,
    pub item_level: u32,
    pub required_level: u32,
    pub required_skill: u32,
    pub required_skill_rank: u32,
    pub required_spell: u32,
    pub required_honor_rank: u32,
    pub required_city_rank: u32,
    pub required_reputation_faction: u32,
    pub required_reputation_rank: u32,
    pub max_count: u32,
    pub stackable: u32,
    pub container_slots: u32,
    pub item_stats: [ItemStat; 10],
    pub damage: [ItemDamage; 5],

    pub armor: u32,
    pub holy_resistance: u32,
    pub fire_resistance: u32,
    pub nature_resistance: u32,
    pub frost_resistance: u32,
    pub shadow_resistance: u32,
    pub arcane_resistance: u32,

    pub delay: u32,
    pub ammo_type: u32,
    pub ranged_mod_range: f32,

    pub spells: [Option<ItemSpell>; 5],

    pub bonding: u32,
    pub page_text: u32,
    pub language_id: u32,
    pub page_material: u32,
    pub start_quest: u32,
    pub lock_id: u32,
    pub material: u32,
    pub sheath: u32,
    pub random_property: u32,
    pub block: u32,
    pub item_set: u32,
    pub max_durability: u32,
    pub area: u32,
    pub map: u32,
    pub bag_family: u32,
    pub duration: Option<u32>,
}

impl ItemPrototype {
    pub async fn load_from_db(db: &Pool<Sqlite>, id: u32) -> Result<Self, sqlx::Error> {
        let model = sqlx::query!("SELECT * FROM item_prototype WHERE id = ?", id)
            .fetch_one(db)
            .await?;

        Ok(Self {
            item_id: model.id as u32,
            class: model.class as u32,
            sub_class: model.sub_class as u32,
            name: model.name,
            description: model.description,
            display_info_id: model.display_id as u32,
            quality: model.quality as u32,
            flags: ItemFlags::new(), //TODO
            buy_price: model.buy_price as u32,
            sell_price: model.sell_price as u32,
            inventory_type: model.inventory_type as u8,
            allowable_class: model.allowable_class as u32,
            allowable_race: model.allowable_race as u32,
            item_level: model.item_level as u32,
            required_level: model.required_level as u32,
            required_skill: model.required_skill as u32,
            required_skill_rank: model.required_skill_rank as u32,
            required_spell: model.required_spell as u32,
            required_honor_rank: model.required_honor_rank as u32,
            required_city_rank: model.required_city_rank as u32,
            required_reputation_faction: model.required_reputation_faction as u32,
            required_reputation_rank: model.required_reputation_rank as u32,
            max_count: model.max_count as u32,
            stackable: model.stackable as u32,
            container_slots: model.container_slots as u32,
            item_stats: [
                ItemStat {
                    ty: model.item_stat1_type as u32,
                    value: model.item_stat1_value as u32,
                },
                ItemStat {
                    ty: model.item_stat2_type as u32,
                    value: model.item_stat2_value as u32,
                },
                ItemStat {
                    ty: model.item_stat3_type as u32,
                    value: model.item_stat3_value as u32,
                },
                ItemStat {
                    ty: model.item_stat4_type as u32,
                    value: model.item_stat4_value as u32,
                },
                ItemStat {
                    ty: model.item_stat5_type as u32,
                    value: model.item_stat5_value as u32,
                },
                ItemStat {
                    ty: model.item_stat6_type as u32,
                    value: model.item_stat6_value as u32,
                },
                ItemStat {
                    ty: model.item_stat7_type as u32,
                    value: model.item_stat7_value as u32,
                },
                ItemStat {
                    ty: model.item_stat8_type as u32,
                    value: model.item_stat8_value as u32,
                },
                ItemStat {
                    ty: model.item_stat9_type as u32,
                    value: model.item_stat9_value as u32,
                },
                ItemStat {
                    ty: model.item_stat10_type as u32,
                    value: model.item_stat10_value as u32,
                },
            ],
            damage: [
                ItemDamage {
                    min: model.item_damage1_min as u32,
                    max: model.item_damage1_max as u32,
                    ty: model.item_damage1_type as u32,
                },
                ItemDamage {
                    min: model.item_damage2_min as u32,
                    max: model.item_damage2_max as u32,
                    ty: model.item_damage2_type as u32,
                },
                ItemDamage {
                    min: model.item_damage3_min as u32,
                    max: model.item_damage3_max as u32,
                    ty: model.item_damage3_type as u32,
                },
                ItemDamage {
                    min: model.item_damage4_min as u32,
                    max: model.item_damage4_max as u32,
                    ty: model.item_damage4_type as u32,
                },
                ItemDamage {
                    min: model.item_damage5_min as u32,
                    max: model.item_damage5_max as u32,
                    ty: model.item_damage5_type as u32,
                },
            ],
            armor: model.armor as u32,
            holy_resistance: model.holy_resistance as u32,
            fire_resistance: model.fire_resistance as u32,
            nature_resistance: model.nature_resistance as u32,
            frost_resistance: model.frost_resistance as u32,
            shadow_resistance: model.shadow_resistance as u32,
            arcane_resistance: model.arcane_resistance as u32,
            delay: model.delay as u32,
            ammo_type: model.ammo_type as u32,
            ranged_mod_range: model.ranged_mod_range as f32,
            spells: [
                if model.spell1_id != 0 {
                    Some(ItemSpell {
                        id: model.spell1_id as u32,
                        trigger: model.spell1_trigger as u32,
                        charges: model.spell1_charges as u32,
                        cooldown: model.spell1_cooldown as u32,
                        category: model.spell1_category as u32,
                        category_cooldown: model.spell1_category_cooldown as u32,
                    })
                } else {
                    None
                },
                if model.spell2_id != 0 {
                    Some(ItemSpell {
                        id: model.spell2_id as u32,
                        trigger: model.spell2_trigger as u32,
                        charges: model.spell2_charges as u32,
                        cooldown: model.spell2_cooldown as u32,
                        category: model.spell2_category as u32,
                        category_cooldown: model.spell2_category_cooldown as u32,
                    })
                } else {
                    None
                },
                if model.spell3_id != 0 {
                    Some(ItemSpell {
                        id: model.spell3_id as u32,
                        trigger: model.spell3_trigger as u32,
                        charges: model.spell3_charges as u32,
                        cooldown: model.spell3_cooldown as u32,
                        category: model.spell3_category as u32,
                        category_cooldown: model.spell3_category_cooldown as u32,
                    })
                } else {
                    None
                },
                if model.spell4_id != 0 {
                    Some(ItemSpell {
                        id: model.spell4_id as u32,
                        trigger: model.spell4_trigger as u32,
                        charges: model.spell4_charges as u32,
                        cooldown: model.spell4_cooldown as u32,
                        category: model.spell4_category as u32,
                        category_cooldown: model.spell4_category_cooldown as u32,
                    })
                } else {
                    None
                },
                if model.spell5_id != 0 {
                    Some(ItemSpell {
                        id: model.spell5_id as u32,
                        trigger: model.spell5_trigger as u32,
                        charges: model.spell5_charges as u32,
                        cooldown: model.spell5_cooldown as u32,
                        category: model.spell5_category as u32,
                        category_cooldown: model.spell5_category_cooldown as u32,
                    })
                } else {
                    None
                },
            ],
            bonding: model.bonding as u32,
            page_text: model.page_text as u32,
            language_id: model.language_id as u32,
            page_material: model.page_material as u32,
            start_quest: model.start_quest as u32,
            lock_id: model.lock_id as u32,
            material: model.material as u32,
            sheath: model.sheath as u32,
            random_property: model.random_property as u32,
            block: model.block as u32,
            item_set: model.item_set as u32,
            max_durability: model.max_durability as u32,
            area: model.area as u32,
            map: model.map as u32,
            bag_family: model.bag_family as u32,
            duration: model.duration.map(|v| v as u32),
        })
    }

    pub async fn create_item(
        &self,
        db: &Pool<Sqlite>,
        owner: Option<Guid<guid::Player>>,
        creator: Option<Guid<guid::Player>>,
        gift_creator: Option<Guid<guid::Player>>,
        count: u32,
    ) -> Result<u32, sqlx::Error> {
        let owner = owner.map(|v| v.get_u32());
        let creator = creator.map(|v| v.get_u32());
        let gift_creator = gift_creator.map(|v| v.get_u32());
        let spell1_charges = self.spells[0].map(|v| v.id);
        let spell2_charges = self.spells[1].map(|v| v.id);
        let spell3_charges = self.spells[2].map(|v| v.id);
        let spell4_charges = self.spells[3].map(|v| v.id);
        let spell5_charges = self.spells[4].map(|v| v.id);

        let id = sqlx::query!(
            "INSERT INTO item(
                prototype_id, owner, creator, gift_creator, stack_count,
                expires_in, spell1_charges, spell2_charges, spell3_charges,
                spell4_charges, spell5_charges, is_binded, is_unlocked,
                is_wrapped, is_readable, random_properties_id, item_text_id,
                durability, enchantment1_id, enchantment1_duration, enchantment1_charges,
                enchantment2_id, enchantment2_duration, enchantment2_charges,
                enchantment3_id, enchantment3_duration, enchantment3_charges,
                enchantment4_id, enchantment4_duration, enchantment4_charges,
                enchantment5_id, enchantment5_duration, enchantment5_charges,
                enchantment6_id, enchantment6_duration, enchantment6_charges,
                enchantment7_id, enchantment7_duration, enchantment7_charges,
                enchantment8_id, enchantment8_duration, enchantment8_charges,
                enchantment9_id, enchantment9_duration, enchantment9_charges
            )
            VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            self.item_id,
            owner,
            creator,
            gift_creator,
            count,
            self.duration,
            spell1_charges,
            spell2_charges,
            spell3_charges,
            spell4_charges,
            spell5_charges,
            false, //TODO
            false, //TODO
            false, //TODO
            false, //TODO
            0, //TODO: random_properties_id
            0, //TODO: item_text_id
            self.max_durability,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
            0, 0, 0,
        )
        .execute(db)
        .await?.last_insert_rowid();

        Ok(id as u32)
    }
}
