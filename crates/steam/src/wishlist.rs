//! The wishlist: what is on it and taking games off it.

use serde::Serialize;
use serde_json::Value;

use crate::library::PAUSE;
use crate::{list, num, Client, Result};

#[derive(Debug, Clone, Serialize)]
pub struct WishItem {
    pub appid: u32,
    /// Unix time.
    pub added: u64,
    /// The user's own order (0 - not ranked).
    pub priority: u64,
}

fn items(r: &Value) -> Vec<WishItem> {
    list(r, "items")
        .iter()
        .filter_map(|x| {
            let appid = u32::try_from(num(x.get("appid")?)).ok().filter(|&a| a != 0)?;
            Some(WishItem { appid, added: x.get("date_added").map_or(0, num), priority: x.get("priority").map_or(0, num) })
        })
        .collect()
}

/// A wishlist game's capsule picture from the store.
pub fn capsule_url(appid: u32) -> String {
    format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/capsule_184x69.jpg")
}

impl Client {
    pub fn wishlist(&self) -> Result<Vec<WishItem>> {
        Ok(items(&self.get("IWishlistService/GetWishlist/v1", &[("steamid", self.steamid())])?))
    }

    /// One request per game; `progress(done, total)` after each.
    pub fn remove_from_wishlist(&self, apps: &[u32], mut progress: impl FnMut(usize, usize)) -> Result<()> {
        for (i, appid) in apps.iter().enumerate() {
            if i > 0 {
                std::thread::sleep(PAUSE);
            }
            self.post("IWishlistService/RemoveFromWishlist/v1", &[("appid".into(), appid.to_string())])?;
            progress(i + 1, apps.len());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wishlist_is_read() {
        let v: Value = serde_json::from_str(r#"{"items":[{"appid":620,"priority":1,"date_added":1600000000},{"appid":0},{"priority":2}]}"#).unwrap();
        let w = items(&v);
        assert_eq!(w.len(), 1);
        assert_eq!((w[0].appid, w[0].priority, w[0].added), (620, 1, 1600000000));
        assert!(items(&Value::Null).is_empty());
    }
}
