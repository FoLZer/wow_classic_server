/* Races START */
INSERT INTO race(id, name)
VALUES(1, "Human");
INSERT INTO race(id, name)
VALUES(2, "Orc");
INSERT INTO race(id, name)
VALUES(3, "Dwarf");
INSERT INTO race(id, name)
VALUES(4, "Night Elf");
INSERT INTO race(id, name)
VALUES(5, "Undead");
INSERT INTO race(id, name)
VALUES(6, "Tauren");
INSERT INTO race(id, name)
VALUES(7, "Gnome");
INSERT INTO race(id, name)
VALUES(8, "Troll");
/* Races END */
/* Classes START */
INSERT INTO class(id, name)
VALUES(1, "Warrior");
INSERT INTO class(id, name)
VALUES(2, "Paladin");
INSERT INTO class(id, name)
VALUES(3, "Hunter");
INSERT INTO class(id, name)
VALUES(4, "Rogue");
INSERT INTO class(id, name)
VALUES(5, "Priest");
INSERT INTO class(id, name)
VALUES(7, "Shaman");
INSERT INTO class(id, name)
VALUES(8, "Mage");
INSERT INTO class(id, name)
VALUES(9, "Warlock");
INSERT INTO class(id, name)
VALUES(11, "Druid");
/* Classes END */
/* Character Display Ids START */
INSERT INTO character_display_id(race, gender, display_id)
VALUES(1, 0, 49);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(1, 1, 50);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(2, 0, 51);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(2, 1, 52);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(3, 0, 53);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(3, 1, 54);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(4, 0, 55);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(4, 1, 56);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(5, 0, 57);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(5, 1, 58);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(6, 0, 59);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(6, 1, 60);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(7, 0, 1563);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(7, 1, 1564);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(8, 0, 1478);
INSERT INTO character_display_id(race, gender, display_id)
VALUES(8, 1, 1479);
/* Character Display Ids END */
/* Item Prototypes START */
INSERT INTO item_prototype(
        id,
        class,
        sub_class,
        name,
        description,
        display_id,
        quality,
        buy_price,
        sell_price,
        inventory_type,
        allowable_class,
        allowable_race,
        item_level,
        required_level,
        required_skill,
        required_skill_rank,
        required_spell,
        required_honor_rank,
        required_city_rank,
        required_reputation_faction,
        required_reputation_rank,
        max_count,
        stackable,
        container_slots,
        item_stat1_type,
        item_stat1_value,
        item_stat2_type,
        item_stat2_value,
        item_stat3_type,
        item_stat3_value,
        item_stat4_type,
        item_stat4_value,
        item_stat5_type,
        item_stat5_value,
        item_stat6_type,
        item_stat6_value,
        item_stat7_type,
        item_stat7_value,
        item_stat8_type,
        item_stat8_value,
        item_stat9_type,
        item_stat9_value,
        item_stat10_type,
        item_stat10_value,
        item_damage1_min,
        item_damage1_max,
        item_damage1_type,
        item_damage2_min,
        item_damage2_max,
        item_damage2_type,
        item_damage3_min,
        item_damage3_max,
        item_damage3_type,
        item_damage4_min,
        item_damage4_max,
        item_damage4_type,
        item_damage5_min,
        item_damage5_max,
        item_damage5_type,
        armor,
        holy_resistance,
        fire_resistance,
        nature_resistance,
        frost_resistance,
        shadow_resistance,
        arcane_resistance,
        delay,
        ammo_type,
        ranged_mod_range,
        spell1_id,
        spell1_trigger,
        spell1_charges,
        spell1_cooldown,
        spell1_category,
        spell1_category_cooldown,
        spell2_id,
        spell2_trigger,
        spell2_charges,
        spell2_cooldown,
        spell2_category,
        spell2_category_cooldown,
        spell3_id,
        spell3_trigger,
        spell3_charges,
        spell3_cooldown,
        spell3_category,
        spell3_category_cooldown,
        spell4_id,
        spell4_trigger,
        spell4_charges,
        spell4_cooldown,
        spell4_category,
        spell4_category_cooldown,
        spell5_id,
        spell5_trigger,
        spell5_charges,
        spell5_cooldown,
        spell5_category,
        spell5_category_cooldown,
        bonding,
        page_text,
        language_id,
        page_material,
        start_quest,
        lock_id,
        material,
        sheath,
        random_property,
        block,
        item_set,
        max_durability,
        area,
        map,
        bag_family,
        duration
    )
