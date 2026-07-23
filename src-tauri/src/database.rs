use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tauri::Manager;

pub(crate) const CURRENT_SCHEMA_VERSION: i64 = 4;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManagedRunner {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) version: String,
    pub(crate) label: String,
    pub(crate) source: String,
    pub(crate) install_path: String,
    pub(crate) executable_path: String,
    pub(crate) status: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlaytimeSession {
    pub(crate) id: i64,
    pub(crate) game_id: String,
    pub(crate) process_id: Option<i64>,
    pub(crate) runner: Option<String>,
    pub(crate) started_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) exit_code: Option<i64>,
    pub(crate) end_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlaytimeSummary {
    pub(crate) game_id: String,
    pub(crate) total_seconds: i64,
    pub(crate) completed_sessions: i64,
    pub(crate) last_played_at: Option<String>,
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
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| format!("Não foi possível configurar espera do SQLite: {error}"))?;

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
    Migration {
        version: 3,
        name: "create_runners",
        sql: "
            CREATE TABLE IF NOT EXISTS runners (
                id TEXT PRIMARY KEY NOT NULL,
                kind TEXT NOT NULL,
                version TEXT NOT NULL,
                label TEXT NOT NULL,
                source TEXT NOT NULL,
                install_path TEXT NOT NULL,
                executable_path TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'available',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS runners_kind_status_idx
                ON runners (kind, status);
        ",
    },
    Migration {
        version: 4,
        name: "create_playtime_sessions",
        sql: "
            CREATE TABLE IF NOT EXISTS playtime_sessions (
                id INTEGER PRIMARY KEY NOT NULL,
                game_id TEXT NOT NULL,
                process_id INTEGER,
                runner TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                duration_seconds INTEGER,
                exit_code INTEGER,
                end_reason TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS playtime_sessions_game_id_idx
                ON playtime_sessions (game_id);
            CREATE INDEX IF NOT EXISTS playtime_sessions_open_idx
                ON playtime_sessions (ended_at) WHERE ended_at IS NULL;
        ",
    },
];

pub(crate) fn list_managed_runners(connection: &Connection) -> Result<Vec<ManagedRunner>, String> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, kind, version, label, source, install_path, executable_path,
                   status, created_at, updated_at
            FROM runners
            ORDER BY created_at DESC, id ASC
            ",
        )
        .map_err(|error| format!("Não foi possível consultar runners gerenciados: {error}"))?;

    let runners = statement
        .query_map([], managed_runner_from_row)
        .map_err(|error| format!("Não foi possível ler runners gerenciados: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Registro de runner gerenciado inválido: {error}"))?;

    Ok(runners)
}

pub(crate) fn get_managed_runner(
    connection: &Connection,
    runner_id: &str,
) -> Result<Option<ManagedRunner>, String> {
    let result = connection.query_row(
        "
        SELECT id, kind, version, label, source, install_path, executable_path,
               status, created_at, updated_at
        FROM runners
        WHERE id = ?1
        ",
        params![runner_id],
        managed_runner_from_row,
    );

    match result {
        Ok(runner) => Ok(Some(runner)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!(
            "Não foi possível carregar o runner gerenciado {runner_id}: {error}"
        )),
    }
}

