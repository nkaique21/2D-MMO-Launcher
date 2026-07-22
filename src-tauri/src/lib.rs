use crc32fast::Hasher;
use flate2::read::ZlibDecoder;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

mod runners;

use runners::{build_runner_command, list_runners, managed_windows_prefix_dir, resolve_runner};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameManifest {
    id: String,
    name: String,
    description: String,
    assets: ManifestAssets,
    installation: InstallationConfig,
    launch: LaunchConfig,
    update: UpdateConfig,
    #[serde(default)]
    verification: VerificationConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationConfig {
    #[serde(default)]
    required_files: Vec<String>,
    #[serde(default)]
    checksums: Vec<VerificationChecksumConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationChecksumConfig {
    path: String,
    algorithm: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestAssets {
    banner: String,
    icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallationConfig {
    methods: Vec<InstallMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallMethod {
    #[serde(rename = "type")]
    kind: String,
    label: String,
    url: Option<String>,
    runner: Option<String>,
    #[serde(rename = "compatPrefix")]
    compat_prefix: Option<String>,
    #[serde(rename = "installPath")]
    install_path: Option<String>,
    #[serde(rename = "launchAfterInstall")]
    launch_after_install: Option<bool>,
    format: Option<String>,
    #[serde(rename = "stripTopLevelDir", default)]
    strip_top_level_dir: bool,
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LaunchConfig {
    runner: String,
    executable: Option<String>,
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(rename = "unsetEnv", default)]
    unset_env: Vec<String>,
    #[serde(rename = "battlEye")]
    battl_eye: Option<BattlEyeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BattlEyeConfig {
    #[serde(default = "default_true")]
    enabled: bool,
    executable: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    install_args: Vec<String>,
    #[serde(default)]
    install_before_launch: bool,
    launch_mode: Option<String>,
    path_base: Option<String>,
    working_dir: Option<String>,
    working_dir_base: Option<String>,
    #[serde(default = "default_true")]
    required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateConfig {
    strategy: String,
    runner: Option<String>,
    #[serde(rename = "compatPrefix")]
    compat_prefix: Option<String>,
    executable: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(rename = "pathBase")]
    path_base: Option<String>,
    #[serde(rename = "workingDir")]
    working_dir: Option<String>,
    #[serde(rename = "workingDirBase")]
    working_dir_base: Option<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(rename = "unsetEnv", default)]
    unset_env: Vec<String>,
    #[serde(rename = "manifestUrl")]
    manifest_url: Option<String>,
    #[serde(rename = "manifestFormat")]
    manifest_format: Option<String>,
    #[serde(rename = "targetDir")]
    target_dir: Option<String>,
    #[serde(rename = "targetDirBase")]
    target_dir_base: Option<String>,
    #[serde(rename = "maxConcurrentDownloads")]
    max_concurrent_downloads: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteFileEntry {
    checksum: String,
    size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteBinaryEntry {
    file: String,
    checksum: String,
    size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteUpdateManifest {
    url: String,
    fallback_url: Option<String>,
    binary: Option<RemoteBinaryEntry>,
    #[serde(default)]
    files: HashMap<String, RemoteFileEntry>,
    keep_files: Option<bool>,
    min_client_version: Option<u64>,
}

#[derive(Debug, Clone)]
struct RemotePlannedFile {
    remote_path: String,
    expected: RemoteFileEntry,
    destination: PathBuf,
    staging_destination: PathBuf,
    url: String,
}

fn default_true() -> bool {
    true
}

const HTTP_TIMEOUT_SECONDS: u64 = 60;
const REMOTE_UPDATE_PROGRESS_INTERVAL: usize = 100;
const REMOTE_UPDATE_LOG_INTERVAL: usize = 100;
const REMOTE_UPDATE_DOWNLOAD_PROGRESS_INTERVAL: usize = 1;
const REMOTE_UPDATE_APPLY_PROGRESS_INTERVAL: usize = 50;
const DOWNLOAD_RETRY_ATTEMPTS: usize = 3;
const DOWNLOAD_RETRY_DELAY_SECONDS: u64 = 2;
const DEFAULT_REMOTE_UPDATE_CONCURRENCY: usize = 6;
const MAX_REMOTE_UPDATE_CONCURRENCY: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameInstall {
    game_id: String,
    install_path: String,
    runner_override: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchResult {
    game_id: String,
    runner: String,
    command: String,
    working_dir: String,
    log_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallVerificationResult {
    game_id: String,
    valid: bool,
    install_path: String,
    install_path_exists: bool,
    executable_path: Option<String>,
    executable_exists: bool,
    missing_files: Vec<String>,
    checksum_results: Vec<ChecksumVerificationResult>,
    issues: Vec<String>,
    repair_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChecksumVerificationResult {
    path: String,
    algorithm: String,
    expected: String,
    actual: Option<String>,
    valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameUpdateResult {
    game_id: String,
    checked_files: usize,
    updated_files: usize,
    skipped_files: usize,
    downloaded_bytes: u64,
    target_dir: String,
    log_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameUpdateProgress {
    game_id: String,
    status: String,
    stage: Option<String>,
    stage_label: Option<String>,
    checked_files: usize,
    updated_files: usize,
    total_files: usize,
    current_file: Option<String>,
    message: String,
    target_dir: Option<String>,
    log_path: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameInstallFlowProgress {
    game_id: String,
    status: String,
    message: String,
}

fn emit_install_flow_progress(app: &tauri::AppHandle, game_id: &str, status: &str, message: &str) {
    let _ = app.emit(
        "game-install-flow",
        GameInstallFlowProgress {
            game_id: game_id.to_string(),
            status: status.to_string(),
            message: message.to_string(),
        },
    );
}

fn manifests_dir() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir().map_err(|error| error.to_string())?;

    if current_dir.join("manifests").is_dir() {
        return Ok(current_dir.join("manifests"));
    }

    Ok(current_dir.join("src-tauri").join("manifests"))
}

fn database_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        format!("Não foi possível resolver o diretório de dados do app: {error}")
    })?;

    fs::create_dir_all(&app_data_dir).map_err(|error| {
        format!(
            "Não foi possível criar o diretório de dados {}: {error}",
            app_data_dir.display()
        )
    })?;

    Ok(app_data_dir.join("launcher.sqlite"))
}

fn open_database(app: &tauri::AppHandle) -> Result<Connection, String> {
    let path = database_path(app)?;
    let connection = Connection::open(&path)
        .map_err(|error| format!("Não foi possível abrir o banco {}: {error}", path.display()))?;

    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS installs (
                game_id TEXT PRIMARY KEY NOT NULL,
                install_path TEXT NOT NULL,
                runner_override TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )
        .map_err(|error| format!("Não foi possível preparar o schema SQLite: {error}"))?;

    Ok(connection)
}

fn get_install(connection: &Connection, game_id: &str) -> Result<GameInstall, String> {
    connection
        .query_row(
            "
            SELECT game_id, install_path, runner_override, created_at, updated_at
            FROM installs
            WHERE game_id = ?1
            ",
            params![game_id],
            |row| {
                Ok(GameInstall {
                    game_id: row.get(0)?,
                    install_path: row.get(1)?,
                    runner_override: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .map_err(|error| format!("Não foi possível carregar a instalação de {game_id}: {error}"))
}

fn save_install(
    connection: &Connection,
    game_id: &str,
    install_path: &str,
    runner_override: Option<&str>,
) -> Result<GameInstall, String> {
    connection
        .execute(
            "
            INSERT INTO installs (game_id, install_path, runner_override)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(game_id) DO UPDATE SET
                install_path = excluded.install_path,
                runner_override = excluded.runner_override,
                updated_at = CURRENT_TIMESTAMP
            ",
            params![game_id, install_path, runner_override],
        )
        .map_err(|error| format!("Não foi possível salvar a instalação: {error}"))?;

    get_install(connection, game_id)
}

fn emit_install_updated(app: &tauri::AppHandle, install: &GameInstall) {
    let _ = app.emit("install-updated", install);
}

fn open_path(path: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");

    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("start").arg("");
        command
    };

    command
        .arg(path)
        .spawn()
        .map_err(|error| format!("Não foi possível abrir o caminho {path}: {error}"))?;

    Ok(())
}

fn get_manifest(game_id: &str) -> Result<GameManifest, String> {
    list_games()?
        .into_iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| format!("Manifesto não encontrado para o jogo {game_id}."))
}

fn push_unique_path(paths: &mut Vec<PathBuf>, seen: &mut HashSet<String>, path: PathBuf) {
    let key = path.to_string_lossy().to_string();

    if seen.insert(key) {
        paths.push(path);
    }
}

fn relative_executable_install_path(command_path: &Path, executable: &str) -> Option<PathBuf> {
    let executable_path = Path::new(executable);

    if executable_path.is_absolute() {
        return command_path.parent().map(Path::to_path_buf);
    }

    let levels_to_strip = executable_path.components().count().max(1);
    let mut install_path = command_path.to_path_buf();

    for _ in 0..levels_to_strip {
        if !install_path.pop() {
            return None;
        }
    }

    Some(install_path)
}

fn command_path_for_install(install_path: &Path, executable: &str) -> PathBuf {
    let executable_path = PathBuf::from(executable);

    if executable_path.is_absolute() {
        executable_path
    } else {
        install_path.join(executable_path)
    }
}

fn configured_base_path(
    app: &tauri::AppHandle,
    game_id: &str,
    install_path: &Path,
    base: Option<&str>,
    default_runner: &str,
) -> Result<PathBuf, String> {
    match base.unwrap_or("installPath") {
        "installPath" => Ok(install_path.to_path_buf()),
        "compatPrefix" => managed_windows_prefix_dir(app, game_id, default_runner),
        runner_kind => managed_windows_prefix_dir(app, game_id, runner_kind),
    }
}

fn configured_path_for_install(
    app: &tauri::AppHandle,
    game_id: &str,
    install_path: &Path,
    path: &str,
    base: Option<&str>,
    default_runner: &str,
) -> Result<PathBuf, String> {
    let configured_path = PathBuf::from(path);

    if configured_path.is_absolute() {
        return Ok(configured_path);
    }

    Ok(
        configured_base_path(app, game_id, install_path, base, default_runner)?
            .join(configured_path),
    )
}

fn repair_strategy_for_manifest(manifest: &GameManifest) -> Option<String> {
    if manifest.update.strategy == "remoteManifest" {
        return Some("remoteManifest".to_string());
    }

    for strategy in ["archive", "windowsInstaller", "existing"] {
        if manifest
            .installation
            .methods
            .iter()
            .any(|method| method.kind == strategy)
        {
            return Some(strategy.to_string());
        }
    }

    None
}

fn effective_executable_path_for_verification(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    install_path: &Path,
) -> Result<Option<PathBuf>, String> {
    if battl_eye_replaces_main_process(manifest) {
        let Some(battl_eye) = manifest.launch.battl_eye.as_ref() else {
            return Ok(None);
        };

        return configured_path_for_install(
            app,
            game_id,
            install_path,
            &battl_eye.executable,
            battl_eye.path_base.as_deref(),
            &manifest.launch.runner,
        )
        .map(Some);
    }

    Ok(manifest
        .launch
        .executable
        .as_deref()
        .map(|executable| command_path_for_install(install_path, executable)))
}

fn missing_required_files(
    install_path: &Path,
    install_path_exists: bool,
    required_files: &[String],
) -> Vec<String> {
    if !install_path_exists {
        return required_files.to_vec();
    }

    required_files
        .iter()
        .filter(|required_file| !command_path_for_install(install_path, required_file).exists())
        .cloned()
        .collect()
}

fn verify_configured_checksums(
    install_path: &Path,
    install_path_exists: bool,
    checksums: &[VerificationChecksumConfig],
) -> Result<Vec<ChecksumVerificationResult>, String> {
    checksums
        .iter()
        .map(|checksum| {
            let algorithm = checksum.algorithm.trim().to_ascii_lowercase();

            if algorithm != "crc32" {
                return Err(format!(
                    "Algoritmo de checksum não suportado para {}: {}",
                    checksum.path, checksum.algorithm
                ));
            }

            let expected = checksum.value.trim().to_ascii_lowercase();

            if expected.len() != 8
                || !expected
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                return Err(format!(
                    "Checksum CRC32 inválido para {}: {}",
                    checksum.path, checksum.value
                ));
            }

            if Path::new(&checksum.path).is_absolute()
                || checksum.path.starts_with('/')
                || checksum.path.starts_with('\\')
            {
                return Err(format!(
                    "Caminho absoluto não permitido em verification.checksums: {}",
                    checksum.path
                ));
            }

            let relative_path = safe_remote_relative_path(&checksum.path).map_err(|_| {
                format!(
                    "Caminho inseguro em verification.checksums: {}",
                    checksum.path
                )
            })?;
            let checksum_path = install_path.join(relative_path);
            let actual = if install_path_exists && checksum_path.is_file() {
                Some(crc32_file(&checksum_path)?)
            } else {
                None
            };
            let valid = actual
                .as_deref()
                .is_some_and(|actual_value| actual_value.eq_ignore_ascii_case(&expected));

            Ok(ChecksumVerificationResult {
                path: checksum.path.clone(),
                algorithm,
                expected,
                actual,
                valid,
            })
        })
        .collect()
}

fn expand_manifest_env_value(value: &str) -> String {
    let Some(home_dir) = std::env::var_os("HOME").map(PathBuf::from) else {
        return value.to_string();
    };
    let home = home_dir.to_string_lossy();

    if value == "~" || value == "$HOME" || value == "${HOME}" {
        return home.to_string();
    }

    if let Some(rest) = value.strip_prefix("~/") {
        return format!("{home}/{rest}");
    }

    if let Some(rest) = value.strip_prefix("$HOME/") {
        return format!("{home}/{rest}");
    }

    if let Some(rest) = value.strip_prefix("${HOME}/") {
        return format!("{home}/{rest}");
    }

    value.to_string()
}

fn apply_environment_overrides(
    command: &mut runners::RunnerCommand,
    env: &HashMap<String, String>,
    unset_env: &[String],
) {
    for (key, value) in env {
        command.envs.retain(|(existing_key, _)| existing_key != key);
        command
            .envs
            .push((key.clone(), expand_manifest_env_value(value)));
    }

    for key in unset_env {
        command.envs.retain(|(existing_key, _)| existing_key != key);

        if !command
            .unset_envs
            .iter()
            .any(|existing_key| existing_key == key)
        {
            command.unset_envs.push(key.clone());
        }
    }
}

fn apply_launch_environment(manifest: &GameManifest, command: &mut runners::RunnerCommand) {
    apply_environment_overrides(command, &manifest.launch.env, &manifest.launch.unset_env);
}

fn apply_update_environment(manifest: &GameManifest, command: &mut runners::RunnerCommand) {
    apply_launch_environment(manifest, command);
    apply_environment_overrides(command, &manifest.update.env, &manifest.update.unset_env);
}

fn is_windows_system_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("windows")
    })
}

fn path_matches_manifest_executable(command_path: &Path, executable: &str) -> bool {
    let executable_path = Path::new(executable);

    if executable_path.is_absolute() {
        return command_path == executable_path;
    }

    command_path.ends_with(executable_path)
}

fn find_executable_under(root: &Path, executable: &str) -> Option<PathBuf> {
    let executable_name = Path::new(executable)
        .file_name()?
        .to_string_lossy()
        .to_string();
    let mut stack = vec![(root.to_path_buf(), 0_usize)];
    let mut matches = Vec::new();
    const MAX_SEARCH_DEPTH: usize = 10;

    while let Some((current_dir, depth)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                if depth < MAX_SEARCH_DEPTH && !is_windows_system_path(&path) {
                    stack.push((path, depth + 1));
                }

                continue;
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if !file_name.eq_ignore_ascii_case(&executable_name) {
                continue;
            }

            if Path::new(executable).components().count() > 1
                && !path_matches_manifest_executable(&path, executable)
            {
                continue;
            }

            matches.push(path);
        }
    }

    matches.sort();
    matches.into_iter().next()
}

fn install_candidate_paths(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
    let mut install_paths = Vec::new();
    let mut search_roots = Vec::new();
    let mut seen_install_paths = HashSet::new();
    let mut seen_search_roots = HashSet::new();

    for method in &manifest.installation.methods {
        let mut prefix_kinds = Vec::new();

        if let Some(prefix_kind) = method.compat_prefix.as_deref() {
            prefix_kinds.push(prefix_kind);
        }

        if let Some(runner) = method.runner.as_deref() {
            prefix_kinds.push(runner);
        }

        prefix_kinds.push(&manifest.launch.runner);

        if method.kind == "windowsInstaller" {
            prefix_kinds.push("proton");
            prefix_kinds.push("wine");
        }

        for prefix_kind in prefix_kinds {
            let Ok(prefix_root) = managed_windows_prefix_dir(app, game_id, prefix_kind) else {
                continue;
            };

            push_unique_path(
                &mut search_roots,
                &mut seen_search_roots,
                prefix_root.clone(),
            );

            if let Some(relative_install_path) = method.install_path.as_deref() {
                push_unique_path(
                    &mut install_paths,
                    &mut seen_install_paths,
                    prefix_root.join(relative_install_path),
                );
            }
        }
    }

    Ok((install_paths, search_roots))
}

fn discover_manifest_install_path(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    registered_install_path: Option<&Path>,
) -> Result<Option<(PathBuf, PathBuf)>, String> {
    let Some(executable) = manifest.launch.executable.as_deref() else {
        return Ok(None);
    };

    if let Some(install_path) = registered_install_path {
        let command_path = command_path_for_install(install_path, executable);

        if command_path.exists() {
            return Ok(Some((install_path.to_path_buf(), command_path)));
        }
    }

    let (install_paths, search_roots) = install_candidate_paths(app, game_id, manifest)?;

    for install_path in install_paths {
        let command_path = command_path_for_install(&install_path, executable);

        if command_path.exists() {
            return Ok(Some((install_path, command_path)));
        }
    }

    for search_root in search_roots {
        let Some(command_path) = find_executable_under(&search_root, executable) else {
            continue;
        };
        let Some(install_path) = relative_executable_install_path(&command_path, executable) else {
            continue;
        };

        return Ok(Some((install_path, command_path)));
    }

    Ok(None)
}

fn reconcile_registered_install_path(
    app: &tauri::AppHandle,
    connection: &Connection,
    game_id: &str,
    manifest: &GameManifest,
    install: GameInstall,
) -> Result<GameInstall, String> {
    let registered_install_path = PathBuf::from(&install.install_path);

    if battl_eye_replaces_main_process(manifest) {
        let Some(battl_eye) = manifest.launch.battl_eye.as_ref() else {
            return Ok(install);
        };
        let battl_eye_path = configured_path_for_install(
            app,
            game_id,
            &registered_install_path,
            &battl_eye.executable,
            battl_eye.path_base.as_deref(),
            &manifest.launch.runner,
        )?;

        append_runner_log(
            app,
            game_id,
            &[
                "install_path_reconcile_skipped=main_battl_eye_launch".to_string(),
                format!("effective_launch_executable={}", battl_eye_path.display()),
                format!(
                    "effective_launch_executable_exists={}",
                    battl_eye_path.exists()
                ),
            ],
        )?;

        return Ok(install);
    }

    let Some((resolved_install_path, command_path)) =
        discover_manifest_install_path(app, game_id, manifest, Some(&registered_install_path))?
    else {
        return Ok(install);
    };

    if resolved_install_path == registered_install_path {
        return Ok(install);
    }

    let saved_install = save_install(
        connection,
        game_id,
        &resolved_install_path.to_string_lossy(),
        install.runner_override.as_deref(),
    )?;
    emit_install_updated(app, &saved_install);

    append_runner_log(
        app,
        game_id,
        &[
            "install_path_reconciled=true".to_string(),
            format!(
                "previous_install_path={}",
                registered_install_path.display()
            ),
            format!("resolved_install_path={}", resolved_install_path.display()),
            format!("resolved_executable_path={}", command_path.display()),
        ],
    )?;

    Ok(saved_install)
}

fn reconcile_or_register_install_path(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
) -> Result<Option<GameInstall>, String> {
    let connection = open_database(app)?;
    let existing_install = get_install(&connection, game_id).ok();
    let registered_install_path = existing_install
        .as_ref()
        .map(|install| PathBuf::from(&install.install_path));

    let Some((resolved_install_path, command_path)) =
        discover_manifest_install_path(app, game_id, manifest, registered_install_path.as_deref())?
    else {
        append_runner_log(
            app,
            game_id,
            &["install_path_reconcile_result=not_found".to_string()],
        )?;

        return Ok(None);
    };

    let should_save = registered_install_path.as_ref() != Some(&resolved_install_path);

    if should_save {
        let runner_override = existing_install
            .as_ref()
            .and_then(|install| install.runner_override.as_deref());
        let saved_install = save_install(
            &connection,
            game_id,
            &resolved_install_path.to_string_lossy(),
            runner_override,
        )?;
        emit_install_updated(app, &saved_install);

        append_runner_log(
            app,
            game_id,
            &[
                "install_registered_after_installer=true".to_string(),
                format!("registered_install_path={}", saved_install.install_path),
                format!("resolved_executable_path={}", command_path.display()),
            ],
        )?;

        return Ok(Some(saved_install));
    } else {
        append_runner_log(
            app,
            game_id,
            &[
                "install_registered_after_installer=false".to_string(),
                format!(
                    "registered_install_path={}",
                    resolved_install_path.display()
                ),
                format!("resolved_executable_path={}", command_path.display()),
            ],
        )?;
    }

    Ok(existing_install)
}

fn should_launch_after_install(manifest: &GameManifest) -> bool {
    manifest.installation.methods.iter().any(|method| {
        method.kind == "windowsInstaller" && method.launch_after_install.unwrap_or(false)
    })
}

fn battl_eye_replaces_main_process(manifest: &GameManifest) -> bool {
    manifest
        .launch
        .battl_eye
        .as_ref()
        .filter(|battl_eye| battl_eye.enabled)
        .and_then(|battl_eye| battl_eye.launch_mode.as_deref())
        .map(|launch_mode| {
            matches!(
                launch_mode,
                "main" | "replaceMain" | "replace-main" | "replace_main"
            )
        })
        .unwrap_or(false)
}

fn build_battl_eye_runner_command(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    resolved_runner: &runners::ResolvedRunner,
    install_path: &Path,
) -> Result<Option<runners::RunnerCommand>, String> {
    build_battl_eye_runner_command_with_args(
        app,
        game_id,
        manifest,
        resolved_runner,
        install_path,
        None,
        &manifest
            .launch
            .battl_eye
            .as_ref()
            .map(|battl_eye| battl_eye.args.as_slice())
            .unwrap_or(&[]),
    )
}

fn build_battl_eye_runner_command_with_args(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    resolved_runner: &runners::ResolvedRunner,
    install_path: &Path,
    log_prefix: Option<&str>,
    args: &[String],
) -> Result<Option<runners::RunnerCommand>, String> {
    let Some(battl_eye) = manifest.launch.battl_eye.as_ref() else {
        return Ok(None);
    };

    if !battl_eye.enabled {
        append_runner_log(app, game_id, &["battl_eye_skipped=disabled".to_string()])?;
        return Ok(None);
    }

    let executable_path = configured_path_for_install(
        app,
        game_id,
        install_path,
        &battl_eye.executable,
        battl_eye.path_base.as_deref(),
        &resolved_runner.kind,
    )?;
    let working_dir = if let Some(working_dir) = battl_eye.working_dir.as_deref() {
        configured_path_for_install(
            app,
            game_id,
            install_path,
            working_dir,
            battl_eye.working_dir_base.as_deref(),
            &resolved_runner.kind,
        )?
    } else {
        executable_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| install_path.to_path_buf())
    };

    if !executable_path.exists() {
        let message = format!(
            "BattlEye configurado para {}, mas o executável não foi encontrado: {}",
            manifest.name,
            executable_path.display()
        );

        append_runner_log(app, game_id, &[format!("battl_eye_missing={message}")])?;

        if battl_eye.required {
            return Err(message);
        }

        return Ok(None);
    }

    let mut battl_eye_command = build_runner_command(
        app,
        game_id,
        resolved_runner,
        &executable_path,
        &working_dir,
        args,
        None,
    )?;
    apply_launch_environment(manifest, &mut battl_eye_command);
    let launch_mode = battl_eye.launch_mode.as_deref().unwrap_or("beforeMain");
    let prefix = log_prefix.unwrap_or("battl_eye");
    let mut command_log = if log_prefix.is_some() {
        vec![
            format!("{prefix}_start=true"),
            format!("{prefix}_launch_mode={launch_mode}"),
        ]
    } else {
        vec![
            "battl_eye_start=true".to_string(),
            format!("battl_eye_launch_mode={launch_mode}"),
        ]
    };

    command_log.extend(
        format_runner_command_for_log(&battl_eye_command)
            .into_iter()
            .map(|line| format!("{prefix}.{line}")),
    );
    append_runner_log(app, game_id, &command_log)?;

    Ok(Some(battl_eye_command))
}

fn install_battl_eye_if_configured(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    resolved_runner: &runners::ResolvedRunner,
    install_path: &Path,
) -> Result<(), String> {
    let Some(battl_eye) = manifest.launch.battl_eye.as_ref() else {
        return Ok(());
    };

    if !battl_eye.enabled || !battl_eye.install_before_launch {
        return Ok(());
    }

    if battl_eye.install_args.is_empty() {
        append_runner_log(
            app,
            game_id,
            &["battl_eye_install_skipped=no_install_args".to_string()],
        )?;
        return Ok(());
    }

    let Some(install_command) = build_battl_eye_runner_command_with_args(
        app,
        game_id,
        manifest,
        resolved_runner,
        install_path,
        Some("battl_eye_install"),
        &battl_eye.install_args,
    )?
    else {
        return Ok(());
    };

    let mut command = Command::new(&install_command.program);

    command
        .args(&install_command.args)
        .current_dir(&install_command.working_dir);
    apply_runner_command_environment(&mut command, &install_command);

    let log_path = attach_process_logs(app, game_id, &mut command)?;
    let mut child = command.spawn().map_err(|error| {
        format!(
            "Não foi possível preparar BattlEye para {} usando {}: {error}. Log: {}",
            manifest.name,
            install_command.program.display(),
            log_path.display()
        )
    })?;
    let process_id = child.id();

    append_runner_log(
        app,
        game_id,
        &[
            "battl_eye_install_process_started=true".to_string(),
            format!("battl_eye_install_process_pid={process_id}"),
        ],
    )?;

    let status = child.wait().map_err(|error| {
        format!(
            "Não foi possível aguardar a preparação do BattlEye para {}: {error}. Log: {}",
            manifest.name,
            log_path.display()
        )
    })?;

    append_runner_log(
        app,
        game_id,
        &[
            format!("battl_eye_install_process_pid={process_id}"),
            format!("battl_eye_install_exit_status={status}"),
            format!("battl_eye_install_exit_code={:?}", status.code()),
        ],
    )?;

    if !status.success() && battl_eye.required {
        return Err(format!(
            "A preparação do BattlEye para {} falhou com status {status}. Log: {}",
            manifest.name,
            log_path.display()
        ));
    }

    Ok(())
}

fn build_game_runner_command(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    resolved_runner: &runners::ResolvedRunner,
    command_path: &Path,
    install_path: &Path,
) -> Result<runners::RunnerCommand, String> {
    if battl_eye_replaces_main_process(manifest) {
        append_runner_log(
            app,
            game_id,
            &[
                "main_executable_replaced_by_battl_eye=true".to_string(),
                format!("main_executable_path={}", command_path.display()),
            ],
        )?;

        if let Some(battl_eye_command) =
            build_battl_eye_runner_command(app, game_id, manifest, resolved_runner, install_path)?
        {
            return Ok(battl_eye_command);
        }
    }

    let mut runner_command = build_runner_command(
        app,
        game_id,
        resolved_runner,
        command_path,
        install_path,
        &manifest.launch.args,
        None,
    )?;
    apply_launch_environment(manifest, &mut runner_command);

    Ok(runner_command)
}

fn build_update_runner_command(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    resolved_runner: &runners::ResolvedRunner,
    install_path: &Path,
) -> Result<runners::RunnerCommand, String> {
    if manifest.update.strategy != "externalLauncher" {
        return Err(format!(
            "{} não possui atualização por launcher externo configurada.",
            manifest.name
        ));
    }

    let executable = manifest
        .update
        .executable
        .as_deref()
        .or(manifest.launch.executable.as_deref())
        .ok_or_else(|| {
            format!(
                "O update de {} não define executable, e launch.executable também está vazio.",
                manifest.name
            )
        })?;
    let executable_path = configured_path_for_install(
        app,
        game_id,
        install_path,
        executable,
        manifest.update.path_base.as_deref(),
        &resolved_runner.kind,
    )?;
    let working_dir = if let Some(working_dir) = manifest.update.working_dir.as_deref() {
        configured_path_for_install(
            app,
            game_id,
            install_path,
            working_dir,
            manifest.update.working_dir_base.as_deref(),
            &resolved_runner.kind,
        )?
    } else {
        executable_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| install_path.to_path_buf())
    };

    if !executable_path.exists() {
        return Err(format!(
            "Updater externo não encontrado para {}: {}",
            manifest.name,
            executable_path.display()
        ));
    }

    let mut runner_command = build_runner_command(
        app,
        game_id,
        resolved_runner,
        &executable_path,
        &working_dir,
        &manifest.update.args,
        manifest.update.compat_prefix.as_deref(),
    )?;
    apply_update_environment(manifest, &mut runner_command);

    Ok(runner_command)
}

fn emit_update_progress_detail(
    app: &tauri::AppHandle,
    game_id: &str,
    status: &str,
    stage: Option<&str>,
    stage_label: Option<&str>,
    checked_files: usize,
    updated_files: usize,
    total_files: usize,
    current_file: Option<String>,
    message: String,
    target_dir: Option<&Path>,
    log_path: Option<&Path>,
    error: Option<String>,
) {
    let _ = app.emit(
        "game-update-progress",
        GameUpdateProgress {
            game_id: game_id.to_string(),
            status: status.to_string(),
            stage: stage.map(str::to_string),
            stage_label: stage_label.map(str::to_string),
            checked_files,
            updated_files,
            total_files,
            current_file,
            message,
            target_dir: target_dir.map(|path| path.to_string_lossy().to_string()),
            log_path: log_path.map(|path| path.to_string_lossy().to_string()),
            error,
        },
    );
}

fn append_update_stage_log(
    app: &tauri::AppHandle,
    game_id: &str,
    stage: &str,
    stage_label: &str,
) -> Result<PathBuf, String> {
    append_runner_log(
        app,
        game_id,
        &[
            format!("remote_update_stage={stage}"),
            format!("remote_update_stage_label={stage_label}"),
        ],
    )
}

fn emit_and_log_update_stage(
    app: &tauri::AppHandle,
    game_id: &str,
    status: &str,
    stage: &str,
    stage_label: &str,
    message: &str,
    log_path: Option<&Path>,
) -> Result<(), String> {
    append_update_stage_log(app, game_id, stage, stage_label)?;
    emit_update_progress_detail(
        app,
        game_id,
        status,
        Some(stage),
        Some(stage_label),
        0,
        0,
        0,
        None,
        message.to_string(),
        None,
        log_path,
        None,
    );

    Ok(())
}

fn log_and_emit_update_error(
    app: &tauri::AppHandle,
    game_id: &str,
    stage: &str,
    stage_label: &str,
    message: String,
    log_path: Option<&Path>,
    target_dir: Option<&Path>,
) -> String {
    let logged_message = log_error_message(app, game_id, message);

    emit_update_progress_detail(
        app,
        game_id,
        "error",
        Some(stage),
        Some(stage_label),
        0,
        0,
        0,
        None,
        format!("Falha em {stage_label}."),
        target_dir,
        log_path,
        Some(logged_message.clone()),
    );

    logged_message
}

fn update_stage_label_from_stage(stage: &str) -> String {
    match stage {
        "start" => "Preparar update",
        "openDatabase" => "Abrir banco local",
        "loadInstall" => "Carregar instalação",
        "loadLocalManifest" => "Carregar manifesto local",
        "reconcileInstall" => "Reconciliar instalação",
        "validateInstallPath" => "Validar pasta registrada",
        "spawnBlockingTask" => "Enviar tarefa para background",
        "resolveRemoteManifest" => "Resolver manifesto remoto",
        "resolveTargetDir" => "Resolver pasta alvo",
        "prepareTargetDir" => "Preparar pasta alvo",
        "downloadRemoteManifest" => "Baixar manifesto remoto",
        "decodeRemoteManifest" => "Decodificar manifesto remoto",
        "buildFileList" => "Montar lista de arquivos",
        "checkingFiles" => "Verificar arquivos locais",
        "planUpdate" => "Planejar arquivos divergentes",
        "prepareStagingDir" => "Preparar staging do update",
        "downloadingFiles" => "Baixar arquivos divergentes",
        "validateDownloadedFile" => "Validar arquivo baixado",
        "validateStagedFiles" => "Validar staging completo",
        "applyDownloadedFile" => "Aplicar arquivo baixado",
        "applyStagedFiles" => "Aplicar arquivos no jogo",
        "done" => "Concluído",
        _ => stage,
    }
    .to_string()
}

fn update_status_from_stage(stage: &str) -> String {
    match stage {
        "downloadRemoteManifest" | "decodeRemoteManifest" | "buildFileList" => "manifest",
        "checkingFiles" | "planUpdate" => "checking",
        "prepareStagingDir" | "downloadingFiles" => "downloading",
        "validateDownloadedFile" | "validateStagedFiles" => "validating",
        "applyDownloadedFile" | "applyStagedFiles" => "applying",
        "done" => "done",
        _ => "preparing",
    }
    .to_string()
}

fn read_recent_log_text(log_path: &Path, max_bytes: u64) -> Result<String, String> {
    let mut file = fs::File::open(log_path)
        .map_err(|error| format!("Não foi possível abrir log {}: {error}", log_path.display()))?;
    let len = file
        .metadata()
        .map_err(|error| {
            format!(
                "Não foi possível inspecionar log {}: {error}",
                log_path.display()
            )
        })?
        .len();
    let start = len.saturating_sub(max_bytes);
    let mut bytes = Vec::new();

    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("Não foi possível posicionar leitura do log: {error}"))?;
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("Não foi possível ler log {}: {error}", log_path.display()))?;

    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn progress_message_from_log(
    status: &str,
    stage_label: &str,
    checked: usize,
    total: usize,
    current_file: Option<&str>,
) -> String {
    if status == "done" {
        return "Update concluído.".to_string();
    }

    if status == "downloading" {
        return current_file
            .map(|file| format!("Baixando {file}"))
            .unwrap_or_else(|| "Baixando arquivos divergentes...".to_string());
    }

    if status == "validating" {
        return current_file
            .map(|file| format!("Validando {file}"))
            .unwrap_or_else(|| "Validando staging do update...".to_string());
    }

    if status == "applying" {
        return current_file
            .map(|file| format!("Aplicando {file}"))
            .unwrap_or_else(|| "Aplicando arquivos no jogo...".to_string());
    }

    if status == "checking" && total > 0 {
        return format!("Verificando {checked} de {total} arquivos...");
    }

    format!("{stage_label}...")
}

fn parse_latest_update_progress_from_log(
    game_id: &str,
    log_path: &Path,
    log_text: &str,
) -> Option<GameUpdateProgress> {
    let mut found_remote_update = false;
    let mut status = "preparing".to_string();
    let mut stage = "start".to_string();
    let mut stage_label = "Preparar update".to_string();
    let mut checked_files = 0_usize;
    let mut updated_files = 0_usize;
    let mut total_files = 0_usize;
    let mut current_file: Option<String> = None;
    let mut target_dir: Option<String> = None;
    let mut error: Option<String> = None;

    for line in log_text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        match key {
            "action" if value == "run_game_remote_update" => {
                found_remote_update = true;
                status = "preparing".to_string();
                stage = "start".to_string();
                stage_label = update_stage_label_from_stage(&stage);
                checked_files = 0;
                updated_files = 0;
                total_files = 0;
                current_file = None;
                target_dir = None;
                error = None;
            }
            "remote_update_stage" => {
                found_remote_update = true;
                stage = value.to_string();
                stage_label = update_stage_label_from_stage(value);
                status = update_status_from_stage(value);
            }
            "remote_update_stage_label" => {
                stage_label = value.to_string();
            }
            "remote_update_progress" => {
                found_remote_update = true;
                status = value.to_string();

                if value == "checking" {
                    stage = "checkingFiles".to_string();
                    stage_label = update_stage_label_from_stage(&stage);
                } else if value == "downloading" {
                    stage = "downloadingFiles".to_string();
                    stage_label = update_stage_label_from_stage(&stage);
                } else if value == "validating" {
                    stage = "validateStagedFiles".to_string();
                    stage_label = update_stage_label_from_stage(&stage);
                } else if value == "applying" {
                    stage = "applyStagedFiles".to_string();
                    stage_label = update_stage_label_from_stage(&stage);
                }
            }
            "remote_update_finished" if value == "true" => {
                found_remote_update = true;
                status = "done".to_string();
                stage = "done".to_string();
                stage_label = update_stage_label_from_stage(&stage);
            }
            "checked_files" => checked_files = value.parse().unwrap_or(checked_files),
            "updated_files" => updated_files = value.parse().unwrap_or(updated_files),
            "total_files" | "remote_file_count" => {
                total_files = value.parse().unwrap_or(total_files)
            }
            "current_file" => current_file = Some(value.to_string()),
            "update_target_dir" => target_dir = Some(value.to_string()),
            "error" => {
                error = Some(value.to_string());
                status = "error".to_string();
            }
            _ => {}
        }
    }

    if !found_remote_update {
        return None;
    }

    let message = progress_message_from_log(
        &status,
        &stage_label,
        checked_files,
        total_files,
        current_file.as_deref(),
    );

    Some(GameUpdateProgress {
        game_id: game_id.to_string(),
        status,
        stage: Some(stage),
        stage_label: Some(stage_label),
        checked_files,
        updated_files,
        total_files,
        current_file,
        message,
        target_dir,
        log_path: Some(log_path.to_string_lossy().to_string()),
        error,
    })
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECONDS))
        .build()
        .map_err(|error| format!("Não foi possível preparar cliente HTTP: {error}"))
}

fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = http_client()?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|error| format!("Não foi possível baixar {url}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Servidor retornou erro ao baixar {url}: {error}"))?;
    let mut bytes = Vec::new();

    response
        .copy_to(&mut bytes)
        .map_err(|error| format!("Não foi possível ler resposta de {url}: {error}"))?;

    Ok(bytes)
}