VALUES(
        789,
        2,
        4,
        "Stout Battlehammer",
        "",
        19699,
        2,
        9847,
        1969,
        21,
        4294967295,
        4294967295,
        22,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        17,
        33,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        2300,
        0,
        0.0,
        0,
        0,
        0,
        4294967295,
        0,
        4294967295,
        0,
        0,
        0,
        4294967295,
        0,
        4294967295,
        0,
        0,
        0,
        4294967295,
        0,
        4294967295,
        0,
        0,
        0,
        4294967295,
        0,
        4294967295,
        0,
        0,
        0,
        4294967295,
        0,
        4294967295,
        2,
        0,
        0,
        0,
        0,
        0,
        2,
        3,
        5197,
        0,
        0,
        60,
        0,
        0,
        0,
        0
    );
/* Item Prototypes END */
/* Player Start Data START */
INSERT INTO player_start_data(
        race,
        class,
        area,
        map,
        position_x,
        position_y,
        position_z,
        orientation,
        level,
        equipment_head_id,
        equipment_neck_id,
        equipment_shoulders_id,
        equipment_body_id,
        equipment_chest_id,
        equipment_waist_id,
        equipment_legs_id,
        equipment_feet_id,
        equipment_wrists_id,
        equipment_hands_id,
        equipment_finger1_id,
        equipment_finger2_id,
        equipment_trinket1_id,
        equipment_trinket2_id,
        equipment_back_id,
        equipment_mainhand_id,
        equipment_offhand_id,
        equipment_ranged_id,
        equipment_tabard_id,
        other_items_table_id
    )
VALUES(
        1,
        1,
        12,
        0,
        -8949.95,
        -132.493,
        83.5312,
        0.0,
        1,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        789,
        NULL,
        NULL,
        NULL,
        NULL
    );
