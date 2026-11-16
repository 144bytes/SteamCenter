//! Trading cards: which games still have card drops, read from the account's badges page in the Steam Community (no
//! Web API method tells it), and whether a game uses Valve Anti-Cheat (from the store's app details).
//!
//! The badges page needs the community's own `steamLoginSecure` cookie. It is asked for in English, so the numbers
//! are always written the same way.

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::{agent, answer, failed, Result, SteamError};

const COMMUNITY: &str = "https://steamcommunity.com";
/// Badge rows that are not games: the Steam Controller and the Summer sale badges.
const NOT_GAMES: [u32; 2] = [368020, 335590];
/// The store's category "Valve Anti-Cheat enabled".
const VAC_CATEGORY: u64 = 8;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CardGame {
    pub appid: u32,
    pub name: String,
    /// Card drops left.
    pub cards: u32,
    /// Hours on record.
    pub hours: f64,
}

/// The games with card drops left, over all the badges pages.
pub fn card_drops(steamid: &str, login_secure: &str) -> Result<Vec<CardGame>> {
    let mut games = Vec::new();
    let mut page = 1;
    loop {
        let html = badges_page(steamid, login_secure, page)?;
        let (rows, last) = parse_badges(&html);
        games.extend(rows.into_iter().filter(|g| g.cards > 0));
        if page >= last || page >= 50 {
            break;
        }
        page += 1;
        std::thread::sleep(Duration::from_millis(500));
    }
    Ok(games)
}

fn badges_page(steamid: &str, login_secure: &str, page: u32) -> Result<String> {
    let r = agent()
        .get(&format!("{COMMUNITY}/profiles/{steamid}/badges/"))
        .query("l", "english")
        .query("p", &page.to_string())
        .set("Cookie", &format!("steamLoginSecure={login_secure}; Steam_Language=english"))
        .call();
    let resp = r.map_err(|e| failed("community/badges", e))?;
    // Not signed in to the community: the page sends to the sign-in or shows no one.
    if resp.get_url().contains("/login") {
        return Err(SteamError::Unauthorized);
    }
    let html = resp.into_string().map_err(|e| SteamError::Network(e.to_string()))?;
    if html.contains("g_steamID = false;") {
        return Err(SteamError::Unauthorized);
    }
    Ok(html)
}

/// Badge rows of one page, and the number of the last page.
fn parse_badges(html: &str) -> (Vec<CardGame>, u32) {
    let rows = html
        .split("class=\"badge_row is_link\"")
        .skip(1)
        .filter_map(|row| {
            let href = between(row, "class=\"badge_row_overlay\" href=\"", "\"")?;
            if href.contains("border=1") {
                return None;
            }
            let appid: u32 = between(href, "/gamecards/", "/")?.parse().ok()?;
            if NOT_GAMES.contains(&appid) {
                return None;
            }
            let title = between(row, "class=\"badge_title\">", "<")?;
            let name = decode(title.split("&nbsp;").next().unwrap_or(title)).trim().to_string();
            let cards = between(row, "class=\"progress_info_bold\">", "<").and_then(first_number).unwrap_or(0.0) as u32;
            let hours = between(row, "class=\"badge_title_stats_playtime\">", "<").and_then(first_number).unwrap_or(0.0);
            Some(CardGame { appid, name, cards, hours })
        })
        .collect();
    let last = html
        .split("class=\"pagelink\" href=\"?p=")
        .skip(1)
        .filter_map(|s| s.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok())
        .max()
        .unwrap_or(1);
    (rows, last)
}

/// Whether the game uses Valve Anti-Cheat, by the store's public app details (one game per request).
/// A game the store doesn't describe counts as without it.
pub fn has_vac(appid: u32) -> Result<bool> {
    let r = agent()
        .get("https://store.steampowered.com/api/appdetails")
        .query("appids", &appid.to_string())
        .query("filters", "categories")
        .call();
    let v = answer("store/appdetails", r)?;
    let categories = v.pointer(&format!("/{appid}/data/categories")).and_then(Value::as_array);
    Ok(categories.is_some_and(|c| c.iter().any(|x| x.get("id").and_then(Value::as_u64) == Some(VAC_CATEGORY))))
}

/// The text between `start` and the next `end` after it.
fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = s.find(start)? + start.len();
    let to = s[from..].find(end)? + from;
    Some(&s[from..to])
}

/// The first number in English writing ("1,234.5 hrs on record", "3 card drops remaining").
fn first_number(s: &str) -> Option<f64> {
    let start = s.find(|c: char| c.is_ascii_digit())?;
    let digits: String = s[start..].chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').filter(|c| *c != ',').collect();
    digits.trim_end_matches('.').parse().ok()
}

fn decode(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ").replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"
        <div class="badge_row is_link">
          <a class="badge_row_overlay" href="https://steamcommunity.com/id/me/gamecards/570/"></a>
          <div class="badge_title_stats_playtime">&nbsp; 1,234.5 hrs on record </div>
          <div class="badge_title">Dota 2 &amp; Co&nbsp;<span class="badge_view_details">View details</span></div>
          <span class="progress_info_bold">3 card drops remaining</span>
        </div>
        <div class="badge_row is_link">
          <a class="badge_row_overlay" href="https://steamcommunity.com/id/me/gamecards/730/"></a>
          <div class="badge_title">Counter-Strike 2&nbsp;</div>
          <span class="progress_info_bold">No card drops remaining</span>
        </div>
        <div class="badge_row is_link">
          <a class="badge_row_overlay" href="https://steamcommunity.com/id/me/gamecards/440/?border=1"></a>
          <div class="badge_title">Foil&nbsp;</div>
          <span class="progress_info_bold">1 card drop remaining</span>
        </div>
        <div class="badge_row is_link">
          <a class="badge_row_overlay" href="https://steamcommunity.com/id/me/gamecards/368020/"></a>
          <div class="badge_title">Steam Controller&nbsp;</div>
        </div>
        <a class="pagelink" href="?p=2">2</a> <a class="pagelink" href="?p=3">3</a>"#;

    #[test]
    fn badges_page_is_read() {
        let (rows, last) = parse_badges(PAGE);
        assert_eq!(last, 3);
        assert_eq!(rows, vec![
            CardGame { appid: 570, name: "Dota 2 & Co".into(), cards: 3, hours: 1234.5 },
            CardGame { appid: 730, name: "Counter-Strike 2".into(), cards: 0, hours: 0.0 },
        ]);
        assert_eq!(parse_badges("<html>nothing</html>"), (vec![], 1));
    }

    #[test]
    fn numbers_in_english() {
        assert_eq!(first_number(" 0.4 hrs on record"), Some(0.4));
        assert_eq!(first_number("12 card drops remaining"), Some(12.0));
        assert_eq!(first_number("No card drops remaining"), None);
    }
}
