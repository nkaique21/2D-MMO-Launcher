use super::GameManifest;
use reqwest::blocking::Client;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

pub(crate) const DEFAULT_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/nkaique21/2D-MMO-Launcher-Catalog/main/catalog.json";
const SUPPORTED_CATALOG_SCHEMA_VERSION: u32 = 1;
const MAX_CATALOG_BYTES: usize = 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 512 * 1024;
const MAX_CATALOG_GAMES: usize = 500;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogIndex {
    schema_version: u32,
    catalog_version: String,
    generated_at: String,
    games: Vec<CatalogGameEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogGameEntry {
    id: String,
    manifest: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogMetadata {
    remote_url: String,
    catalog_version: Option<String>,
    generated_at: Option<String>,
    last_checked_at: Option<i64>,
    last_updated_at: Option<i64>,
    last_error: Option<String>,
    game_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogStatus {
    active_source: String,
    remote_url: String,
    catalog_version: Option<String>,
    generated_at: Option<String>,
    last_checked_at: Option<i64>,
    last_updated_at: Option<i64>,
    last_error: Option<String>,
    game_count: usize,
}

fn default_true() -> bool {
    true
}

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or(0)
}

fn catalog_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("catalog"))
        .map_err(|error| format!("Não foi possível resolver o cache do catálogo: {error}"))
}

fn metadata_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(catalog_root(app)?.join("metadata.json"))
}

fn active_catalog_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(catalog_root(app)?.join("official"))
}

fn staging_catalog_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(catalog_root(app)?.join("staging"))
}

fn backup_catalog_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(catalog_root(app)?.join("backup"))
}

fn read_metadata(app: &tauri::AppHandle) -> CatalogMetadata {
    let Ok(path) = metadata_path(app) else {
        return CatalogMetadata {
            remote_url: DEFAULT_CATALOG_URL.to_string(),
            ..CatalogMetadata::default()
        };
    };

    let Ok(content) = fs::read_to_string(path) else {
        return CatalogMetadata {
            remote_url: DEFAULT_CATALOG_URL.to_string(),
            ..CatalogMetadata::default()
        };
    };

    serde_json::from_str(&content).unwrap_or_else(|_| CatalogMetadata {
        remote_url: DEFAULT_CATALOG_URL.to_string(),
        ..CatalogMetadata::default()
    })
}

fn write_metadata(app: &tauri::AppHandle, metadata: &CatalogMetadata) -> Result<(), String> {
    let path = metadata_path(app)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Não foi possível criar o diretório de metadata do catálogo em {}: {error}",
                parent.display()
            )
        })?;
    }

    let content = serde_json::to_string_pretty(metadata)
        .map_err(|error| format!("Não foi possível serializar metadata do catálogo: {error}"))?;
    fs::write(&path, content).map_err(|error| {
        format!(
            "Não foi possível salvar metadata do catálogo em {}: {error}",
            path.display()
        )
    })
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || character == '-'
                    || character == '_'
            })
}

fn is_safe_relative_path(value: &str) -> bool {
    let trimmed = value.trim();

    if trimmed.is_empty()
        || trimmed.contains('\0')
        || trimmed.contains(':')
        || trimmed.contains('\\')
    {
        return false;
    }

    let path = Path::new(trimmed);

    if path.is_absolute() {
        return false;
    }

    path.components().all(|component| {
        matches!(component, Component::Normal(_) | Component::CurDir)
    })
}

fn validate_https_url(value: &str, field: &str) -> Result<(), String> {
    let url = Url::parse(value)
        .map_err(|error| format!("URL inválida em {field}: {value} ({error})"))?;

    if url.scheme() != "https" {
        return Err(format!("A URL de {field} precisa usar HTTPS: {value}"));
    }

    Ok(())
}

fn validate_optional_relative_path(value: Option<&str>, field: &str) -> Result<(), String> {
    if let Some(value) = value {
        if !is_safe_relative_path(value) {
            return Err(format!("Caminho remoto inseguro em {field}: {value}"));
        }
    }

    Ok(())
}