fn decode_remote_update_manifest(
    bytes: &[u8],
    format: Option<&str>,
) -> Result<RemoteUpdateManifest, String> {
    if matches!(format, Some("json")) {
        return serde_json::from_slice(bytes)
            .map_err(|error| format!("Manifesto remoto JSON inválido: {error}"));
    }

    if let Ok(manifest) = serde_json::from_slice::<RemoteUpdateManifest>(bytes) {
        return Ok(manifest);
    }

    let max_offset = if matches!(format, Some("ravenquestZlib" | "zlibWithSizePrefix")) {
        8
    } else {
        16
    };

    for offset in 0..=max_offset.min(bytes.len()) {
        let mut decoder = ZlibDecoder::new(&bytes[offset..]);
        let mut decoded = Vec::new();

        if decoder.read_to_end(&mut decoded).is_err() {
            continue;
        }

        if let Ok(manifest) = serde_json::from_slice::<RemoteUpdateManifest>(&decoded) {
            return Ok(manifest);
        }
    }

    Err("Não foi possível decodificar o manifesto remoto de update.".to_string())
}

fn safe_remote_relative_path(remote_path: &str) -> Result<PathBuf, String> {
    let trimmed_path = remote_path.trim_start_matches('/').replace('\\', "/");
    let path = PathBuf::from(&trimmed_path);

    if trimmed_path.is_empty() {
        return Err("Caminho remoto vazio no manifesto de update.".to_string());
    }

    for component in path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(format!(
                "Caminho remoto inseguro no manifesto de update: {remote_path}"
            ));
        }
    }

    Ok(path)
}

