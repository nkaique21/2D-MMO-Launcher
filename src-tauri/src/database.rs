use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tauri::Manager;

pub(crate) const CURRENT_SCHEMA_VERSION: i64 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameInstall {
    pub(crate) game_id: String,
    pub(crate) install_path: String,
    pub(crate) runner_override: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameSettings {
    pub(crate) game_id: String,
    pub(crate) runner_override: Option<String>,
    pub(crate) env_overrides: HashMap<String, String>,
    pub(crate) created_at: Option<String>,
    pub(crate) updated_at: Option<String>,
}

pub(crate) fn open(app: &tauri::AppHandle) -> Result<Connection, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        format!("Não foi possível resolver o diretório de dados do app: {error}")
    })?;

    fs::create_dir_all(&app_data_dir).map_err(|error| {
        format!(
            "Não foi possível criar o diretório de dados {}: {error}",
            app_data_dir.display()
        )
    })?;

    open_path(&app_data_dir.join("launcher.sqlite"))
}

fn open_path(path: &Path) -> Result<Connection, String> {
    let mut connection = Connection::open(path)
        .map_err(|error| format!("Não foi possível abrir o banco {}: {error}", path.display()))?;

    migrate(&mut connection)?;

    Ok(connection)
}

fn migrate(connection: &mut Connection) -> Result<(), String> {
    let version = schema_version(connection)?;

    if version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "O banco usa schema {version}, mais novo que o suportado ({CURRENT_SCHEMA_VERSION})."
        ));
    }

    for migration in MIGRATIONS
        .iter()
        .filter(|migration| migration.version > version)
    {
        let transaction = connection.transaction().map_err(|error| {
            format!(
                "Não foi possível iniciar migration SQLite {} ({}): {error}",
                migration.version, migration.name
            )
        })?;

        transaction.execute_batch(migration.sql).map_err(|error| {
            format!(
                "Não foi possível aplicar migration SQLite {} ({}): {error}",
                migration.version, migration.name
            )
        })?;
        transaction
            .pragma_update(None, "user_version", migration.version)
            .map_err(|error| {
                format!(
                    "Não foi possível registrar migration SQLite {} ({}): {error}",
                    migration.version, migration.name
                )
            })?;
        transaction.commit().map_err(|error| {
            format!(
                "Não foi possível concluir migration SQLite {} ({}): {error}",
                migration.version, migration.name
            )
        })?;
    }

    Ok(())
}

fn schema_version(connection: &Connection) -> Result<i64, String> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| format!("Não foi possível consultar a versão do schema SQLite: {error}"))
}

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create_installs",
        sql: "
            CREATE TABLE IF NOT EXISTS installs (
                game_id TEXT PRIMARY KEY NOT NULL,
                install_path TEXT NOT NULL,
                runner_override TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
        ",
    },
    Migration {
        version: 2,
        name: "create_game_settings",
        sql: "
            CREATE TABLE IF NOT EXISTS game_settings (
                game_id TEXT PRIMARY KEY NOT NULL,
                runner_override TEXT,
                env_overrides_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
        ",
    },
];

pub(crate) fn list_installs(connection: &Connection) -> Result<Vec<GameInstall>, String> {
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
        .query_map([], install_from_row)
        .map_err(|error| format!("Não foi possível ler instalações: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Registro de instalação inválido: {error}"))?;

    Ok(installs)
}

pub(crate) fn get_install(connection: &Connection, game_id: &str) -> Result<GameInstall, String> {
    connection
        .query_row(
            "
            SELECT game_id, install_path, runner_override, created_at, updated_at
            FROM installs
            WHERE game_id = ?1
            ",
            params![game_id],
            install_from_row,
        )
        .map_err(|error| format!("Não foi possível carregar a instalação de {game_id}: {error}"))
}

