use std::{
    fs,
    path::{Path, PathBuf},
};
use zed::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result};

const SERVER_PATH: &str = "node_modules/.bin/oxc_language_server";

struct OxlintExtension;

impl OxlintExtension {
    fn server_exists(&self, path: &PathBuf) -> bool {
        fs::metadata(path).map_or(false, |stat| stat.is_file())
    }

    fn lang_server_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        let bin_path = Path::new(worktree.root_path().as_str()).join(SERVER_PATH);

        if self.server_exists(&bin_path) {
            return Ok(bin_path.to_string_lossy().to_string());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let (platform, arch) = zed::current_platform();
        let bin_name = format!(
            "@oxlint/{platform}-{arch}{linux_build}",
            platform = match platform {
                zed::Os::Mac => "darwin",
                zed::Os::Linux => "linux",
                zed::Os::Windows => "win32",
            },
            arch = match arch {
                zed::Architecture::Aarch64 => "arm64",
                zed::Architecture::X8664 => "x64",
                _ => return Err(format!("unsupported architecture: {arch:?}")),
            },
            linux_build = match platform {
                zed::Os::Linux => "-gnu",
                _ => "",
            },
        );
        let fallback = &Path::new("./node_modules").join(format!("{bin_name}/oxc_language_server"));
        let version = zed::npm_package_latest_version(&bin_name)?;

        if !self.server_exists(fallback)
            || zed::npm_package_installed_version(&bin_name)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                &language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            zed::npm_install_package(&bin_name, &version)
                .map_err(|e| format!("failed to install package {bin_name}@{version}: {e}"))?;
        }

        Ok(fallback.to_string_lossy().to_string())
    }
}

impl zed::Extension for OxlintExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let bin = self.lang_server_binary(language_server_id, worktree)?;
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;

        if let Some(binary) = settings.binary {
            return Ok(zed::Command {
                command: binary.path.map_or(bin, |path| path),
                args: vec![],
                env: Default::default(),
            });
        }

        Ok(zed::Command {
            command: bin,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(OxlintExtension);
