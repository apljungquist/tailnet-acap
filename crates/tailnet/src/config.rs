use std::{fmt, fs, path::PathBuf};

use anyhow::Context;
use tailscale::keys::PersistState;
use url::Url;

#[derive(Clone)]
pub struct AuthKey(String);

impl AuthKey {
    pub fn new(key: String) -> Self {
        Self(key)
    }

    pub fn into_exposed(self) -> String {
        self.0
    }
}

impl fmt::Debug for AuthKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AuthKey").field(&"<redacted>").finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Preferences {
    pub control_server_url: Url,
    pub requested_hostname: String,
    pub requested_tags: Vec<String>,
    pub ephemeral: bool,
}

impl Preferences {
    pub fn try_default() -> anyhow::Result<Self> {
        Ok(Self {
            control_server_url: "https://controlplane.tailscale.com"
                .parse()
                .context("could not parse control plane literal")?,
            requested_hostname: nix::unistd::gethostname()
                .map(|s| s.to_string_lossy().to_string())
                .context("could not get hostname")?,
            requested_tags: Vec::new(),
            ephemeral: false,
        })
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct StoredConfig {
    pub preferences: Preferences,
    pub persist_state: PersistState,
}

impl StoredConfig {
    fn try_default() -> anyhow::Result<Self> {
        Ok(Self {
            preferences: Preferences::try_default()?,
            persist_state: PersistState::default(),
        })
    }

    pub fn ensure() -> anyhow::Result<Self> {
        if let Some(v) = Self::read()? {
            return Ok(v);
        }

        let v = Self::try_default()?;
        v.write()?;
        Ok(v)
    }

    fn file() -> anyhow::Result<PathBuf> {
        Ok(easy_acap_dirs::localdata_dir()
            .context("could not determine the localdata dir")?
            .join("config.json"))
    }

    fn read() -> anyhow::Result<Option<Self>> {
        match fs::read_to_string(Self::file()?) {
            Ok(text) => Ok(Some(serde_json::from_str(&text)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn write(&self) -> anyhow::Result<()> {
        let path = Self::file()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("could not create localdata")?;
        }
        let staged = path.with_extension("json.tmp");
        fs::write(&staged, serde_json::to_string_pretty(self)?)?;
        fs::rename(&staged, &path)?;
        Ok(())
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "should panic: panicking unwraps are used to signal test failures"
)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_can_be_built() {
        let Preferences { .. } = Preferences::try_default().unwrap();
    }
}