/* Player Start Data END */
/* Factions START */
INSERT INTO faction(id, name)
VALUES(1, "PLAYER, Human");
INSERT INTO faction(id, name)
VALUES(2, "PLAYER, Orc");
INSERT INTO faction(id, name)
VALUES(3, "PLAYER, Dwarf");
INSERT INTO faction(id, name)
VALUES(4, "PLAYER, Night Elf");
INSERT INTO faction(id, name)
VALUES(5, "PLAYER, Undead");
INSERT INTO faction(id, name)
VALUES(6, "PLAYER, Tauren");
INSERT INTO faction(id, name)
VALUES(7, "Creature");
INSERT INTO faction(id, name)
VALUES(10, "Escortee");
INSERT INTO faction(id, name)
VALUES(11, "Stormwind");
INSERT INTO faction(id, name)
VALUES(12, "Stormwind");
INSERT INTO faction(id, name)
VALUES(14, "Monster");
INSERT INTO faction(id, name)
VALUES(15, "Creature");
INSERT INTO faction(id, name)
VALUES(16, "Monster");
INSERT INTO faction(id, name)
VALUES(17, "Defias Brotherhood");
INSERT INTO faction(id, name)
VALUES(18, "Murloc");
INSERT INTO faction(id, name)
VALUES(19, "Gnoll - Redridge");
INSERT INTO faction(id, name)
VALUES(20, "Gnoll - Riverpaw");
INSERT INTO faction(id, name)
VALUES(21, "Undead, Scourge");
INSERT INTO faction(id, name)
VALUES(22, "Beast - Spider");
INSERT INTO faction(id, name)
VALUES(23, "Gnomeregan Exiles");
INSERT INTO faction(id, name)
VALUES(24, "Worgen");
INSERT INTO faction(id, name)
VALUES(25, "Kobold");
INSERT INTO faction(id, name)
VALUES(26, "Kobold");
INSERT INTO faction(id, name)
VALUES(27, "Defias Brotherhood");
INSERT INTO faction(id, name)
VALUES(28, "Troll, Bloodscalp");
INSERT INTO faction(id, name)
VALUES(29, "Orgrimmar");
INSERT INTO faction(id, name)
VALUES(30, "Troll, Skullsplitter");
INSERT INTO faction(id, name)
VALUES(31, "Prey");
INSERT INTO faction(id, name)
VALUES(32, "Beast - Wolf");
INSERT INTO faction(id, name)
VALUES(33, "Escortee");
INSERT INTO faction(id, name)
VALUES(34, "Defias Brotherhood");
INSERT INTO faction(id, name)
VALUES(35, "Friendly");
INSERT INTO faction(id, name)
VALUES(115, "PLAYER, Gnome");
INSERT INTO faction(id, name)
VALUES(116, "PLAYER, Troll");
/* Factions END */
/* Race Factions START */
INSERT INTO race_faction(race, faction)
VALUES(1, 1);
INSERT INTO race_faction(race, faction)
VALUES(2, 2);
INSERT INTO race_faction(race, faction)
VALUES(3, 3);
INSERT INTO race_faction(race, faction)
VALUES(4, 4);
INSERT INTO race_faction(race, faction)
VALUES(5, 5);
INSERT INTO race_faction(race, faction)
VALUES(6, 6);
INSERT INTO race_faction(race, faction)
VALUES(7, 115);
INSERT INTO race_faction(race, faction)
VALUES(8, 116);
/* Race Factions END */
/* Creature Type START */
INSERT INTO creature_type(id, name)
VALUES(7, "Humanoid");
/* Creature Type END */
/* Creature Family START */
/* Creature Family END */
/* Languages START */
INSERT INTO language(id, name)
VALUES(1, "Orcish");
INSERT INTO language(id, name)
VALUES(2, "Darnassian");
INSERT INTO language(id, name)
VALUES(3, "Taurahe");
INSERT INTO language(id, name)
VALUES(6, "Dwarvish");
INSERT INTO language(id, name)
VALUES(7, "Common");
INSERT INTO language(id, name)
VALUES(8, "Demonic");
INSERT INTO language(id, name)
VALUES(9, "Titan");
INSERT INTO language(id, name)
VALUES(10, "Thalassian");
INSERT INTO language(id, name)
VALUES(11, "Draconic");
INSERT INTO language(id, name)
VALUES(12, "Kalimag");
INSERT INTO language(id, name)
VALUES(13, "Gnomish");
INSERT INTO language(id, name)
VALUES(14, "Troll");
INSERT INTO language(id, name)
VALUES(33, "Gutterspeak");
/* Languages END */
/* NPC Texts START */
INSERT INTO npc_text(id, text_male, text_female, language)
VALUES(1, "Greetings, $N.", "Greetings, $N.", 0),
    (2, "What else would you like to know?", "What else would you like to know?", 0);
/* NPC Texts END */
/* Gossip Menus START */
INSERT INTO gossip_menu(id, title_text_id)
VALUES(1, 1),
    (2, 2);
INSERT INTO gossip_menu_item(menu_id, position, icon, coded, text, next_menu_id)
VALUES(1, 0, 0, FALSE, "Tell me more.", 2),
    (2, 0, 0, FALSE, "Back.", 1);
/* Gossip Menus END */
/* Creatures START */
INSERT INTO creature(
        id,
        name,
        sub_name,
        health,
        max_health,
        level_min,
        level_max,
        faction,
        race,
        class,
        gender,
        power,
        equipment_mainhand_id,
        equipment_offhand_id,
        equipment_ranged_id,
        base_attack_time,
        offhand_attack_time,
        ranged_attack_time,
        combat_reach,
        display_id,
        native_display_id,
        initial_mount_display_id,
        flags,
        type,
        family,
        rank,
        civilian,
        npc_flags,
        gossip_menu_root_id
    )
VALUES(
        6,
        "Kobold Vermin",
        NULL,
        /* 100, */
        2,
        500,
        1,
        1,
        25,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        0,
        NULL
    );
INSERT INTO creature(
        id,
        name,
        sub_name,
        health,
        max_health,
        level_min,
        level_max,
        faction,
        race,
        class,
        gender,
        power,
        equipment_mainhand_id,
        equipment_offhand_id,
        equipment_ranged_id,
        base_attack_time,
        offhand_attack_time,
        ranged_attack_time,
        combat_reach,
        display_id,
        native_display_id,
        initial_mount_display_id,
        flags,
        type,
        family,
        rank,
        civilian,
        npc_flags,
        gossip_menu_root_id
    )