fn validate_remote_manifest(manifest: &GameManifest, expected_id: &str) -> Result<(), String> {
    if manifest.schema_version != SUPPORTED_CATALOG_SCHEMA_VERSION {
        return Err(format!(
            "Versão de schema do manifesto {} não suportada: {}.",
            expected_id, manifest.schema_version
        ));
    }

    if manifest.id != expected_id {
        return Err(format!(
            "O manifesto remoto de {expected_id} declarou id diferente: {}",
            manifest.id
        ));
    }

    if !is_safe_id(&manifest.id) {
        return Err(format!("ID de jogo inválido no catálogo remoto: {}", manifest.id));
    }

    if manifest.name.trim().is_empty() {
        return Err(format!("O manifesto {} não possui nome.", manifest.id));
    }

    if manifest.launch.runner.trim().is_empty() {
        return Err(format!("O manifesto {} não define launch.runner.", manifest.id));
    }

    validate_optional_relative_path(manifest.launch.executable.as_deref(), "launch.executable")?;

    if let Some(battl_eye) = manifest.launch.battl_eye.as_ref() {
        validate_optional_relative_path(
            Some(&battl_eye.executable),
            "launch.battlEye.executable",
        )?;
        validate_optional_relative_path(
            battl_eye.working_dir.as_deref(),
            "launch.battlEye.workingDir",
        )?;
    }

    for method in &manifest.installation.methods {
        if let Some(url) = method.url.as_deref() {
            validate_https_url(url, "installation.methods[].url")?;
        }

        validate_optional_relative_path(
            method.install_path.as_deref(),
            "installation.methods[].installPath",
        )?;
    }

    if let Some(url) = manifest.update.manifest_url.as_deref() {
        validate_https_url(url, "update.manifestUrl")?;
    }

    validate_optional_relative_path(
        manifest.update.executable.as_deref(),
        "update.executable",
    )?;
    validate_optional_relative_path(
        manifest.update.working_dir.as_deref(),
        "update.workingDir",
    )?;
    validate_optional_relative_path(
        manifest.update.target_dir.as_deref(),
        "update.targetDir",
    )?;

    for required_file in &manifest.verification.required_files {
        if !is_safe_relative_path(required_file) {
            return Err(format!(
                "Arquivo obrigatório inseguro no manifesto {}: {required_file}",
                manifest.id
            ));
        }
    }

    for checksum in &manifest.verification.checksums {
        if !is_safe_relative_path(&checksum.path) {
            return Err(format!(
                "Caminho de checksum inseguro no manifesto {}: {}",
                manifest.id, checksum.path
            ));
        }
    }

    Ok(())
}

fn validate_catalog_index(index: &CatalogIndex) -> Result<(), String> {
    if index.schema_version != SUPPORTED_CATALOG_SCHEMA_VERSION {
        return Err(format!(
            "Versão de schema do catálogo não suportada: {}. O launcher suporta {}.",
            index.schema_version, SUPPORTED_CATALOG_SCHEMA_VERSION
        ));
    }

    if index.catalog_version.trim().is_empty() {
        return Err("O catálogo remoto não define catalogVersion.".to_string());
    }

    if index.games.len() > MAX_CATALOG_GAMES {
        return Err(format!(
            "O catálogo remoto excede o limite de {MAX_CATALOG_GAMES} jogos."
        ));
    }

    let mut ids = HashSet::new();

    for game in &index.games {
        if !is_safe_id(&game.id) {
            return Err(format!("ID inválido no catálogo remoto: {}", game.id));
        }

        if !ids.insert(game.id.clone()) {
            return Err(format!("ID duplicado no catálogo remoto: {}", game.id));
        }

        if game.enabled && !is_safe_relative_path(&game.manifest) {
            return Err(format!(
                "Caminho de manifesto inseguro para {}: {}",
                game.id, game.manifest
            ));
        }
    }

    Ok(())
}

fn resolve_remote_asset_url(catalog_url: &Url, value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(format!("Asset vazio em {field}."));
    }

    if let Ok(url) = Url::parse(trimmed) {
        if url.scheme() != "https" {
            return Err(format!("Asset remoto precisa usar HTTPS em {field}: {trimmed}"));
        }

        return Ok(url.to_string());
    }

    let relative = trimmed.trim_start_matches('/');

    if !is_safe_relative_path(relative) {
        return Err(format!("Caminho de asset inseguro em {field}: {trimmed}"));
    }

    catalog_url
        .join(relative)
        .map(|url| url.to_string())
        .map_err(|error| format!("Não foi possível resolver asset {trimmed}: {error}"))
}

