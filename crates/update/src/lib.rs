//! Updates from GitHub Releases of the `SteamCenter` repository: find the latest version and download a release file.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The repository whose releases carry the program.
const REPO: &str = "144bytes/SteamCenter";
pub const ASSET: &str = "SteamCenter.exe";

/// Releases are small; anything bigger is not ours.
const MAX_DOWNLOAD: u64 = 64 * 1024 * 1024;

/// `major.minor.patch`, as in the root Cargo.toml and the release tag (`v1.2.3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32);

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.trim().trim_start_matches('v').split('.').map(|p| p.parse::<u32>().ok());
        let v = Version(parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(v)
    }

    /// The version of the program this crate is built into (the whole workspace shares one version).
    pub fn current() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION")).expect("workspace version is major.minor.patch")
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Why an update failed. `Display` is for the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    NoRelease,
    Unreachable(String),
    /// Unexpected tag or files.
    BadRelease(String),
    Download(String),
    /// Writing or swapping the exe.
    File(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::NoRelease => f.write_str("there are no releases on GitHub yet"),
            UpdateError::Unreachable(e) => write!(f, "GitHub is unreachable: {e}"),
            UpdateError::BadRelease(e) | UpdateError::Download(e) | UpdateError::File(e) => f.write_str(e),
        }
    }
}

#[derive(Debug, Clone)]
struct Asset {
    name: String,
    url: String,
    size: u64,
}

/// The latest published release.
#[derive(Debug, Clone)]
pub struct Release {
    pub version: Version,
    assets: Vec<Asset>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .user_agent(concat!("SteamCenter/", env!("CARGO_PKG_VERSION")))
        .build()
}

pub fn latest() -> Result<Release, UpdateError> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let body = match agent().get(&url).set("Accept", "application/vnd.github+json").call() {
        Ok(r) => r.into_string().map_err(|e| UpdateError::Unreachable(format!("could not read the answer: {e}")))?,
        Err(ureq::Error::Status(404, _)) => return Err(UpdateError::NoRelease),
        Err(e) => return Err(UpdateError::Unreachable(e.to_string())),
    };
    parse_release(&body)
}

fn parse_release(body: &str) -> Result<Release, UpdateError> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| UpdateError::BadRelease(format!("unexpected GitHub answer: {e}")))?;
    let tag = v["tag_name"].as_str().unwrap_or_default();
    let version = Version::parse(tag).ok_or_else(|| UpdateError::BadRelease(format!("unexpected release version '{tag}'")))?;
    let assets = v["assets"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    Some(Asset {
                        name: x["name"].as_str()?.to_string(),
                        url: x["browser_download_url"].as_str()?.to_string(),
                        size: x["size"].as_u64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Release { version, assets })
}

impl Release {
    fn asset(&self, name: &str) -> Result<&Asset, UpdateError> {
        self.assets.iter().find(|a| a.name == name).ok_or_else(|| UpdateError::BadRelease(format!("release {} has no file {name}", self.version)))
    }

    /// Download the release file `name` to `dest`. A download cut short (the size differs from what GitHub
    /// reports) is an error, so a broken file never replaces the program.
    pub fn download_to(&self, name: &str, dest: &Path) -> Result<(), UpdateError> {
        let asset = self.asset(name)?;
        if asset.size > MAX_DOWNLOAD {
            return Err(UpdateError::BadRelease(format!("{name} is too large ({} bytes)", asset.size)));
        }
        let r = agent().get(&asset.url).call().map_err(|e| UpdateError::Download(format!("could not download {name}: {e}")))?;
        let mut data = Vec::with_capacity(asset.size as usize);
        r.into_reader().take(MAX_DOWNLOAD).read_to_end(&mut data).map_err(|e| UpdateError::Download(format!("download of {name} interrupted: {e}")))?;
        if data.len() as u64 != asset.size {
            return Err(UpdateError::Download(format!("{name} downloaded incompletely ({} of {} bytes)", data.len(), asset.size)));
        }
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| UpdateError::File(format!("{}: {e}", dir.display())))?;
        }
        std::fs::write(dest, data).map_err(|e| UpdateError::File(format!("{}: {e}", dest.display())))
    }
}

