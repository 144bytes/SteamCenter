//! The profile: its summary, level, recent games and its look (background, mini-profile, avatar frame,
//! animated avatar, theme) chosen from the items the account owns.

use base64::Engine;
use serde::Serialize;
use serde_json::Value;

use crate::library::icon_url;
use crate::{list, num, text, Client, Result, SteamError};

const ITEM_IMAGES: &str = "https://cdn.akamai.steamstatic.com/steamcommunity/public/images/";
/// The avatar of an account that never set one.
const DEFAULT_AVATAR: &str = "fef49e7fa7e1997310d705b2a6158ff8dc1cdfeb";

#[derive(Debug, Clone, Default, Serialize)]
pub struct PlayerSummary {
    pub steamid: String,
    pub name: String,
    pub profile_url: String,
    pub avatar: String,
    /// "public", "friends", "private".
    pub visibility: &'static str,
    /// Unix time; 0 - Steam doesn't say.
    pub created: u64,
    /// Online now (any state but "offline").
    pub online: bool,
    /// When last seen online (unix time); 0 - Steam doesn't say.
    pub last_online: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentGame {
    pub appid: u32,
    pub name: String,
    pub icon: String,
    /// Minutes in the last two weeks and in all.
    pub playtime_2weeks: u64,
    pub playtime: u64,
}

/// What the profile can wear: one item of each kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Background,
    MiniBackground,
    AvatarFrame,
    AnimatedAvatar,
}