fn normalize_remote_assets(
    manifest: &mut GameManifest,
    catalog_url: &Url,
) -> Result<(), String> {
    manifest.assets.banner =
        resolve_remote_asset_url(catalog_url, &manifest.assets.banner, "assets.banner")?;
    manifest.assets.icon =
        resolve_remote_asset_url(catalog_url, &manifest.assets.icon, "assets.icon")?;

    Ok(())
}

fn response_bytes_with_limit(
    client: &Client,
    url: Url,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let response = client
        .get(url.clone())
        .send()
        .map_err(|error| format!("Não foi possível baixar {label} em {url}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("O servidor recusou {label} em {url}: {error}"))?;

    if response
        .content_length()
        .is_some_and(|content_length| content_length > max_bytes as u64)
    {
        return Err(format!("{label} excede o limite de {max_bytes} bytes."));
    }

    let bytes = response
        .bytes()
        .map_err(|error| format!("Não foi possível ler {label} em {url}: {error}"))?;

    if bytes.len() > max_bytes {
        return Err(format!("{label} excede o limite de {max_bytes} bytes."));
    }

    Ok(bytes.to_vec())
}

fn parse_manifest_file(path: &Path) -> Result<GameManifest, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Não foi possível ler {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("Manifesto inválido em {}: {error}", path.display()))
}

fn load_manifests_from_dir(dir: &Path) -> Result<Vec<GameManifest>, String> {
    let entries = fs::read_dir(dir)
        .map_err(|error| format!("Não foi possível listar {}: {error}", dir.display()))?;
    let mut games = Vec::new();
    let mut ids = HashSet::new();

    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        let manifest = parse_manifest_file(&path)?;

        if !ids.insert(manifest.id.clone()) {
            return Err(format!(
                "Manifesto duplicado para {} em {}.",
                manifest.id,
                dir.display()
            ));
        }

        games.push(manifest);
    }

    if games.is_empty() {
        return Err(format!("Nenhum manifesto encontrado em {}.", dir.display()));
    }

    games.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(games)
}

fn embedded_manifest_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("manifests"));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("manifests"));
        candidates.push(current_dir.join("src-tauri").join("manifests"));
    }

    candidates
}

fn load_embedded_games(app: &tauri::AppHandle) -> Result<Vec<GameManifest>, String> {
    let mut errors = Vec::new();

    for candidate in embedded_manifest_candidates(app) {
        if !candidate.is_dir() {
            continue;
        }

        match load_manifests_from_dir(&candidate) {
            Ok(games) => return Ok(games),
            Err(error) => errors.push(error),
        }
    }

    Err(if errors.is_empty() {
        "Nenhum diretório de manifestos embutidos foi encontrado.".to_string()
    } else {
        errors.join(" ")
    })
}

fn load_cached_games(app: &tauri::AppHandle) -> Result<Vec<GameManifest>, String> {
    let active_dir = active_catalog_dir(app)?;
    let index_path = active_dir.join("catalog.json");
    let index_content = fs::read_to_string(&index_path).map_err(|error| {
        format!(
            "Não foi possível ler o catálogo remoto em cache {}: {error}",
            index_path.display()
        )
    })?;
    let index: CatalogIndex = serde_json::from_str(&index_content).map_err(|error| {
        format!(
            "Catálogo remoto em cache inválido em {}: {error}",
            index_path.display()
        )
    })?;
    validate_catalog_index(&index)?;

    let mut games = Vec::new();

    for entry in index.games.iter().filter(|entry| entry.enabled) {
        let manifest_path = active_dir
            .join("manifests")
            .join(format!("{}.json", entry.id));
        let manifest = parse_manifest_file(&manifest_path)?;
        validate_remote_manifest(&manifest, &entry.id)?;
        games.push(manifest);
    }

    if games.is_empty() {
        return Err("O catálogo remoto em cache não possui jogos habilitados.".to_string());
    }

    games.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(games)
}

