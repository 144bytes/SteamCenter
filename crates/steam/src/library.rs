//! The game library: owned games, the games marked private ("Mark as private" in the Steam client), and what the
//! store says about apps (names, tags, content descriptors).

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use crate::{list, num, text, Client, Result};

/// Apps per ToggleAppPrivacy / GetItems request.
const BATCH: usize = 100;
/// A pause between write requests: Steam limits how fast an account may change things.
pub(crate) const PAUSE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Serialize)]
pub struct OwnedGame {
    pub appid: u32,
    pub name: String,
    /// Minutes.
    pub playtime: u64,
    /// Unix time; 0 - never.
    pub last_played: u64,
    /// Icon hash, see `icon_url`.
    pub icon: String,
}

impl OwnedGame {
    fn from(v: &Value) -> Option<Self> {
        Some(Self {
            appid: u32::try_from(num(v.get("appid")?)).ok().filter(|&a| a != 0)?,
            name: v.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
            playtime: v.get("playtime_forever").map_or(0, num),
            last_played: v.get("rtime_last_played").map_or(0, num),
            icon: v.get("img_icon_url").map(text).unwrap_or_default(),
        })
    }
}

/// The small square icon of an app (from its icon hash).
pub fn icon_url(appid: u32, hash: &str) -> String {
    if hash.is_empty() {
        return String::new();
    }
    format!("https://media.steampowered.com/steamcommunity/public/images/apps/{appid}/{hash}.jpg")
}

fn appids(v: &Value) -> BTreeSet<u32> {
    v.as_array().map(|a| a.iter().filter_map(|x| u32::try_from(num(x)).ok()).filter(|&a| a != 0).collect()).unwrap_or_default()
}

impl Client {
    pub fn owned_games(&self) -> Result<Vec<OwnedGame>> {
        let r = self.get(
            "IPlayerService/GetOwnedGames/v1",
            &[
                ("steamid", self.steamid()),
                ("include_appinfo", "true"),
                ("include_played_free_games", "true"),
                ("skip_unvetted_apps", "false"),
                ("language", &self.language),
            ],
        )?;
        Ok(list(&r, "games").iter().filter_map(OwnedGame::from).collect())
    }

    /// The apps marked private.
    pub fn private_apps(&self) -> Result<BTreeSet<u32>> {
        let r = self.get("IAccountPrivateAppsService/GetPrivateAppList/v1", &[])?;
        Ok(appids(r.pointer("/private_apps/appids").unwrap_or(&Value::Null)))
    }

    /// Mark the apps private (`private`) or not, in batches. `progress(done, total)` after each batch.
    pub fn set_private(&self, apps: &[u32], private: bool, mut progress: impl FnMut(usize, usize)) -> Result<()> {
        let mut done = 0;
        for chunk in apps.chunks(BATCH) {
            if done > 0 {
                std::thread::sleep(PAUSE);
            }
            let mut params = vec![("private".to_string(), private.to_string())];
            params.extend(chunk.iter().enumerate().map(|(i, id)| (format!("appids[{i}]"), id.to_string())));
            self.post("IAccountPrivateAppsService/ToggleAppPrivacy/v1", &params)?;
            done += chunk.len();
            progress(done, apps.len());
        }
        Ok(())
    }

    /// Store names of apps (for those the library doesn't name: private ones, the wishlist). Unknown ones are left out.
    pub fn app_names(&self, apps: &[u32]) -> Result<HashMap<u32, String>> {
        Ok(self.store_items(apps, false)?.into_iter().filter(|(_, i)| !i.name.is_empty()).map(|(id, i)| (id, i.name)).collect())
    }

    /// What the store says about apps: name, and with `tags` their user tags and content descriptors.
    /// Apps the store doesn't know (removed from sale) are left out.
    pub fn store_items(&self, apps: &[u32], tags: bool) -> Result<HashMap<u32, StoreInfo>> {
        let mut items = HashMap::new();
        for chunk in apps.chunks(BATCH) {
            let input = json!({
                "ids": chunk.iter().map(|a| json!({ "appid": a })).collect::<Vec<_>>(),
                "context": { "language": self.language, "country_code": "US" },
                "data_request": if tags { json!({ "include_tag_count": 20, "include_ratings": true }) } else { json!({}) },
            });
            let r = self.get("IStoreBrowseService/GetItems/v1", &[("input_json", &input.to_string())])?;
            items.extend(parse_items(&r));
        }
        Ok(items)
    }

