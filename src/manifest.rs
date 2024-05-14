use std::path::PathBuf;

use anyhow::Context as _;

#[derive(Clone)]
pub struct ManifestInformation {
    /// The raw manifest as a string
    raw: String,
    /// Absolute path to the manifest file
    path: PathBuf,
    /// The parsed manifest's config for the component under test
    component: spin_manifest::schema::v2::Component,
}

impl ManifestInformation {
    pub fn resolve(provided_path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let manifest_path = spin_common::paths::resolve_manifest_file_path(provided_path)
            .context("failed to find spin.toml manifest file")?;
        let manifest_path = manifest_path
            .canonicalize()
            .context("failed to canonicalize path")?;
        let raw_manifest = std::fs::read_to_string(&manifest_path).with_context(|| {
            format!(
                "failed to read spin.toml manifest at {}",
                manifest_path.display()
            )
        })?;
        let manifest = spin_manifest::manifest_from_str(&raw_manifest).with_context(|| {
            format!(
                "failed to deserialize spin.toml manifest at {}",
                manifest_path.display()
            )
        })?;
        if manifest.components.len() > 1 {
            anyhow::bail!("Spin applications with more than one component are not yet supported by `spin-test`")
        }
        let component = manifest
            .components
            .values()
            .next()
            .context("spin.toml did not contain any components")?
            .clone();
        Ok(Self {
            raw: raw_manifest,
            path: manifest_path,
            component,
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn component(&self) -> &spin_manifest::schema::v2::Component {
        &self.component
    }

    /// Get the source of the component under test
    pub fn app_source(&self) -> anyhow::Result<&str> {
        match &self.component.source {
            spin_manifest::schema::v2::ComponentSource::Local(path) => Ok(path),
            spin_manifest::schema::v2::ComponentSource::Remote { .. } => {
                anyhow::bail!("components with remote sources are not yet supported by `spin-test`")
            }
        }
    }

    /// Get information about how to build the test component
    pub fn build_info(&self) -> anyhow::Result<Option<BuildInfo>> {
        let spin_test_config = self.test_config()?;

        Ok(spin_test_config
            .get("build")
            .and_then(|b| b.as_str())
            .map(|build| {
                let dir = spin_test_config.get("dir").and_then(|d| d.as_str());
                BuildInfo {
                    cmd: build.to_owned(),
                    dir: dir.map(|d| d.to_owned()),
                }
            }))
    }

    /// Get the path to the test component's source
    pub fn test_path(&self) -> anyhow::Result<&std::path::Path> {
        let test_source = self
            .test_config()?
            .get("source")
            .context("component did not have a `spin-test.source` configuration")?
            .as_str()
            .context("component `spin-test.source` was not a string")?;
        Ok(std::path::Path::new(test_source))
    }

    /// Resolve a relative path from the manifest file
    pub fn relative_from(&self, path: impl AsRef<std::path::Path>) -> PathBuf {
        self.path.parent().unwrap().join(path)
    }

    fn test_config(&self) -> anyhow::Result<&toml::map::Map<String, toml::Value>> {
        let spin_test_config = self
            .component
            .tool
            .get("spin-test")
            .context("component did not have a `spin-test` tool configuration")?;
        Ok(spin_test_config)
    }
}

/// Information about how to build the test
pub struct BuildInfo {
    cmd: String,
    dir: Option<String>,
}

impl BuildInfo {
    /// Run the build command
    pub fn exec(self) -> anyhow::Result<()> {
        let mut cmd = std::process::Command::new("/bin/sh");
        if let Some(dir) = self.dir {
            cmd.current_dir(dir);
        }
        cmd.args(["-c", &self.cmd])
            .status()
            .context("failed to build component")?;
        Ok(())
    }
}
