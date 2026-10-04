use bitfield_struct::bitfield;
use common::guid::{self, Guid};
use macros::tracked;

use crate::tracked_field::{TrackedWriteTrait, UpdateWritable};

#[tracked]
pub struct PlayerFields {
    pub duel_arbiter: Option<Guid<guid::GameObject>>,
    pub flags: u32,
    pub guild_id: u32,
    pub guild_rank: u32,
    pub _unknown_bytes_1: u32,
    pub bytes_2: PlayerFieldBytes2,
    pub bytes_3: PlayerFieldBytes3,
    pub duel_team: u32,
    pub guild_timestamp: u32,
    pub quest_log: [QuestLogFields; 20],
    pub visible_items: [VisibleItemFields; 19],

    pub equipment_slots: [Option<Guid<guid::Item>>; 19],
    pub bag_slots: [Option<Guid<guid::Item>>; 4],
    pub main_backpack_slots: [Option<Guid<guid::Item>>; 16],
    pub bank_slots: [Option<Guid<guid::Item>>; 28],
    pub bank_bag_slots: [Option<Guid<guid::Item>>; 7],
    pub vendor_buyback_slots: [Option<Guid<guid::Item>>; 12],
    pub keyring_slots: [Option<Guid<guid::Item>>; 12],
    pub _unkn: [Option<Guid<guid::Item>>; 15],

    pub far_sight: Option<Guid<guid::DynamicObject>>,
    pub field_combo_target: Option<Guid<guid::GameObject>>, //unknown
    pub xp: u32,
    pub next_level_xp: u32,
    pub skill_infos: [u32; 384],
    pub character_points_1: u32,
    pub character_points_2: u32,
    pub track_creatures: u32,
    pub track_resources: u32,
    pub block_percentage: u32,
    pub dodge_percentage: u32,
    pub parry_percentage: u32,
    pub crit_percentage: u32,
    pub ranged_crit_percentage: u32,
    pub explored_zones: [u32; 64],
    pub rest_state_experience: u32,
    pub coinage: u32,
    pub pos_stats: [u32; 5],
    pub neg_stats: [u32; 5],
    pub resistance_buff_mods_positive: [u32; 7],
    pub resistance_buff_mods_negative: [u32; 7],
    pub mod_damage_done_pos: [u32; 7],
    pub mod_damage_done_neg: [u32; 7],
    pub mod_damage_done_pct: [u32; 7],
    pub _unknown_bytes_4: u32,
    pub ammo_id: u32,
    pub self_res_spell: u32,
    pub pvp_medals: u32,
    pub buyback_prices: [u32; 12],
    pub buyback_timestamps: [u32; 12],
    pub session_kills: u32,
    pub yesterday_kills: u32,
    pub last_week_kills: u32,
    pub this_week_kills: u32,
    pub this_week_contribution: u32,
    pub lifetime_honorable_kills: u32,
    pub lifetime_dishonorable_kills: u32,
    pub yesterday_contribution: u32,
    pub last_week_contribution: u32,
    pub last_week_rank: u32,
    pub _unknown_bytes_5: u32,
    pub watched_faction_index: u32,
    pub combat_ratings: [u32; 20],
}

#[derive(Clone, Copy)]
pub struct QuestLogFields {
    pub log_1: u32,
    pub log_2: u32,
    pub log_3: u32,
}

impl UpdateWritable for QuestLogFields {
    fn get_update_blocks_count() -> usize {
        3
    }

    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.log_1;
        blocks[1] = self.log_2;
        blocks[2] = self.log_3;
    }
}

pub struct VisibleItemFields {
    pub creator: Option<Guid<guid::Player>>,
    pub item_id: u32,
    pub enchantment_ids: [u32; 2],
    pub unkn: [u32; 5],
    pub random_properties_id: u32,
    pub property_seed: u32,
}

impl Default for VisibleItemFields {
    fn default() -> Self {
        Self {
            creator: None,
            item_id: 0,
            enchantment_ids: [0; 2],
            unkn: [0; 5],
            random_properties_id: 0,
            property_seed: 0,
        }
    }
}

impl UpdateWritable for VisibleItemFields {
    fn get_update_blocks_count() -> usize {
        12
    }

    fn write(&self, blocks: &mut [u32]) {
        self.creator.write(&mut blocks[0..=1]);
        self.item_id.write(&mut blocks[2..=2]);
        for (i, v) in self
            .enchantment_ids
            .iter()
            .enumerate()
            .map(|(i, v)| (i + 3, v))
        {
            v.write(&mut blocks[i..=i]);
        }
        for (i, v) in self.unkn.iter().enumerate().map(|(i, v)| (i + 5, v)) {
            v.write(&mut blocks[i..=i]);
        }
        blocks[10] = self.random_properties_id;
        blocks[11] = self.property_seed;
    }
}

#[bitfield(u32)]
pub struct PlayerFieldBytes2 {
    pub facial_hair: u8,
    pub _unknown: u8,
    pub bank_bag_slots: u8,
    pub rested_state: u8,
}

impl UpdateWritable for PlayerFieldBytes2 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}