fn crc32_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("Não foi possível abrir {}: {error}", path.display()))?;
    let mut hasher = Hasher::new();
    let mut buffer = [0_u8; 1024 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Não foi possível ler {}: {error}", path.display()))?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:08x}", hasher.finalize()))
}

fn local_file_matches(path: &Path, expected: &RemoteFileEntry) -> Result<bool, String> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(false);
    };

    if metadata.len() != expected.size {
        return Ok(false);
    }

    Ok(crc32_file(path)?.eq_ignore_ascii_case(&expected.checksum))
}

fn remote_file_url(base_url: &str, remote_path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = remote_path
        .trim_start_matches('/')
        .split('/')
        .map(percent_encode_url_path_segment)
        .collect::<Vec<_>>()
        .join("/");

    format!("{base}/{path}")
}

fn percent_encode_url_path_segment(segment: &str) -> String {
    let mut encoded = String::new();

    for byte in segment.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

fn remote_update_target_dir(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    install_path: &Path,
) -> Result<PathBuf, String> {
    if let Some(target_dir) = manifest.update.target_dir.as_deref() {
        return configured_path_for_install(
            app,
            game_id,
            install_path,
            target_dir,
            manifest.update.target_dir_base.as_deref(),
            &manifest.launch.runner,
        );
    }

    Ok(install_path.to_path_buf())
}

fn remote_update_concurrency(manifest: &GameManifest) -> usize {
    manifest
        .update
        .max_concurrent_downloads
        .unwrap_or(DEFAULT_REMOTE_UPDATE_CONCURRENCY)
        .clamp(1, MAX_REMOTE_UPDATE_CONCURRENCY)
}

fn remote_update_staging_dir(app: &tauri::AppHandle, game_id: &str) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        format!("Não foi possível resolver o diretório de dados do app: {error}")
    })?;
    let game_updates_dir = app_data_dir
        .join("updates")
        .join(sanitize_path_segment(game_id));
    let stable_staging_dir = game_updates_dir.join("staging");

    if stable_staging_dir.exists() {
        return Ok(stable_staging_dir);
    }

    let latest_legacy_staging = fs::read_dir(&game_updates_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let timestamp = entry.file_name().to_string_lossy().parse::<u64>().ok()?;
            let staging = entry.path().join("staging");

            staging.is_dir().then_some((timestamp, staging))
        })
        .max_by_key(|(timestamp, _)| *timestamp)
        .map(|(_, staging)| staging);

    Ok(latest_legacy_staging.unwrap_or(stable_staging_dir))
}

fn remove_dir_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_dir_all(path).map_err(|error| {
        format!(
            "Não foi possível remover diretório temporário {}: {error}",
            path.display()
        )
    })
}

fn apply_staged_file(staged_path: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Não foi possível criar diretório {}: {error}",
                parent.display()
            )
        })?;
    }

    if destination.exists() {
        fs::remove_file(destination).map_err(|error| {
            format!(
                "Não foi possível remover arquivo antigo {}: {error}",
                destination.display()
            )
        })?;
    }

    fs::rename(staged_path, destination).map_err(|error| {
        format!(
            "Não foi possível aplicar update em {}: {error}",
            destination.display()
        )
    })
}

