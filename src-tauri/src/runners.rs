use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunnerInfo {
    id: String,
    kind: String,
    label: String,
    status: String,
    source: String,
    path: Option<String>,
    installable: bool,
    install_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedRunner {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) label: String,
    pub(crate) source: String,
    pub(crate) path: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RunnerCommand {
    pub(crate) runner_kind: String,
    pub(crate) program: PathBuf,
    pub(crate) args: Vec<String>,
    pub(crate) working_dir: PathBuf,
    pub(crate) envs: Vec<(String, String)>,
}

fn path_is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        return fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;

    env::split_paths(&paths)
        .map(|path| path.join(binary))
        .find(|candidate| path_is_executable(candidate))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn path_to_proton_windows_path(path: &Path) -> String {
    let normalized_path = path.to_string_lossy().replace('/', "\\");

    if normalized_path.starts_with('\\') {
        format!("z:{normalized_path}")
    } else {
        normalized_path
    }
}

fn synthetic_steam_app_id(game_id: &str) -> String {
    let mut hash = 2_166_136_261_u32;

    for byte in game_id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }

    (2_000_000_000_u32 + (hash % 1_000_000_000)).to_string()
}

fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn runner_info(
    id: &str,
    kind: &str,
    label: &str,
    status: &str,
    source: &str,
    path: Option<PathBuf>,
    installable: bool,
    install_hint: Option<&str>,
) -> RunnerInfo {
    RunnerInfo {
        id: id.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        source: source.to_string(),
        path: path.as_deref().map(path_to_string),
        installable,
        install_hint: install_hint.map(str::to_string),
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn steam_library_dirs() -> Vec<PathBuf> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };

    vec![
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".local/share/Steam"),
    ]
}

fn steam_client_install_path() -> Option<PathBuf> {
    steam_library_dirs()
        .into_iter()
        .find(|steam_dir| steam_dir.is_dir())
}

fn managed_prefix_dir(
    app: &tauri::AppHandle,
    game_id: &str,
    runner_kind: &str,
) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        format!("Não foi possível resolver o diretório de dados do app: {error}")
    })?;
    let prefix_dir = app_data_dir
        .join("compat-data")
        .join(sanitize_path_segment(game_id))
        .join(sanitize_path_segment(runner_kind));

    fs::create_dir_all(&prefix_dir).map_err(|error| {
        format!(
            "Não foi possível criar o prefixo gerenciado {}: {error}",
            prefix_dir.display()
        )
    })?;

    Ok(prefix_dir)
}

pub(crate) fn managed_windows_prefix_dir(
    app: &tauri::AppHandle,
    game_id: &str,
    prefix_kind: &str,
) -> Result<PathBuf, String> {
    match prefix_kind {
        "proton" => Ok(managed_prefix_dir(app, game_id, "proton")?.join("pfx")),
        "wine" => managed_prefix_dir(app, game_id, "wine"),
        unsupported_prefix => Err(format!(
            "Prefixo compatível '{}' não é suportado para instaladores Windows.",
            unsupported_prefix
        )),
    }
}

fn managed_logs_dir(app: &tauri::AppHandle, game_id: &str) -> Result<PathBuf, String> {
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

fn discover_steam_proton_runners() -> Vec<RunnerInfo> {
    let mut runners = Vec::new();

    for steam_dir in steam_library_dirs() {
        let candidate_dirs = [
            steam_dir.join("compatibilitytools.d"),
            steam_dir.join("steamapps/common"),
        ];

        for candidate_dir in candidate_dirs {
            let Ok(entries) = fs::read_dir(&candidate_dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };

                if !name.to_lowercase().contains("proton") {
                    continue;
                }

                let proton_binary = path.join("proton");

                if !path_is_executable(&proton_binary) {
                    continue;
                }

                runners.push(runner_info(
                    &format!("steam-proton-{name}"),
                    "proton",
                    name,
                    "available",
                    "Steam",
                    Some(proton_binary),
                    false,
                    None,
                ));
            }
        }
    }

    runners
}

