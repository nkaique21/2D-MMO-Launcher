use crate::database::{self, ManagedRunner};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases/latest";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManagedRunnerRelease {
    version: String,
    name: String,
    download_url: String,
    size: u64,
    release_url: String,
    installed: bool,
    runner_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunnerInstallProgress {
    status: String,
    stage: String,
    version: String,
    downloaded_bytes: u64,
    total_bytes: u64,
    message: String,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    html_url: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("2D-MMO-Launcher")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| format!("Não foi possível preparar o cliente HTTP: {error}"))
}

fn safe_segment(value: &str) -> String {
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
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn runner_id(version: &str) -> String {
    format!("managed-proton-ge-{}", safe_segment(version).to_lowercase())
}

fn runners_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("runners"))
        .map_err(|error| format!("Não foi possível resolver a pasta de runners: {error}"))
}

fn emit_progress(
    app: &tauri::AppHandle,
    status: &str,
    stage: &str,
    version: &str,
    downloaded_bytes: u64,
    total_bytes: u64,
    message: impl Into<String>,
    error: Option<String>,
) {
    let _ = app.emit(
        "runner-install-progress",
        RunnerInstallProgress {
            status: status.to_string(),
            stage: stage.to_string(),
            version: version.to_string(),
            downloaded_bytes,
            total_bytes,
            message: message.into(),
            error,
        },
    );
}

fn fetch_latest_release(app: &tauri::AppHandle) -> Result<ManagedRunnerRelease, String> {
    let response = http_client()?
        .get(LATEST_RELEASE_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("Não foi possível consultar o Proton-GE mais recente: {error}"))?;
    let response_text = response
        .text()
        .map_err(|error| format!("Não foi possível ler o catálogo Proton-GE: {error}"))?;
    let release: GithubRelease = serde_json::from_str(&response_text)
        .map_err(|error| format!("Resposta inválida do catálogo Proton-GE: {error}"))?;
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.ends_with(".tar.gz"))
        .ok_or_else(|| {
            "A release mais recente do Proton-GE não possui pacote .tar.gz.".to_string()
        })?;
    let id = runner_id(&release.tag_name);
    let installed = database::get_managed_runner(&database::open(app)?, &id)?
        .map(|runner| Path::new(&runner.executable_path).is_file())
        .unwrap_or(false);

    Ok(ManagedRunnerRelease {
        version: release.tag_name.clone(),
        name: release.name.unwrap_or_else(|| release.tag_name.clone()),
        download_url: asset.browser_download_url,
        size: asset.size,
        release_url: release.html_url,
        installed,
        runner_id: id,
    })
}

#[tauri::command]
pub(crate) async fn get_latest_proton_ge_release(
    app: tauri::AppHandle,
) -> Result<ManagedRunnerRelease, String> {
    tauri::async_runtime::spawn_blocking(move || fetch_latest_release(&app))
        .await
        .map_err(|error| format!("A consulta do catálogo Proton-GE foi interrompida: {error}"))?
}

