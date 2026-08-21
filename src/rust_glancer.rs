use std::{env, fs};

use zed_extension_api::{
    self as zed, Architecture, Command, DownloadedFileType, LanguageServerId, Os, Result, Worktree,
    settings::LspSettings,
};

const LANGUAGE_SERVER_ID: &str = "rust-glancer";
const GITHUB_REPO: &str = "rust-glancer/rust-glancer";
const PACKAGE_PREFIX: &str = "rust-glancer";
/// Path of the native server binary inside the published VSIX archives.
const BINARY_PATH_IN_PACKAGE: &str = "extension/server/rust-glancer";
/// Subcommand that runs the language server over stdio.
const LSP_SUBCOMMAND: &str = "lsp";

/// A resolved language server invocation.
struct ResolvedServer {
    command: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

struct RustGlancerExtension;

impl zed::Extension for RustGlancerExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        match language_server_id.as_ref() {
            LANGUAGE_SERVER_ID => {
                let resolved = self.language_server_binary(language_server_id, worktree)?;
                Ok(Command {
                    command: resolved.command,
                    args: resolved.args,
                    env: resolved.env,
                })
            }
            _ => Err(format!("unknown language server: {language_server_id}")),
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Ok(None);
        }

        LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
            .map(|settings| settings.initialization_options)
            .map_err(|e| format!("failed to get initialization options: {e}"))
    }
}

impl RustGlancerExtension {
    /// Resolves the language server binary, arguments and env.
    ///
    /// Resolution order:
    /// 1. User-provided `lsp.rust-glancer.binary` settings (`path`, optional `arguments`, `env`).
    /// 2. Prebuilt binary downloaded from the latest GitHub release VSIX asset.
    fn language_server_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<ResolvedServer> {
        let binary = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|settings| settings.binary);
        if let Some(binary) = binary {
            let env = binary
                .env
                .into_iter()
                .flat_map(|env| env.into_iter())
                .collect();
            let args = binary
                .arguments
                .unwrap_or_else(|| vec![LSP_SUBCOMMAND.into()]);
            if let Some(path) = binary.path {
                return Ok(ResolvedServer {
                    command: path,
                    args,
                    env,
                });
            }
        }

        let command = self.download_latest_server_binary(language_server_id)?;
        Ok(ResolvedServer {
            command,
            args: vec![LSP_SUBCOMMAND.into()],
            env: Vec::new(),
        })
    }

    /// Ensures the prebuilt server is downloaded into the extension working directory
    /// and returns its path.
    fn download_latest_server_binary(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            GITHUB_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )
        .map_err(|e| format!("failed to query latest rust-glancer release: {e}"))?;

        // Strip 'v' prefix from the tag name.
        let version = release
            .version
            .strip_prefix('v')
            .unwrap_or(&release.version);
        let package_name = format!("{PACKAGE_PREFIX}-{version}");
        let binary_path = format!("{package_name}/{BINARY_PATH_IN_PACKAGE}");

        if !self.server_binary_exists(&binary_path) {
            let platform_suffix = platform_suffix()?;
            let asset_name = format!("{PACKAGE_PREFIX}-{version}-{platform_suffix}.vsix");
            let download_url = format!(
                "https://github.com/{GITHUB_REPO}/releases/download/v{version}/{asset_name}"
            );

            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(&download_url, &package_name, DownloadedFileType::Zip).map_err(
                |e| format!("failed to download {asset_name}: {e} (from {download_url})"),
            )?;

            zed::make_file_executable(&binary_path)
                .map_err(|e| format!("failed to make {binary_path} executable: {e}"))?;

            self.clean_old_packages(&package_name)
                .map_err(|e| format!("failed to clean old packages: {e}"))?;
        }

        Ok(binary_path)
    }

    fn server_binary_exists(&self, binary_path: &str) -> bool {
        fs::metadata(binary_path).is_ok_and(|stat| stat.is_file())
    }

    /// Removes old downloaded packages but keeps the latest one.
    fn clean_old_packages(&self, latest_package_name: &str) -> Result<()> {
        let current_dir =
            env::current_dir().map_err(|e| format!("failed to get current directory: {e}"))?;
        for entry in fs::read_dir(current_dir)
            .map_err(|e| format!("failed to read extension directory: {e}"))?
        {
            let path = match entry.map(|entry| entry.path()) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("failed to read directory entry: {e}");
                    continue;
                }
            };

            if !path.is_dir() {
                continue;
            }

            let dir_name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string());
            let Some(dir_name) = dir_name else {
                continue;
            };

            if dir_name.starts_with(PACKAGE_PREFIX)
                && dir_name != latest_package_name
                && fs::remove_dir_all(&path).is_err()
            {
                eprintln!("failed to remove old package directory {dir_name}");
            }
        }

        Ok(())
    }
}

/// Maps the current platform onto the platform suffix used by rust-glancer release assets.
fn platform_suffix() -> Result<&'static str> {
    let (os, architecture) = zed::current_platform();
    match (os, architecture) {
        (Os::Mac, Architecture::Aarch64) => Ok("darwin-arm64"),
        (Os::Mac, Architecture::X8664) => Ok("darwin-x64"),
        (Os::Linux, Architecture::Aarch64) => Ok("linux-arm64"),
        (Os::Linux, Architecture::X8664) => Ok("linux-x64"),
        (os, architecture) => Err(format!(
            "unsupported platform for rust-glancer: {os:?} {architecture:?}. \
             Prebuilt binaries are only published for macOS and Linux. \
             You can point Zed at a local build via `lsp.rust-glancer.binary.path`."
        )),
    }
}

zed::register_extension!(RustGlancerExtension);
