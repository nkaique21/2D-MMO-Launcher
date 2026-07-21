use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LaunchConfig {
    runner: String,
    executable: Option<String>,
    args: Vec<String>,
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
}

fn default_true() -> bool {
    true
}

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

    let battl_eye_command = build_runner_command(
        app,
        game_id,
        resolved_runner,
        &executable_path,
        &working_dir,
        &battl_eye.args,
        None,
    )?;
    let launch_mode = battl_eye
        .launch_mode
        .as_deref()
        .unwrap_or("beforeMain");
    let mut command_log = vec![
        "battl_eye_start=true".to_string(),
        format!("battl_eye_launch_mode={launch_mode}"),
    ];

    command_log.extend(
        format_runner_command_for_log(&battl_eye_command)
            .into_iter()
            .map(|line| format!("battl_eye.{line}")),
    );
    append_runner_log(app, game_id, &command_log)?;

    Ok(Some(battl_eye_command))
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

        if let Some(battl_eye_command) = build_battl_eye_runner_command(
            app,
            game_id,
            manifest,
            resolved_runner,
            install_path,
        )? {
            return Ok(battl_eye_command);
        }
    }

    build_runner_command(
        app,
        game_id,
        resolved_runner,
        command_path,
        install_path,
        &manifest.launch.args,
        None,
    )
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

    let Some(battl_eye_command) = build_battl_eye_runner_command(
        app,
        game_id,
        manifest,
        resolved_runner,
        install_path,
    )? else {
        return Ok(());
    };

    let mut command = Command::new(&battl_eye_command.program);

    command
        .args(&battl_eye_command.args)
        .current_dir(&battl_eye_command.working_dir)
        .envs(
            battl_eye_command
                .envs
                .iter()
                .map(|(key, value)| (key, value)),
        );

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

    if !command_path.exists() {
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
        .current_dir(&runner_command.working_dir)
        .envs(runner_command.envs.iter().map(|(key, value)| (key, value)));

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
    let parent = destination
        .parent()
        .ok_or_else(|| format!("Destino de download inválido: {}", destination.display()))?;

    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Não foi possível criar o diretório de download {}: {error}",
            parent.display()
        )
    })?;

    let mut response = reqwest::blocking::get(url)
        .map_err(|error| format!("Não foi possível baixar {url}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Servidor retornou erro ao baixar {url}: {error}"))?;
    let temporary_destination = destination.with_extension("download");
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

    lines
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

        let install = match reconcile_or_register_install_path(&app, &game_id, &manifest) {
            Ok(install) => install,
            Err(error) => {
                let _ = append_runner_log(
                    &app,
                    &game_id,
                    &[format!("install_reconcile_error={error}")],
                );
                None
            }
        };

        if should_launch_after_install(&manifest) {
            if let Some(install) = install {
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
                    }
                    Err(error) => {
                        let _ = append_runner_log(
                            &app,
                            &game_id,
                            &[format!("launch_after_install_error={error}")],
                        );
                    }
                }
            } else {
                let _ = append_runner_log(
                    &app,
                    &game_id,
                    &["launch_after_install_skipped=no_install_found".to_string()],
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
        .current_dir(&runner_command.working_dir)
        .envs(runner_command.envs.iter().map(|(key, value)| (key, value)));

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

    let command_path = command_path_for_install(&install_path, executable);

    if !command_path.exists() {
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
        .current_dir(&runner_command.working_dir)
        .envs(runner_command.envs.iter().map(|(key, value)| (key, value)));

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
            open_install_folder,
            remove_install,
            launch_game
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o 2D MMO Launcher");
}
