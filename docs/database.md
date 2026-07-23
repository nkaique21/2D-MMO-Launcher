# Banco de dados

## Localização

```text
~/.local/share/dev.kaiquelb.2d-mmo-launcher/launcher.sqlite
```

## Módulo

Persistência fica em:

```text
src-tauri/src/database.rs
```

O módulo concentra:

- abertura;
- migrations;
- modelos persistidos;
- queries;
- testes de banco.

Conexões abertas pelo app usam `busy_timeout` de 5 segundos para reduzir
falhas transitórias quando uma thread de processo finaliza sessão ao mesmo tempo
que outra operação acessa o SQLite.

Comandos Tauri permanecem responsáveis por validação de domínio e orquestração.

## Tabelas atuais

### `installs`

Estado da instalação local por jogo, incluindo caminho e override legado.

### `game_settings`

Configurações locais por jogo:

- runner override;
- JSON de variáveis de ambiente;
- timestamps.

### `runners`

Runners gerenciados e suas versões/caminhos.

### `playtime_sessions`

Uma linha por execução confirmada do processo principal do jogo:

- `game_id`;
- PID e runner usados;
- início/fim em Unix time UTC;
- duração em segundos;
- exit code;
- motivo do encerramento.

Sessões são criadas somente depois de spawn bem-sucedido. O tempo acumulado é
calculado pela soma de `duration_seconds` das sessões encerradas.

## Migrations

- Versão controlada por `PRAGMA user_version`.
- Cada versão adiciona uma migration.
- Migration executa em transação.
- Usar operações idempotentes quando necessário para adoção de legado.
- Nunca editar migration já distribuída.
- Banco com versão maior que a suportada deve falhar com mensagem clara.
- Testes devem cobrir banco vazio e banco legado.

## Separação de responsabilidades

Manifesto:

- o que o jogo aceita;
- defaults;
- métodos;
- runners possíveis.

SQLite:

- o que o usuário possui localmente;
- caminho real;
- overrides;
- runners instalados;
- sessões de tempo jogado e estado futuro de downloads.

## Tabelas futuras

- `download_tasks`
- talvez cache de catálogo, somente se houver necessidade real

## Regras de mudança

Ao alterar schema:

1. criar nova migration;
2. atualizar modelos e queries;
3. escrever teste de criação;
4. escrever teste de upgrade de versão anterior;
5. preservar dados existentes;
6. executar a suíte Rust;
7. documentar o novo estado em `PROJECT.md`.