fn discover_managed_runners(app: &tauri::AppHandle) -> Result<Vec<RunnerInfo>, String> {
    let runners_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Não foi possível resolver o diretório de dados do app: {error}"))?
        .join("runners");

    let Ok(entries) = fs::read_dir(&runners_dir) else {
        return Ok(Vec::new());
    };

    let mut runners = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let proton_binary = path.join("proton");
        let wine_binary = path.join("bin/wine");

        if path_is_executable(&proton_binary) {
            runners.push(runner_info(
                &format!("managed-proton-{name}"),
                "proton",
                name,
                "available",
                "Launcher",
                Some(proton_binary),
                false,
                None,
            ));
        } else if path_is_executable(&wine_binary) {
            runners.push(runner_info(
                &format!("managed-wine-{name}"),
                "wine",
                name,
                "available",
                "Launcher",
                Some(wine_binary),
                false,
                None,
            ));
        }
    }

    Ok(runners)
}

fn discover_runners(app: &tauri::AppHandle) -> Result<Vec<RunnerInfo>, String> {
    let mut runners = vec![runner_info(
        "native-system",
        "native",
        "Linux nativo",
        "available",
        "Sistema",
        None,
        false,
        None,
    )];

    if let Some(wine_path) = find_in_path("wine") {
        runners.push(runner_info(
            "system-wine",
            "wine",
            "Wine do sistema",
            "available",
            "PATH",
            Some(wine_path),
            false,
            None,
        ));
    }

    if let Some(wine64_path) = find_in_path("wine64") {
        runners.push(runner_info(
            "system-wine64",
            "wine",
            "Wine64 do sistema",
            "available",
            "PATH",
            Some(wine64_path),
            false,
            None,
        ));
    }

    if let Some(proton_path) = find_in_path("proton") {
        runners.push(runner_info(
            "system-proton",
            "proton",
            "Proton do sistema",
            "available",
            "PATH",
            Some(proton_path),
            false,
            None,
        ));
    }

    if let Some(umu_path) = find_in_path("umu-run") {
        runners.push(runner_info(
            "system-umu-run",
            "proton",
            "UMU Launcher / umu-run",
            "available",
            "PATH",
            Some(umu_path),
            false,
            None,
        ));
    }

    runners.extend(discover_steam_proton_runners());
    runners.extend(discover_managed_runners(&app)?);

    let has_wine = runners
        .iter()
        .any(|runner| runner.kind == "wine" && runner.status == "available");
    let has_proton = runners
        .iter()
        .any(|runner| runner.kind == "proton" && runner.status == "available");

    if !has_wine {
        runners.push(runner_info(
            "managed-wine-installable",
            "wine",
            "Wine gerenciado pelo launcher",
            "installable",
            "Launcher",
            None,
            true,
            Some("Opção planejada para baixar/registrar um Wine isolado quando o sistema não tiver Wine disponível."),
        ));
    }

    if !has_proton {
        runners.push(runner_info(
            "managed-proton-ge-installable",
            "proton",
            "Proton-GE gerenciado pelo launcher",
            "installable",
            "Launcher",
            None,
            true,
            Some("Opção planejada para baixar/registrar Proton-GE em uma pasta controlada pelo launcher."),
        ));
    }

    Ok(runners)
}

pub(crate) fn resolve_runner(
    app: &tauri::AppHandle,
    requested_runner: &str,
) -> Result<ResolvedRunner, String> {
    let normalized_runner = requested_runner.trim().to_lowercase();

    if normalized_runner.is_empty() {
        return Err("Runner solicitado não pode ser vazio.".to_string());
    }

    let runners = discover_runners(app)?;

    if normalized_runner == "proton" {
        if let Some(runner) = runners
            .iter()
            .find(|runner| runner.id == "system-umu-run" && runner.status == "available")
        {
            return Ok(ResolvedRunner {
                id: runner.id.clone(),
                kind: runner.kind.clone(),
                label: runner.label.clone(),
                source: runner.source.clone(),
                path: runner.path.clone(),
            });
        }
    }

    if let Some(runner) = runners
        .iter()
        .find(|runner| runner.kind == normalized_runner && runner.status == "available")
    {
        return Ok(ResolvedRunner {
            id: runner.id.clone(),
            kind: runner.kind.clone(),
            label: runner.label.clone(),
            source: runner.source.clone(),
            path: runner.path.clone(),
        });
    }

    if let Some(runner) = runners
        .iter()
        .find(|runner| runner.kind == normalized_runner && runner.installable)
    {
        let hint = runner
            .install_hint
            .as_deref()
            .unwrap_or("Instalação de runner gerenciado ainda será implementada.");

        return Err(format!(
            "Runner '{}' não está disponível no sistema. Opção preparada: {}. {}",
            normalized_runner, runner.label, hint
        ));
    }

    Err(format!(
        "Nenhum runner compatível com '{}' foi encontrado. Verifique o manifesto ou configure um runner suportado.",
        normalized_runner
    ))
}