pub(crate) fn save_managed_runner(
    connection: &Connection,
    runner: &ManagedRunner,
) -> Result<ManagedRunner, String> {
    connection
        .execute(
            "
            INSERT INTO runners (
                id, kind, version, label, source, install_path, executable_path, status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                version = excluded.version,
                label = excluded.label,
                source = excluded.source,
                install_path = excluded.install_path,
                executable_path = excluded.executable_path,
                status = excluded.status,
                updated_at = CURRENT_TIMESTAMP
            ",
            params![
                runner.id,
                runner.kind,
                runner.version,
                runner.label,
                runner.source,
                runner.install_path,
                runner.executable_path,
                runner.status,
            ],
        )
        .map_err(|error| format!("Não foi possível salvar runner gerenciado: {error}"))?;

    get_managed_runner(connection, &runner.id)?.ok_or_else(|| {
        format!(
            "O runner gerenciado {} não foi encontrado após a persistência.",
            runner.id
        )
    })
}

pub(crate) fn remove_managed_runner(
    connection: &Connection,
    runner_id: &str,
) -> Result<bool, String> {
    connection
        .execute("DELETE FROM runners WHERE id = ?1", params![runner_id])
        .map(|removed_rows| removed_rows > 0)
        .map_err(|error| format!("Não foi possível remover runner gerenciado: {error}"))
}

fn managed_runner_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ManagedRunner> {
    Ok(ManagedRunner {
        id: row.get(0)?,
        kind: row.get(1)?,
        version: row.get(2)?,
        label: row.get(3)?,
        source: row.get(4)?,
        install_path: row.get(5)?,
        executable_path: row.get(6)?,
        status: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

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

// PlaytimeSession functions

pub(crate) fn create_playtime_session(
    connection: &Connection,
    game_id: &str,
    process_id: Option<u32>,
    runner: Option<&str>,
    started_at: &str,
) -> Result<PlaytimeSession, String> {
    let game_id = game_id.trim();

    if game_id.is_empty() {
        return Err("ID do jogo não pode ser vazio ao iniciar uma sessão.".to_string());
    }

    connection
        .execute(
            "
            INSERT INTO playtime_sessions (
                game_id, process_id, runner, started_at, duration_seconds
            ) VALUES (?1, ?2, ?3, ?4, 0)
            ",
            params![game_id, process_id.map(i64::from), runner, started_at],
        )
        .map_err(|error| format!("Não foi possível criar sessão de tempo jogado: {error}"))?;

    let session_id = connection.last_insert_rowid();

    get_playtime_session(connection, session_id)?.ok_or_else(|| {
        format!("Sessão de tempo jogado {session_id} não encontrada após a criação.")
    })
}

pub(crate) fn update_playtime_session_progress(
    connection: &Connection,
    session_id: i64,
    duration_seconds: i64,
) -> Result<bool, String> {
    connection
        .execute(
            "
            UPDATE playtime_sessions
            SET duration_seconds = ?2
            WHERE id = ?1 AND ended_at IS NULL
            ",
            params![session_id, duration_seconds.max(0)],
        )
        .map(|updated_rows| updated_rows > 0)
        .map_err(|error| {
            format!("Não foi possível atualizar o progresso da sessão {session_id}: {error}")
        })
}

pub(crate) fn get_playtime_session(
    connection: &Connection,
    session_id: i64,
) -> Result<Option<PlaytimeSession>, String> {
    let result = connection.query_row(
        "
        SELECT id, game_id, process_id, runner, started_at, ended_at,
               duration_seconds, exit_code, end_reason
        FROM playtime_sessions
        WHERE id = ?1
        ",
        params![session_id],
        playtime_session_from_row,
    );

    match result {
        Ok(session) => Ok(Some(session)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!(
            "Não foi possível carregar sessão de tempo jogado {session_id}: {error}"
        )),
    }
}

pub(crate) fn list_sessions_by_game(
    connection: &Connection,
    game_id: &str,
) -> Result<Vec<PlaytimeSession>, String> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, game_id, process_id, runner, started_at, ended_at,
                   duration_seconds, exit_code, end_reason
            FROM playtime_sessions
            WHERE game_id = ?1
            ORDER BY started_at DESC, id DESC
            ",
        )
        .map_err(|error| format!("Não foi possível consultar sessões do jogo: {error}"))?;

    let sessions = statement
        .query_map(params![game_id], playtime_session_from_row)
        .map_err(|error| format!("Não foi possível ler sessões do jogo: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Registro de sessão do jogo inválido: {error}"))?;

    Ok(sessions)
}

pub(crate) fn finalize_playtime_session(
    connection: &Connection,
    session_id: i64,
    ended_at: &str,
    duration_seconds: i64,
    exit_code: Option<i64>,
    end_reason: &str,
) -> Result<PlaytimeSession, String> {
    let updated_rows = connection
        .execute(
            "
            UPDATE playtime_sessions
            SET ended_at = ?2,
                duration_seconds = ?3,
                exit_code = ?4,
                end_reason = ?5
            WHERE id = ?1 AND ended_at IS NULL
            ",
            params![
                session_id,
                ended_at,
                duration_seconds.max(0),
                exit_code,
                end_reason
            ],
        )
        .map_err(|error| format!("Não foi possível finalizar sessão de tempo jogado: {error}"))?;

    if updated_rows == 0 {
        return match get_playtime_session(connection, session_id)? {
            Some(_) => Err(format!(
                "A sessão de tempo jogado {session_id} já foi finalizada."
            )),
            None => Err(format!(
                "Sessão de tempo jogado {session_id} não encontrada para finalização."
            )),
        };
    }

    get_playtime_session(connection, session_id)?.ok_or_else(|| {
        format!("Sessão de tempo jogado {session_id} não encontrada após a finalização.")
    })
}

pub(crate) fn mark_open_sessions_as_interrupted(
    connection: &Connection,
) -> Result<usize, String> {
    connection
        .execute(
            "
            UPDATE playtime_sessions
            SET duration_seconds = COALESCE(duration_seconds, 0),
                ended_at = CAST(
                    CAST(started_at AS INTEGER) + COALESCE(duration_seconds, 0)
                    AS TEXT
                ),
                exit_code = NULL,
                end_reason = 'interrupted'
            WHERE ended_at IS NULL
            ",
            [],
        )
        .map_err(|error| {
            format!("Não foi possível marcar sessões abertas como interrompidas: {error}")
        })
}

pub(crate) fn get_playtime_summary(
    connection: &Connection,
    game_id: &str,
) -> Result<PlaytimeSummary, String> {
    connection
        .query_row(
            "
            SELECT COALESCE(SUM(duration_seconds), 0),
                   COUNT(*),
                   MAX(ended_at)
            FROM playtime_sessions
            WHERE game_id = ?1 AND ended_at IS NOT NULL
            ",
            params![game_id],
            |row| {
                Ok(PlaytimeSummary {
                    game_id: game_id.to_string(),
                    total_seconds: row.get(0)?,
                    completed_sessions: row.get(1)?,
                    last_played_at: row.get(2)?,
                })
            },
        )
        .map_err(|error| format!("Não foi possível calcular o tempo jogado de {game_id}: {error}"))
}

fn playtime_session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlaytimeSession> {
    Ok(PlaytimeSession {
        id: row.get(0)?,
        game_id: row.get(1)?,
        process_id: row.get(2)?,
        runner: row.get(3)?,
        started_at: row.get(4)?,
        ended_at: row.get(5)?,
        duration_seconds: row.get(6)?,
        exit_code: row.get(7)?,
        end_reason: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn migrates_empty_database_to_current_version() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");

        migrate(&mut connection).expect("apply migrations");

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert!(table_exists(&connection, "installs"));
        assert!(table_exists(&connection, "game_settings"));
        assert!(table_exists(&connection, "runners"));
        assert!(table_exists(&connection, "playtime_sessions"));
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
    fn migrates_version_three_to_playtime_sessions_without_losing_data() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");

        for migration in MIGRATIONS.iter().filter(|migration| migration.version <= 3) {
            connection
                .execute_batch(migration.sql)
                .expect("apply legacy migration");
            connection
                .pragma_update(None, "user_version", migration.version)
                .expect("set legacy schema version");
        }
        connection
            .execute(
                "INSERT INTO installs (game_id, install_path) VALUES (?1, ?2)",
                params!["medivia", "/games/medivia"],
            )
            .expect("insert legacy install");

        migrate(&mut connection).expect("migrate schema version 3");

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert!(table_exists(&connection, "playtime_sessions"));
        assert_eq!(
            get_install(&connection, "medivia")
                .expect("legacy install preserved")
                .install_path,
            "/games/medivia"
        );
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

    #[test]
    fn persists_lists_and_removes_managed_runner() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&mut connection).unwrap();
        let runner = ManagedRunner {
            id: "managed-proton-ge-test".to_string(),
            kind: "proton".to_string(),
            version: "GE-Proton-Test".to_string(),
            label: "GE-Proton-Test".to_string(),
            source: "Launcher".to_string(),
            install_path: "/tmp/runners/proton-ge/GE-Proton-Test".to_string(),
            executable_path: "/tmp/runners/proton-ge/GE-Proton-Test/proton".to_string(),
            status: "available".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        };

        let saved = save_managed_runner(&connection, &runner).expect("save managed runner");
        assert_eq!(saved.version, "GE-Proton-Test");
        assert_eq!(list_managed_runners(&connection).unwrap().len(), 1);
        assert!(remove_managed_runner(&connection, &runner.id).unwrap());
        assert!(get_managed_runner(&connection, &runner.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn creates_and_finalizes_playtime_session() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&mut connection).unwrap();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("relógio do sistema válido")
            .as_secs()
            .to_string();

        let session = create_playtime_session(
            &connection,
            "ravenquest",
            Some(12345),
            Some("proton"),
            &timestamp,
        )
        .expect("create session");

        assert!(session.ended_at.is_none());
        assert_eq!(session.duration_seconds, Some(0));
        assert_eq!(session.game_id, "ravenquest");
        assert_eq!(session.process_id, Some(12345));

        let finalized = finalize_playtime_session(
            &connection,
            session.id,
            &timestamp,
            3600,
            Some(0),
            "normal",
        )
        .expect("finalize session");

        assert!(finalized.ended_at.is_some());
        assert_eq!(finalized.duration_seconds, Some(3600));
        assert_eq!(finalized.exit_code, Some(0));
        assert_eq!(finalized.end_reason, Some("normal".to_string()));

        let second_finalize = finalize_playtime_session(
            &connection,
            session.id,
            &timestamp,
            7200,
            Some(0),
            "normal",
        );
        assert!(second_finalize.is_err());
    }

    #[test]
    fn lists_sessions_by_game() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&mut connection).unwrap();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("relógio do sistema válido")
            .as_secs()
            .to_string();

        create_playtime_session(&connection, "ravenquest", None, None, &timestamp)
            .expect("create session 1");
        create_playtime_session(&connection, "medivia", None, None, &timestamp)
            .expect("create session 2");

        let ravenquest_sessions =
            list_sessions_by_game(&connection, "ravenquest").expect("list ravenquest sessions");
        assert_eq!(ravenquest_sessions.len(), 1);

        let medivia_sessions =
            list_sessions_by_game(&connection, "medivia").expect("list medivia sessions");
        assert_eq!(medivia_sessions.len(), 1);
    }

    #[test]
    fn marks_open_sessions_as_interrupted() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&mut connection).unwrap();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("relógio do sistema válido")
            .as_secs()
            .to_string();

        let session = create_playtime_session(&connection, "ravenquest", None, None, &timestamp)
            .expect("create open session");
        assert!(update_playtime_session_progress(&connection, session.id, 45)
            .expect("persist heartbeat"));

        let open_before = get_open_sessions(&connection).expect("get open sessions before");
        assert_eq!(open_before.len(), 1);

        let interrupted_count =
            mark_open_sessions_as_interrupted(&connection).expect("mark interrupted");
        assert_eq!(interrupted_count, 1);

        let open_after = get_open_sessions(&connection).expect("get open sessions after");
        assert_eq!(open_after.len(), 0);

        let sessions = list_sessions_by_game(&connection, "ravenquest").expect("list sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].end_reason, Some("interrupted".to_string()));
        assert_eq!(sessions[0].duration_seconds, Some(45));
        let expected_end = (timestamp.parse::<i64>().unwrap() + 45).to_string();
        assert_eq!(
            sessions[0].ended_at.as_deref(),
            Some(expected_end.as_str())
        );
    }

    #[test]
    fn calculates_accumulated_playtime_from_completed_sessions_only() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        migrate(&mut connection).unwrap();

        let timestamp = "1700000000";
        let first = create_playtime_session(
            &connection,
            "ravenquest",
            Some(100),
            Some("proton"),
            timestamp,
        )
        .expect("create first session");
        finalize_playtime_session(
            &connection,
            first.id,
            "1700000120",
            120,
            Some(0),
            "normal",
        )
        .expect("finalize first session");

        let second = create_playtime_session(
            &connection,
            "ravenquest",
            Some(101),
            Some("proton"),
            "1700000200",
        )
        .expect("create second session");
        finalize_playtime_session(
            &connection,
            second.id,
            "1700000500",
            300,
            Some(1),
            "nonzero_exit",
        )
        .expect("finalize second session");

        create_playtime_session(
            &connection,
            "ravenquest",
            Some(102),
            Some("proton"),
            "1700000600",
        )
        .expect("create open session");

        let summary = get_playtime_summary(&connection, "ravenquest")
            .expect("calculate playtime summary");

        assert_eq!(summary.total_seconds, 420);
        assert_eq!(summary.completed_sessions, 2);
        assert_eq!(summary.last_played_at.as_deref(), Some("1700000500"));
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