pub(crate) fn save_install(
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

pub(crate) fn remove_install(connection: &Connection, game_id: &str) -> Result<bool, String> {
    connection
        .execute("DELETE FROM installs WHERE game_id = ?1", params![game_id])
        .map(|removed_rows| removed_rows > 0)
        .map_err(|error| format!("Não foi possível remover a instalação: {error}"))
}

fn install_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GameInstall> {
    Ok(GameInstall {
        game_id: row.get(0)?,
        install_path: row.get(1)?,
        runner_override: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

pub(crate) fn empty_game_settings(game_id: &str) -> GameSettings {
    GameSettings {
        game_id: game_id.to_string(),
        ..GameSettings::default()
    }
}

pub(crate) fn get_game_settings(
    connection: &Connection,
    game_id: &str,
) -> Result<GameSettings, String> {
    let result = connection.query_row(
        "
        SELECT game_id, runner_override, env_overrides_json, created_at, updated_at
        FROM game_settings
        WHERE game_id = ?1
        ",
        params![game_id],
        |row| {
            let env_overrides_json: String = row.get(2)?;
            let env_overrides = serde_json::from_str(&env_overrides_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;

            Ok(GameSettings {
                game_id: row.get(0)?,
                runner_override: row.get(1)?,
                env_overrides,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    );

    match result {
        Ok(settings) => Ok(settings),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(empty_game_settings(game_id)),
        Err(error) => Err(format!(
            "Não foi possível carregar configurações de {game_id}: {error}"
        )),
    }
}

pub(crate) fn save_game_settings(
    connection: &Connection,
    game_id: &str,
    runner_override: Option<&str>,
    env_overrides: &HashMap<String, String>,
) -> Result<GameSettings, String> {
    let env_overrides_json = serde_json::to_string(env_overrides)
        .map_err(|error| format!("Não foi possível serializar configurações: {error}"))?;

    connection
        .execute(
            "
            INSERT INTO game_settings (game_id, runner_override, env_overrides_json)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(game_id) DO UPDATE SET
                runner_override = excluded.runner_override,
                env_overrides_json = excluded.env_overrides_json,
                updated_at = CURRENT_TIMESTAMP
            ",
            params![game_id, runner_override, env_overrides_json],
        )
        .map_err(|error| format!("Não foi possível salvar configurações do jogo: {error}"))?;

    get_game_settings(connection, game_id)
}

pub(crate) fn reset_game_settings(
    connection: &Connection,
    game_id: &str,
) -> Result<GameSettings, String> {
    connection
        .execute(
            "DELETE FROM game_settings WHERE game_id = ?1",
            params![game_id],
        )
        .map_err(|error| format!("Não foi possível restaurar configurações do jogo: {error}"))?;

    Ok(empty_game_settings(game_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_empty_database_to_current_version() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");

        migrate(&mut connection).expect("apply migrations");

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert!(table_exists(&connection, "installs"));
        assert!(table_exists(&connection, "game_settings"));
    }

    #[test]
    fn migrates_legacy_schema_without_losing_install_data() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(
                "
                CREATE TABLE installs (
                    game_id TEXT PRIMARY KEY NOT NULL,
                    install_path TEXT NOT NULL,
                    runner_override TEXT,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                INSERT INTO installs (game_id, install_path) VALUES ('legacy', '/games/legacy');
                ",
            )
            .unwrap();

        migrate(&mut connection).expect("migrate legacy schema");

        let install = get_install(&connection, "legacy").expect("legacy install preserved");
        assert_eq!(install.install_path, "/games/legacy");
        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn persists_and_resets_game_settings() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&mut connection).unwrap();
        let env = HashMap::from([("PROTONPATH".to_string(), "~/GE-Proton".to_string())]);

        let saved = save_game_settings(&connection, "ravenquest", Some("system-umu-run"), &env)
            .expect("save settings");
        assert_eq!(saved.runner_override.as_deref(), Some("system-umu-run"));
        assert_eq!(saved.env_overrides, env);

        let reset = reset_game_settings(&connection, "ravenquest").expect("reset settings");
        assert!(reset.runner_override.is_none());
        assert!(reset.env_overrides.is_empty());
        assert!(reset.created_at.is_none());
    }

    fn table_exists(connection: &Connection, table: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                params![table],
                |row| row.get(0),
            )
            .unwrap()
    }
}