pub(crate) fn load_active_games(app: &tauri::AppHandle) -> Result<Vec<GameManifest>, String> {
    match load_cached_games(app) {
        Ok(games) => Ok(games),
        Err(cache_error) => load_embedded_games(app).map_err(|embedded_error| {
            format!(
                "Não foi possível carregar catálogo remoto nem fallback embutido. Cache: {cache_error} Fallback: {embedded_error}"
            )
        }),
    }
}

pub(crate) fn get_status(app: &tauri::AppHandle) -> Result<CatalogStatus, String> {
    let metadata = read_metadata(app);
    let cached_games = load_cached_games(app);

    let (active_source, game_count) = match cached_games {
        Ok(games) => ("remote-cache".to_string(), games.len()),
        Err(_) => ("embedded".to_string(), load_embedded_games(app)?.len()),
    };

    Ok(CatalogStatus {
        active_source,
        remote_url: if metadata.remote_url.is_empty() {
            DEFAULT_CATALOG_URL.to_string()
        } else {
            metadata.remote_url
        },
        catalog_version: metadata.catalog_version,
        generated_at: metadata.generated_at,
        last_checked_at: metadata.last_checked_at,
        last_updated_at: metadata.last_updated_at,
        last_error: metadata.last_error,
        game_count,
    })
}

fn activate_staging(app: &tauri::AppHandle) -> Result<(), String> {
    let active_dir = active_catalog_dir(app)?;
    let staging_dir = staging_catalog_dir(app)?;
    let backup_dir = backup_catalog_dir(app)?;

    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir).map_err(|error| {
            format!(
                "Não foi possível remover backup antigo do catálogo em {}: {error}",
                backup_dir.display()
            )
        })?;
    }

    if active_dir.exists() {
        fs::rename(&active_dir, &backup_dir).map_err(|error| {
            format!(
                "Não foi possível preparar troca do catálogo ativo em {}: {error}",
                active_dir.display()
            )
        })?;
    }

    if let Err(error) = fs::rename(&staging_dir, &active_dir) {
        if backup_dir.exists() {
            let _ = fs::rename(&backup_dir, &active_dir);
        }

        return Err(format!(
            "Não foi possível ativar o novo catálogo em {}: {error}",
            active_dir.display()
        ));
    }

    if backup_dir.exists() {
        let _ = fs::remove_dir_all(backup_dir);
    }

    Ok(())
}

pub(crate) fn refresh_remote_catalog(app: &tauri::AppHandle) -> Result<CatalogStatus, String> {
    let checked_at = now_timestamp();
    let remote_url = DEFAULT_CATALOG_URL.to_string();
    let result = refresh_remote_catalog_inner(app, &remote_url, checked_at);

    if let Err(error) = result.as_ref() {
        let mut metadata = read_metadata(app);
        metadata.remote_url = remote_url;
        metadata.last_checked_at = Some(checked_at);
        metadata.last_error = Some(error.clone());
        let _ = write_metadata(app, &metadata);
    }

    result
}

