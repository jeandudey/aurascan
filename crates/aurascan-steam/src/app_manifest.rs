use std::io::BufReader;
use std::path::Path;
use std::{fs::File, path::PathBuf};

use eyre::Context;
use serde::{Deserialize, Deserializer};

use crate::ConfigInfo;

// NOTE: Uses String for the fields I don't care about, if needed
// switch to proper type deserialization.
#[derive(Debug, Deserialize)]
pub struct AppManifest {
    #[serde(deserialize_with = "de_str", rename = "appid")]
    pub app_id: u64,
    pub universe: Option<String>,
    pub name: String,
    #[serde(rename = "StateFlags")]
    pub state_flags: String,
    pub installdir: String,
    #[serde(rename = "LastUpdated")]
    pub last_updated: String,
    #[serde(rename = "LastPlayed")]
    pub last_played: String,
    #[serde(rename = "SizeOnDisk")]
    pub size_on_disk: String,
    #[serde(rename = "buildid")]
    pub build_id: String,
    #[serde(rename = "LastOwner")]
    pub last_owner: String,
    #[serde(rename = "DownloadType")]
    pub download_type: String,
    #[serde(rename = "UpdateResult")]
    pub update_result: String,
}

impl AppManifest {
    pub fn from_file(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let file = File::open(path).wrap_err("Failed to open app manifest")?;
        keyvalues_serde::from_reader(BufReader::new(file))
            .wrap_err("Failed to deserialize app manifest")
    }

    pub fn wine_prefix(&self, steam_dir: impl AsRef<Path>) -> PathBuf {
        steam_dir
            .as_ref()
            .join(format!("steamapps/compatdata/{}/pfx", self.app_id))
    }

    pub fn config_info(&self, steam_dir: impl AsRef<Path>) -> eyre::Result<Option<ConfigInfo>> {
        let path = steam_dir
            .as_ref()
            .join(format!("steamapps/compatdata/{}/config_info", self.app_id));

        if path.exists() {
            ConfigInfo::from_file(path).map(Some)
        } else {
            Ok(None)
        }
    }
}

fn de_str<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let s = String::deserialize(d)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_file() {
        let app_manifest = AppManifest::from_file("data/appmanifest_480.acf").unwrap();
        assert_eq!(app_manifest.app_id, 480);
    }
}