impl ItemKind {
    pub const ALL: [ItemKind; 4] = [ItemKind::Background, ItemKind::MiniBackground, ItemKind::AvatarFrame, ItemKind::AnimatedAvatar];

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == s)
    }

    pub fn key(self) -> &'static str {
        match self {
            ItemKind::Background => "background",
            ItemKind::MiniBackground => "mini_background",
            ItemKind::AvatarFrame => "avatar_frame",
            ItemKind::AnimatedAvatar => "animated_avatar",
        }
    }

    /// The list in GetProfileItemsOwned.
    fn owned(self) -> &'static str {
        match self {
            ItemKind::Background => "profile_backgrounds",
            ItemKind::MiniBackground => "mini_profile_backgrounds",
            ItemKind::AvatarFrame => "avatar_frames",
            ItemKind::AnimatedAvatar => "animated_avatars",
        }
    }

    /// The field in GetProfileItemsEquipped.
    fn equipped(self) -> &'static str {
        match self {
            ItemKind::Background => "profile_background",
            ItemKind::MiniBackground => "mini_profile_background",
            ItemKind::AvatarFrame => "avatar_frame",
            ItemKind::AnimatedAvatar => "animated_avatar",
        }
    }

    fn setter(self) -> &'static str {
        match self {
            ItemKind::Background => "IPlayerService/SetProfileBackground/v1",
            ItemKind::MiniBackground => "IPlayerService/SetMiniProfileBackground/v1",
            ItemKind::AvatarFrame => "IPlayerService/SetAvatarFrame/v1",
            ItemKind::AnimatedAvatar => "IPlayerService/SetAnimatedAvatar/v1",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileItem {
    pub id: String,
    pub name: String,
    pub image: String,
    pub appid: u32,
}

impl ProfileItem {
    fn from(v: &Value) -> Option<Self> {
        let id = v.get("communityitemid").map(text).filter(|id| !id.is_empty() && id != "0")?;
        let name = [v.get("item_title"), v.get("name")].into_iter().flatten().filter_map(Value::as_str).find(|s| !s.is_empty());
        let image = v.get("image_small").or_else(|| v.get("image_large")).and_then(Value::as_str).unwrap_or_default();
        Some(Self {
            id,
            name: name.unwrap_or_default().to_string(),
            image: if image.is_empty() || image.starts_with("http") { image.to_string() } else { format!("{ITEM_IMAGES}{image}") },
            appid: v.get("appid").map_or(0, |a| num(a) as u32),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemSlot {
    pub kind: ItemKind,
    /// The worn item's id; empty - none.
    pub equipped: String,
    pub items: Vec<ProfileItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Theme {
    pub id: String,
    pub title: String,
}

/// One account of a GetPlayerLinkDetails answer (the service method the Steam site's mini-profiles use: unlike the
/// old ISteamUser/GetPlayerSummaries it takes the web session's token). `steamid` - when the answer leaves it out.
fn persona(account: &Value, steamid: &str) -> Option<PlayerSummary> {
    let public = account.get("public_data")?;
    let s = |k: &str| public.get(k).map(text).unwrap_or_default();
    let steamid = Some(s("steamid")).filter(|id| !id.is_empty() && id != "0").unwrap_or_else(|| steamid.to_string());
    let private = |k: &str| account.get("private_data").and_then(|p| p.get(k)).map_or(0, num);
    // The custom address of the profile ("id/<name>"), or the full address, or none.
    let vanity = s("profile_url");
    let profile_url = if vanity.starts_with("https://") {
        vanity
    } else if vanity.is_empty() {
        format!("https://steamcommunity.com/profiles/{steamid}/")
    } else {
        format!("https://steamcommunity.com/id/{vanity}/")
    };
    let hash = public.get("sha_digest_avatar").and_then(Value::as_str).and_then(avatar_hash).unwrap_or_else(|| DEFAULT_AVATAR.into());
    Some(PlayerSummary {
        name: s("persona_name"),
        profile_url,
        avatar: format!("https://avatars.steamstatic.com/{hash}_full.jpg"),
        visibility: match public.get("visibility_state").map_or(0, num) {
            3 => "public",
            2 => "friends",
            _ => "private",
        },
        created: private("time_created"),
        online: private("persona_state") > 0,
        last_online: private("last_logoff_time").max(private("last_seen_online")),
        steamid,
    })
}

fn summary(steamid: &str, v: &Value) -> Option<PlayerSummary> {
    persona(list(v, "accounts").first()?, steamid)
}

/// The avatar's SHA-1 as hex: Steam sends the bytes in base64 (or, at times, already as hex). All zeros - no avatar.
fn avatar_hash(s: &str) -> Option<String> {
    let hex = if s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        s.to_ascii_lowercase()
    } else {
        let bytes = base64::engine::general_purpose::STANDARD.decode(s).ok().filter(|b| b.len() == 20)?;
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    };
    hex.bytes().any(|b| b != b'0').then_some(hex)
}

fn slots(owned: &Value, equipped: &Value) -> Vec<ItemSlot> {
    ItemKind::ALL
        .into_iter()
        .map(|kind| ItemSlot {
            kind,
            equipped: equipped.get(kind.equipped()).and_then(|e| e.get("communityitemid")).map(text).filter(|id| id != "0").unwrap_or_default(),
            items: list(owned, kind.owned()).iter().filter_map(ProfileItem::from).collect(),
        })
        .collect()
}

impl Client {
    pub fn summary(&self) -> Result<PlayerSummary> {
        let r = self.get("IPlayerService/GetPlayerLinkDetails/v1", &[("steamids[0]", self.steamid())])?;
        summary(self.steamid(), &r).ok_or_else(|| SteamError::BadAnswer("no account in GetPlayerLinkDetails".into()))
    }

    /// Profiles of many accounts (friends), in batches.
    pub(crate) fn personas(&self, steamids: &[String]) -> Result<Vec<PlayerSummary>> {
        let mut out = Vec::with_capacity(steamids.len());
        for chunk in steamids.chunks(100) {
            let params: Vec<(String, &str)> = chunk.iter().enumerate().map(|(i, id)| (format!("steamids[{i}]"), id.as_str())).collect();
            let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            let r = self.get("IPlayerService/GetPlayerLinkDetails/v1", &params)?;
            out.extend(list(&r, "accounts").iter().filter_map(|a| persona(a, "")).filter(|p| !p.steamid.is_empty()));
        }
        Ok(out)
    }

    pub fn level(&self) -> Result<u32> {
        let r = self.get("IPlayerService/GetSteamLevel/v1", &[("steamid", self.steamid())])?;
        Ok(r.get("player_level").map_or(0, num) as u32)
    }

    /// Games played in the last two weeks.
    pub fn recent_games(&self) -> Result<Vec<RecentGame>> {
        let r = self.get("IPlayerService/GetRecentlyPlayedGames/v1", &[("steamid", self.steamid()), ("count", "0")])?;
        Ok(list(&r, "games")
            .iter()
            .filter_map(|g| {
                let appid = num(g.get("appid")?) as u32;
                Some(RecentGame {
                    appid,
                    name: g.get("name").map(text).unwrap_or_default(),
                    icon: icon_url(appid, &g.get("img_icon_url").map(text).unwrap_or_default()),
                    playtime_2weeks: g.get("playtime_2weeks").map_or(0, num),
                    playtime: g.get("playtime_forever").map_or(0, num),
                })
            })
            .collect())
    }

    /// The items the account owns, by kind, and which of them the profile wears.
    pub fn item_slots(&self) -> Result<Vec<ItemSlot>> {
        let owned = self.get("IPlayerService/GetProfileItemsOwned/v1", &[("language", &self.language)])?;
        let equipped = self.get("IPlayerService/GetProfileItemsEquipped/v1", &[("steamid", self.steamid()), ("language", &self.language)])?;
        Ok(slots(&owned, &equipped))
    }

    /// Wear the item (`id`), or take the worn one off (`id` empty).
    pub fn set_item(&self, kind: ItemKind, id: &str) -> Result<()> {
        let id = if id.is_empty() { "0" } else { id };
        self.post(kind.setter(), &[("communityitemid".into(), id.into())]).map(drop)
    }

    /// The themes the profile may use and the current one (empty id - the default theme).
    pub fn themes(&self) -> Result<(Vec<Theme>, String)> {
        let r = self.get("IPlayerService/GetProfileThemesAvailable/v1", &[])?;
        let themes = list(&r, "profile_themes")
            .iter()
            .filter_map(|t| {
                let id = t.get("theme_id").map(text).filter(|id| !id.is_empty())?;
                let title = t.get("title").map(text).filter(|s| !s.is_empty()).unwrap_or_else(|| id.clone());
                Some(Theme { id, title })
            })
            .collect();
        let c = self.get("IPlayerService/GetProfileCustomization/v1", &[("steamid", self.steamid())])?;
        let current = c.pointer("/profile_theme/theme_id").map(text).unwrap_or_default();
        Ok((themes, current))
    }

    pub fn set_theme(&self, id: &str) -> Result<()> {
        self.post("IPlayerService/SetProfileTheme/v1", &[("theme_id".into(), id.into())]).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_is_read() {
        // sha_digest_avatar: 20 bytes in base64.
        let v: Value = serde_json::from_str(
            r#"{"accounts":[{"public_data":{"steamid":"76561198000000001","visibility_state":3,"persona_name":"Gabe",
                "profile_url":"gaben","sha_digest_avatar":"ASNFZ4mrze8BI0VniavN7wEjRWc="},
                "private_data":{"persona_state":1,"time_created":1100000000,"last_logoff_time":1700000000,"last_seen_online":1700000500}}]}"#,
        )
        .unwrap();
        let s = summary("76561198000000009", &v).unwrap();
        assert_eq!((s.name.as_str(), s.visibility, s.created), ("Gabe", "public", 1100000000));
        // The answer's own steamid wins over the one asked for.
        assert_eq!(s.steamid, "76561198000000001");
        assert_eq!((s.online, s.last_online), (true, 1700000500));
        assert_eq!(s.profile_url, "https://steamcommunity.com/id/gaben/");
        assert_eq!(s.avatar, "https://avatars.steamstatic.com/0123456789abcdef0123456789abcdef01234567_full.jpg");
        assert!(summary("1", &serde_json::json!({ "accounts": [] })).is_none());
    }

    #[test]
    fn summary_without_custom_address_or_avatar() {
        let v: Value = serde_json::from_str(
            r#"{"accounts":[{"public_data":{"visibility_state":1,"persona_name":"x","sha_digest_avatar":"AAAAAAAAAAAAAAAAAAAAAAAAAAA="}}]}"#,
        )
        .unwrap();
        let s = summary("76561198000000001", &v).unwrap();
        assert_eq!(s.profile_url, "https://steamcommunity.com/profiles/76561198000000001/");
        assert_eq!(s.avatar, format!("https://avatars.steamstatic.com/{DEFAULT_AVATAR}_full.jpg"));
        assert_eq!((s.visibility, s.created), ("private", 0));
        assert_eq!(avatar_hash("FEF49E7FA7E1997310D705B2A6158FF8DC1CDFEB").as_deref(), Some(DEFAULT_AVATAR));
    }

    #[test]
    fn item_slots_are_read() {
        let owned: Value = serde_json::from_str(
            r#"{"profile_backgrounds":[{"communityitemid":"111","item_title":"Night","image_small":"items/570/a.jpg","appid":570},
                                       {"communityitemid":"0"}],
                "avatar_frames":[{"communityitemid":222,"name":"Frame","image_small":"https://x/f.png"}]}"#,
        )
        .unwrap();
        let equipped: Value = serde_json::from_str(r#"{"profile_background":{"communityitemid":"111"},"avatar_frame":{"communityitemid":"0"}}"#).unwrap();
        let s = slots(&owned, &equipped);
        assert_eq!(s.len(), 4);
        assert_eq!((s[0].kind, s[0].equipped.as_str(), s[0].items.len()), (ItemKind::Background, "111", 1));
        assert_eq!(s[0].items[0].image, format!("{ITEM_IMAGES}items/570/a.jpg"));
        assert_eq!((s[2].equipped.as_str(), s[2].items[0].id.as_str(), s[2].items[0].image.as_str()), ("", "222", "https://x/f.png"));
        assert!(s[3].items.is_empty());
        assert_eq!(ItemKind::parse("mini_background"), Some(ItemKind::MiniBackground));
        assert_eq!(ItemKind::parse("hat"), None);
    }
}
