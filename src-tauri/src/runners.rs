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
pub(crate) fn list_runners(app: tauri::AppHandle) -> Result<Vec<RunnerInfo>, String> {
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