pub(crate) fn build_runner_command(
    app: &tauri::AppHandle,
    game_id: &str,
    runner: &ResolvedRunner,
    executable_path: &Path,
    install_path: &Path,
    launch_args: &[String],
    compat_prefix_kind: Option<&str>,
) -> Result<RunnerCommand, String> {
    match runner.kind.as_str() {
        "native" => Ok(RunnerCommand {
            runner_kind: runner.kind.clone(),
            program: executable_path.to_path_buf(),
            args: launch_args.to_vec(),
            working_dir: install_path.to_path_buf(),
            envs: Vec::new(),
        }),
        "wine" => {
            let runner_path = runner.path.as_ref().ok_or_else(|| {
                format!(
                    "Runner Wine '{}' foi resolvido sem caminho executável.",
                    runner.label
                )
            })?;
            let prefix_dir =
                managed_windows_prefix_dir(app, game_id, compat_prefix_kind.unwrap_or("wine"))?;
            let mut args = vec![executable_path.to_string_lossy().to_string()];

            args.extend_from_slice(launch_args);

            Ok(RunnerCommand {
                runner_kind: runner.kind.clone(),
                program: PathBuf::from(runner_path),
                args,
                working_dir: install_path.to_path_buf(),
                envs: vec![("WINEPREFIX".to_string(), path_to_string(&prefix_dir))],
            })
        }
        "proton" => {
            let runner_path = runner.path.as_ref().ok_or_else(|| {
                format!(
                    "Runner Proton '{}' foi resolvido sem caminho executável.",
                    runner.label
                )
            })?;
            let prefix_dir = managed_prefix_dir(app, game_id, "proton")?;
            let logs_dir = managed_logs_dir(app, game_id)?;
            let steam_app_id = synthetic_steam_app_id(game_id);
            let executable_arg = if runner.id == "system-umu-run" {
                executable_path.to_string_lossy().to_string()
            } else {
                path_to_proton_windows_path(executable_path)
            };
            let mut args = if runner.id == "system-umu-run" {
                vec![executable_arg]
            } else {
                vec!["waitforexitandrun".to_string(), executable_arg]
            };
            let mut envs = vec![
                (
                    "STEAM_COMPAT_DATA_PATH".to_string(),
                    path_to_string(&prefix_dir),
                ),
                ("STEAM_COMPAT_APP_ID".to_string(), steam_app_id.clone()),
                ("SteamAppId".to_string(), steam_app_id.clone()),
                ("SteamGameId".to_string(), steam_app_id.clone()),
                ("PROTON_LOG".to_string(), "1".to_string()),
                ("PROTON_LOG_DIR".to_string(), path_to_string(&logs_dir)),
            ];

            if runner.id == "system-umu-run" {
                envs.extend([
                    ("GAMEID".to_string(), sanitize_path_segment(game_id)),
                    ("STORE".to_string(), "none".to_string()),
                    (
                        "WINEPREFIX".to_string(),
                        path_to_string(&prefix_dir.join("pfx")),
                    ),
                ]);
            }

            args.extend_from_slice(launch_args);

            if let Some(steam_dir) = steam_client_install_path() {
                envs.push((
                    "STEAM_COMPAT_CLIENT_INSTALL_PATH".to_string(),
                    path_to_string(&steam_dir),
                ));
            }

            Ok(RunnerCommand {
                runner_kind: runner.kind.clone(),
                program: PathBuf::from(runner_path),
                args,
                working_dir: install_path.to_path_buf(),
                envs,
            })
        }
        unsupported_runner => Err(format!(
            "Runner '{}' foi resolvido, mas ainda não possui montagem de comando implementada.",
            unsupported_runner
        )),
    }
}

#[tauri::command]
pub(crate) fn list_runners(app: tauri::AppHandle) -> Result<Vec<RunnerInfo>, String> {
    discover_runners(&app)
}
