use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
    TarBz2,
}

impl ArchiveFormat {
    pub(crate) fn resolve(explicit: Option<&str>, source: &str) -> Result<Self, String> {
        if let Some(explicit) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
            return Self::parse(explicit).ok_or_else(|| unsupported_format_error(explicit));
        }

        Self::infer(source).ok_or_else(|| {
            format!(
                "Não foi possível inferir o formato do arquivo por `{source}`. Defina installation.methods[].format. {}",
                supported_formats_message()
            )
        })
    }

    fn parse(value: &str) -> Option<Self> {
        let normalized = value
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase()
            .replace('_', ".")
            .replace('-', ".");

        match normalized.as_str() {
            "zip" => Some(Self::Zip),
            "tar" => Some(Self::Tar),
            "tar.gz" | "tgz" | "gzip" | "gz" => Some(Self::TarGz),
            "tar.bz2" | "tbz2" | "tbz" | "bzip2" | "bz2" => Some(Self::TarBz2),
            _ => None,
        }
    }

    fn infer(source: &str) -> Option<Self> {
        let without_fragment = source.split('#').next().unwrap_or(source);
        let without_query = without_fragment
            .split('?')
            .next()
            .unwrap_or(without_fragment);
        let lowercase = without_query.to_ascii_lowercase();

        if lowercase.ends_with(".tar.gz") || lowercase.ends_with(".tgz") {
            Some(Self::TarGz)
        } else if lowercase.ends_with(".tar.bz2")
            || lowercase.ends_with(".tbz2")
            || lowercase.ends_with(".tbz")
        {
            Some(Self::TarBz2)
        } else if lowercase.ends_with(".zip") {
            Some(Self::Zip)
        } else if lowercase.ends_with(".tar") {
            Some(Self::Tar)
        } else {
            None
        }
    }

    pub(crate) fn canonical_name(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
            Self::TarBz2 => "tar.bz2",
        }
    }
}

pub(crate) fn extract_archive(
    archive_path: &Path,
    destination: &Path,
    format: ArchiveFormat,
    strip_top_level_dir: bool,
) -> Result<usize, String> {
    match format {
        ArchiveFormat::Zip => extract_zip(archive_path, destination, strip_top_level_dir),
        ArchiveFormat::Tar => {
            let file = open_archive(archive_path)?;
            extract_tar(file, archive_path, destination, strip_top_level_dir)
        }
        ArchiveFormat::TarGz => {
            let file = open_archive(archive_path)?;
            extract_tar(
                GzDecoder::new(file),
                archive_path,
                destination,
                strip_top_level_dir,
            )
        }
        ArchiveFormat::TarBz2 => {
            let file = open_archive(archive_path)?;
            extract_tar(
                BzDecoder::new(file),
                archive_path,
                destination,
                strip_top_level_dir,
            )
        }
    }
}

fn supported_formats_message() -> &'static str {
    "Formatos aceitos: zip, tar, tar.gz/tgz e tar.bz2/tbz2."
}

fn unsupported_format_error(value: &str) -> String {
    format!(
        "Formato de arquivo não suportado: `{value}`. {}",
        supported_formats_message()
    )
}

fn open_archive(path: &Path) -> Result<File, String> {
    File::open(path).map_err(|error| {
        format!(
            "Não foi possível abrir o arquivo compactado {}: {error}",
            path.display()
        )
    })
}

fn safe_relative_path(path: &Path, strip_top_level_dir: bool) -> Result<Option<PathBuf>, String> {
    let mut safe_components = Vec::new();

    for component in path.components() {
        match component {
            Component::Normal(segment) => safe_components.push(segment.to_os_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "O arquivo compactado contém um caminho inseguro: {}",
                    path.display()
                ));
            }
        }
    }

    if strip_top_level_dir && !safe_components.is_empty() {
        safe_components.remove(0);
    }

    if safe_components.is_empty() {
        return Ok(None);
    }

    let mut relative = PathBuf::new();
    for component in safe_components {
        relative.push(component);
    }

    Ok(Some(relative))
}