fn download_planned_files_to_staging(
    app: &tauri::AppHandle,
    game_id: &str,
    planned_files: &[RemotePlannedFile],
    concurrency: usize,
    total_files: usize,
    target_dir: &Path,
    attempt_log_path: &Path,
) -> Result<u64, String> {
    if planned_files.is_empty() {
        return Ok(0);
    }

    for planned_file in planned_files {
        let parent = planned_file.staging_destination.parent().ok_or_else(|| {
            format!(
                "Destino de staging inválido: {}",
                planned_file.staging_destination.display()
            )
        })?;

        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Não foi possível preparar diretório de staging {}: {error}",
                parent.display()
            )
        })?;
    }

    let worker_count = concurrency.min(planned_files.len()).max(1);
    let (result_tx, result_rx) = mpsc::channel::<Result<(String, u64), String>>();
    let planned_files = Arc::new(planned_files.to_vec());
    let next_file_index = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::with_capacity(worker_count);

    append_runner_log(
        app,
        game_id,
        &[
            "remote_update_parallel_downloads=true".to_string(),
            format!("remote_update_download_workers={worker_count}"),
            format!("remote_update_planned_downloads={}", planned_files.len()),
        ],
    )?;

    emit_update_progress_detail(
        app,
        game_id,
        "downloading",
        Some("downloadingFiles"),
        Some("Baixar arquivos divergentes"),
        0,
        0,
        planned_files.len(),
        None,
        format!(
            "Baixando em paralelo com {worker_count} worker(s): 0 de {} arquivos...",
            planned_files.len()
        ),
        Some(target_dir),
        Some(attempt_log_path),
        None,
    );

    for worker_index in 0..worker_count {
        let planned_files = Arc::clone(&planned_files);
        let next_file_index = Arc::clone(&next_file_index);
        let result_tx = result_tx.clone();
        let app = app.clone();
        let game_id = game_id.to_string();

        workers.push(thread::spawn(move || {
            let client = match http_client() {
                Ok(client) => client,
                Err(error) => {
                    let _ = result_tx.send(Err(error));
                    return;
                }
            };

            loop {
                let planned_file_index = next_file_index.fetch_add(1, Ordering::Relaxed);
                let Some(planned_file) = planned_files.get(planned_file_index).cloned() else {
                    break;
                };

                let staged_file_is_valid =
                    local_file_matches(&planned_file.staging_destination, &planned_file.expected);
                let result = staged_file_is_valid
                    .and_then(|matches| {
                        if matches {
                            return Ok(());
                        }

                        download_file_with_retry_using_client(
                            &client,
                            &planned_file.url,
                            &planned_file.staging_destination,
                            Some((&app, &game_id, &planned_file.remote_path)),
                            None,
                        )
                    })
                    .and_then(|_| {
                        match local_file_matches(
                            &planned_file.staging_destination,
                            &planned_file.expected,
                        )? {
                            true => Ok(()),
                            false => Err(format!(
                                "Arquivo em staging falhou na validação: {}",
                                planned_file.remote_path
                            )),
                        }
                    })
                    .and_then(|_| {
                        fs::metadata(&planned_file.staging_destination)
                            .map(|metadata| (planned_file.remote_path.clone(), metadata.len()))
                            .map_err(|error| {
                                format!(
                                    "Não foi possível inspecionar arquivo em staging {}: {error}",
                                    planned_file.staging_destination.display()
                                )
                            })
                    });

                let should_stop = result_tx.send(result).is_err();
                if should_stop {
                    break;
                }
            }

            let _ = append_runner_log(
                &app,
                &game_id,
                &[
                    "remote_update_download_worker_finished=true".to_string(),
                    format!("remote_update_download_worker_index={worker_index}"),
                ],
            );
        }));
    }

    drop(result_tx);

    let mut downloaded_files = 0_usize;
    let mut downloaded_bytes = 0_u64;
    let mut download_errors = Vec::new();

    for result in result_rx {
        match result {
            Ok((remote_path, bytes)) => {
                downloaded_files += 1;
                downloaded_bytes += bytes;

                if downloaded_files == 1
                    || downloaded_files % REMOTE_UPDATE_DOWNLOAD_PROGRESS_INTERVAL == 0
                    || downloaded_files == planned_files.len()
                {
                    emit_update_progress_detail(
                        app,
                        game_id,
                        "downloading",
                        Some("downloadingFiles"),
                        Some("Baixar arquivos divergentes"),
                        downloaded_files,
                        downloaded_files,
                        planned_files.len(),
                        Some(remote_path.clone()),
                        format!(
                            "Baixando e validando staging: {downloaded_files} de {} arquivos...",
                            planned_files.len()
                        ),
                        Some(target_dir),
                        Some(attempt_log_path),
                        None,
                    );
                }

                if downloaded_files == 1
                    || downloaded_files % REMOTE_UPDATE_LOG_INTERVAL == 0
                    || downloaded_files == planned_files.len()
                {
                    append_runner_log(
                        app,
                        game_id,
                        &[
                            "remote_update_progress=downloading".to_string(),
                            "remote_update_download_validation=parallel_worker".to_string(),
                            format!("checked_files={downloaded_files}"),
                            format!("updated_files={downloaded_files}"),
                            format!("total_files={}", planned_files.len()),
                            format!("remote_manifest_total_files={total_files}"),
                            format!("downloaded_bytes={downloaded_bytes}"),
                            format!("current_file={remote_path}"),
                        ],
                    )?;
                }
            }
            Err(error) => {
                download_errors.push(error);
            }
        }
    }

    for worker in workers {
        worker
            .join()
            .map_err(|_| "Worker de download encerrou com panic.".to_string())?;
    }

    if !download_errors.is_empty() {
        let error_count = download_errors.len();
        let first_error = download_errors
            .first()
            .cloned()
            .unwrap_or_else(|| "erro desconhecido".to_string());

        append_runner_log(
            app,
            game_id,
            &[
                "remote_update_download_errors=true".to_string(),
                format!("remote_update_download_error_count={error_count}"),
                format!("remote_update_first_download_error={first_error}"),
            ],
        )?;

        return Err(format!(
            "Falha ao baixar/validar {error_count} arquivo(s). Primeiro erro: {first_error}"
        ));
    }

    Ok(downloaded_bytes)
}

fn run_remote_manifest_update(
    app: tauri::AppHandle,
    game_id: String,
    manifest: GameManifest,
    install_path: PathBuf,
    attempt_log_path: PathBuf,
) -> Result<GameUpdateResult, String> {
    emit_and_log_update_stage(
        &app,
        &game_id,
        "preparing",
        "resolveRemoteManifest",
        "Resolver manifesto remoto",
        "Resolvendo configuração do manifesto remoto...",
        Some(&attempt_log_path),
    )?;

    let manifest_url = manifest.update.manifest_url.as_deref().ok_or_else(|| {
        log_and_emit_update_error(
            &app,
            &game_id,
            "resolveRemoteManifest",
            "Resolver manifesto remoto",
            format!("{} não define update.manifestUrl.", manifest.name),
            Some(&attempt_log_path),
            None,
        )
    })?;

    emit_and_log_update_stage(
        &app,
        &game_id,
        "preparing",
        "resolveTargetDir",
        "Resolver pasta alvo",
        "Resolvendo pasta onde os arquivos serão verificados...",
        Some(&attempt_log_path),
    )?;

    let target_dir =
        remote_update_target_dir(&app, &game_id, &manifest, &install_path).map_err(|error| {
            log_and_emit_update_error(
                &app,
                &game_id,
                "resolveTargetDir",
                "Resolver pasta alvo",
                error,
                Some(&attempt_log_path),
                None,
            )
        })?;

    emit_update_progress_detail(
        &app,
        &game_id,
        "preparing",
        Some("prepareTargetDir"),
        Some("Preparar pasta alvo"),
        0,
        0,
        0,
        None,
        "Preparando pasta alvo do update...".to_string(),
        Some(&target_dir),
        Some(&attempt_log_path),
        None,
    );
    append_update_stage_log(&app, &game_id, "prepareTargetDir", "Preparar pasta alvo")?;

    fs::create_dir_all(&target_dir).map_err(|error| {
        log_and_emit_update_error(
            &app,
            &game_id,
            "prepareTargetDir",
            "Preparar pasta alvo",
            format!(
                "Não foi possível criar a pasta de update {}: {error}",
                target_dir.display()
            ),
            Some(&attempt_log_path),
            Some(&target_dir),
        )
    })?;

    append_runner_log(
        &app,
        &game_id,
        &[
            format!("manifest={}", manifest.name),
            format!("install_path={}", install_path.display()),
            format!("update_strategy={}", manifest.update.strategy),
            format!("update_manifest_url={manifest_url}"),
            format!(
                "update_manifest_format={:?}",
                manifest.update.manifest_format
            ),
            format!("update_target_dir={}", target_dir.display()),
            format!("log_path={}", attempt_log_path.display()),
        ],
    )?;

    append_update_stage_log(
        &app,
        &game_id,
        "downloadRemoteManifest",
        "Baixar manifesto remoto",
    )?;
    emit_update_progress_detail(
        &app,
        &game_id,
        "manifest",
        Some("downloadRemoteManifest"),
        Some("Baixar manifesto remoto"),
        0,
        0,
        0,
        Some(manifest_url.to_string()),
        "Baixando manifesto remoto...".to_string(),
        Some(&target_dir),
        Some(&attempt_log_path),
        None,
    );

    let manifest_bytes = download_bytes(manifest_url).map_err(|error| {
        log_and_emit_update_error(
            &app,
            &game_id,
            "downloadRemoteManifest",
            "Baixar manifesto remoto",
            error,
            Some(&attempt_log_path),
            Some(&target_dir),
        )
    })?;

    append_update_stage_log(
        &app,
        &game_id,
        "decodeRemoteManifest",
        "Decodificar manifesto remoto",
    )?;
    emit_update_progress_detail(
        &app,
        &game_id,
        "manifest",
        Some("decodeRemoteManifest"),
        Some("Decodificar manifesto remoto"),
        0,
        0,
        0,
        None,
        "Decodificando manifesto remoto...".to_string(),
        Some(&target_dir),
        Some(&attempt_log_path),
        None,
    );

    let remote_manifest =
        decode_remote_update_manifest(&manifest_bytes, manifest.update.manifest_format.as_deref())
            .map_err(|error| {
                log_and_emit_update_error(
                    &app,
                    &game_id,
                    "decodeRemoteManifest",
                    "Decodificar manifesto remoto",
                    error,
                    Some(&attempt_log_path),
                    Some(&target_dir),
                )
            })?;

    append_update_stage_log(&app, &game_id, "buildFileList", "Montar lista de arquivos")?;
    emit_update_progress_detail(
        &app,
        &game_id,
        "manifest",
        Some("buildFileList"),
        Some("Montar lista de arquivos"),
        0,
        0,
        0,
        None,
        "Montando lista de arquivos do update...".to_string(),
        Some(&target_dir),
        Some(&attempt_log_path),
        None,
    );

    let mut remote_file_map = remote_manifest.files;
    let remote_binary_file = remote_manifest
        .binary
        .as_ref()
        .map(|binary| binary.file.clone());

    if let Some(binary) = remote_manifest.binary {
        remote_file_map.insert(
            binary.file,
            RemoteFileEntry {
                checksum: binary.checksum,
                size: binary.size,
            },
        );
    }

    let mut remote_files = remote_file_map.into_iter().collect::<Vec<_>>();

    let total_files = remote_files.len();
    append_runner_log(
        &app,
        &game_id,
        &[
            format!("remote_update_url={}", remote_manifest.url),
            format!("remote_fallback_url={:?}", remote_manifest.fallback_url),
            format!("remote_keep_files={:?}", remote_manifest.keep_files),
            format!("remote_binary_file={remote_binary_file:?}"),
            format!(
                "remote_min_client_version={:?}",
                remote_manifest.min_client_version
            ),
            format!("remote_file_count={total_files}"),
            "remote_extra_files_delete_skipped=true".to_string(),
        ],
    )?;

    append_update_stage_log(&app, &game_id, "checkingFiles", "Verificar arquivos locais")?;
    emit_update_progress_detail(
        &app,
        &game_id,
        "checking",
        Some("checkingFiles"),
        Some("Verificar arquivos locais"),
        0,
        0,
        total_files,
        None,
        "Verificando arquivos locais...".to_string(),
        Some(&target_dir),
        Some(&attempt_log_path),
        None,
    );

    let mut checked_files = 0_usize;
    let mut updated_files = 0_usize;
    let mut skipped_files = 0_usize;
    let mut planned_files = Vec::new();
    let staging_dir = remote_update_staging_dir(&app, &game_id).map_err(|error| {
        log_and_emit_update_error(
            &app,
            &game_id,
            "prepareStagingDir",
            "Preparar staging do update",
            error,
            Some(&attempt_log_path),
            Some(&target_dir),
        )
    })?;
    let download_concurrency = remote_update_concurrency(&manifest);

    remote_files.sort_by(|left, right| left.0.cmp(&right.0));

    for (remote_path, expected) in remote_files {
        checked_files += 1;

        if checked_files == 1
            || checked_files % REMOTE_UPDATE_PROGRESS_INTERVAL == 0
            || checked_files == total_files
        {
            emit_update_progress_detail(
                &app,
                &game_id,
                "checking",
                Some("checkingFiles"),
                Some("Verificar arquivos locais"),
                checked_files,
                updated_files,
                total_files,
                Some(remote_path.clone()),
                format!("Verificando {checked_files} de {total_files} arquivos..."),
                Some(&target_dir),
                Some(&attempt_log_path),
                None,
            );
        }

        if checked_files % REMOTE_UPDATE_LOG_INTERVAL == 0 {
            append_runner_log(
                &app,
                &game_id,
                &[
                    "remote_update_progress=checking".to_string(),
                    format!("checked_files={checked_files}"),
                    format!("updated_files={updated_files}"),
                    format!("skipped_files={skipped_files}"),
                    format!("total_files={total_files}"),
                    format!("current_file={remote_path}"),
                ],
            )?;
        }

        let relative_path = safe_remote_relative_path(&remote_path).map_err(|error| {
            log_and_emit_update_error(
                &app,
                &game_id,
                "checkingFiles",
                "Verificar arquivos locais",
                error,
                Some(&attempt_log_path),
                Some(&target_dir),
            )
        })?;
        let destination = target_dir.join(&relative_path);

        if local_file_matches(&destination, &expected).map_err(|error| {
            log_and_emit_update_error(
                &app,
                &game_id,
                "checkingFiles",
                "Verificar arquivos locais",
                error,
                Some(&attempt_log_path),
                Some(&target_dir),
            )
        })? {
            skipped_files += 1;
            continue;
        }

        let file_url = remote_file_url(&remote_manifest.url, &remote_path);
        let staging_destination = staging_dir.join(&relative_path);

        planned_files.push(RemotePlannedFile {
            remote_path,
            expected,
            destination,
            staging_destination,
            url: file_url,
        });
    }

    append_update_stage_log(
        &app,
        &game_id,
        "planUpdate",
        "Planejar arquivos divergentes",
    )?;
    append_runner_log(
        &app,
        &game_id,
        &[
            "remote_update_progress=checking".to_string(),
            format!("checked_files={checked_files}"),
            format!("updated_files={updated_files}"),
            format!("skipped_files={skipped_files}"),
            format!("total_files={total_files}"),
            format!("planned_update_files={}", planned_files.len()),
            format!("remote_update_download_concurrency={download_concurrency}"),
            format!("remote_update_staging_dir={}", staging_dir.display()),
        ],
    )?;
    emit_update_progress_detail(
        &app,
        &game_id,
        "checking",
        Some("planUpdate"),
        Some("Planejar arquivos divergentes"),
        checked_files,
        updated_files,
        total_files,
        None,
        format!(
            "Verificação concluída: {} arquivo(s) divergente(s) serão baixados em staging.",
            planned_files.len()
        ),
        Some(&target_dir),
        Some(&attempt_log_path),
        None,
    );

    let mut downloaded_bytes = 0_u64;

    if !planned_files.is_empty() {
        append_update_stage_log(
            &app,
            &game_id,
            "prepareStagingDir",
            "Preparar staging do update",
        )?;
        emit_update_progress_detail(
            &app,
            &game_id,
            "downloading",
            Some("prepareStagingDir"),
            Some("Preparar staging do update"),
            0,
            0,
            planned_files.len(),
            None,
            "Preparando pasta temporária para baixar tudo antes de substituir...".to_string(),
            Some(&target_dir),
            Some(&attempt_log_path),
            None,
        );

        fs::create_dir_all(&staging_dir).map_err(|error| {
            log_and_emit_update_error(
                &app,
                &game_id,
                "prepareStagingDir",
                "Preparar staging do update",
                format!(
                    "Não foi possível criar staging {}: {error}",
                    staging_dir.display()
                ),
                Some(&attempt_log_path),
                Some(&target_dir),
            )
        })?;

        append_update_stage_log(
            &app,
            &game_id,
            "downloadingFiles",
            "Baixar arquivos divergentes",
        )?;
        downloaded_bytes = download_planned_files_to_staging(
            &app,
            &game_id,
            &planned_files,
            download_concurrency,
            total_files,
            &target_dir,
            &attempt_log_path,
        )
        .map_err(|error| {
            log_and_emit_update_error(
                &app,
                &game_id,
                "downloadingFiles",
                "Baixar arquivos divergentes",
                error,
                Some(&attempt_log_path),
                Some(&target_dir),
            )
        })?;

        append_update_stage_log(
            &app,
            &game_id,
            "validateStagedFiles",
            "Validar staging completo",
        )?;
        append_runner_log(
            &app,
            &game_id,
            &[
                "remote_update_progress=validating".to_string(),
                "remote_update_staging_validation=completed_during_parallel_download".to_string(),
                format!("checked_files={}", planned_files.len()),
                format!("updated_files={updated_files}"),
                format!("total_files={}", planned_files.len()),
            ],
        )?;
        emit_update_progress_detail(
            &app,
            &game_id,
            "validating",
            Some("validateStagedFiles"),
            Some("Validar staging completo"),
            planned_files.len(),
            updated_files,
            planned_files.len(),
            None,
            "Staging validado em paralelo durante os downloads.".to_string(),
            Some(&target_dir),
            Some(&attempt_log_path),
            None,
        );

        append_update_stage_log(
            &app,
            &game_id,
            "applyStagedFiles",
            "Aplicar arquivos no jogo",
        )?;
        for planned_file in &planned_files {
            apply_staged_file(&planned_file.staging_destination, &planned_file.destination)
                .map_err(|error| {
                    log_and_emit_update_error(
                        &app,
                        &game_id,
                        "applyStagedFiles",
                        "Aplicar arquivos no jogo",
                        error,
                        Some(&attempt_log_path),
                        Some(&target_dir),
                    )
                })?;

            updated_files += 1;

            if updated_files == 1
                || updated_files % REMOTE_UPDATE_APPLY_PROGRESS_INTERVAL == 0
                || updated_files == planned_files.len()
            {
                emit_update_progress_detail(
                    &app,
                    &game_id,
                    "applying",
                    Some("applyStagedFiles"),
                    Some("Aplicar arquivos no jogo"),
                    updated_files,
                    updated_files,
                    planned_files.len(),
                    Some(planned_file.remote_path.clone()),
                    format!(
                        "Aplicando arquivos: {updated_files} de {} concluídos...",
                        planned_files.len()
                    ),
                    Some(&target_dir),
                    Some(&attempt_log_path),
                    None,
                );

                append_runner_log(
                    &app,
                    &game_id,
                    &[
                        "remote_update_progress=applying".to_string(),
                        format!("checked_files={updated_files}"),
                        format!("updated_files={updated_files}"),
                        format!("total_files={}", planned_files.len()),
                        format!("current_file={}", planned_file.remote_path),
                    ],
                )?;
            }
        }

        remove_dir_if_exists(&staging_dir).map_err(|error| {
            log_and_emit_update_error(
                &app,
                &game_id,
                "applyStagedFiles",
                "Aplicar arquivos no jogo",
                error,
                Some(&attempt_log_path),
                Some(&target_dir),
            )
        })?;
    }

    append_runner_log(
        &app,
        &game_id,
        &[
            "remote_update_finished=true".to_string(),
            format!("checked_files={checked_files}"),
            format!("updated_files={updated_files}"),
            format!("skipped_files={skipped_files}"),
            format!("downloaded_bytes={downloaded_bytes}"),
        ],
    )?;

    emit_update_progress_detail(
        &app,
        &game_id,
        "done",
        Some("done"),
        Some("Concluído"),
        checked_files,
        updated_files,
        total_files,
        None,
        "Update concluído.".to_string(),
        Some(&target_dir),
        Some(&attempt_log_path),
        None,
    );

    Ok(GameUpdateResult {
        game_id,
        checked_files,
        updated_files,
        skipped_files,
        downloaded_bytes,
        target_dir: target_dir.to_string_lossy().to_string(),
        log_path: Some(attempt_log_path.to_string_lossy().to_string()),
    })
}

