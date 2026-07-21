use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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

fn manifests_dir() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir().map_err(|error| error.to_string())?;

    if current_dir.join("manifests").is_dir() {
        return Ok(current_dir.join("manifests"));
    }

    Ok(current_dir.join("src-tauri").join("manifests"))
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

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, list_games])
        .run(tauri::generate_context!())
        .expect("erro ao executar o 2D MMO Launcher");
}
