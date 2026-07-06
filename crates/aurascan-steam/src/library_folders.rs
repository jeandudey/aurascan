use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use serde::{Deserialize, Deserializer};

use crate::AppManifest;

#[derive(Debug, Deserialize)]
pub struct LibraryFolders {
    #[serde(flatten)]
    pub folders: HashMap<String, LibraryFolder>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryFolder {
    pub path: PathBuf,
    pub label: String,
    #[serde(rename = "contentid")]
    pub content_id: String,
    #[serde(deserialize_with = "de_str", rename = "totalsize")]
    pub total_size: u64,
    #[serde(deserialize_with = "de_str")]
    pub update_clean_bytes_tally: u64,
    #[serde(deserialize_with = "de_str")]
    pub time_last_update_verified: u64,
    #[serde(deserialize_with = "de_apps_map")]
    pub apps: HashMap<u64, u64>,
}

impl LibraryFolder {
    /// Reads the application manifests for each application.
    pub fn manifests(&self) -> eyre::Result<BTreeMap<u64, AppManifest>> {
        let mut manifests = BTreeMap::new();
        for (&app_id, _bytes) in self.apps.iter() {
            let path = self
                .path
                .join(format!("steamapps/appmanifest_{app_id}.acf"));
            match AppManifest::from_file(&path) {
                Ok(manifest) => {
                    manifests.insert(app_id, manifest);
                }
                Err(err) => {
                    log::error!(
                        "Failed to parse app manifest {app_id} at {}: {err}",
                        path.display()
                    );
                }
            }
        }
        Ok(manifests)
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

fn de_apps_map<'de, D>(d: D) -> Result<HashMap<u64, u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: HashMap<String, String> = HashMap::deserialize(d)?;
    raw.into_iter()
        .map(|(k, v)| {
            Ok((
                k.parse().map_err(serde::de::Error::custom)?,
                v.parse().map_err(serde::de::Error::custom)?,
            ))
        })
        .collect()
}