VALUES(
        1,
        "Gossip Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        1,
        1
    ),
    (
        2,
        "Quest Giver Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        2,
        NULL
    ),
    (
        3,
        "Vendor Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        4,
        NULL
    ),
    (
        4,
        "Flight Master Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        8,
        NULL
    ),
    (
        5,
        "Trainer Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        16,
        NULL
    ),
    (
        7,
        "Spirit Healer Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        32,
        NULL
    ),
    (
        8,
        "Spirit Guide Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        64,
        NULL
    ),
    (
        9,
        "Innkeeper Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        128,
        NULL
    ),
    (
        10,
        "Banker Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        256,
        NULL
    ),
    (
        11,
        "Petitioner Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        512,
        NULL
    ),
    (
        12,
        "Tabard Designer Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        1024,
        NULL
    ),
    (
        13,
        "Battlemaster Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        2048,
        NULL
    ),
    (
        14,
        "Auctioneer Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        4096,
        NULL
    ),
    (
        15,
        "Stable Master Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        8192,
        NULL
    ),
    (
        16,
        "Repairer Test",
        NULL,
        100,
        100,
        1,
        1,
        12,
        0,
        1,
        1,
        0,
        NULL,
        NULL,
        NULL,
        1,
        2,
        3,
        5.0,
        10913,
        10913,
        NULL,
        0,
        7,
        NULL,
        2,
        0,
        16384,
        NULL
    );
/* Creatures END */
/* Creature Vendor Items START */
INSERT INTO creature_vendor_item(
        creature_id,
        item_id,
        position,
        num_available,
        purchase_amount
    )
VALUES(3, 789, 0, 5, 1),
    (16, 789, 0, 5, 1);
/* Creature Vendor Items END */
/* Static Creature Spawners START */
INSERT INTO creature_spawner_static(
        position_x,
        position_y,
        position_z,
        orientation,
        spawn_creature_id,
        respawn_time
    )
VALUES(
        -8949.95,
        -132.493,
        83.5312,
        0.0,
        6,
        /* 30000 */
        10000
    ),
    (
        -8910.4795,
        -106.27133,
        81.85089,
        4.146894,
        1,
        /* 30000 */
        10000
    ),
    (
        -8907.7,
        -108.81111,
        81.84876,
        4.146894,
        2,
        /* 30000 */
        10000
    ),
    (
        -8904.616,
        -111.07647,
        81.84876,
        4.146894,
        3,
        /* 30000 */
        10000
    ),
    (
        -8901.532,
        -113.34183,
        81.84876,
        4.146894,
        4,
        /* 30000 */
        10000
    ),
    (
        -8898.448,
        -115.60719,
        81.84876,
        4.146894,
        5,
        /* 30000 */
        10000
    ),
    (
        -8895.364,
        -117.87255,
        81.84876,
        4.146894,
        7,
        /* 30000 */
        10000
    ),
    (
        -8892.280,
        -120.13791,
        81.84876,
        4.146894,
        8,
        /* 30000 */
        10000
    ),
    (
        -8889.196,
        -122.40327,
        81.84876,
        4.146894,
        9,
        /* 30000 */
        10000
    ),
    (
        -8886.112,
        -124.66863,
        81.84876,
        4.146894,
        10,
        /* 30000 */
        10000
    ),
    (
        -8883.028,
        -126.93399,
        81.84876,
        4.146894,
        11,
        /* 30000 */
        10000
    ),
    (
        -8879.944,
        -129.19935,
        81.84876,
        4.146894,
        12,
        /* 30000 */
        10000
    ),
    (
        -8876.860,
        -131.46471,
        81.84876,
        4.146894,
        13,
        /* 30000 */
        10000
    ),
    (
        -8873.776,
        -133.73007,
        81.84876,
        4.146894,
        14,
        /* 30000 */
        10000
    ),
    (
        -8870.692,
        -135.99543,
        81.84876,
        4.146894,
        15,
        /* 30000 */
        10000
    ),
    (
        -8867.608,
        -138.26079,
        81.84876,
        4.146894,
        16,
        /* 30000 */
        10000
    );
/* Static Creature Spawners END */