/// Where the running exe goes while it is replaced: into `old_dir` (the app's data folder), so nothing is left next
/// to the exe; next to it only when `old_dir` is on another disk (a running exe can't be moved across disks).
fn old_paths(exe: &Path, old_dir: &Path) -> [PathBuf; 2] {
    let mut name = exe.file_name().unwrap_or_default().to_os_string();
    name.push(".old");
    [old_dir.join(&name), exe.with_file_name(name)]
}

/// Put `new_file` in place of the running exe: Windows does not let a running exe be overwritten, but it can be
/// moved. The running file becomes `<exe>.old` (removed at the next start), `new_file` takes its name.
/// Returns the exe path to start.
pub fn replace_running_exe(new_file: &Path, old_dir: &Path) -> Result<PathBuf, UpdateError> {
    let exe = std::env::current_exe().map_err(|e| UpdateError::File(e.to_string()))?;
    let _ = std::fs::create_dir_all(old_dir);
    let mut moved = None;
    let mut last_err = String::new();
    for old in old_paths(&exe, old_dir) {
        let _ = std::fs::remove_file(&old);
        match std::fs::rename(&exe, &old) {
            Ok(()) => {
                moved = Some(old);
                break;
            }
            Err(e) => last_err = e.to_string(),
        }
    }
    let Some(old) = moved else { return Err(UpdateError::File(format!("{}: {last_err}", exe.display()))) };
    if let Err(e) = std::fs::rename(new_file, &exe).or_else(|_| std::fs::copy(new_file, &exe).map(|_| ())) {
        let _ = std::fs::rename(&old, &exe);
        return Err(UpdateError::File(format!("{}: {e}", exe.display())));
    }
    let _ = std::fs::remove_file(new_file);
    Ok(exe)
}

/// Remove `<exe>.old` left by an update; the previous process may still be exiting, so retry for a while.
pub fn remove_previous_exe(old_dir: &Path) {
    let Ok(exe) = std::env::current_exe() else { return };
    for old in old_paths(&exe, old_dir) {
        for _ in 0..30 {
            if !old.exists() || std::fs::remove_file(&old).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert_eq!(Version::parse("v1.2.3"), Some(Version(1, 2, 3)));
        assert_eq!(Version::parse("1.10.0").unwrap().cmp(&Version::parse("1.9.9").unwrap()), std::cmp::Ordering::Greater);
        assert_eq!(Version::parse("1.2"), None);
        assert_eq!(Version::parse("1.2.3.4"), None);
        assert_eq!(Version::parse("v1.x.3"), None);
        assert_eq!(Version::current().to_string(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn github_release_json_is_parsed() {
        let body = r#"{"tag_name":"v1.0.1","assets":[
            {"name":"SteamCenter.exe","browser_download_url":"https://example/SteamCenter.exe","size":11602432}]}"#;
        let r = parse_release(body).unwrap();
        assert_eq!(r.version, Version(1, 0, 1));
        let a = r.asset(ASSET).unwrap();
        assert_eq!((a.url.as_str(), a.size), ("https://example/SteamCenter.exe", 11602432));
        assert!(r.asset("other.exe").is_err());
        assert!(parse_release(r#"{"tag_name":"latest"}"#).is_err());
    }

    #[test]
    fn old_exe_goes_to_the_data_folder_first() {
        let [first, second] = old_paths(Path::new(r"C:\Downloads\SteamCenter.exe"), Path::new(r"C:\Data\update"));
        assert_eq!(first, Path::new(r"C:\Data\update\SteamCenter.exe.old"));
        assert_eq!(second, Path::new(r"C:\Downloads\SteamCenter.exe.old"));
    }
}