fn spawn_battl_eye_if_configured(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    resolved_runner: &runners::ResolvedRunner,
    install_path: &Path,
) -> Result<(), String> {
    if battl_eye_replaces_main_process(manifest) {
        append_runner_log(
            app,
            game_id,
            &["battl_eye_separate_spawn_skipped=main_launch_mode".to_string()],
        )?;
        return Ok(());
    }

    let Some(battl_eye_command) =
        build_battl_eye_runner_command(app, game_id, manifest, resolved_runner, install_path)?
    else {
        return Ok(());
    };

    let mut command = Command::new(&battl_eye_command.program);

    command
        .args(&battl_eye_command.args)
        .current_dir(&battl_eye_command.working_dir);
    apply_runner_command_environment(&mut command, &battl_eye_command);

    let log_path = attach_process_logs(app, game_id, &mut command)?;
    let child = command.spawn().map_err(|error| {
        format!(
            "Não foi possível iniciar BattlEye para {} usando {}: {error}. Log: {}",
            manifest.name,
            battl_eye_command.program.display(),
            log_path.display()
        )
    })?;
    let process_id = child.id();

    append_runner_log(
        app,
        game_id,
        &[
            "battl_eye_process_started=true".to_string(),
            format!("battl_eye_process_pid={process_id}"),
        ],
    )?;
    log_process_exit(app.clone(), game_id.to_string(), process_id, child);

    Ok(())
}

fn launch_install(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    install: &GameInstall,
    action: &str,
) -> Result<LaunchResult, String> {
    let attempt_log_path = append_runner_log(
        app,
        game_id,
        &[action.to_string(), format!("game_id={game_id}")],
    )?;
    let requested_runner = install
        .runner_override
        .clone()
        .unwrap_or_else(|| manifest.launch.runner.clone());
    let resolved_runner = resolve_runner(app, &requested_runner)?;
    let executable = manifest.launch.executable.as_ref().ok_or_else(|| {
        format!(
            "O manifesto de {} ainda não define launch.executable. Configure o executável antes de jogar.",
            manifest.name
        )
    })?;
    let install_path = PathBuf::from(&install.install_path);

    if !install_path.exists() {
        return Err(format!(
            "A pasta registrada para {} não existe mais: {}",
            manifest.name,
            install_path.display()
        ));
    }

    let command_path = command_path_for_install(&install_path, executable);

    if !command_path.exists() && !battl_eye_replaces_main_process(manifest) {
        return Err(format!(
            "Executável não encontrado para {}: {}",
            manifest.name,
            command_path.display()
        ));
    }

    append_runner_log(
        app,
        game_id,
        &[
            format!("manifest={}", manifest.name),
            format!("install_path={}", install.install_path),
            format!("requested_runner={requested_runner}"),
            format!("resolved_runner_id={}", resolved_runner.id),
            format!("resolved_runner_kind={}", resolved_runner.kind),
            format!("resolved_runner_label={}", resolved_runner.label),
            format!("resolved_runner_source={}", resolved_runner.source),
            format!("resolved_runner_path={:?}", resolved_runner.path),
            format!("log_path={}", attempt_log_path.display()),
        ],
    )?;

    install_battl_eye_if_configured(app, game_id, manifest, &resolved_runner, &install_path)?;

    let runner_command = build_game_runner_command(
        app,
        game_id,
        manifest,
        &resolved_runner,
        &command_path,
        &install_path,
    )?;
    let mut command_log = format_runner_command_for_log(&runner_command);

    command_log.extend(host_environment_for_log());
    append_runner_log(app, game_id, &command_log)?;

    spawn_battl_eye_if_configured(app, game_id, manifest, &resolved_runner, &install_path)?;

    let mut command = Command::new(&runner_command.program);

    command
        .args(&runner_command.args)
        .current_dir(&runner_command.working_dir);
    apply_runner_command_environment(&mut command, &runner_command);

    let log_path = attach_process_logs(app, game_id, &mut command)?;
    let child = command.spawn().map_err(|error| {
        format!(
            "Não foi possível iniciar {} usando {}: {error}. Log: {}",
            manifest.name,
            runner_command.program.display(),
            log_path.display()
        )
    })?;
    let process_id = child.id();

    append_runner_log(
        app,
        game_id,
        &[
            "process_started=true".to_string(),
            format!("process_pid={process_id}"),
        ],
    )?;
    log_process_exit(app.clone(), game_id.to_string(), process_id, child);

    Ok(LaunchResult {
        game_id: game_id.to_string(),
        runner: runner_command.runner_kind,
        command: runner_command.program.to_string_lossy().to_string(),
        working_dir: runner_command.working_dir.to_string_lossy().to_string(),
        log_path: Some(log_path.to_string_lossy().to_string()),
    })
}

fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || character == '-'
                || character == '_'
                || character == '.'
            {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn filename_from_url(url: &str) -> String {
    url.split('/')
        .next_back()
        .and_then(|segment| segment.split('?').next())
        .filter(|segment| !segment.trim().is_empty())
        .map(sanitize_path_segment)
        .unwrap_or_else(|| "installer.exe".to_string())
}

fn download_file(url: &str, destination: &PathBuf) -> Result<(), String> {
    let client = http_client()?;

    download_file_with_retry_using_client(&client, url, destination, None, None)
}

fn download_file_with_retry_using_client(
    client: &reqwest::blocking::Client,
    url: &str,
    destination: &PathBuf,
    log_context: Option<(&tauri::AppHandle, &str, &str)>,
    headers: Option<&HashMap<String, String>>,
) -> Result<(), String> {
    let mut last_error = None;

    for attempt in 1..=DOWNLOAD_RETRY_ATTEMPTS {
        match download_file_once(client, url, destination, headers) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let _ = fs::remove_file(temporary_download_path(destination));
                let _ = fs::remove_file(destination);

                if let Some((app, game_id, remote_path)) = log_context {
                    let _ = append_runner_log(
                        app,
                        game_id,
                        &[
                            "remote_update_download_attempt_failed=true".to_string(),
                            format!("download_attempt={attempt}"),
                            format!("download_max_attempts={DOWNLOAD_RETRY_ATTEMPTS}"),
                            format!("current_file={remote_path}"),
                            format!("download_url={url}"),
                            format!("download_error={error}"),
                        ],
                    );
                }

                last_error = Some(error);

                if attempt < DOWNLOAD_RETRY_ATTEMPTS {
                    thread::sleep(Duration::from_secs(
                        DOWNLOAD_RETRY_DELAY_SECONDS * attempt as u64,
                    ));
                }
            }
        }
    }

    Err(format!(
        "Falha ao baixar {url} após {DOWNLOAD_RETRY_ATTEMPTS} tentativa(s): {}",
        last_error.unwrap_or_else(|| "erro desconhecido".to_string())
    ))
}

fn temporary_download_path(destination: &Path) -> PathBuf {
    let temporary_name = destination
        .file_name()
        .map(|file_name| format!("{}.download", file_name.to_string_lossy()))
        .unwrap_or_else(|| "download.tmp".to_string());

    destination.with_file_name(temporary_name)
}

fn download_file_once(
    client: &reqwest::blocking::Client,
    url: &str,
    destination: &PathBuf,
    headers: Option<&HashMap<String, String>>,
) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("Destino de download inválido: {}", destination.display()))?;

    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Não foi possível criar o diretório de download {}: {error}",
            parent.display()
        )
    })?;

    let mut request = client.get(url);

    if let Some(headers) = headers {
        for (name, value) in headers {
            request = request.header(name, value);
        }
    }

    let mut response = request
        .send()
        .map_err(|error| format!("Não foi possível baixar {url}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Servidor retornou erro ao baixar {url}: {error}"))?;
    let temporary_destination = temporary_download_path(destination);
    let mut output = fs::File::create(&temporary_destination).map_err(|error| {
        format!(
            "Não foi possível criar o arquivo temporário {}: {error}",
            temporary_destination.display()
        )
    })?;

    io::copy(&mut response, &mut output).map_err(|error| {
        format!(
            "Não foi possível salvar o download em {}: {error}",
            temporary_destination.display()
        )
    })?;
    fs::rename(&temporary_destination, destination).map_err(|error| {
        format!(
            "Não foi possível finalizar o download em {}: {error}",
            destination.display()
        )
    })?;

    Ok(())
}

