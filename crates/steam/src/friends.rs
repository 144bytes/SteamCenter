//! Friends: who is on the friends list, when each was last online, and removing them.

use serde_json::Value;

use crate::library::PAUSE;
use crate::{list, text, Client, PlayerSummary, Result};

/// EFriendRelationship: an accepted friend (not a request, not blocked).
const FRIEND: u64 = 3;

fn friend_ids(r: &Value) -> Vec<String> {
    let friends = r.get("friendslist").map_or(&[][..], |l| list(l, "friends"));
    friends
        .iter()
        .filter(|f| f.get("efriendrelationship").map_or(0, crate::num) == FRIEND)
        .filter_map(|f| f.get("ulfriendid").map(text).filter(|id| !id.is_empty() && id != "0"))
        .collect()
}

impl Client {
    /// The friends with their profiles. A friend whose profile Steam doesn't return is left out.
    pub fn friends(&self) -> Result<Vec<PlayerSummary>> {
        let r = self.get("IFriendsListService/GetFriendsList/v1", &[])?;
        self.personas(&friend_ids(&r))
    }

    /// One request per friend; `progress(done, total)` after each.
    pub fn remove_friends(&self, steamids: &[String], mut progress: impl FnMut(usize, usize)) -> Result<()> {
        for (i, id) in steamids.iter().enumerate() {
            if i > 0 {
                std::thread::sleep(PAUSE);
            }
            self.post("IPlayerService/RemoveFriend/v1", &[("steamid".into(), id.clone())])?;
            progress(i + 1, steamids.len());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepted_friends_are_listed() {
        let v: Value = serde_json::from_str(
            r#"{"friendslist":{"bincremental":false,"friends":[
                {"ulfriendid":"76561198000000002","efriendrelationship":3},
                {"ulfriendid":"76561198000000003","efriendrelationship":2},
                {"ulfriendid":"76561198000000004","efriendrelationship":1},
                {"ulfriendid":"0","efriendrelationship":3}]}}"#,
        )
        .unwrap();
        assert_eq!(friend_ids(&v), vec!["76561198000000002".to_string()]);
        assert!(friend_ids(&Value::Null).is_empty());
    }
}
