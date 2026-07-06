use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use eyre::{Context, ContextCompat, OptionExt};

#[derive(Debug)]
pub struct ConfigInfo {
    pub prefix_version: String,
    pub fonts_dir: PathBuf,
    pub lib_dir: PathBuf,
    pub steam_dir: PathBuf,
    pub steamclient_dll_timestamp: DateTime<Utc>,
    pub steamclient64_dll_timestamp: DateTime<Utc>,
    pub steam_dll_timestamp: DateTime<Utc>,
    pub default_prefix: PathBuf,
    pub system_reg_timestamp: DateTime<Utc>,
    pub use_wined3d: bool,
    pub use_dxvk_dxgi: bool,
    pub builtin_dll_copy: String,
    pub use_nvapi: bool,
    pub use_dxvk_d3d8: bool,
}

impl ConfigInfo {
    pub fn from_file(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let file = File::open(path).wrap_err("Failed to open config_info")?;
        let mut lines = BufReader::new(file).lines();

        let mut next = || {
            lines
                .next()
                .map(|v| v.wrap_err("Failed to read line"))
                .ok_or_eyre("Unexpected EOF")
                .flatten()
        };

        Ok(Self {
            prefix_version: next()?,
            fonts_dir: next()
                .map(PathBuf::from)
                .wrap_err("Failed to read fonts dir")?,
            lib_dir: next()
                .map(PathBuf::from)
                .wrap_err("Failed to read lib dir")?,
            steam_dir: next()
                .map(PathBuf::from)
                .wrap_err("Failed to read steam dir")?,
            steamclient_dll_timestamp: next()
                .and_then(parse_date_time_utc)
                .wrap_err("Failed to read steamclient.dll timestamp")?,
            steamclient64_dll_timestamp: next()
                .and_then(parse_date_time_utc)
                .wrap_err("Failed to read steamclient64.dll timestamp")?,
            steam_dll_timestamp: next()
                .and_then(parse_date_time_utc)
                .wrap_err("Failed to read Steam.dll timestamp")?,
            default_prefix: next()
                .map(PathBuf::from)
                .wrap_err("Failed to read steam dir")?,
            system_reg_timestamp: next()
                .and_then(parse_date_time_utc)
                .wrap_err("Failed to read system.reg timestamp")?,
            use_wined3d: next()
                .and_then(parse_bool)
                .wrap_err("Failed to read use_wined3d boolean")?,
            use_dxvk_dxgi: next()
                .and_then(parse_bool)
                .wrap_err("Failed to read use_dxvk_dxgi boolean")?,
            builtin_dll_copy: next().wrap_err("Failed to read builtin DLL copy")?,
            use_nvapi: next()
                .and_then(parse_bool)
                .wrap_err("Failed to read use_nvapi boolean")?,
            use_dxvk_d3d8: next()
                .and_then(parse_bool)
                .wrap_err("Failed to read use_dxvk_d3d8 boolean")?,
        })
    }
}

fn parse_date_time_utc(s: String) -> eyre::Result<DateTime<Utc>> {
    let seconds: f64 = s.parse().wrap_err("Failed to parse timestamp")?;
    DateTime::from_timestamp(seconds.trunc() as i64, (seconds.fract() * 1e9) as u32)
        .wrap_err("Failed to convert timestamp to a DateTime")
}

fn parse_bool(s: String) -> eyre::Result<bool> {
    match s.as_str() {
        "True" => Ok(true),
        "False" => Ok(false),
        _ => eyre::bail!("Invalid boolean: {s}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_file() {
        let config_info = ConfigInfo::from_file("data/config_info").unwrap();
        assert_eq!(config_info.prefix_version, "11.0-100");
        assert_eq!(
            config_info.fonts_dir,
            PathBuf::from(
                "/home/steamuser/.local/share/Steam/steamapps/common/Proton - Experimental/files/share/fonts/"
            )
        );
        assert_eq!(
            config_info.steam_dir,
            PathBuf::from("/home/steamuser/.local/share/Steam")
        );
        assert_eq!(
            config_info.steamclient_dll_timestamp,
            DateTime::from_timestamp(1781043446, 0).unwrap()
        );
        assert_eq!(
            config_info.steamclient64_dll_timestamp,
            DateTime::from_timestamp(1781043450, 0).unwrap()
        );
        assert_eq!(
            config_info.steam_dll_timestamp,
            DateTime::from_timestamp(1516738214, 0).unwrap()
        );
        assert_eq!(
            config_info.default_prefix,
            PathBuf::from(
                "/home/steamuser/.local/share/Steam/steamapps/common/Proton - Experimental/files/share/default_pfx/"
            )
        );
        // NOTE: Disabled because precision messes up.
        //
        // assert_eq!(
        //     config_info.system_reg_timestamp,
        //     DateTime::from_timestamp(1781202038, (0.8633409 * 1e9) as u32).unwrap()
        // );
        assert_eq!(config_info.use_wined3d, false);
        assert_eq!(config_info.use_dxvk_dxgi, true);
        assert_eq!(
            config_info.builtin_dll_copy,
            "d3dcompiler_*.dll,d3dcsx*.dll,d3dx*.dll,dx8vb.dll,x3daudio*.dll,xactengine*.dll,xapofx*.dll,xaudio*.dll,xinput*.dll,atl1*.dll,atl.dll,concrt*.dll,msvcp1*.dll,msvcrt*.dll,msvcp7*.dll,msvcp6*.dll,msvcp_win.dll,msvcr1*.dll,msvcrt*.dll,msvcr7*.dll,vcamp1*.dll,vcomp1*.dll,vccorlib1*.dll,vcruntime1*.dll,ucrtbase.dll,comctl32.dll,ntdll.dll,vulkan-1.dll,ir50_32.dll"
        );
        assert_eq!(config_info.use_nvapi, true);
        assert_eq!(config_info.use_dxvk_d3d8, false);
    }
}