fn refresh_remote_catalog_inner(
    app: &tauri::AppHandle,
    remote_url: &str,
    checked_at: i64,
) -> Result<CatalogStatus, String> {
    validate_https_url(remote_url, "catalogUrl")?;
    let catalog_url = Url::parse(remote_url)
        .map_err(|error| format!("URL do catálogo oficial inválida: {error}"))?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("2D-MMO-Launcher/0.1")
        .build()
        .map_err(|error| format!("Não foi possível criar cliente HTTP do catálogo: {error}"))?;
    let catalog_bytes =
        response_bytes_with_limit(&client, catalog_url.clone(), MAX_CATALOG_BYTES, "catálogo")?;
    let index: CatalogIndex = serde_json::from_slice(&catalog_bytes)
        .map_err(|error| format!("Catálogo remoto inválido: {error}"))?;
    validate_catalog_index(&index)?;

    let root = catalog_root(app)?;
    let staging_dir = staging_catalog_dir(app)?;

    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "Não foi possível criar diretório do catálogo em {}: {error}",
            root.display()
        )
    })?;

    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir).map_err(|error| {
            format!(
                "Não foi possível limpar staging do catálogo em {}: {error}",
                staging_dir.display()
            )
        })?;
    }

    let staging_manifests = staging_dir.join("manifests");
    fs::create_dir_all(&staging_manifests).map_err(|error| {
        format!(
            "Não foi possível criar staging de manifestos em {}: {error}",
            staging_manifests.display()
        )
    })?;

    fs::write(staging_dir.join("catalog.json"), &catalog_bytes)
        .map_err(|error| format!("Não foi possível salvar catálogo em staging: {error}"))?;

    let mut enabled_count = 0usize;

    for entry in index.games.iter().filter(|entry| entry.enabled) {
        let manifest_url = catalog_url.join(&entry.manifest).map_err(|error| {
            format!(
                "Não foi possível resolver manifesto de {} em {}: {error}",
                entry.id, entry.manifest
            )
        })?;

        if manifest_url.scheme() != "https" {
            return Err(format!(
                "Manifesto remoto de {} precisa usar HTTPS: {manifest_url}",
                entry.id
            ));
        }

        let manifest_bytes = response_bytes_with_limit(
            &client,
            manifest_url,
            MAX_MANIFEST_BYTES,
            &format!("manifesto de {}", entry.id),
        )?;
        let mut manifest: GameManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| format!("Manifesto remoto de {} inválido: {error}", entry.id))?;
        validate_remote_manifest(&manifest, &entry.id)?;
        normalize_remote_assets(&mut manifest, &catalog_url)?;
        let serialized = serde_json::to_vec_pretty(&manifest).map_err(|error| {
            format!("Não foi possível serializar manifesto de {}: {error}", entry.id)
        })?;
        fs::write(
            staging_manifests.join(format!("{}.json", entry.id)),
            serialized,
        )
        .map_err(|error| format!("Não foi possível salvar manifesto de {}: {error}", entry.id))?;
        enabled_count += 1;
    }

    if enabled_count == 0 {
        return Err("O catálogo remoto não possui jogos habilitados.".to_string());
    }

    let _ = load_manifests_from_dir(&staging_manifests)?;
    activate_staging(app)?;

    let metadata = CatalogMetadata {
        remote_url: remote_url.to_string(),
        catalog_version: Some(index.catalog_version.clone()),
        generated_at: Some(index.generated_at.clone()),
        last_checked_at: Some(checked_at),
        last_updated_at: Some(now_timestamp()),
        last_error: None,
        game_count: enabled_count,
    };
    write_metadata(app, &metadata)?;

    get_status(app)
}

pub(crate) fn spawn_background_refresh(app: tauri::AppHandle) {
    std::thread::spawn(move || match refresh_remote_catalog(&app) {
        Ok(status) => {
            let _ = app.emit("catalog-updated", status);
        }
        Err(error) => {
            let _ = app.emit("catalog-update-failed", error);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        is_safe_id, is_safe_relative_path, validate_catalog_index, CatalogGameEntry,
        CatalogIndex,
    };

    #[test]
    fn catalog_rejects_duplicate_ids() {
        let index = CatalogIndex {
            schema_version: 1,
            catalog_version: "0.1.0".to_string(),
            generated_at: "2026-07-23T00:00:00Z".to_string(),
            games: vec![
                CatalogGameEntry {
                    id: "medivia".to_string(),
                    manifest: "manifests/medivia.json".to_string(),
                    enabled: true,
                },
                CatalogGameEntry {
                    id: "medivia".to_string(),
                    manifest: "manifests/other.json".to_string(),
                    enabled: true,
                },
            ],
        };

        assert!(validate_catalog_index(&index).is_err());
    }

    #[test]
    fn catalog_paths_reject_traversal_and_absolute_paths() {
        assert!(is_safe_relative_path("manifests/medivia.json"));
        assert!(!is_safe_relative_path("../medivia.json"));
        assert!(!is_safe_relative_path("/tmp/medivia.json"));
        assert!(!is_safe_relative_path("C:\\medivia.json"));
    }

    #[test]
    fn catalog_ids_accept_only_stable_safe_characters() {
        assert!(is_safe_id("grand-line-adventures"));
        assert!(is_safe_id("game_2"));
        assert!(!is_safe_id("Grand Line Adventures"));
        assert!(!is_safe_id("../game"));
    }
}