fn validate_archive_path(path: &Path) -> Result<(), String> {
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "O pacote contém um caminho inseguro: {}",
            path.display()
        ));
    }

    Ok(())
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let archive_file = File::open(archive_path)
        .map_err(|error| format!("Não foi possível abrir o pacote baixado: {error}"))?;
    let mut archive = tar::Archive::new(GzDecoder::new(archive_file));
    let entries = archive
        .entries()
        .map_err(|error| format!("Não foi possível ler o pacote Proton-GE: {error}"))?;

    for entry in entries {
        let mut entry = entry.map_err(|error| format!("Entrada inválida no pacote: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("Caminho inválido no pacote: {error}"))?
            .into_owned();
        validate_archive_path(&path)?;
        let unpacked = entry
            .unpack_in(destination)
            .map_err(|error| format!("Não foi possível extrair {}: {error}", path.display()))?;
        if !unpacked {
            return Err(format!("A extração recusou o caminho {}.", path.display()));
        }
    }

    Ok(())
}

fn find_proton_root(directory: &Path, depth: usize) -> Option<PathBuf> {
    if directory.join("proton").is_file() {
        return Some(directory.to_path_buf());
    }
    if depth == 0 {
        return None;
    }

    fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .find_map(|path| find_proton_root(&path, depth - 1))
}

fn download_release(
    app: &tauri::AppHandle,
    release: &ManagedRunnerRelease,
    archive_path: &Path,
) -> Result<u64, String> {
    let mut response = http_client()?
        .get(&release.download_url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("Não foi possível baixar {}: {error}", release.version))?;
    let total_bytes = response.content_length().unwrap_or(release.size);
    let mut output = File::create(archive_path)
        .map_err(|error| format!("Não foi possível criar o arquivo temporário: {error}"))?;
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut downloaded_bytes = 0_u64;

    loop {
        let bytes = response
            .read(&mut buffer)
            .map_err(|error| format!("Falha durante o download do Proton-GE: {error}"))?;
        if bytes == 0 {
            break;
        }
        output
            .write_all(&buffer[..bytes])
            .map_err(|error| format!("Não foi possível gravar o pacote Proton-GE: {error}"))?;
        downloaded_bytes += bytes as u64;
        emit_progress(
            app,
            "downloading",
            "download",
            &release.version,
            downloaded_bytes,
            total_bytes,
            format!("Baixando {}...", release.version),
            None,
        );
    }

    if release.size > 0 && downloaded_bytes != release.size {
        return Err(format!(
            "Download incompleto: esperado {} bytes, recebido {downloaded_bytes}.",
            release.size
        ));
    }

    Ok(downloaded_bytes)
}

fn install_release(app: tauri::AppHandle) -> Result<ManagedRunner, String> {
    emit_progress(
        &app,
        "preparing",
        "catalog",
        "",
        0,
        0,
        "Consultando release mais recente...",
        None,
    );
    let release = fetch_latest_release(&app)?;
    let root = runners_root(&app)?;
    let proton_ge_root = root.join("proton-ge");
    let version_segment = safe_segment(&release.version);
    if version_segment.is_empty() {
        return Err("A versão retornada pelo GitHub não gera um diretório seguro.".to_string());
    }
    let final_dir = proton_ge_root.join(&version_segment);
    let final_executable = final_dir.join("proton");

    if final_executable.is_file() {
        let runner = ManagedRunner {
            id: release.runner_id,
            kind: "proton".to_string(),
            version: release.version.clone(),
            label: release.version.clone(),
            source: "Launcher".to_string(),
            install_path: final_dir.to_string_lossy().to_string(),
            executable_path: final_executable.to_string_lossy().to_string(),
            status: "available".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        return database::save_managed_runner(&database::open(&app)?, &runner);
    }

    fs::create_dir_all(&proton_ge_root)
        .map_err(|error| format!("Não foi possível preparar a pasta de runners: {error}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Relógio do sistema inválido: {error}"))?
        .as_millis();
    let staging_dir = root.join(".staging").join(format!("proton-ge-{timestamp}"));
    let extract_dir = staging_dir.join("extract");
    let archive_path = staging_dir.join("proton-ge.tar.gz");
    fs::create_dir_all(&extract_dir)
        .map_err(|error| format!("Não foi possível preparar o staging: {error}"))?;

    let operation = (|| {
        emit_progress(
            &app,
            "downloading",
            "download",
            &release.version,
            0,
            release.size,
            "Iniciando download...",
            None,
        );
        let downloaded_bytes = download_release(&app, &release, &archive_path)?;
        emit_progress(
            &app,
            "extracting",
            "extract",
            &release.version,
            downloaded_bytes,
            release.size,
            "Extraindo pacote em staging...",
            None,
        );
        extract_archive(&archive_path, &extract_dir)?;
        let extracted_root = find_proton_root(&extract_dir, 3).ok_or_else(|| {
            "O pacote extraído não contém um executável proton válido.".to_string()
        })?;
        emit_progress(
            &app,
            "applying",
            "apply",
            &release.version,
            downloaded_bytes,
            release.size,
            "Aplicando runner validado...",
            None,
        );
        if final_dir.exists() {
            fs::remove_dir_all(&final_dir).map_err(|error| {
                format!("Não foi possível limpar instalação incompleta anterior: {error}")
            })?;
        }
        fs::rename(&extracted_root, &final_dir).map_err(|error| {
            format!("Não foi possível aplicar o runner no diretório final: {error}")
        })?;
        if !final_executable.is_file() {
            return Err(
                "O executável proton não foi encontrado após aplicar o staging.".to_string(),
            );
        }

        let runner = ManagedRunner {
            id: release.runner_id.clone(),
            kind: "proton".to_string(),
            version: release.version.clone(),
            label: release.version.clone(),
            source: "Launcher".to_string(),
            install_path: final_dir.to_string_lossy().to_string(),
            executable_path: final_executable.to_string_lossy().to_string(),
            status: "available".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let saved = database::save_managed_runner(&database::open(&app)?, &runner)?;
        emit_progress(
            &app,
            "done",
            "done",
            &release.version,
            downloaded_bytes,
            release.size,
            format!("{} instalado com sucesso.", release.version),
            None,
        );
        Ok(saved)
    })();

    let _ = fs::remove_dir_all(&staging_dir);
    if let Err(error) = &operation {
        emit_progress(
            &app,
            "error",
            "error",
            &release.version,
            0,
            release.size,
            "Falha ao instalar Proton-GE.",
            Some(error.clone()),
        );
    }
    operation
}

#[tauri::command]
pub(crate) async fn install_latest_proton_ge(
    app: tauri::AppHandle,
) -> Result<ManagedRunner, String> {
    tauri::async_runtime::spawn_blocking(move || install_release(app))
        .await
        .map_err(|error| format!("A instalação do Proton-GE foi interrompida: {error}"))?
}

#[tauri::command]
pub(crate) fn remove_managed_runner(
    app: tauri::AppHandle,
    runner_id: String,
) -> Result<bool, String> {
    let connection = database::open(&app)?;
    let runner = database::get_managed_runner(&connection, &runner_id)?
        .ok_or_else(|| format!("Runner gerenciado não encontrado: {runner_id}"))?;
    if runner.source != "Launcher" {
        return Err("Somente runners instalados pelo launcher podem ser removidos.".to_string());
    }

    let root = runners_root(&app)?;
    let canonical_root = fs::canonicalize(&root)
        .map_err(|error| format!("Não foi possível validar a raiz de runners: {error}"))?;
    let install_path = PathBuf::from(&runner.install_path);
    let canonical_install = fs::canonicalize(&install_path)
        .map_err(|error| format!("Não foi possível validar a instalação do runner: {error}"))?;
    if canonical_install == canonical_root || !canonical_install.starts_with(&canonical_root) {
        return Err("O runner está fora da pasta gerenciada e não pode ser removido.".to_string());
    }

    fs::remove_dir_all(&canonical_install).map_err(|error| {
        format!(
            "Não foi possível remover {}: {error}",
            canonical_install.display()
        )
    })?;
    database::remove_managed_runner(&connection, &runner_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_archive_paths() {
        assert!(validate_archive_path(Path::new("GE-Proton/proton")).is_ok());
        assert!(validate_archive_path(Path::new("../escape")).is_err());
        assert!(validate_archive_path(Path::new("/absolute/path")).is_err());
    }

    #[test]
    fn creates_stable_runner_id() {
        assert_eq!(
            runner_id("GE-Proton10-12"),
            "managed-proton-ge-ge-proton10-12"
        );
    }
}
