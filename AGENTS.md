# 2D MMO Launcher — Contexto para IA

Este arquivo serve como contexto permanente para qualquer IA, agente de código ou novo chat que venha trabalhar neste projeto. Leia antes de propor mudanças.

## Ambiente do usuário

- Sistema operacional principal: **Arch Linux**.
- Shell padrão: **fish**.
- Preferir comandos simples, curtos e compatíveis com fish.
- Evitar comandos longos com sintaxe específica de bash, especialmente heredocs complexos, pipelines grandes e substituições avançadas.
- Quando uma operação precisar de lógica mais longa, prefira:
  - usar ferramentas de patch/edição de arquivo;
  - criar um script temporário simples;
  - dividir em comandos menores.
- Não assumir que comandos copiados de bash funcionarão diretamente no fish.
- Evitar comandos grandes via `execute_command`, especialmente `python - <<'PY'`, heredocs longos ou scripts inline complexos. Eles tendem a falhar/interagir mal no fish e dificultam troubleshooting.
- Para inspeções Git, preferir comandos curtos e diretos como `git status --short`, `git diff --name-status`, `git diff --stat` e leituras pontuais de arquivos.

## Projeto

- Nome: **2D MMO Launcher**.
- Direcionamento: desenvolver um launcher próprio inspirado em conceitos de arquitetura, organização e experiência do **Twintail Launcher**, mas voltado para MMORPGs 2D/Tibia-like.
- Importante: usar o Twintail apenas como **referência conceitual**. Não copiar código, implementações específicas, assets ou estrutura proprietária.
- Stack atual:
  - Tauri 2;
  - React;
  - Vite;
  - TypeScript;
  - Tailwind CSS;
  - Rust no backend Tauri;
  - SQLite planejado para persistência local.
- Objetivo do projeto: criar um launcher desktop genérico para MMORPGs 2D, baseado em manifestos JSON, evitando lógica específica hardcoded por jogo sempre que possível.

## Filosofia central

Todo jogo deve ser descrito por um **manifesto**.

Adicionar um novo jogo deve exigir, idealmente, apenas:

- criar um manifesto JSON;
- adicionar imagens/assets;
- informar como instalar;
- informar como executar;
- informar estratégia de update, quando existir.

O launcher não deve exigir alteração de código para cada novo jogo. Sempre que surgir uma necessidade específica, primeiro avaliar se ela pode virar configuração de manifesto, configuração de runner ou configuração persistida no banco.

Evite criar `if game.id === "..."` no frontend ou backend. Exceções temporárias precisam ser documentadas e tratadas como dívida técnica.

## Estrutura relevante

- `src/App.tsx`: shell visual principal do launcher.
- `src/styles.css`: estilos globais e base Tailwind.
- `src/types/manifest.ts`: tipos TypeScript dos manifestos.
- `src/lib/tauri.ts`: ponte frontend para comandos Tauri.
- `src-tauri/src/lib.rs`: comandos/backend Tauri.
- `src-tauri/src/runners.rs`: descoberta/listagem inicial de runners disponíveis ou instaláveis.
- `src-tauri/manifests/*.json`: manifestos locais dos jogos.
- `src-tauri/tauri.conf.json`: configuração principal do Tauri.

## Arquitetura alvo

Arquitetura conceitual desejada:

```text
Launcher UI (React)
        │
        │ invoke()
        ▼
Backend (Rust/Tauri)
        │
        ├── SQLite
        ├── Downloader
        ├── Gerenciador de Manifestos
        ├── Gerenciador de Instalações
        ├── Gerenciador de Runners
        ├── Processo de Execução
        └── Configurações
```

### Backend Rust/Tauri

Organizar gradualmente o backend em serviços/módulos:

- `catalog`: leitura, validação e exposição dos manifestos disponíveis.
- `installation`: registro, localização e estado de instalações existentes.
- `downloader`: fila e execução de downloads.
- `extractor`: extração de `.zip`, `.tar.gz` e outros formatos suportados.
- `launcher`: resolução de comando final para executar um jogo.
- `process`: spawn, monitoramento e encerramento de processos de jogo.
- `settings`: configurações globais e por jogo.
- `database`: conexão SQLite, migrations e queries.

### Frontend React

Organizar gradualmente o frontend por domínios/telas:

- `Library`: biblioteca/lista de jogos instalados.
- `Game Details`: hero/banner, informações, ação principal e ações secundárias.
- `Downloads`: progresso, fila e histórico de downloads.
- `Settings`: configurações globais, runners e preferências.

