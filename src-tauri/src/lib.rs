use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LaunchConfig {
    runner: String,
    executable: Option<String>,
    args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateConfig {
    strategy: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunnerInfo {
    id: String,
    kind: String,
    label: String,
    status: String,
    source: String,
    path: Option<String>,
    installable: bool,
    install_hint: Option<String>,
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
fn list_runners(app: tauri::AppHandle) -> Result<Vec<RunnerInfo>, String> {
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

    connection
        .execute(
            "
            INSERT INTO installs (game_id, install_path, runner_override)
            VALUES (?1, ?2, NULL)
            ON CONFLICT(game_id) DO UPDATE SET
                install_path = excluded.install_path,
                updated_at = CURRENT_TIMESTAMP
            ",
            params![game_id, install_path],
        )
        .map_err(|error| format!("Não foi possível salvar a instalação localizada: {error}"))?;

    Ok(Some(get_install(&connection, &game_id)?))
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
fn launch_game(app: tauri::AppHandle, game_id: String) -> Result<LaunchResult, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let connection = open_database(&app)?;
    let install = get_install(&connection, &game_id)?;
    let manifest = get_manifest(&game_id)?;
    let runner = install
        .runner_override
        .clone()
        .unwrap_or_else(|| manifest.launch.runner.clone());

    if runner != "native" {
        return Err(format!(
            "O runner '{runner}' ainda não possui execução implementada. Próxima etapa: camada Wine/Proton/runners."
        ));
    }

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

    let executable_path = PathBuf::from(executable);
    let command_path = if executable_path.is_absolute() {
        executable_path
    } else {
        install_path.join(executable_path)
    };

    if !command_path.exists() {
        return Err(format!(
            "Executável não encontrado para {}: {}",
            manifest.name,
            command_path.display()
        ));
    }

    Command::new(&command_path)
        .args(&manifest.launch.args)
        .current_dir(&install_path)
        .spawn()
        .map_err(|error| {
            format!(
                "Não foi possível iniciar {} usando {}: {error}",
                manifest.name,
                command_path.display()
            )
        })?;

    Ok(LaunchResult {
        game_id,
        runner,
        command: command_path.to_string_lossy().to_string(),
        working_dir: install_path.to_string_lossy().to_string(),
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
            open_install_folder,
            remove_install,
            launch_game
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o 2D MMO Launcher");
}
