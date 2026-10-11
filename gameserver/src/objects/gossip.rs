use std::collections::HashMap;

use common::guid::{self, Guid};
use sqlx::{Pool, Sqlite};

pub struct GossipMenuTree {
    pub root_menu_id: u32,
    pub menus: HashMap<u32, GossipMenu>,
}

impl GossipMenuTree {
    pub async fn load_from_db(db: &Pool<Sqlite>, root_menu_id: u32) -> Result<Self, sqlx::Error> {
        let mut menus = HashMap::new();
        let mut menus_to_load = vec![root_menu_id];

        while let Some(menu_id) = menus_to_load.pop() {
            if menus.contains_key(&menu_id) {
                continue;
            }

            let menu = GossipMenu::load_from_db(db, menu_id).await?;
            menus_to_load.extend(menu.items.iter().filter_map(|item| item.next_menu_id));
            menus.insert(menu_id, menu);
        }

        Ok(Self {
            root_menu_id,
            menus,
        })
    }
}

pub struct GossipMenu {
    pub title_text_id: u32,
    pub items: Vec<GossipMenuItem>,
}

impl GossipMenu {
    pub async fn load_from_db(db: &Pool<Sqlite>, id: u32) -> Result<Self, sqlx::Error> {
        let title_text_id =
            sqlx::query_scalar!("SELECT title_text_id FROM gossip_menu WHERE id = ?", id)
                .fetch_one(db)
                .await?;

        let items = sqlx::query!(
            "SELECT position, icon, coded, text, next_menu_id
             FROM gossip_menu_item
             WHERE menu_id = ?
             ORDER BY position",
            id
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|item| GossipMenuItem {
            index: item.position as u32,
            icon: item.icon as u8,
            coded: item.coded,
            text: item.text,
            next_menu_id: item.next_menu_id.map(|id| id as u32),
        })
        .collect();

        Ok(Self {
            title_text_id: title_text_id as u32,
            items,
        })
    }
}

pub struct GossipMenuItem {
    pub index: u32,
    pub icon: u8,
    pub coded: bool,
    pub text: String,
    pub next_menu_id: Option<u32>,
}

#[derive(Clone, Copy)]
pub struct GossipCursor {
    pub source_guid: Guid<guid::Unit>,
    pub menu_id: u32,
}

pub struct NpcText {
    pub id: u32,
    pub text_male: String,
    pub text_female: String,
    pub language: u32,
}
