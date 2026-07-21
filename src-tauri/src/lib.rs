use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

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

fn manifests_dir() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir().map_err(|error| error.to_string())?;

    if current_dir.join("manifests").is_dir() {
        return Ok(current_dir.join("manifests"));
    }

    Ok(current_dir.join("src-tauri").join("manifests"))
}

fn database_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Não foi possível resolver o diretório de dados do app: {error}"))?;

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

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, list_games, list_installs])
        .run(tauri::generate_context!())
        .expect("erro ao executar o 2D MMO Launcher");
}
