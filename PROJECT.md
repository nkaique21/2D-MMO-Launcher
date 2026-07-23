# PROJECT.md — 2D MMO Launcher

Este arquivo descreve o **estado atual**, não o histórico do projeto.

## Visão

Launcher desktop genérico para MMORPGs 2D/Tibia-like, inspirado conceitualmente na experiência do Twintail sem copiar código, assets ou estrutura proprietária.

Novos jogos devem entrar principalmente por manifesto JSON, assets e configuração, sem regras específicas espalhadas pelo frontend ou backend.

## Stack e ambiente

- Tauri 2 + Rust.
- React + TypeScript + Vite + Tailwind.
- SQLite com `rusqlite`.
- HTTP com `reqwest`.
- CachyOS/Arch Linux, KDE e Fish.
- Validação final na janela Tauri nativa.

## Princípio de dados

**Manifesto:** identidade, assets, instalação, launch, runner, update, verificação e defaults.

**SQLite:** instalações, caminhos, configurações locais, overrides e runners gerenciados.

Evitar `if game.id === "..."` quando um campo de manifesto, runner ou configuração persistida puder representar o caso.

## Estrutura relevante

```text
src/
├── App.tsx
├── styles.css
├── lib/tauri.ts
└── types/manifest.ts

src-tauri/
├── manifests/*.json
├── src/lib.rs
├── src/catalog.rs
├── src/database.rs
├── src/runners.rs
├── src/managed_runners.rs
└── tauri.conf.json
```

- `App.tsx`: composição e fluxos da UI.
- `tauri.ts`: comandos/eventos Tauri.
- `manifest.ts`: tipos TypeScript.
- `lib.rs`: comandos e orquestração.
- `catalog.rs`: catálogo remoto, cache, fallback e validação.
- `database.rs`: migrations, modelos e queries.
- `runners.rs`: detecção, resolução e comandos.
- `managed_runners.rs`: lifecycle de runners baixados.
- `manifests`: catálogo local.

Detalhes: `docs/architecture.md`.

## Funcionalidades atuais

### Catálogo e biblioteca

- Catálogo oficial remoto em repositório separado.
- Cache local transacional e fallback para manifestos embutidos.
- Atualização automática em background e ação manual no drawer.
- Validação de schema, IDs, HTTPS, tamanho e paths seguros.
- Estado instalado vindo do SQLite.
- Localizar instalação existente.
- Abrir pasta e desvincular sem apagar arquivos.
- Hero do jogo, ação principal contextual e drawer de ações.

### Execução

- Nativo, Wine, Proton e UMU.
- Override por jogo e seleção de runner concreto.
- Prefixo isolado por jogo.
- Ambiente do manifesto + override local.
- stdout/stderr em `logs/<game_id>/runner.log`.
- BattlEye opcional, inclusive como processo principal.

### Instalação

- Instalação existente.
- Instalador Windows.
- Archive ZIP gerenciado.
- Download em background com retry.
- Extração em staging e proteção contra path traversal.
- Validação de executável e permissão no Linux.
- Registro no SQLite e auto-launch opcional.

### Update e reparo

- `externalLauncher`.
- `remoteManifest` com tamanho/CRC32.
- Plano de divergências, staging e aplicação transacional.
- Download concorrente limitado, retry e encoding de URL.
- Progresso por eventos Tauri e fallback por `runner.log`.
- Reparo remoto explícito.
- Update não roda automaticamente antes de `Jogar`.

### Verificação

- `verification.requiredFiles`.
- `verification.checksums` com CRC32.
- Diagnóstico sem modificar arquivos.
- Executável efetivo respeita BattlEye em modo principal.
- Estratégia de reparo informada à UI.

### Configurações locais

- Runner override.
- Variáveis de ambiente por jogo.
- Defaults do manifesto.
- Persistência e restauração de padrões.

### Processos e tempo jogado

- Estado genérico de processo por jogo.
- Bloqueio de launch duplicado enquanto a execução está ativa.
- Sessões persistidas e tempo acumulado no SQLite.
- Checkpoint periódico e recuperação de sessões interrompidas.
- Fluxo validado com runners nativo, Proton/UMU e RavenQuest/BattlEye.

### Runners gerenciados

- Catálogo da release mais recente do Proton-GE.
- Download e extração em staging.
- Registro no banco.
- Seleção por ID estável.
- Remoção segura.

## Banco

Local esperado:

```text
~/.local/share/dev.kaiquelb.2d-mmo-launcher/launcher.sqlite
```

Tabelas atuais:

- `installs`;
- `game_settings`;
- `runners`;
- `playtime_sessions`.

O schema usa `PRAGMA user_version` e migrations incrementais em `database.rs`.
Migration distribuída não deve ser editada. Upgrade precisa preservar dados.

Detalhes: `docs/database.md`.

## Jogos e estado conhecido

### RavenQuest

- Launch via Proton/UMU.
- Instalador pode usar Wine com prefixo compatível.
- `ravenquest_dx_BE.exe` é o processo principal efetivo.
- Ambiente BattlEye/EAC configurável.
- Update e reparo por manifesto remoto.
- Prefixo em `compat-data/ravenquest/proton`.
- Fluxo real de launch e reparo já validado.

### Archlight

- ZIP portátil.
- `abaldar.exe` via Proton/UMU.
- Instalação e launch validados.

### Medivia

- ZIP Linux.
- Executável nativo `medivia`.
- Instalação e auto-launch validados.

### PokeMMO

- ZIP Linux.
- Entrada `PokeMMO.sh`.
- Execução nativa, estrutura e CRC32 validados.

### PokeXGames

- Executável esperado `pxgme-linux`.
- Pode usar instalação existente até completar fluxo gerenciado.

### Grand Line Adventures

- Executável esperado `glaclient-linux`.
- Pode usar instalação existente até completar fluxo gerenciado.

## UX atual

- Janela estática para ações comuns.
- Sidebar estreita de instalados.
- Banner como foco principal.
- Informações sobre o hero.
- Ação principal no canto inferior direito.
- Ações secundárias no drawer `⋯`.
- Progresso compacto no hero e diagnóstico no drawer.
- Selects com tema escuro compatível com WebKitGTK.

Detalhes: `docs/ui.md`.

## Comandos padrão

```fish
npm run build
npm run tauri dev
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

O navegador serve para debug rápido; não substitui o Tauri real.

## Limites atuais

- Metadados visuais temporários ainda existem no frontend; não adicionar regra de negócio neles.
- Reparo por `archive` e `windowsInstaller` ainda precisa de desenho não destrutivo.
- Proton-GE gerenciado cobre inicialmente a release mais recente.
- Múltiplas versões, verificação criptográfica e Wine gerenciado são evoluções.
- Alguns jogos ainda têm manifestos incompletos.
- Assets do catálogo remoto ainda não possuem cache local; a UI usa fallback visual offline.
- Assinatura criptográfica do catálogo é evolução futura.

## Roadmap resumido

1. Completar manifestos restantes.
2. Projetar reparo seguro para archive/installer.
3. Evoluir runners gerenciados e múltiplas versões.
4. Adicionar assinatura e cache opcional de assets do catálogo.
5. Evoluir fila, histórico e retomada de downloads.
6. Adicionar recursos opcionais: notícias, Discord RPC e Steam.

Consulte `docs/README.md` para escolher o contexto temático da tarefa.