    /// The names of the store's user tags.
    pub fn tag_names(&self) -> Result<HashMap<u32, String>> {
        let r = self.get("IStoreService/GetTagList/v1", &[("language", &self.language)])?;
        Ok(list(&r, "tags")
            .iter()
            .filter_map(|t| Some((u32::try_from(num(t.get("tagid")?)).ok()?, t.get("name").map(text).filter(|n| !n.is_empty())?)))
            .collect())
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct StoreInfo {
    pub name: String,
    pub tags: Vec<u32>,
    /// Marked by the developer as having sexual content or nudity (content descriptors 1, 3, 4).
    pub adult: bool,
    pub free: bool,
}

/// EContentDescriptorID: some nudity or sexual content, adult only sexual content, frequent nudity or sexual content.
const ADULT_DESCRIPTORS: [u64; 3] = [1, 3, 4];

fn parse_items(r: &Value) -> impl Iterator<Item = (u32, StoreInfo)> + '_ {
    list(r, "store_items").iter().filter_map(|x| {
        let id = u32::try_from(num(x.get("appid").or_else(|| x.get("id"))?)).ok().filter(|&a| a != 0)?;
        // Ordered by weight when given ("tags"), else as listed ("tagids").
        let tags = match x.get("tags").and_then(Value::as_array) {
            Some(t) => t.iter().filter_map(|t| u32::try_from(num(t.get("tagid")?)).ok()).collect(),
            None => list(x, "tagids").iter().filter_map(|t| u32::try_from(num(t)).ok()).collect(),
        };
        Some((
            id,
            StoreInfo {
                name: x.get("name").map(text).unwrap_or_default(),
                tags,
                adult: list(x, "content_descriptorids").iter().any(|d| ADULT_DESCRIPTORS.contains(&num(d))),
                free: x.get("is_free").and_then(Value::as_bool).unwrap_or(false),
            },
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_games_are_read() {
        let v: Value = serde_json::from_str(
            r#"{"game_count":2,"games":[
                {"appid":570,"name":"Dota 2","playtime_forever":1234,"img_icon_url":"abc","rtime_last_played":1700000000},
                {"appid":0,"name":"bad"},
                {"appid":"730"}]}"#,
        )
        .unwrap();
        let games: Vec<OwnedGame> = list(&v, "games").iter().filter_map(OwnedGame::from).collect();
        assert_eq!(games.len(), 2);
        assert_eq!((games[0].appid, games[0].playtime, games[0].last_played), (570, 1234, 1700000000));
        assert_eq!(games[1].name, "");
        assert_eq!(icon_url(570, "abc"), "https://media.steampowered.com/steamcommunity/public/images/apps/570/abc.jpg");
        assert_eq!(icon_url(570, ""), "");
    }

    #[test]
    fn private_list_and_names_are_read() {
        let v: Value = serde_json::from_str(r#"{"private_apps":{"appids":[10,"20",0]}}"#).unwrap();
        assert_eq!(appids(v.pointer("/private_apps/appids").unwrap()), BTreeSet::from([10, 20]));
    }

    #[test]
    fn store_items_are_read() {
        let n: Value = serde_json::from_str(
            r#"{"store_items":[
                {"id":10,"appid":10,"name":"Counter-Strike","tagids":[19,1663],"is_free":false},
                {"id":20,"appid":20,"name":"Visual Novel","tags":[{"tagid":24904,"weight":900},{"tagid":492}],"content_descriptorids":[3,5],"is_free":true},
                {"id":30,"appid":30,"name":"Shooter","content_descriptorids":[2,5]},
                {"id":11,"success":2}]}"#,
        )
        .unwrap();
        let items: HashMap<u32, StoreInfo> = parse_items(&n).collect();
        assert_eq!(items.len(), 4);
        assert_eq!((items[&10].name.as_str(), items[&10].tags.clone(), items[&10].adult), ("Counter-Strike", vec![19, 1663], false));
        assert_eq!((items[&20].tags.clone(), items[&20].adult, items[&20].free), (vec![24904, 492], true, true));
        assert!(!items[&30].adult);
        assert_eq!(items[&11].name, "");
    }
}