fn extract_zip_archive(
    archive_path: &Path,
    destination: &Path,
    strip_top_level_dir: bool,
) -> Result<usize, String> {
    let archive_file = fs::File::open(archive_path).map_err(|error| {
        format!(
            "Não foi possível abrir o arquivo ZIP {}: {error}",
            archive_path.display()
        )
    })?;
    let mut archive = zip::ZipArchive::new(archive_file).map_err(|error| {
        format!(
            "O arquivo baixado não é um ZIP válido ({}): {error}",
            archive_path.display()
        )
    })?;
    let mut extracted_files = 0_usize;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Não foi possível ler a entrada {index} do ZIP: {error}"))?;
        let enclosed_path = entry
            .enclosed_name()
            .ok_or_else(|| format!("O ZIP contém um caminho inseguro: {}", entry.name()))?;
        let relative_path = if strip_top_level_dir {
            let mut components = enclosed_path.components();
            components.next();
            components.as_path().to_path_buf()
        } else {
            enclosed_path.to_path_buf()
        };

        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let output_path = destination.join(relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|error| {
                format!(
                    "Não foi possível criar diretório extraído {}: {error}",
                    output_path.display()
                )
            })?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Não foi possível criar diretório extraído {}: {error}",
                    parent.display()
                )
            })?;
        }

        let mut output = fs::File::create(&output_path).map_err(|error| {
            format!(
                "Não foi possível criar arquivo extraído {}: {error}",
                output_path.display()
            )
        })?;
        io::copy(&mut entry, &mut output).map_err(|error| {
            format!(
                "Não foi possível extrair {}: {error}",
                output_path.display()
            )
        })?;

        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&output_path, fs::Permissions::from_mode(mode)).map_err(
                |error| {
                    format!(
                        "Não foi possível restaurar permissões de {}: {error}",
                        output_path.display()
                    )
                },
            )?;
        }

        extracted_files += 1;
    }

    Ok(extracted_files)
}

fn ensure_executable_permission(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path).map_err(|error| {
            format!(
                "Não foi possível inspecionar o executável {}: {error}",
                path.display()
            )
        })?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(path, permissions).map_err(|error| {
            format!(
                "Não foi possível marcar {} como executável: {error}",
                path.display()
            )
        })?;
    }

    Ok(())
}

fn install_archive_files(
    app: &tauri::AppHandle,
    game_id: &str,
    manifest: &GameManifest,
    method: &InstallMethod,
    archive_path: &Path,
    staging_dir: &Path,
    target_dir: &Path,
) -> Result<(), String> {
    let archive_url = method
        .url
        .as_deref()
        .ok_or_else(|| format!("O método archive de {} não define uma URL.", manifest.name))?;

    emit_install_flow_progress(
        app,
        game_id,
        "downloading",
        "Baixando o arquivo do cliente Linux...",
    );
    let client = http_client()?;
    download_file_with_retry_using_client(
        &client,
        archive_url,
        &archive_path.to_path_buf(),
        None,
        Some(&method.headers),
    )?;

    emit_install_flow_progress(
        app,
        game_id,
        "extracting",
        "Download concluído. Extraindo os arquivos...",
    );
    remove_dir_if_exists(staging_dir)?;
    fs::create_dir_all(staging_dir).map_err(|error| {
        format!(
            "Não foi possível preparar staging {}: {error}",
            staging_dir.display()
        )
    })?;
    let extracted_files =
        extract_zip_archive(archive_path, staging_dir, method.strip_top_level_dir)?;
    let executable = manifest.launch.executable.as_deref().ok_or_else(|| {
        format!(
            "O manifesto de {} não define launch.executable.",
            manifest.name
        )
    })?;
    let staged_executable = command_path_for_install(staging_dir, executable);

    if !staged_executable.is_file() {
        return Err(format!(
            "O ZIP de {} foi extraído, mas o executável esperado não foi encontrado: {}",
            manifest.name,
            staged_executable.display()
        ));
    }

    emit_install_flow_progress(
        app,
        game_id,
        "preparing",
        "Preparando o executável do jogo...",
    );
    ensure_executable_permission(&staged_executable)?;

    if target_dir.exists() {
        remove_dir_if_exists(target_dir)?;
    }
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Não foi possível preparar a pasta de jogos {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::rename(staging_dir, target_dir).map_err(|error| {
        format!(
            "Não foi possível finalizar a instalação em {}: {error}",
            target_dir.display()
        )
    })?;

    append_runner_log(
        app,
        game_id,
        &[
            "archive_install_completed=true".to_string(),
            format!("archive_url={archive_url}"),
            format!("archive_path={}", archive_path.display()),
            format!("archive_format={:?}", method.format),
            format!("archive_strip_top_level_dir={}", method.strip_top_level_dir),
            format!("archive_extracted_files={extracted_files}"),
            format!("archive_install_path={}", target_dir.display()),
            format!(
                "archive_executable={}",
                command_path_for_install(target_dir, executable).display()
            ),
        ],
    )?;

    Ok(())
}

fn logs_dir(app: &tauri::AppHandle, game_id: &str) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        format!("Não foi possível resolver o diretório de dados do app: {error}")
    })?;
    let logs_dir = app_data_dir
        .join("logs")
        .join(sanitize_path_segment(game_id));

    fs::create_dir_all(&logs_dir).map_err(|error| {
        format!(
            "Não foi possível criar o diretório de logs {}: {error}",
            logs_dir.display()
        )
    })?;

    Ok(logs_dir)
}

fn runner_log_path(app: &tauri::AppHandle, game_id: &str) -> Result<PathBuf, String> {
    Ok(logs_dir(app, game_id)?.join("runner.log"))
}

fn append_runner_log(
    app: &tauri::AppHandle,
    game_id: &str,
    lines: &[String],
) -> Result<PathBuf, String> {
    let log_path = runner_log_path(app, game_id)?;
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|error| format!("Não foi possível abrir log {}: {error}", log_path.display()))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "unknown-time".to_string());

    writeln!(log, "\n=== launcher attempt {timestamp} ===").map_err(|error| {
        format!(
            "Não foi possível escrever log {}: {error}",
            log_path.display()
        )
    })?;

    for line in lines {
        writeln!(log, "{line}").map_err(|error| {
            format!(
                "Não foi possível escrever log {}: {error}",
                log_path.display()
            )
        })?;
    }

    Ok(log_path)
}

fn log_error_message(app: &tauri::AppHandle, game_id: &str, message: String) -> String {
    match append_runner_log(app, game_id, &[format!("error={message}")]) {
        Ok(log_path) => format!("{message} Log: {}", log_path.display()),
        Err(log_error) => {
            format!("{message} Também não foi possível escrever o runner.log: {log_error}")
        }
    }
}

fn format_runner_command_for_log(command: &runners::RunnerCommand) -> Vec<String> {
    let mut lines = vec![
        format!("runner_kind={}", command.runner_kind),
        format!("program={}", command.program.display()),
        format!("working_dir={}", command.working_dir.display()),
        format!("args={:?}", command.args),
    ];

    for (key, value) in &command.envs {
        lines.push(format!("env.{key}={value}"));
    }

    for key in &command.unset_envs {
        lines.push(format!("unset_env.{key}=true"));
    }

    lines
}

fn apply_runner_command_environment(
    command: &mut Command,
    runner_command: &runners::RunnerCommand,
) {
    command.envs(runner_command.envs.iter().map(|(key, value)| (key, value)));

    for key in &runner_command.unset_envs {
        command.env_remove(key);
    }
}

fn host_environment_for_log() -> Vec<String> {
    [
        "DISPLAY",
        "XAUTHORITY",
        "XDG_SESSION_TYPE",
        "WAYLAND_DISPLAY",
        "DESKTOP_SESSION",
    ]
    .into_iter()
    .map(|key| {
        let value = std::env::var(key).unwrap_or_else(|_| "<unset>".to_string());

        format!("host_env.{key}={value}")
    })
    .collect()
}

fn log_process_exit(app: tauri::AppHandle, game_id: String, pid: u32, mut child: Child) {
    thread::spawn(move || {
        let lines = match child.wait() {
            Ok(status) => vec![
                format!("process_pid={pid}"),
                format!("process_exit_status={status}"),
                format!("process_exit_code={:?}", status.code()),
            ],
            Err(error) => vec![
                format!("process_pid={pid}"),
                format!("process_wait_error={error}"),
            ],
        };

        let _ = append_runner_log(&app, &game_id, &lines);
    });
}

fn log_installer_exit_and_reconcile(
    app: tauri::AppHandle,
    game_id: String,
    pid: u32,
    mut child: Child,
    manifest: GameManifest,
) {
    thread::spawn(move || {
        let lines = match child.wait() {
            Ok(status) => vec![
                format!("process_pid={pid}"),
                format!("process_exit_status={status}"),
                format!("process_exit_code={:?}", status.code()),
            ],
            Err(error) => vec![
                format!("process_pid={pid}"),
                format!("process_wait_error={error}"),
            ],
        };

        let _ = append_runner_log(&app, &game_id, &lines);
        emit_install_flow_progress(
            &app,
            &game_id,
            "reconciling",
            "Instalador concluído. Localizando os arquivos do jogo...",
        );

        let install = match reconcile_or_register_install_path(&app, &game_id, &manifest) {
            Ok(install) => install,
            Err(error) => {
                let _ = append_runner_log(
                    &app,
                    &game_id,
                    &[format!("install_reconcile_error={error}")],
                );
                emit_install_flow_progress(
                    &app,
                    &game_id,
                    "error",
                    &format!("Não foi possível localizar a instalação: {error}"),
                );
                None
            }
        };

        if should_launch_after_install(&manifest) {
            if let Some(install) = install {
                if manifest.update.strategy == "remoteManifest" {
                    emit_install_flow_progress(
                        &app,
                        &game_id,
                        "updating",
                        "Instalação localizada. Atualizando os arquivos antes de abrir...",
                    );
                    let update_log_path = match append_runner_log(
                        &app,
                        &game_id,
                        &[
                            "action=run_game_remote_update".to_string(),
                            "update_trigger=after_install".to_string(),
                            format!("game_id={game_id}"),
                        ],
                    ) {
                        Ok(log_path) => log_path,
                        Err(error) => {
                            let _ = append_runner_log(
                                &app,
                                &game_id,
                                &[format!("update_after_install_error={error}")],
                            );
                            emit_install_flow_progress(
                                &app,
                                &game_id,
                                "error",
                                &format!("Não foi possível preparar o update: {error}"),
                            );
                            return;
                        }
                    };
                    let install_path = PathBuf::from(&install.install_path);

                    match run_remote_manifest_update(
                        app.clone(),
                        game_id.clone(),
                        manifest.clone(),
                        install_path,
                        update_log_path,
                    ) {
                        Ok(result) => {
                            let _ = append_runner_log(
                                &app,
                                &game_id,
                                &[
                                    "update_after_install_completed=true".to_string(),
                                    format!(
                                        "update_after_install_updated_files={}",
                                        result.updated_files
                                    ),
                                    format!(
                                        "update_after_install_skipped_files={}",
                                        result.skipped_files
                                    ),
                                ],
                            );
                        }
                        Err(error) => {
                            let _ = append_runner_log(
                                &app,
                                &game_id,
                                &[
                                    "launch_after_install_skipped=update_failed".to_string(),
                                    format!("update_after_install_error={error}"),
                                ],
                            );
                            emit_install_flow_progress(
                                &app,
                                &game_id,
                                "error",
                                &format!("A atualização falhou: {error}"),
                            );
                            return;
                        }
                    }
                }

                emit_install_flow_progress(
                    &app,
                    &game_id,
                    "launching",
                    "Arquivos prontos. Iniciando o jogo...",
                );
                match launch_install(
                    &app,
                    &game_id,
                    &manifest,
                    &install,
                    "action=launch_after_install",
                ) {
                    Ok(result) => {
                        let _ = append_runner_log(
                            &app,
                            &game_id,
                            &[
                                "launch_after_install_started=true".to_string(),
                                format!("launch_after_install_runner={}", result.runner),
                                format!("launch_after_install_command={}", result.command),
                            ],
                        );
                        emit_install_flow_progress(
                            &app,
                            &game_id,
                            "done",
                            "Instalação e atualização concluídas. Jogo iniciado.",
                        );
                    }
                    Err(error) => {
                        let _ = append_runner_log(
                            &app,
                            &game_id,
                            &[format!("launch_after_install_error={error}")],
                        );
                        emit_install_flow_progress(
                            &app,
                            &game_id,
                            "error",
                            &format!("Os arquivos ficaram prontos, mas o jogo não abriu: {error}"),
                        );
                    }
                }
            } else {
                let _ = append_runner_log(
                    &app,
                    &game_id,
                    &["launch_after_install_skipped=no_install_found".to_string()],
                );
                emit_install_flow_progress(
                    &app,
                    &game_id,
                    "error",
                    "O instalador terminou, mas a pasta do jogo não foi encontrada.",
                );
            }
        }
    });
}

fn attach_process_logs(
    app: &tauri::AppHandle,
    game_id: &str,
    command: &mut Command,
) -> Result<PathBuf, String> {
    let log_path = runner_log_path(app, game_id)?;
    let stdout_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|error| format!("Não foi possível abrir log {}: {error}", log_path.display()))?;
    let stderr_log = stdout_log.try_clone().map_err(|error| {
        format!(
            "Não foi possível duplicar log {}: {error}",
            log_path.display()
        )
    })?;

    command
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log));

    Ok(log_path)
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Olá, {name}! O backend Tauri está pronto.")
}

#[tauri::command]
fn list_games() -> Result<Vec<GameManifest>, String> {
    let mut games = Vec::new();
    let dir = manifests_dir()?;

    let entries = fs::read_dir(&dir).map_err(|error| error.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let manifest = serde_json::from_str::<GameManifest>(&content)
            .map_err(|error| format!("Manifesto inválido em {}: {error}", path.display()))?;

        games.push(manifest);
    }

    games.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(games)
}