#[bitfield(u32)]
pub struct PlayerFieldBytes3 {
    pub gender: bool,
    #[bits(15)]
    pub drunk_value: u16,
    pub _unknown: u8,
    pub honor_rank: u8,
}

impl UpdateWritable for PlayerFieldBytes3 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}

#[bitfield(u64)]
pub struct TutorialFlags {
    // Questgivers: Questgivers have exclamation marks over their heads. Talk to questgivers by moving close to them and right clicking on them.
    pub entered_world_questgivers: bool, // shown together with entered_world
    // Movement: You can move with the ASDW keys, with the arrow keys or by holding down both the left and right mouse buttons.
    pub not_moved_for_90_seconds: bool,
    // Cameras: You can rotate your camera view by dragging the left mouse button in the play field. You can rotate your character and your view at the same time by dragging the right mouse button in the play field.
    pub reached_level_4_cameras: bool,
    // Targeting: Left-Click selects a target and Right-Click interacts with it.
    pub moved_for_the_first_time: bool, // shown 10 seconds after moving
    // Combat Mode: You enter combat mode by right clicking on your target and then moving into combat range. You will automatically start swinging at your target.
    pub targeted_attackable_unit: bool,
    // Spells and Abilities: You can cast spells and use special abilities on the enemy by clicking on the buttons in your action bar along the lower left portion of the screen.
    pub swung_in_melee: bool,
    // Looting: Right-Click on a creatureâs corpse to loot it. You can then right click on items in the loot pane to place them in your backpack.
    pub unit_became_lootable: bool,
    // Backpack: An item went into your backpack. You can click on the backpack button in the lower right part of the screen to open your backpack. Move the mouse over the item to see what it is.
    pub received_item: bool,
    // Using Items: Right click on items to use them. You can drag usable items to your action bar if you want to be able to use it without opening your backpack.
    pub received_usable_item: bool,
    // Bags: You can put bags in the empty bag spaces in the lower right part of your screen next to the backpack, and then click on them to open them.
    pub received_bag: bool,
    // Food: You can eat some food to regain your health faster. Click on the food icon in the action bar across the bottom left of your screen. Food will not work in combat, however.
    pub low_health_in_combat: bool,
    // Drink: You can drink to regain your mana faster. Click on the drink icon in the action bar across the bottom left of your screen. The mana regeneration will stop if you do any other ability or get in combat.
    pub low_mana_in_combat: bool,
    // Learning Talents: You can learn a new talent in the talent interface. Open the talents page by clicking the pulsing talent button on your action bar.
    pub reached_level_10: bool,
    // Trainers: You can go to your trainer in your the starting area and learn a new skill. You may have to search around a little to find your trainer.
    pub reached_level_4_trainers: bool,
    // Spells and Abilities Book: You can move spells and abilities to your action bar by opening the abilities page with the button in the bottom center of the screen and then dragging the ability icon to your action bar. You can also use a spell or ability from the abilities page by clicking on it.
    pub _unused_spellbook: bool, // never shown by the client
    // Reputation: You can look at your reputation with different groups in the world in the character pane under the reputation tab.
    pub reputation_changed_at_level_10: bool,
    // Replying to Tells: You can respond to that player by hitting the R key and then typing a message, or by typing /tell <theirname> and then the message.
    pub received_whisper: bool,
    // Grouping: You can invite another player to your group by right-clicking on their portrait and selecting the Invite option from the popup menu.
    pub targeted_friendly_player_at_level_5: bool,
    // Players: Thatâs another real person playing a character. You can tell by the blue name above their head.
    pub targeted_player: bool,
    // Vendors: Right-clicking an item in the merchant pane will buy that item if you have enough money. While the merchant pane is open, right-clicking an item in your backpack will sell the item.
    pub opened_vendor: bool,
    // Quest Log: You can open your quest log to look at the quest by clicking on the gold chalice in the middle of the bar across the bottom of the screen.
    pub _unused_quest_log: bool, // never shown by the client
    // Friends: If there is another player you have enjoyed working with, add them to your friends list! Click on the social button and add them to your list of friends.
    pub reached_level_7_friends: bool,
    // Chatting: You can send a message by hitting the enter key and typing a message. Other players nearby will hear what you say.
    pub reached_level_3: bool,
    // Equippable Items: You can equip items by opening your character screen with the button in the bottom center of the screen, and dragging them from your backpack onto your character.
    pub received_equippable_item: bool,
    // Death: You are now a ghost. You can return to life by either finding your corpse or talking to a nearby spirit healer. Your corpse shows up as an icon in the minimap at the upper right hand portion of the screen.
    pub became_ghost: bool,
    // Rested: You are rested. Being rested gives you a temporary bonus to experience from killing monsters.
    pub became_rested: bool,
    // Fatigue: If you stray into deep and uncharted waters, you will see a Fatigue bar. If you become completely fatigued, you will begin to drown.
    pub fatigue_timer_started: bool,
    // Swimming: Swimming is much like walking, except you can steer upwards and downwards by holding down the right mouse button and looking in the direction you want to go.
    pub started_swimming: bool,
    // Breath: You will see a Breath bar pop up when your character becomes submerged in water. If you run out of breath, you will begin to drown.
    pub breath_timer_started: bool,
    // Resting: You are now resting, indicated by your portrait glowing yellow. Time spent resting or logged out gives you a temporary bonus to experience from killing monsters. You may want to find an Innkeeper and get a hearthstone which will allow you to quickly return later.
    pub started_resting: bool,
    // Hearthstones: You now have a hearthstone. Hearthstones can be used to transport you from your current location to the last Inn that you acquired the hearthstone from. You can only use your hearthstone once per hour.
    pub received_hearthstone: bool,
    // Player vs. Player Combat: You are engaging in Player vs. Player combat. While you are participating in Player vs. Player combat the symbol of your alliance will appear next to your portrait and you can be attacked by enemy players.
    pub flagged_for_pvp: bool,
    // Jumping: You can press the spacebar to make your character jump. Jumping can help you past obstacles, and it can be particularly useful if you're having trouble getting out of the water and onto dry land.
    pub reached_level_8: bool,
    // Quest Completion: You have completed your first quest! To collect your reward, you should return to the character who gave you the quest. When you complete a quest, you can see the corresponding quest giver on your minimap, provided you are nearby.
    pub completed_quest_objectives: bool,
    // Travel: You have clicked a flight master who trains flying beasts to carry passengers from one location to another. For a minimal fee, you can swiftly travel to other flight masters that you have interacted with in the past. When you discover a new city, finding the flight master will allow you to return easily in the future.
    pub targeted_flight_master: bool,
    // Damaged Items: The durability of one of your items is getting low. The paper doll below your minimap indicates the damaged item in yellow. Find a merchant in town to repair the item before it breaks.
    pub item_durability_low: bool,
    // Broken Items: One of your items has broken! The paper doll below your minimap indicates the broken item in red. You can get the item repaired by a merchant in town. Until you do, you will gain no benefit from the item.
    pub item_broke: bool,
    // Professions: Your character can learn up to two professions which will allow you to find or create items of value. To learn more about professions ask a guard in a major city for directions to the profession trainers.
    pub reached_level_7_professions: bool,
    // Groups: You may want to invite other players to team up with you to more easily overcome your enemies. Many difficult quests can be quickly completed in a group, as quest credit is shared by the group. Moreover, groups earn bonus experience, relative to solo players.
    pub reached_level_5: bool,
    // The Spellbook: You have learned a new spell or ability! Use the Spellbook & Abilities button in your Action Bar to open your spellbook. Left-click and drag a spell or ability to move it to your Action Bar. The spellbook is organized by category, as indicated by the tabs sticking out from the right side of the book.
    pub learned_spell: bool,
    // Elite Quests: You have accepted an elite quest. Such quests are best undertaken in a group, for they will take you into areas inhabited by elite creatures. These creatures are significantly tougher than normal monsters; however, they are worth more experience. You can tell an elite creature by the golden dragon border around its portrait.
    pub accepted_elite_quest: bool,
    // Welcome to World of Warcraft!: When you encounter something new, a help button will appear in the bottom center of your screen. Click the button for a brief explanation of how to interact with that part of the world. These hints will start with the basics and then progress to more advanced topics as you gain in experience. Thank you for playing, and good luck in your adventures!
    pub entered_world: bool,
    // Unavailable Quest Givers: An NPC with a gray '!' over its head has a quest that you are too low level to accept. Check back once you gain a few levels.
    pub near_unavailable_quest_giver: bool,
    // Ranged Weapons: You have obtained a ranged weapon. To use it, equip it, and then open up your spellbook and drag the Shoot or Throw ability into your Action Bar.
    pub received_ranged_weapon_as_non_hunter: bool,
    // Ammunition: You cannot fire bows and guns without ammunition. To purchase ammunition, visit a gun or bow merchant in a city. To equip ammunition, right-click it.
    pub cast_failed_without_ammo: bool,
    // Raid Groups: You have joined a raid group: a group with an increased limit of 40 members. While in a raid group, you will not earn credit towards most non-raid quests by killing creatures or collecting items.
    pub joined_raid: bool,
    // Meeting Stones: You are now waiting to join a group through a meeting stone. The meeting stone indicator is a bubble attached to the mini-map. You can click on that indicator to remove yourself from the meeting stone queue.
    pub joined_meeting_stone_queue: bool,
    // Battleground Queue: You are now in a queue to enter a battleground. You may check your status by mousing over the icon on your minimap.
    pub queued_for_battleground: bool,
    // Port to Battleground: You are now eligible to join battle. Click "Join Battle" in the dialog or right click the battleground icon on the minimap.
    pub battleground_ready: bool,
    // Keyrings: You now have a keyring to hold your dungeon keys. This keyring appears to the left of your bags on your action bar. The keyring can only store permanent keys.
    pub received_key: bool,
    #[bits(14)]
    __: u16,
}

impl TutorialFlags {
    // The number of flags the client knows about
    pub const COUNT: u32 = 50;
}