fn prepare_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Não foi possível criar diretório extraído {}: {error}",
                parent.display()
            )
        })?;
    }

    Ok(())
}

fn extract_zip(
    archive_path: &Path,
    destination: &Path,
    strip_top_level_dir: bool,
) -> Result<usize, String> {
    let archive_file = open_archive(archive_path)?;
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
        let Some(relative_path) = safe_relative_path(&enclosed_path, strip_top_level_dir)? else {
            continue;
        };
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

        prepare_parent(&output_path)?;
        let mut output = File::create(&output_path).map_err(|error| {
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
            set_unix_mode(&output_path, mode)?;
        }

        extracted_files += 1;
    }

    Ok(extracted_files)
}

fn extract_tar<R: Read>(
    reader: R,
    archive_path: &Path,
    destination: &Path,
    strip_top_level_dir: bool,
) -> Result<usize, String> {
    let mut archive = tar::Archive::new(reader);
    let entries = archive.entries().map_err(|error| {
        format!(
            "Não foi possível ler o arquivo TAR {}: {error}",
            archive_path.display()
        )
    })?;
    let mut extracted_files = 0_usize;

    for entry in entries {
        let mut entry = entry.map_err(|error| {
            format!(
                "Entrada inválida no arquivo TAR {}: {error}",
                archive_path.display()
            )
        })?;
        let original_path = entry.path().map_err(|error| {
            format!(
                "Caminho inválido no arquivo TAR {}: {error}",
                archive_path.display()
            )
        })?;
        let Some(relative_path) = safe_relative_path(&original_path, strip_top_level_dir)? else {
            continue;
        };
        let output_path = destination.join(relative_path);
        let entry_type = entry.header().entry_type();

        if entry_type.is_dir() {
            fs::create_dir_all(&output_path).map_err(|error| {
                format!(
                    "Não foi possível criar diretório extraído {}: {error}",
                    output_path.display()
                )
            })?;
            continue;
        }

        if !entry_type.is_file() {
            return Err(format!(
                "O arquivo TAR contém um tipo de entrada não permitido em {}. Links e arquivos especiais são recusados por segurança.",
                original_path.display()
            ));
        }

        prepare_parent(&output_path)?;
        let mut output = File::create(&output_path).map_err(|error| {
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
        if let Ok(mode) = entry.header().mode() {
            set_unix_mode(&output_path, mode)?;
        }

        extracted_files += 1;
    }

    Ok(extracted_files)
}

#[cfg(unix)]
fn set_unix_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|error| {
        format!(
            "Não foi possível restaurar permissões de {}: {error}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::ArchiveFormat;

    #[test]
    fn parses_supported_aliases() {
        assert_eq!(
            ArchiveFormat::resolve(Some("zip"), "ignored"),
            Ok(ArchiveFormat::Zip)
        );
        assert_eq!(
            ArchiveFormat::resolve(Some("tgz"), "ignored"),
            Ok(ArchiveFormat::TarGz)
        );
        assert_eq!(
            ArchiveFormat::resolve(Some("tbz2"), "ignored"),
            Ok(ArchiveFormat::TarBz2)
        );
    }

    #[test]
    fn infers_compound_extensions_before_simple_extensions() {
        assert_eq!(
            ArchiveFormat::resolve(None, "https://example.test/game.tar.gz?download=1"),
            Ok(ArchiveFormat::TarGz)
        );
        assert_eq!(
            ArchiveFormat::resolve(None, "client.TAR.BZ2#asset"),
            Ok(ArchiveFormat::TarBz2)
        );
    }

    #[test]
    fn rejects_unknown_formats_with_actionable_message() {
        let error = ArchiveFormat::resolve(Some("7z"), "game.7z").unwrap_err();
        assert!(error.contains("7z"));
        assert!(error.contains("Formatos aceitos"));
    }
}