#[tauri::command]
fn verify_game_install(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<InstallVerificationResult, String> {
    let game_id = game_id.trim().to_string();
    let manifest = get_manifest(&game_id)?;
    let connection = open_database(&app)?;
    let install = get_install(&connection, &game_id)?;
    let install_path = PathBuf::from(&install.install_path);
    let install_path_exists = install_path.is_dir();
    let executable_path =
        effective_executable_path_for_verification(&app, &game_id, &manifest, &install_path)?;
    let executable_exists = executable_path.as_ref().is_some_and(|path| path.is_file());
    let mut issues = Vec::new();
    let missing_files = missing_required_files(
        &install_path,
        install_path_exists,
        &manifest.verification.required_files,
    );
    let checksum_results = verify_configured_checksums(
        &install_path,
        install_path_exists,
        &manifest.verification.checksums,
    )?;
    let invalid_checksum_count = checksum_results
        .iter()
        .filter(|result| !result.valid)
        .count();

    if !install_path_exists {
        issues.push(format!(
            "A pasta registrada não existe mais: {}",
            install_path.display()
        ));
    }

    if executable_path.is_none() {
        issues.push("O manifesto não define um executável verificável.".to_string());
    } else if !executable_exists {
        issues.push(format!(
            "O executável principal não foi encontrado: {}",
            executable_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        ));
    }

    if !missing_files.is_empty() {
        issues.push(format!(
            "{} arquivo(s) obrigatório(s) estão ausentes.",
            missing_files.len()
        ));
    }

    if invalid_checksum_count > 0 {
        issues.push(format!(
            "{invalid_checksum_count} arquivo(s) possuem checksum ausente ou divergente."
        ));
    }

    let valid = install_path_exists
        && executable_exists
        && missing_files.is_empty()
        && invalid_checksum_count == 0;

    Ok(InstallVerificationResult {
        game_id,
        valid,
        install_path: install_path.to_string_lossy().to_string(),
        install_path_exists,
        executable_path: executable_path.map(|path| path.to_string_lossy().to_string()),
        executable_exists,
        missing_files,
        checksum_results,
        issues,
        repair_strategy: repair_strategy_for_manifest(&manifest),
    })
}

#[cfg(test)]
mod tests {
    use super::{missing_required_files, verify_configured_checksums, VerificationChecksumConfig};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_only_missing_required_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("relógio do sistema válido")
            .as_nanos();
        let install_path = std::env::temp_dir().join(format!(
            "two-d-mmo-launcher-verification-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(install_path.join("data")).expect("criar instalação temporária");
        fs::write(install_path.join("game.bin"), b"test").expect("criar executável de teste");
        let required_files = vec![
            "game.bin".to_string(),
            "data".to_string(),
            "missing.pak".to_string(),
        ];

        let missing = missing_required_files(&install_path, true, &required_files);

        assert_eq!(missing, vec!["missing.pak"]);
        fs::remove_dir_all(install_path).expect("remover instalação temporária");
    }

    #[test]
    fn reports_all_required_files_when_install_path_is_missing() {
        let required_files = vec!["game.bin".to_string(), "data".to_string()];

        let missing = missing_required_files(
            &std::env::temp_dir().join("nonexistent-two-d-mmo-install"),
            false,
            &required_files,
        );

        assert_eq!(missing, required_files);
    }

    #[test]
    fn verifies_configured_crc32_checksums() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("relógio do sistema válido")
            .as_nanos();
        let install_path = std::env::temp_dir().join(format!(
            "two-d-mmo-launcher-checksum-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&install_path).expect("criar instalação temporária");
        fs::write(install_path.join("game.bin"), b"test").expect("criar arquivo de teste");
        let checksums = vec![VerificationChecksumConfig {
            path: "game.bin".to_string(),
            algorithm: "crc32".to_string(),
            value: "d87f7e0c".to_string(),
        }];

        let results = verify_configured_checksums(&install_path, true, &checksums)
            .expect("verificar checksum");

        assert_eq!(results.len(), 1);
        assert!(results[0].valid);
        assert_eq!(results[0].actual.as_deref(), Some("d87f7e0c"));
        fs::remove_dir_all(install_path).expect("remover instalação temporária");
    }

    #[test]
    fn reports_divergent_and_missing_checksum_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("relógio do sistema válido")
            .as_nanos();
        let install_path = std::env::temp_dir().join(format!(
            "two-d-mmo-launcher-checksum-failure-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&install_path).expect("criar instalação temporária");
        fs::write(install_path.join("game.bin"), b"changed").expect("criar arquivo de teste");
        let checksums = vec![
            VerificationChecksumConfig {
                path: "game.bin".to_string(),
                algorithm: "crc32".to_string(),
                value: "d87f7e0c".to_string(),
            },
            VerificationChecksumConfig {
                path: "missing.bin".to_string(),
                algorithm: "crc32".to_string(),
                value: "00000000".to_string(),
            },
        ];

        let results = verify_configured_checksums(&install_path, true, &checksums)
            .expect("verificar checksums");

        assert!(!results[0].valid);
        assert!(results[0].actual.is_some());
        assert!(!results[1].valid);
        assert!(results[1].actual.is_none());
        fs::remove_dir_all(install_path).expect("remover instalação temporária");
    }

    #[test]
    fn rejects_invalid_checksum_configuration() {
        let install_path = std::env::temp_dir();
        let unsafe_path = vec![VerificationChecksumConfig {
            path: "../outside.bin".to_string(),
            algorithm: "crc32".to_string(),
            value: "00000000".to_string(),
        }];
        let unsupported_algorithm = vec![VerificationChecksumConfig {
            path: "game.bin".to_string(),
            algorithm: "sha256".to_string(),
            value: "00000000".to_string(),
        }];
        let invalid_value = vec![VerificationChecksumConfig {
            path: "game.bin".to_string(),
            algorithm: "crc32".to_string(),
            value: "not-a-crc".to_string(),
        }];
        let absolute_path = vec![VerificationChecksumConfig {
            path: "/tmp/outside.bin".to_string(),
            algorithm: "crc32".to_string(),
            value: "00000000".to_string(),
        }];

        assert!(verify_configured_checksums(&install_path, true, &unsafe_path).is_err());
        assert!(verify_configured_checksums(&install_path, true, &absolute_path).is_err());
        assert!(verify_configured_checksums(&install_path, true, &unsupported_algorithm).is_err());
        assert!(verify_configured_checksums(&install_path, true, &invalid_value).is_err());
    }
}

#[tauri::command]
fn list_installs(app: tauri::AppHandle) -> Result<Vec<GameInstall>, String> {
    let connection = open_database(&app)?;
    let mut statement = connection
        .prepare(
            "
            SELECT game_id, install_path, runner_override, created_at, updated_at
            FROM installs
            ORDER BY game_id ASC
            ",
        )
        .map_err(|error| format!("Não foi possível consultar instalações: {error}"))?;

    let installs = statement
        .query_map([], |row| {
            Ok(GameInstall {
                game_id: row.get(0)?,
                install_path: row.get(1)?,
                runner_override: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|error| format!("Não foi possível ler instalações: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Registro de instalação inválido: {error}"))?;

    Ok(installs)
}

#[tauri::command]
fn locate_existing_install(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<Option<GameInstall>, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let Some(path) = rfd::FileDialog::new()
        .set_title("Localizar instalação existente")
        .pick_folder()
    else {
        return Ok(None);
    };

    let install_path = path.to_string_lossy().to_string();
    let connection = open_database(&app)?;

    let install = save_install(&connection, &game_id, &install_path, None)?;
    emit_install_updated(&app, &install);

    Ok(Some(install))
}

#[tauri::command]
fn open_install_folder(app: tauri::AppHandle, game_id: String) -> Result<(), String> {
    let connection = open_database(&app)?;
    let install = get_install(&connection, game_id.trim())?;
    let path = PathBuf::from(&install.install_path);

    if !path.exists() {
        return Err(format!(
            "A pasta registrada para {} não existe mais: {}",
            install.game_id,
            path.display()
        ));
    }

    open_path(&install.install_path)
}

#[tauri::command]
fn remove_install(app: tauri::AppHandle, game_id: String) -> Result<bool, String> {
    let connection = open_database(&app)?;
    let removed_rows = connection
        .execute(
            "DELETE FROM installs WHERE game_id = ?1",
            params![game_id.trim()],
        )
        .map_err(|error| format!("Não foi possível remover a instalação: {error}"))?;

    Ok(removed_rows > 0)
}

#[tauri::command]
fn download_and_run_installer(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<LaunchResult, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let attempt_log_path = append_runner_log(
        &app,
        &game_id,
        &[
            "action=download_and_run_installer".to_string(),
            format!("game_id={game_id}"),
        ],
    )?;

    let manifest =
        get_manifest(&game_id).map_err(|error| log_error_message(&app, &game_id, error))?;
    let installer = manifest
        .installation
        .methods
        .iter()
        .find(|method| method.kind == "windowsInstaller")
        .ok_or_else(|| {
            log_error_message(
                &app,
                &game_id,
                format!(
                    "{} não possui método windowsInstaller no manifesto.",
                    manifest.name
                ),
            )
        })?;
    let installer_url = installer.url.as_ref().ok_or_else(|| {
        log_error_message(
            &app,
            &game_id,
            format!(
                "O método windowsInstaller de {} não define uma URL de download.",
                manifest.name
            ),
        )
    })?;
    let downloads_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Não foi possível resolver o diretório de dados do app: {error}"))?
        .join("downloads")
        .join(sanitize_path_segment(&game_id));
    let installer_path = downloads_dir.join(filename_from_url(installer_url));

    append_runner_log(
        &app,
        &game_id,
        &[
            format!("manifest={}", manifest.name),
            format!("launch_runner={}", manifest.launch.runner),
            format!("installer_compat_prefix={:?}", installer.compat_prefix),
            format!("installer_install_path={:?}", installer.install_path),
            format!("installer_url={installer_url}"),
            format!("installer_path={}", installer_path.display()),
            format!("log_path={}", attempt_log_path.display()),
        ],
    )?;

    download_file(installer_url, &installer_path)
        .map_err(|error| log_error_message(&app, &game_id, error))?;

    let installer_runner = installer
        .runner
        .as_deref()
        .unwrap_or(&manifest.launch.runner);

    append_runner_log(
        &app,
        &game_id,
        &[
            format!("installer_runner={installer_runner}"),
            format!("requested_runner={installer_runner}"),
        ],
    )?;

    let resolved_runner = resolve_runner(&app, installer_runner)
        .map_err(|error| log_error_message(&app, &game_id, error))?;

    append_runner_log(
        &app,
        &game_id,
        &[
            format!("resolved_runner_id={}", resolved_runner.id),
            format!("resolved_runner_kind={}", resolved_runner.kind),
            format!("resolved_runner_label={}", resolved_runner.label),
            format!("resolved_runner_source={}", resolved_runner.source),
            format!("resolved_runner_path={:?}", resolved_runner.path),
        ],
    )?;

    let runner_command = build_runner_command(
        &app,
        &game_id,
        &resolved_runner,
        &installer_path,
        &downloads_dir,
        &[],
        installer.compat_prefix.as_deref(),
    )
    .map_err(|error| log_error_message(&app, &game_id, error))?;

    let mut command_log = format_runner_command_for_log(&runner_command);
    command_log.extend(host_environment_for_log());

    append_runner_log(&app, &game_id, &command_log)?;

    let mut command = Command::new(&runner_command.program);

    command
        .args(&runner_command.args)
        .current_dir(&runner_command.working_dir);
    apply_runner_command_environment(&mut command, &runner_command);

    let log_path = attach_process_logs(&app, &game_id, &mut command)?;

    let child = command.spawn().map_err(|error| {
        log_error_message(
            &app,
            &game_id,
            format!(
                "Não foi possível iniciar o instalador de {} usando {}: {error}. Log: {}",
                manifest.name,
                runner_command.program.display(),
                log_path.display()
            ),
        )
    })?;
    let process_id = child.id();

    append_runner_log(
        &app,
        &game_id,
        &[
            "process_started=true".to_string(),
            format!("process_pid={process_id}"),
        ],
    )?;

    if should_launch_after_install(&manifest) {
        emit_install_flow_progress(
            &app,
            &game_id,
            "installing",
            "Instalador aberto. Conclua a instalação para continuar automaticamente.",
        );
    }

    if let Some(relative_install_path) = installer.install_path.as_deref() {
        let compat_prefix = installer
            .compat_prefix
            .as_deref()
            .unwrap_or(installer_runner);
        let install_root = managed_windows_prefix_dir(&app, &game_id, compat_prefix)
            .map_err(|error| log_error_message(&app, &game_id, error))?;
        let expected_install_path = install_root.join(relative_install_path);
        let connection =
            open_database(&app).map_err(|error| log_error_message(&app, &game_id, error))?;
        let saved_install = save_install(
            &connection,
            &game_id,
            &expected_install_path.to_string_lossy(),
            None,
        )
        .map_err(|error| log_error_message(&app, &game_id, error))?;
        emit_install_updated(&app, &saved_install);

        append_runner_log(
            &app,
            &game_id,
            &[
                "install_registered=true".to_string(),
                format!("registered_install_path={}", saved_install.install_path),
            ],
        )?;
    }

    log_installer_exit_and_reconcile(
        app.clone(),
        game_id.clone(),
        process_id,
        child,
        manifest.clone(),
    );

    Ok(LaunchResult {
        game_id,
        runner: runner_command.runner_kind,
        command: runner_command.program.to_string_lossy().to_string(),
        working_dir: runner_command.working_dir.to_string_lossy().to_string(),
        log_path: Some(log_path.to_string_lossy().to_string()),
    })
}

#[tauri::command]
async fn download_and_install_archive(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<LaunchResult, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let manifest = get_manifest(&game_id)?;
    let method = manifest
        .installation
        .methods
        .iter()
        .find(|method| method.kind == "archive")
        .cloned()
        .ok_or_else(|| format!("{} não possui método archive no manifesto.", manifest.name))?;

    if !matches!(method.format.as_deref(), None | Some("zip")) {
        return Err(format!(
            "Formato de arquivo não suportado para {}: {:?}",
            manifest.name, method.format
        ));
    }

    let archive_url = method
        .url
        .as_deref()
        .ok_or_else(|| format!("O método archive de {} não define uma URL.", manifest.name))?;
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        format!("Não foi possível resolver o diretório de dados do app: {error}")
    })?;
    let safe_game_id = sanitize_path_segment(&game_id);
    let archive_path = app_data_dir
        .join("downloads")
        .join(&safe_game_id)
        .join(filename_from_url(archive_url));
    let staging_dir = app_data_dir.join("install-staging").join(&safe_game_id);
    let target_dir = app_data_dir.join("games").join(&safe_game_id);

    append_runner_log(
        &app,
        &game_id,
        &[
            "action=download_and_install_archive".to_string(),
            format!("game_id={game_id}"),
            format!("archive_url={archive_url}"),
            format!("archive_target_dir={}", target_dir.display()),
        ],
    )?;
    emit_install_flow_progress(
        &app,
        &game_id,
        "preparing",
        "Preparando instalação do arquivo...",
    );

    let worker_app = app.clone();
    let worker_game_id = game_id.clone();
    let worker_manifest = manifest.clone();
    let worker_method = method.clone();
    let worker_archive_path = archive_path.clone();
    let worker_staging_dir = staging_dir.clone();
    let worker_target_dir = target_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        install_archive_files(
            &worker_app,
            &worker_game_id,
            &worker_manifest,
            &worker_method,
            &worker_archive_path,
            &worker_staging_dir,
            &worker_target_dir,
        )
    })
    .await
    .map_err(|error| format!("A tarefa de instalação do ZIP falhou: {error}"))??;

    let connection = open_database(&app)?;
    let install = save_install(&connection, &game_id, &target_dir.to_string_lossy(), None)?;
    emit_install_updated(&app, &install);
    emit_install_flow_progress(
        &app,
        &game_id,
        "launching",
        "Instalação concluída. Iniciando o jogo...",
    );

    let result = launch_install(
        &app,
        &game_id,
        &manifest,
        &install,
        "action=launch_after_archive_install",
    )?;

    emit_install_flow_progress(&app, &game_id, "done", "Jogo instalado e iniciado.");

    Ok(result)
}

#[tauri::command]
fn run_game_update(app: tauri::AppHandle, game_id: String) -> Result<LaunchResult, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let attempt_log_path = append_runner_log(
        &app,
        &game_id,
        &[
            "action=run_game_update".to_string(),
            format!("game_id={game_id}"),
        ],
    )?;

    let connection =
        open_database(&app).map_err(|error| log_error_message(&app, &game_id, error))?;
    let install = get_install(&connection, &game_id)
        .map_err(|error| log_error_message(&app, &game_id, error))?;
    let manifest =
        get_manifest(&game_id).map_err(|error| log_error_message(&app, &game_id, error))?;
    let install =
        reconcile_registered_install_path(&app, &connection, &game_id, &manifest, install)
            .map_err(|error| log_error_message(&app, &game_id, error))?;
    let install_path = PathBuf::from(&install.install_path);

    if !install_path.exists() {
        return Err(log_error_message(
            &app,
            &game_id,
            format!(
                "A pasta registrada para {} não existe mais: {}",
                manifest.name,
                install_path.display()
            ),
        ));
    }

    let requested_runner = manifest
        .update
        .runner
        .clone()
        .or_else(|| install.runner_override.clone())
        .unwrap_or_else(|| manifest.launch.runner.clone());
    let resolved_runner = resolve_runner(&app, &requested_runner)
        .map_err(|error| log_error_message(&app, &game_id, error))?;

    append_runner_log(
        &app,
        &game_id,
        &[
            format!("manifest={}", manifest.name),
            format!("install_path={}", install.install_path),
            format!("update_strategy={}", manifest.update.strategy),
            format!("update_runner={:?}", manifest.update.runner),
            format!("update_executable={:?}", manifest.update.executable),
            format!("requested_runner={requested_runner}"),
            format!("resolved_runner_id={}", resolved_runner.id),
            format!("resolved_runner_kind={}", resolved_runner.kind),
            format!("resolved_runner_label={}", resolved_runner.label),
            format!("resolved_runner_source={}", resolved_runner.source),
            format!("resolved_runner_path={:?}", resolved_runner.path),
            format!("log_path={}", attempt_log_path.display()),
        ],
    )?;

    let runner_command =
        build_update_runner_command(&app, &game_id, &manifest, &resolved_runner, &install_path)
            .map_err(|error| log_error_message(&app, &game_id, error))?;
    let mut command_log = format_runner_command_for_log(&runner_command);

    command_log.extend(host_environment_for_log());
    append_runner_log(&app, &game_id, &command_log)?;

    let mut command = Command::new(&runner_command.program);

    command
        .args(&runner_command.args)
        .current_dir(&runner_command.working_dir);
    apply_runner_command_environment(&mut command, &runner_command);

    let log_path = attach_process_logs(&app, &game_id, &mut command)?;

    let child = command.spawn().map_err(|error| {
        log_error_message(
            &app,
            &game_id,
            format!(
                "Não foi possível iniciar o updater de {} usando {}: {error}. Log: {}",
                manifest.name,
                runner_command.program.display(),
                log_path.display()
            ),
        )
    })?;
    let process_id = child.id();

    append_runner_log(
        &app,
        &game_id,
        &[
            "update_process_started=true".to_string(),
            format!("update_process_pid={process_id}"),
        ],
    )?;
    log_process_exit(app.clone(), game_id.clone(), process_id, child);

    Ok(LaunchResult {
        game_id,
        runner: runner_command.runner_kind,
        command: runner_command.program.to_string_lossy().to_string(),
        working_dir: runner_command.working_dir.to_string_lossy().to_string(),
        log_path: Some(log_path.to_string_lossy().to_string()),
    })
}

#[tauri::command]
fn get_game_update_progress(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<Option<GameUpdateProgress>, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let log_path = runner_log_path(&app, &game_id)?;

    if !log_path.exists() {
        return Ok(None);
    }

    let log_text = read_recent_log_text(&log_path, 768 * 1024)?;

    Ok(parse_latest_update_progress_from_log(
        &game_id, &log_path, &log_text,
    ))
}

#[tauri::command]
async fn install_game_from_remote_manifest(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<LaunchResult, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let manifest = get_manifest(&game_id)?;

    if manifest.update.strategy != "remoteManifest" {
        return Err(format!(
            "{} não possui instalação gerenciada por remoteManifest.",
            manifest.name
        ));
    }

    emit_install_flow_progress(
        &app,
        &game_id,
        "preparing",
        "Preparando a pasta gerenciada do jogo...",
    );

    let prefix_root = managed_windows_prefix_dir(&app, &game_id, &manifest.launch.runner)?;
    let target_dir = remote_update_target_dir(&app, &game_id, &manifest, &prefix_root)?;
    fs::create_dir_all(&target_dir).map_err(|error| {
        format!(
            "Não foi possível criar a instalação gerenciada em {}: {error}",
            target_dir.display()
        )
    })?;

    let attempt_log_path = append_runner_log(
        &app,
        &game_id,
        &[
            "action=run_game_remote_update".to_string(),
            "update_trigger=managed_install".to_string(),
            format!("game_id={game_id}"),
            format!("managed_install_path={}", target_dir.display()),
        ],
    )?;

    emit_install_flow_progress(
        &app,
        &game_id,
        "updating",
        "Baixando e validando os arquivos do jogo...",
    );

    let update_app = app.clone();
    let update_game_id = game_id.clone();
    let update_manifest = manifest.clone();
    let update_install_path = target_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_remote_manifest_update(
            update_app,
            update_game_id,
            update_manifest,
            update_install_path,
            attempt_log_path,
        )
    })
    .await
    .map_err(|error| format!("A instalação gerenciada falhou: {error}"))??;

    let connection = open_database(&app)?;
    let install = save_install(&connection, &game_id, &target_dir.to_string_lossy(), None)?;
    emit_install_updated(&app, &install);

    emit_install_flow_progress(
        &app,
        &game_id,
        "launching",
        "Instalação concluída. Iniciando o jogo...",
    );

    let result = launch_install(
        &app,
        &game_id,
        &manifest,
        &install,
        "action=launch_after_managed_install",
    )?;

    emit_install_flow_progress(
        &app,
        &game_id,
        "done",
        "Jogo instalado, atualizado e iniciado.",
    );

    Ok(result)
}

