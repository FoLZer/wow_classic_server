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
        17,
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
VALUES(25, "Kobold");
/* Factions END */
/* Creature Type START */
INSERT INTO creature_type(id, name)
VALUES(7, "Humanoid");
/* Creature Type END */
/* Creature Family START */
/* Creature Family END */

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
        civilian
    )
VALUES(
        6,
        "Kobold Vermin",
        NULL,
        100,
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
        0
    );
/* Creatures END */
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
        30000
    );
/* Static Creature Spawners END */
