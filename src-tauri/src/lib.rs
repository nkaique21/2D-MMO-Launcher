use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::Manager;

mod runners;

use runners::{build_runner_command, list_runners, resolve_runner};

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

fn attach_process_logs(
    app: &tauri::AppHandle,
    game_id: &str,
    command: &mut Command,
) -> Result<PathBuf, String> {
    let log_path = logs_dir(app, game_id)?.join("runner.log");
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
fn download_and_run_installer(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<LaunchResult, String> {
    let game_id = game_id.trim().to_string();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio.".to_string());
    }

    let manifest = get_manifest(&game_id)?;
    let installer = manifest
        .installation
        .methods
        .iter()
        .find(|method| method.kind == "windowsInstaller")
        .ok_or_else(|| {
            format!(
                "{} não possui método windowsInstaller no manifesto.",
                manifest.name
            )
        })?;
    let installer_url = installer.url.as_ref().ok_or_else(|| {
        format!(
            "O método windowsInstaller de {} não define uma URL de download.",
            manifest.name
        )
    })?;
    let downloads_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Não foi possível resolver o diretório de dados do app: {error}"))?
        .join("downloads")
        .join(sanitize_path_segment(&game_id));
    let installer_path = downloads_dir.join(filename_from_url(installer_url));

    download_file(installer_url, &installer_path)?;

    let resolved_runner = resolve_runner(&app, &manifest.launch.runner)?;
    let runner_command = build_runner_command(
        &app,
        &game_id,
        &resolved_runner,
        &installer_path,
        &downloads_dir,
        &[],
    )?;
    let mut command = Command::new(&runner_command.program);

    command
        .args(&runner_command.args)
        .current_dir(&runner_command.working_dir)
        .envs(runner_command.envs.iter().map(|(key, value)| (key, value)));

    let log_path = attach_process_logs(&app, &game_id, &mut command)?;

    command.spawn().map_err(|error| {
        format!(
            "Não foi possível iniciar o instalador de {} usando {}: {error}. Log: {}",
            manifest.name,
            runner_command.program.display(),
            log_path.display()
        )
    })?;

    Ok(LaunchResult {
        game_id,
        runner: runner_command.runner_kind,
        command: runner_command.program.to_string_lossy().to_string(),
        working_dir: runner_command.working_dir.to_string_lossy().to_string(),
    })
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
    let requested_runner = install
        .runner_override
        .clone()
        .unwrap_or_else(|| manifest.launch.runner.clone());
    let resolved_runner = resolve_runner(&app, &requested_runner)?;

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

    let runner_command = build_runner_command(
        &app,
        &game_id,
        &resolved_runner,
        &command_path,
        &install_path,
        &manifest.launch.args,
    )?;

    let mut command = Command::new(&runner_command.program);

    command
        .args(&runner_command.args)
        .current_dir(&runner_command.working_dir)
        .envs(runner_command.envs.iter().map(|(key, value)| (key, value)));

    let log_path = attach_process_logs(&app, &game_id, &mut command)?;

    command.spawn().map_err(|error| {
        format!(
            "Não foi possível iniciar {} usando {}: {error}. Log: {}",
            manifest.name,
            runner_command.program.display(),
            log_path.display()
        )
    })?;

    Ok(LaunchResult {
        game_id,
        runner: runner_command.runner_kind,
        command: runner_command.program.to_string_lossy().to_string(),
        working_dir: runner_command.working_dir.to_string_lossy().to_string(),
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