#[tauri::command]
async fn run_game_remote_update(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<GameUpdateResult, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let attempt_log_path = append_runner_log(
        &app,
        &game_id,
        &[
            "action=run_game_remote_update".to_string(),
            format!("game_id={game_id}"),
        ],
    )?;

    emit_and_log_update_stage(
        &app,
        &game_id,
        "preparing",
        "start",
        "Preparar update",
        "Iniciando diagnóstico do update remoto...",
        Some(&attempt_log_path),
    )?;

    emit_and_log_update_stage(
        &app,
        &game_id,
        "preparing",
        "openDatabase",
        "Abrir banco local",
        "Abrindo banco local do launcher...",
        Some(&attempt_log_path),
    )?;
    let connection = open_database(&app).map_err(|error| {
        log_and_emit_update_error(
            &app,
            &game_id,
            "openDatabase",
            "Abrir banco local",
            error,
            Some(&attempt_log_path),
            None,
        )
    })?;

    emit_and_log_update_stage(
        &app,
        &game_id,
        "preparing",
        "loadInstall",
        "Carregar instalação",
        "Carregando instalação registrada no SQLite...",
        Some(&attempt_log_path),
    )?;
    let install = get_install(&connection, &game_id).map_err(|error| {
        log_and_emit_update_error(
            &app,
            &game_id,
            "loadInstall",
            "Carregar instalação",
            error,
            Some(&attempt_log_path),
            None,
        )
    })?;

    emit_and_log_update_stage(
        &app,
        &game_id,
        "preparing",
        "loadLocalManifest",
        "Carregar manifesto local",
        "Lendo manifesto local do jogo...",
        Some(&attempt_log_path),
    )?;
    let manifest = get_manifest(&game_id).map_err(|error| {
        log_and_emit_update_error(
            &app,
            &game_id,
            "loadLocalManifest",
            "Carregar manifesto local",
            error,
            Some(&attempt_log_path),
            None,
        )
    })?;

    if manifest.update.strategy != "remoteManifest" {
        return Err(log_and_emit_update_error(
            &app,
            &game_id,
            "loadLocalManifest",
            "Carregar manifesto local",
            format!(
                "{} não possui update.strategy remoteManifest configurado.",
                manifest.name
            ),
            Some(&attempt_log_path),
            None,
        ));
    }

    emit_and_log_update_stage(
        &app,
        &game_id,
        "preparing",
        "reconcileInstall",
        "Reconciliar instalação",
        "Conferindo se a pasta registrada ainda bate com o manifesto...",
        Some(&attempt_log_path),
    )?;
    let install =
        reconcile_registered_install_path(&app, &connection, &game_id, &manifest, install)
            .map_err(|error| {
                log_and_emit_update_error(
                    &app,
                    &game_id,
                    "reconcileInstall",
                    "Reconciliar instalação",
                    error,
                    Some(&attempt_log_path),
                    None,
                )
            })?;
    let install_path = PathBuf::from(&install.install_path);

    if !install_path.exists() {
        return Err(log_and_emit_update_error(
            &app,
            &game_id,
            "validateInstallPath",
            "Validar pasta registrada",
            format!(
                "A pasta registrada para {} não existe mais: {}",
                manifest.name,
                install_path.display()
            ),
            Some(&attempt_log_path),
            None,
        ));
    }

    emit_and_log_update_stage(
        &app,
        &game_id,
        "preparing",
        "spawnBlockingTask",
        "Enviar tarefa para background",
        "Movendo verificação/download para uma tarefa em background...",
        Some(&attempt_log_path),
    )?;

    tauri::async_runtime::spawn_blocking(move || {
        run_remote_manifest_update(app, game_id, manifest, install_path, attempt_log_path)
    })
    .await
    .map_err(|error| format!("A tarefa de update remoto falhou: {error}"))?
}

#[tauri::command]
fn launch_game(app: tauri::AppHandle, game_id: String) -> Result<LaunchResult, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let attempt_log_path = append_runner_log(
        &app,
        &game_id,
        &[
            "action=launch_game".to_string(),
            format!("game_id={game_id}"),
        ],
    )?;

    let connection =
        open_database(&app).map_err(|error| log_error_message(&app, &game_id, error))?;
    let install = get_install(&connection, &game_id)
        .map_err(|error| log_error_message(&app, &game_id, error))?;
    let manifest =
        get_manifest(&game_id).map_err(|error| log_error_message(&app, &game_id, error))?;
    let install =
        reconcile_registered_install_path(&app, &connection, &game_id, &manifest, install)
            .map_err(|error| log_error_message(&app, &game_id, error))?;
    let requested_runner = install
        .runner_override
        .clone()
        .unwrap_or_else(|| manifest.launch.runner.clone());
    let resolved_runner = resolve_runner(&app, &requested_runner)
        .map_err(|error| log_error_message(&app, &game_id, error))?;

    append_runner_log(
        &app,
        &game_id,
        &[
            format!("manifest={}", manifest.name),
            format!("install_path={}", install.install_path),
            format!("requested_runner={requested_runner}"),
            format!("resolved_runner_id={}", resolved_runner.id),
            format!("resolved_runner_kind={}", resolved_runner.kind),
            format!("resolved_runner_label={}", resolved_runner.label),
            format!("resolved_runner_source={}", resolved_runner.source),
            format!("resolved_runner_path={:?}", resolved_runner.path),
            format!("log_path={}", attempt_log_path.display()),
        ],
    )?;

    let install_path = PathBuf::from(&install.install_path);

    install_battl_eye_if_configured(&app, &game_id, &manifest, &resolved_runner, &install_path)
        .map_err(|error| log_error_message(&app, &game_id, error))?;

    let executable = manifest.launch.executable.as_ref().ok_or_else(|| {
        log_error_message(
            &app,
            &game_id,
            format!(
                "O manifesto de {} ainda não define launch.executable. Configure o executável antes de jogar.",
                manifest.name
            ),
        )
    })?;

    if !install_path.exists() {
        return Err(log_error_message(
            &app,
            &game_id,
            format!(
                "A pasta registrada para {} não existe mais: {}",
                manifest.name,
                install_path.display()
            ),
        ));
    }

    let command_path = command_path_for_install(&install_path, executable);

    if !command_path.exists() && !battl_eye_replaces_main_process(&manifest) {
        return Err(log_error_message(
            &app,
            &game_id,
            format!(
                "Executável não encontrado para {}: {}",
                manifest.name,
                command_path.display()
            ),
        ));
    }

    let runner_command = build_game_runner_command(
        &app,
        &game_id,
        &manifest,
        &resolved_runner,
        &command_path,
        &install_path,
    )
    .map_err(|error| log_error_message(&app, &game_id, error))?;

    let mut command_log = format_runner_command_for_log(&runner_command);
    command_log.extend(host_environment_for_log());

    append_runner_log(&app, &game_id, &command_log)?;

    spawn_battl_eye_if_configured(&app, &game_id, &manifest, &resolved_runner, &install_path)
        .map_err(|error| log_error_message(&app, &game_id, error))?;

    let mut command = Command::new(&runner_command.program);

    command
        .args(&runner_command.args)
        .current_dir(&runner_command.working_dir);
    apply_runner_command_environment(&mut command, &runner_command);

    let log_path = attach_process_logs(&app, &game_id, &mut command)?;

    let child = command.spawn().map_err(|error| {
        log_error_message(
            &app,
            &game_id,
            format!(
                "Não foi possível iniciar {} usando {}: {error}. Log: {}",
                manifest.name,
                runner_command.program.display(),
                log_path.display()
            ),
        )
    })?;
    let process_id = child.id();

    append_runner_log(
        &app,
        &game_id,
        &[
            "process_started=true".to_string(),
            format!("process_pid={process_id}"),
        ],
    )?;
    log_process_exit(app.clone(), game_id.clone(), process_id, child);

    Ok(LaunchResult {
        game_id,
        runner: runner_command.runner_kind,
        command: runner_command.program.to_string_lossy().to_string(),
        working_dir: runner_command.working_dir.to_string_lossy().to_string(),
        log_path: Some(log_path.to_string_lossy().to_string()),
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            list_games,
            list_installs,
            list_runners,
            locate_existing_install,
            download_and_run_installer,
            download_and_install_archive,
            run_game_update,
            get_game_update_progress,
            install_game_from_remote_manifest,
            run_game_remote_update,
            verify_game_install,
            open_install_folder,
            remove_install,
            launch_game
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o 2D MMO Launcher");
}