O frontend deve consumir dados via comandos Tauri (`invoke`) e evitar duplicar estado que deveria vir de manifestos ou SQLite.

## Direção de UX/UI

A interface desejada deve seguir esta direção:

- Visual moderno, dark, com aparência glass/blur e foco visual forte no jogo selecionado.
- Barra lateral esquerda: jogos já instalados.
- Faixa superior: jogos disponíveis por manifesto, com possibilidade de baixar/instalar.
- Área principal: banner/hero grande do jogo selecionado.
- Botão principal deve ser claro e destacado:
  - `Jogar` para jogos instalados;
  - `Baixar e instalar` para jogos disponíveis.
- Ações secundárias devem ser mais discretas, por exemplo:
  - localizar instalação;
  - verificar arquivos;
  - abrir pasta;
  - detalhes do manifesto;
  - configurações do runner.
- Evitar UI muito poluída ou com muitos botões competindo com a ação principal.

## Regras de jogos e runners

- **RavenQuest** deve ser tratado como exclusivo para execução via **Proton**.
- **Archlight** deve ser tratado como exclusivo para execução via **Proton**.
- Os manifestos desses jogos devem manter:

```json
"launch": {
  "runner": "proton"
}
```

- Outros jogos podem usar runner nativo quando o manifesto permitir.

### Runners previstos

O launcher deve evoluir para suportar:

- Linux nativo;
- Wine;
- Proton;
- Steam;
- runner personalizado.

Cada jogo pode usar um runner diferente. A decisão deve vir do manifesto e/ou das configurações persistidas, não de lógica hardcoded espalhada pela UI.

### Descoberta e instalação de runners

- A camada de runners deve seguir uma abordagem parecida com a do Twintail em conceito: primeiro detectar runners já disponíveis no sistema do usuário, depois oferecer opções gerenciadas pelo próprio launcher quando não houver uma opção adequada.
- Detecção inicial desejada:
  - Linux nativo sempre disponível em Linux;
  - Wine/Wine64 no `PATH`;
  - Proton no `PATH`, quando existir;
  - `umu-run` como opção compatível com Proton/UMU;
  - Proton instalado pela Steam em `compatibilitytools.d` e `steamapps/common`;
  - runners gerenciados pelo launcher em uma pasta local de dados do app, como `runners/`.
- Se Wine/Proton não forem encontrados, a UI deve indicar opções instaláveis/gerenciadas pelo launcher, por exemplo Wine isolado ou Proton-GE, sem depender exclusivamente do gerenciador de pacotes do sistema.
- A implementação de download/instalação automática de runners deve ser separada da detecção. Primeiro listar e diagnosticar; depois baixar, extrair, registrar e versionar runners gerenciados.
- Evitar prender RavenQuest/Archlight a um caminho fixo de Proton. Eles exigem runner Proton, mas a instância concreta deve ser resolvida pela camada de runners e/ou configuração persistida do usuário.

### Jogos iniciais do catálogo

- RavenQuest;
- PokeXGames;
- Grand Line Adventures;
- Archlight;
- Zezenia;
- Medivia;
- WoT posteriormente.

## Manifestos

- Manifestos ficam em `src-tauri/manifests`.
- Cada manifesto descreve:
  - `id`;
  - `name`;
  - `description`;
  - assets como `banner` e `icon`;
  - métodos de instalação;
  - configuração de launch;
  - estratégia de update.
- A intenção é evoluir para carregar a UI a partir dos manifestos reais, não manter tudo duplicado no frontend.

Formato conceitual base:

```json
{
  "id": "...",
  "name": "...",
  "description": "...",
  "assets": {},
  "installation": {},
  "launch": {},
  "update": {}
}
```

### Métodos de instalação previstos

Suportar progressivamente:

- Archive (`.zip`, `.tar.gz` etc.);
- AppImage;
- instalador Windows;
- launcher externo;
- Steam;
- instalação já existente.

O MVP pode começar com `existing`/localizar instalação existente, mas a estrutura deve permitir expansão sem refatorações grandes.

## Banco SQLite planejado

SQLite será usado para persistência local. Tabelas iniciais desejadas:

- `games`: índice local/cache de jogos conhecidos, se necessário.
- `installs`: instalações localizadas ou criadas pelo launcher.
- `game_settings`: configurações individuais por jogo.
- `playtime_sessions`: sessões de tempo jogado.
- `download_tasks`: fila/histórico de downloads.
- `runners`: runners configurados/disponíveis.

Separação conceitual importante:

- Manifesto descreve o jogo e possibilidades.
- SQLite descreve o estado local do usuário: instalado ou não, caminho, configurações, runner escolhido, sessões, downloads etc.

## Funcionalidades do MVP

O MVP deve cobrir:

- biblioteca de jogos;
- banner/hero do jogo;
- informações do jogo;
- instalar;
- localizar instalação existente;
- jogar;
- configurações individuais por jogo;
- SQLite para armazenar instalações e configurações.

## Roadmap

### Fase 1 — UI e catálogo

- Interface inspirada no Twintail em termos de experiência, sem copiar código.
- Biblioteca visual.
- Cards/atalhos de jogos.
- Tela de detalhes com banner, descrição, runner e ação principal.
- Carregar jogos a partir dos manifestos reais.

### Fase 2 — Instalações existentes e jogar

- Detectar/localizar instalações existentes.
- Persistir caminho no SQLite.
- Botão `Jogar` funcionando para runners simples/nativos.
- Configurações individuais por jogo.

### Fase 3 — Download e instalação automática

- Downloader.
- Fila de downloads.
- Extração/instalação automática.
- Prioridade inicial: Zezenia, GLA e PokeXGames, conforme viabilidade dos manifestos.

### Fase 4 — Wine/Proton

- Camada de runners.
- Suporte a Wine.
- Suporte a Proton.
- RavenQuest via Proton.
- Archlight via Proton.

### Fase 5 — Recursos avançados

- Atualizações.
- Reparo/verificação de arquivos.
- Tempo jogado.
- Notícias.
- Discord RPC.
- Integração opcional com Steam.
- Auto update do launcher.

## Comandos comuns

Use comandos simples:

```sh
npm run build
```

Valida TypeScript e build Vite.

```sh
npm run dev -- --host 127.0.0.1
```

Sobe apenas o Vite para debug web. Não é o preview final do app.

```sh
npm run tauri dev
```

Roda o app na janela nativa Tauri. Este é o modo correto para validar o visual real do launcher.

## Observações sobre preview

- Para avaliar o visual final, preferir sempre Tauri nativo.
- Browser/Puppeteer pode servir para inspeção rápida, mas não substitui a janela Tauri.
- Se `npm run tauri dev` falhar no Arch Linux, verificar dependências do Tauri/WebKitGTK e informar exatamente quais pacotes estão faltando.

## Estado recente do projeto

- A UI foi ajustada para separar jogos instalados à esquerda e jogos disponíveis por manifesto no topo.
- A área principal agora usa um hero/banner grande do jogo selecionado.
- RavenQuest e Archlight foram marcados na UI como Proton-only.
- Os manifestos `ravenquest.json` e `archlight.json` foram ajustados para `runner: "proton"`.
- `npm run build` passou com sucesso após esses ajustes.
- `npm run tauri dev` compilou o backend Rust e iniciou `target/debug/two-d-mmo-launcher` com sucesso no ambiente local.
- `src/App.tsx` foi refatorado para carregar o catálogo real via `listGames()`/`list_games`, usando `GameManifest[]` vindo do backend Tauri.
- O frontend agora usa descrição, assets, runner e métodos de instalação vindos dos manifestos locais.
- Foi adicionada uma base SQLite inicial com `rusqlite` no backend Tauri.
- O banco local é criado no diretório de dados do app como `launcher.sqlite`.
- A tabela inicial `installs` foi criada para registrar instalações locais por `game_id`, `install_path`, `runner_override`, `created_at` e `updated_at`.
- O backend expõe `list_installs`, e o frontend consome esse comando via `listInstalls()`.
- `src/App.tsx` não usa mais `temporaryInstalledGameIds`; o estado instalado/disponível agora vem da tabela `installs`.
- Foi implementado o comando `locate_existing_install`, que abre um seletor de diretório via `rfd`, registra/atualiza o caminho escolhido na tabela `installs` e retorna a instalação salva para o frontend.
- A ação secundária `Localizar instalação existente` no frontend agora chama `locateExistingInstall(gameId)`, atualiza a lista local de instalações e move o jogo para a sidebar de instalados quando o usuário escolhe uma pasta.
- A localização existente atualmente seleciona diretórios/pastas. Validação de executável específico, verificação de arquivos e ajuste fino por manifesto ainda ficam para próximas etapas.
- O painel lateral do jogo selecionado agora exibe o caminho salvo da instalação quando existir registro em `installs`.
- O backend expõe `open_install_folder` para abrir a pasta registrada da instalação e `remove_install` para desvincular/remover o registro local do SQLite.
- O frontend expõe `openInstallFolder(gameId)` e `removeInstall(gameId)` em `src/lib/tauri.ts` e conectou as ações secundárias `Abrir pasta` e `Desvincular instalação`.
- Ao desvincular uma instalação, o jogo sai da sidebar de instalados e volta para o catálogo sem remover arquivos do disco.
- Foi adicionada a infraestrutura inicial do botão `Jogar` com o comando Tauri `launch_game`.
- `launch_game` resolve manifesto + instalação salva, usa `runner_override` quando existir e executa via `Command::spawn` apenas para runner `native`.
- Quando o runner ainda não é suportado ou `launch.executable` está ausente no manifesto, o backend retorna mensagens explícitas em vez de tentar executar algo indefinido.
- O frontend expõe `launchGame(gameId)` e conectou o botão principal `Jogar`, mostrando estado `Iniciando...`, sucesso ou erro retornado pelo backend.
- `InstallMethod` agora aceita `url` opcional no backend Rust e nos tipos TypeScript.
- O manifesto do RavenQuest recebeu método `windowsInstaller` com URL `https://dw.ravenquest.io/ravenquest_installer.exe`, servindo como base para a futura etapa Proton/instalador Windows.
- O manifesto do PokeXGames agora define `launch.executable` como `pxgme-linux` para execução nativa a partir da pasta registrada.
- O manifesto do Grand Line Adventures agora define `launch.executable` como `glaclient-linux` para execução nativa a partir da pasta registrada.
- Foi criada a base inicial de descoberta de runners com o comando Tauri `list_runners`.
- `list_runners` detecta Linux nativo, Wine/Wine64/Proton/UMU via `PATH`, Proton instalado pela Steam e runners gerenciados pelo launcher no diretório de dados do app.
- Quando Wine ou Proton não são encontrados, `list_runners` retorna opções `installable` para Wine gerenciado e Proton-GE gerenciado, preparando a futura instalação automática pelo launcher.
- O frontend consome `listRunners()` e exibe um painel inicial de compatibilidade/runners na lateral, mostrando runners disponíveis e opções instaláveis.
- A descoberta de runners foi extraída de `src-tauri/src/lib.rs` para o módulo dedicado `src-tauri/src/runners.rs`, mantendo o contrato Tauri `list_runners` sem alteração para o frontend.
- `src-tauri/src/runners.rs` agora expõe `resolve_runner`, que resolve o runner concreto a partir do valor pedido pelo manifesto ou `runner_override`.
- `launch_game` passou a chamar `resolve_runner` antes de executar. A execução nativa continua funcionando; Wine/Proton ainda retornam erro orientativo quando resolvidos, até a etapa de spawn via runner ser implementada.
- `src-tauri/src/runners.rs` agora também expõe `build_runner_command`, que monta o comando final por tipo de runner.
- `build_runner_command` mantém execução nativa direta e adiciona montagem inicial para Wine como `wine <executável> ...args`.
- `RunnerCommand` agora carrega variáveis de ambiente específicas do runner e `launch_game` aplica essas variáveis no `Command::spawn`.
- Prefixos gerenciados por jogo são criados no diretório de dados do app em `compat-data/<game_id>/<runner_kind>`.
- Wine recebe `WINEPREFIX` apontando para o prefixo gerenciado do jogo.
- Proton monta comando `proton run <executável> ...args`, recebe `STEAM_COMPAT_DATA_PATH` apontando para o prefixo gerenciado e tenta preencher `STEAM_COMPAT_CLIENT_INSTALL_PATH` quando uma pasta Steam conhecida existe.
- Foi adicionada dependência `reqwest` para downloads HTTP bloqueantes no backend Tauri.
- O backend expõe `download_and_run_installer`, que lê o método `windowsInstaller` do manifesto, baixa o arquivo para `downloads/<game_id>/` no diretório de dados do app e inicia o instalador via runner resolvido.
- O frontend expõe `downloadAndRunInstaller(gameId)` e a ação `Baixar instalador Windows` agora baixa/inicia o instalador do RavenQuest usando a URL do manifesto.
- O botão principal `Baixar e instalar` também chama `downloadAndRunInstaller` quando o jogo disponível possui método `windowsInstaller`, exibindo estado `Baixando...` durante a operação.
- O nome do arquivo baixado preserva extensões como `.exe`, importante para Wine/Proton reconhecerem o instalador Windows corretamente.
- Processos iniciados por `launch_game` e `download_and_run_installer` redirecionam stdout/stderr para `logs/<game_id>/runner.log` no diretório de dados do app, para facilitar diagnóstico de Wine/Proton.
- `LaunchResult` agora inclui `logPath`, e o frontend mostra esse caminho nas mensagens de sucesso de jogo/instalador iniciado.
- No Linux, com identifier `dev.kaiquelb.2d-mmo-launcher`, o diretório de dados tende a ficar em `~/.local/share/dev.kaiquelb.2d-mmo-launcher`; os logs do RavenQuest ficam em `logs/ravenquest/runner.log` dentro desse diretório.
- Diagnóstico importante: se `launch_game` abortar antes do `Command::spawn` (por exemplo, RavenQuest/Archlight ainda com `launch.executable: null`) ou se `download_and_run_installer` falhar antes de montar o processo, o Proton não será executado.
- Para facilitar troubleshooting, `launch_game` e `download_and_run_installer` agora criam/escrevem `runner.log` desde o início da tentativa, registrando ação, runner resolvido, comando final, variáveis de ambiente e erros pré-spawn quando existirem.
- Validação real do RavenQuest mostrou que o Proton era executado e criava prefixo, mas o instalador não abria janela; o arquivo baixado era um Nullsoft Installer Windows válido de 134 MB e a sessão estava em X11.
- Para Proton fora da Steam, o runner agora prefere `umu-run` quando disponível. Sem UMU, Proton direto usa `waitforexitandrun` e caminho Windows `z:\...` em vez de `proton run /home/...`.
- O runner Proton agora define IDs sintéticos (`STEAM_COMPAT_APP_ID`, `SteamAppId`, `SteamGameId`), ativa `PROTON_LOG=1`, direciona `PROTON_LOG_DIR` para `logs/<game_id>` e mantém `STEAM_COMPAT_DATA_PATH` no prefixo gerenciado.
- O log de execução agora registra também ambiente gráfico herdado (`DISPLAY`, `XAUTHORITY`, `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `DESKTOP_SESSION`), PID iniciado e exit status/código quando o processo termina.
- Troubleshooting do RavenQuest confirmou que o instalador Windows abre corretamente via Wine puro mesmo quando não abre via Proton. O manifesto agora permite `runner` opcional em métodos de instalação; `download_and_run_installer` usa `installation.methods[].runner` quando definido e só cai para `launch.runner` quando o método não especificar runner. RavenQuest continua com `launch.runner: "proton"` para execução, mas o método `windowsInstaller` declara `runner: "wine"` para abrir o instalador.
- Ajuste posterior do RavenQuest: usar Wine puro para abrir o instalador, mas apontando `WINEPREFIX` para o prefixo compatível do Proton (`compatPrefix: "proton"`). Assim instalador e execução compartilham a mesma instalação em `compat-data/ravenquest/proton/pfx`. O manifesto declara `installPath` relativo ao prefixo Windows e `launch.executable: "launcher.exe"`; depois de iniciar o instalador, o backend registra automaticamente esse caminho esperado no SQLite e o frontend recarrega `listInstalls()`.
- O backend agora reconcilia automaticamente instalações registradas com o manifesto: se o caminho esperado não contiver o executável, ele procura `launch.executable` nos prefixos compatíveis (`compatPrefix`, runner do instalador, runner de launch, Proton/Wine) e atualiza o SQLite para a pasta real encontrada.
- `download_and_run_installer` agora monitora o término do instalador em background; quando o processo encerra, reconcilia/localiza a instalação real, emite o evento Tauri `install-updated` e, se o método declarar `launchAfterInstall: true`, tenta iniciar o jogo automaticamente usando `launch.runner` do manifesto.
- O frontend escuta o evento `install-updated` via `@tauri-apps/api/event`, atualiza a lista local de instalações e seleciona o jogo afetado sem depender de recarregamento manual.
- O manifesto do RavenQuest declara `launchAfterInstall: true`, mantendo instalador via Wine quando necessário e execução via Proton/UMU conforme `launch.runner: "proton"`.
- Em testes locais, uma instalação antiga do RavenQuest foi encontrada em `compat-data/ravenquest/wine/.../RavenQuest Launcher/launcher.exe`; o novo reconciliador atualiza o SQLite para esse caminho real em vez de exigir desvincular/reinstalar.
- Foi adicionado suporte opcional a BattlEye no manifesto via `launch.battlEye`, sem alterar o comportamento dos jogos que não declaram esse bloco.
- `src-tauri/src/lib.rs` agora resolve caminhos configuráveis de BattlEye por base (`installPath`, `compatPrefix` ou runner/prefixo específico), monta o comando com o mesmo runner do jogo e inicia o processo auxiliar antes do executável principal.
- O RavenQuest declara `launch.battlEye` apontando para `drive_c/Program Files (x86)/Tavernlight Games/RavenQuest/ravenquest_dx_BE.exe`, com `workingDir` em `drive_c/Program Files (x86)/Tavernlight Games/RavenQuest`, ambos resolvidos a partir do prefixo Proton. O `belauncher.exe` de `system32` foi descartado como entrada principal porque encerra rapidamente sem abrir o jogo.
- O `runner.log` registra `battl_eye_start`, comando/ambiente do processo auxiliar, `battl_eye_process_started=true`, PID e erros de ausência/spawn quando houver.
- `npm run build` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram após a integração do BattlEye.
- Após teste real, o RavenQuest continuou exibindo “BattlEye service is not running”. O diagnóstico mostrou que não é necessário reinstalar de imediato: há arquivos do BattlEye e `ravenquest_dx.exe` no prefixo Proton, e o `BELauncher.ini` declara `64BitExe=ravenquest_dx.exe`.
- Foi adicionado `launch.battlEye.launchMode`, permitindo configurar se o BattlEye roda antes do executável principal ou se substitui o processo principal. Valores aceitos pelo backend para substituição: `main`, `replaceMain`, `replace-main`, `replace_main`.
- O manifesto do RavenQuest agora usa `launch.battlEye.launchMode: "main"`, então `launch_game` e o auto-launch pós-instalação iniciam `ravenquest_dx_BE.exe` como entrada principal e pulam o spawn separado de `launcher.exe`. O log passa a registrar `main_executable_replaced_by_battl_eye=true`, `battl_eye_launch_mode=main` e `battl_eye_separate_spawn_skipped=main_launch_mode`.
- Validações executadas após esse ajuste: `cargo check --manifest-path src-tauri/Cargo.toml` e `npm run build` passaram.
- Teste manual posterior confirmou que o RavenQuest com BattlEye funciona no Linux quando iniciado via `umu-run` com ambiente equivalente ao Lutris: `PROTONPATH=~/.local/share/Steam/compatibilitytools.d/GE-Proton11-1`, `PROTON_BATTLEYE_RUNTIME=~/.local/share/lutris/runtime/battleye_runtime`, `PROTON_EAC_RUNTIME=~/.local/share/lutris/runtime/eac_runtime`, `WINEESYNC=1`, `WINEFSYNC=1`, `WINEARCH=win64`, `WINEDEBUG=-all`, e sem `GAMEID`/`STORE` definidos manualmente para o UMU usar `umu-default`.
- O manifesto agora aceita `launch.env` e `launch.unsetEnv` opcionais, permitindo aplicar variáveis de ambiente e remover variáveis herdadas/configuradas pelo runner sem hardcode por jogo. Valores iniciados por `~/`, `$HOME/` ou `${HOME}/` são expandidos para o home do usuário no backend Rust.
- `RunnerCommand` agora carrega `unset_envs`, e os spawns de jogo/BattlEye aplicam `envs` e depois `env_remove`. O `runner.log` registra `env.<KEY>=...` e `unset_env.<KEY>=true` para diagnóstico.
- O manifesto do RavenQuest foi atualizado com o ambiente confirmado: `launch.env` declara `PROTONPATH`, runtimes BattlEye/EAC do Lutris e flags Wine; `launch.unsetEnv` remove `GAMEID` e `STORE`; o passo `installBeforeLaunch` com `installArgs: ["1", "0"]` foi removido porque não foi o método funcional.
- Validações executadas após esse ajuste: `cargo check --manifest-path src-tauri/Cargo.toml` e `npm run build` passaram.
- Foi adicionada atualização delegada por manifesto via `update.strategy: "externalLauncher"`. `UpdateConfig` agora aceita `runner`, `compatPrefix`, `executable`, `args`, `pathBase`, `workingDir`, `workingDirBase`, `env` e `unsetEnv` opcionais.
- O backend expõe `run_game_update`, que resolve instalação/manifesto, monta comando com o runner configurado, aplica ambiente de launch + update, registra `action=run_game_update` no `runner.log` e inicia o updater externo.
- O frontend expõe `runGameUpdate(gameId)` e mostra a ação secundária `Atualizar pelo launcher oficial` apenas para jogos instalados com `update.strategy: "externalLauncher"`.
- Foi adicionada atualização/verificação por manifesto remoto via `update.strategy: "remoteManifest"`. `UpdateConfig` agora aceita `manifestUrl`, `manifestFormat`, `targetDir` e `targetDirBase` opcionais.
- O backend expõe `run_game_remote_update`, que baixa/decodifica o manifesto remoto, resolve o diretório alvo por manifesto, verifica CRC32/tamanho dos arquivos locais e baixa apenas arquivos ausentes ou divergentes.
- O RavenQuest agora usa `update.strategy: "remoteManifest"`, lendo `https://dw.ravenquest.io/ravenquest/checksums.txt.gz` no formato `ravenquestZlib` e aplicando em `drive_c/Program Files (x86)/Tavernlight Games/RavenQuest` dentro do prefixo Proton.
- No update remoto, o backend mescla `files` e `binary` do manifesto remoto em um único mapa antes de iterar. Isso garante que `binary.file` (ex.: `ravenquest_dx.exe`) também seja verificado/baixado e evita duplicidade se o mesmo caminho aparecer em `files`. O `runner.log` registra `remote_binary_file=...` para diagnóstico.
- O frontend expõe `runGameRemoteUpdate(gameId)`, escuta o evento `game-update-progress` e mostra ação secundária de verificação/update remoto para jogos instalados com `update.strategy: "remoteManifest"`, preservando `Atualizar pelo launcher oficial` para `externalLauncher`.
- Validações executadas após esse ajuste: `cargo fmt --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml` e `npm run build` passaram.
- Validações executadas após o update remoto e inclusão de `binary`: `npm run build` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram.
- Diagnóstico real do update remoto do RavenQuest mostrou que o manifesto remoto possui cerca de 27k arquivos (`remote_file_count=27452`), e a implementação síncrona anterior fazia o launcher parecer travado durante verificação/download.
- `run_game_remote_update` agora é assíncrono e executa a verificação/download dentro de `tauri::async_runtime::spawn_blocking`, evitando bloquear a UI enquanto percorre muitos arquivos.
- Downloads HTTP do manifesto remoto e dos arquivos agora usam cliente `reqwest::blocking::Client` com timeout de 60s, evitando ficar pendurado indefinidamente em uma requisição remota.
- O update remoto emite eventos `game-update-progress` durante a fase `checking` a cada 100 arquivos e registra checkpoints no `runner.log` a cada 1000 arquivos com `remote_update_progress=checking`; cada download também registra `remote_update_progress=downloading` e `current_file=...`.
- A UI agora exibe um painel visual de atualização no hero do jogo selecionado, com porcentagem grande, barra progressiva em gradiente, fase atual, arquivo sendo verificado/baixado e contadores de arquivos verificados/baixados, inspirado na experiência visual de launchers como Twintail.
- Validações executadas após o ajuste anti-travamento/progresso visual e instrumentação por etapas: `cargo fmt --manifest-path src-tauri/Cargo.toml`, `npm run build` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram.
- A UI do update remoto agora usa uma segunda fonte de diagnóstico: quando os eventos `game-update-progress` não chegam ou ficam parados por alguns segundos, o React chama `get_game_update_progress(gameId)` e reconstrói o painel a partir do `runner.log`. O painel mostra a fonte atual do progresso (`local`, `evento Tauri` ou `runner.log`) e limpa o estado pendente quando o log indica `done` ou `error`.
- Validações executadas após o fallback por `runner.log`: `npm run build` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram.
- Ainda existem metadados visuais temporários por jogo no frontend, como abreviação, gradiente e categoria curta; eles não devem conter regra de negócio.

## Onde prosseguir daqui

Próximo passo recomendado para desenvolvimento:

1. **Completar dados de execução nos manifestos**
   - Definir `launch.executable` para outros jogos nativos quando houver executável conhecido.
   - Confirmar executable/path real de Zezenia e Medivia.
   - Avaliar se o manifesto precisa de campos adicionais para validação de pasta, executáveis alternativos ou argumentos por plataforma.

2. **Validar instalações registradas**
   - Preparar validação por manifesto para confirmar executável/estrutura esperada.
   - Fazer `Verificar arquivos` indicar se a pasta registrada ainda existe e se contém o executável esperado quando esse dado estiver modelado.

3. **Camada de runners**
   - Validar o botão `Baixar instalador Windows` do RavenQuest em ambiente com Proton/UMU disponível.
   - Se o instalador não abrir, consultar `logs/ravenquest/runner.log` no diretório de dados do app para analisar stdout/stderr do Wine/Proton.
   - Validar o auto-launch pós-instalação do RavenQuest (`launchAfterInstall`) após uma instalação limpa e após uma instalação reconciliada de prefixo antigo.
   - Validar o RavenQuest em execução real com BattlEye e conferir no `runner.log` se aparecem `main_executable_replaced_by_battl_eye=true`, `battl_eye_launch_mode=main`, `env.PROTONPATH=...GE-Proton11-1`, `env.PROTON_BATTLEYE_RUNTIME=...battleye_runtime`, `unset_env.GAMEID=true` e `unset_env.STORE=true`.
   - Testar a ação de update/verificação remota do RavenQuest instalado e conferir se a UI mostra barra, porcentagem, stepper/timeline, stage atual, último evento, fonte do progresso (`evento Tauri` ou fallback `runner.log`), pasta alvo, log e arquivo atual sem congelar. No `runner.log`, conferir `action=run_game_remote_update`, `update_strategy=remoteManifest`, `remote_update_stage=...`, `remote_binary_file=Some("/ravenquest_dx.exe")` ou caminho equivalente vindo do manifesto remoto, `remote_update_progress=checking` e `remote_update_progress=downloading` quando houver arquivos divergentes.
   - Se o jogo ainda reclamar anti-cheat, conferir se os runtimes do Lutris existem nos caminhos declarados no manifesto e se o runner resolvido é `system-umu-run`; depois avaliar tornar esses caminhos configuráveis por UI/SQLite.
   - Testar execução via Wine quando houver jogo/instalador Windows simples e Wine disponível.
   - Validar RavenQuest com Proton usando o prefixo gerenciado criado em `compat-data/ravenquest/proton`.
   - Persistir configurações avançadas de prefixo/runner no SQLite quando houver UI de configurações por jogo.
   - Ajustar variáveis de ambiente adicionais de Proton/UMU conforme necessário após teste real.
   - Instalar/testar `umu-run` (`umu-launcher` no Arch/AUR conforme disponibilidade) para validar RavenQuest com ambiente Proton mais adequado fora da Steam.
   - Criar fluxo para instalar/registrar runners gerenciados pelo launcher quando Wine/Proton não existirem no sistema.
   - Implementar suporte progressivo a Wine/Proton para RavenQuest e Archlight.
   - Usar o instalador Windows do RavenQuest como base de teste para Proton/Wine.
   - Registrar sessão para futuro tempo jogado quando o spawn for bem-sucedido.

4. **Modularizar backend SQLite**
   - Extrair a lógica SQLite atual de `src-tauri/src/lib.rs` para um módulo `database`.
   - Preparar uma estrutura simples de migrations para evoluir `installs`, `game_settings` e `runners` sem concentrar tudo em `lib.rs`.

5. **Depois avançar para download/instalação automática**
   - Só iniciar depois que catálogo, instalações existentes e execução básica estiverem bem definidos.

Critério de arquitetura: sempre que uma funcionalidade parecer específica demais para um jogo, tentar modelar como manifesto, runner, método de instalação ou configuração persistida.

## Preferências de colaboração

- Responder em **pt-BR**.
- Explicar mudanças de forma direta e prática.
- Antes de editar arquivos importantes, conferir padrões existentes do projeto.
- Manter arquitetura extensível e evitar acoplamento desnecessário.
- Ao validar visual, lembrar que o usuário quer ver no **Tauri**, não só no navegador.

## Fluxo obrigatório de etapas Git

- Ao concluir cada etapa funcional aprovada pelo usuário, atualizar este próprio `AGENTS.md` com o estado recente, decisões importantes, próximos passos e qualquer nova regra operacional definida durante a etapa.
- A atualização do `AGENTS.md` deve acontecer antes do commit da etapa, para que o contexto versionado acompanhe a evolução real do projeto.
- Ao concluir cada etapa funcional aprovada pelo usuário, criar um commit Git específico para aquela etapa.
- Depois do commit local, subir as alterações para o remoto configurado com `git push` antes de iniciar a próxima etapa.
- Antes de commitar, revisar `git status` e, quando útil, o diff para evitar incluir mudanças acidentais.
- Mensagens de commit devem ser curtas, descritivas e em português ou inglês técnico consistente com o histórico do projeto.
- Se `git push` falhar por credenciais, rede ou divergência com o remoto, informar o erro e aguardar orientação antes de prosseguir para a próxima etapa.