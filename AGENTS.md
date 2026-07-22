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
- `src-tauri/src/managed_runners.rs`: catálogo, download, extração transacional e remoção segura de runners instalados pelo launcher.
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
- PokeMMO;
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
- Prioridade inicial: PokeMMO, GLA e PokeXGames, conforme viabilidade dos manifestos.

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
- Diagnóstico real posterior mostrou que o update remoto chegou a baixar milhares de arquivos, mas falhou em um arquivo com caractere especial no caminho (`#...`) durante `downloadingFiles`. O backend agora percent-encode cada segmento do caminho remoto antes de montar a URL, então caracteres como `#` viram `%23` sem quebrar a URL. Downloads de arquivos também têm retry com backoff, logs de tentativa (`remote_update_download_attempt`, `download_attempt`, `download_url`, `download_error`) e reutilizam um único cliente HTTP durante o update remoto para reduzir overhead em muitos arquivos pequenos.
- Validações executadas após encode/retry/reuso de cliente HTTP no update remoto: `cargo fmt --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml` e `npm run build` passaram.
- O update remoto foi alterado para um fluxo transacional com staging: primeiro verifica todos os arquivos locais e monta um plano de divergentes, depois baixa os arquivos planejados para `updates/<game_id>/<timestamp>/staging`, valida o staging completo e só então aplica/substitui os arquivos no diretório alvo. Esse comportamento aproxima o launcher da experiência “verifica o que falta, baixa tudo e depois substitui”, reduzindo risco de instalação parcialmente atualizada.
- `UpdateConfig` agora aceita `maxConcurrentDownloads` opcional. O backend usa `DEFAULT_REMOTE_UPDATE_CONCURRENCY = 6`, limita internamente a no máximo 16 workers e o RavenQuest declara `maxConcurrentDownloads: 8`. O `runner.log` registra `remote_update_parallel_downloads=true`, `remote_update_download_workers=...`, `planned_update_files=...`, `remote_update_download_concurrency=...` e `remote_update_staging_dir=...` para diagnóstico.
- A UI do update remoto e o fallback por `runner.log` agora reconhecem as novas fases `planUpdate`, `prepareStagingDir`, `validateStagedFiles` e `applyStagedFiles`, além dos statuses `validating` e `applying`, exibindo o stepper atualizado durante staging/validação/aplicação.
- Validações executadas após o fluxo transacional/paralelo com staging: `cargo fmt --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml` e `npm run build` passaram.
- Otimização posterior do update remoto: o staging deixou de fazer uma segunda validação sequencial completa após baixar. Cada worker de download agora baixa e valida CRC/tamanho imediatamente, em paralelo, antes de reportar sucesso. A fase `validateStagedFiles` permanece para UX/log, mas registra `remote_update_staging_validation=completed_during_parallel_download`.
- Para reduzir overhead em updates com milhares de arquivos pequenos, o backend reduziu logs/eventos por arquivo: sucessos individuais de download não geram mais `download_attempt`/`download_success` no `runner.log`; falhas e retries continuam detalhados com `download_url=...`/`download_error=...`, e os checkpoints agregados registram `remote_update_download_validation=parallel_worker`. O buffer de leitura CRC também aumentou de 64 KiB para 1 MiB.
- Validações executadas após essa otimização de staging/download: `cargo fmt --manifest-path src-tauri/Cargo.toml` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram.
- Correção posterior do hotfix de update remoto: o cancelamento no primeiro erro fazia o fluxo parecer baixar apenas 1 arquivo e podia impedir o comportamento de launcher normal. O pool de downloads voltou a trabalhar com concorrência real (`maxConcurrentDownloads`, RavenQuest = 8), usando índice atômico compartilhado entre workers em vez de `Mutex<Receiver>`, emitindo progresso a cada arquivo concluído e acumulando erros para reportar no final sem abortar a fila inteira no primeiro arquivo problemático. Validações: `cargo fmt --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml` e `npm run build` passaram.
- Ajuste posterior de UX do update remoto: quando a UI usa o fallback por `runner.log`, o progresso não deve parecer travado entre lotes grandes. `REMOTE_UPDATE_LOG_INTERVAL` foi reduzido de 1000 para 100 arquivos, mantendo logs agregados em vez de sucesso por arquivo. A fase `applyStagedFiles` também passou a registrar `remote_update_progress=applying`, `checked_files`, `updated_files`, `total_files` e `current_file` no `runner.log` a cada 50 arquivos (e no primeiro/último), permitindo que o painel mostre avanço visível também durante a aplicação. Validações após a alteração: `cargo fmt --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml` e `npm run build` passaram.
- Teste real da instalação gerenciada do RavenQuest confirmou o fluxo completo: criação do prefixo em `compat-data/ravenquest/proton`, update remoto, registro no SQLite e primeiro auto-launch via `system-umu-run` com `ravenquest_dx_BE.exe`; o `runner.log` registrou `action=launch_game`, `main_executable_replaced_by_battl_eye=true`, PID e ambiente BattlEye esperados.
- Diagnóstico posterior dos cliques em `Jogar`: eles não chegavam a registrar uma nova `action=launch_game`, pois o frontend executava `runGameRemoteUpdate` antes de todo launch e a tentativa mais recente permanecia na etapa `reconcileInstall`. O botão principal de jogos já instalados agora chama `launchGame` diretamente. O update remoto continua disponível como ação secundária explícita `Atualizar arquivos do jogo`, evitando bloquear cada sessão por uma verificação completa de ~27k arquivos.
- A causa backend do bloqueio em `reconcileInstall` também foi corrigida: quando `launch.battlEye.launchMode` substitui o processo principal, a reconciliação não procura mais recursivamente `launch.executable` (como `launcher.exe`) em todo o prefixo. Ela resolve/diagnostica o executável efetivo do BattlEye e preserva a instalação registrada, registrando `install_path_reconcile_skipped=main_battl_eye_launch` e `effective_launch_executable_exists=...` no `runner.log`.
- O layout principal foi reorganizado para manter o painel compacto de atualização sempre visível logo abaixo do cartão do jogo selecionado. As ações secundárias passaram para uma grade compacta de duas colunas que ocupa apenas o espaço restante da coluna direita, preservando a janela estática sem rolagem; diagnóstico detalhado continua no drawer `Ver detalhes`.
- Validações executadas após a correção do launch manual e do encaixe do painel: `npm run build` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram.
- Validação interativa final no Tauri real: após reinício limpo da build, o usuário confirmou que o RavenQuest abriu pelo botão `Jogar` sem congelar e que o painel azul ficou corretamente visível/encaixado.
- A composição principal do frontend foi posteriormente aproximada da disposição conceitual do Twintail, sem copiar código ou assets: o banner do jogo passou a ocupar toda a área útil, a biblioteca instalada permanece em uma barra lateral estreita, as informações do jogo ficam sobre o banner no canto inferior esquerdo e a ação principal fica numa barra flutuante no canto inferior direito.
- O cabeçalho alto e o painel lateral direito permanente foram removidos da composição principal. Com isso, a janela fica estática, sem necessidade de rolar para acessar funções comuns, e o jogo selecionado volta a ser o foco visual dominante.
- As ações secundárias foram transferidas para o drawer aberto pelo botão `⋯`. O drawer preserva localizar/abrir pasta, atualizar, verificar, desvincular, configurar e métodos de instalação sem competir visualmente com `Jogar`/`Baixar e instalar`.
- Para jogos com update remoto, especialmente RavenQuest, o progresso visível na tela principal agora é uma faixa fina e discreta acima da barra de ações, contendo status, etapa, porcentagem e barra de progresso. Clicar nessa faixa abre o drawer com o diagnóstico completo (fonte, evento, stage, arquivo, alvo e log).
- Validações desse redesign: `npm run build` passou; `npm run tauri dev` compilou e abriu a janela nativa; o usuário confirmou que o novo layout ficou bem encaixado, sem rolagem, com banner em tela cheia, botão principal no canto inferior direito e progresso sem cobrir o conteúdo.
- Foi implementado um fluxo genérico de instalação para métodos `installation.methods[].type: "archive"` em pacotes ZIP. O backend baixa em background com retry e headers opcionais, extrai em staging com proteção contra caminhos inseguros, aceita `stripTopLevelDir`, valida `launch.executable`, garante permissão executável no Linux, move os arquivos para `games/<game_id>`, registra a instalação no SQLite e inicia o jogo via runner do manifesto.
- O Medivia agora possui instalação Linux gerenciada pelo manifesto usando `https://download.medivia.online/medivia-linux-build.zip`, `format: "zip"`, `stripTopLevelDir: true` e `launchAfterInstall: true`. O ZIP oficial possui a pasta superior `medivia-linux-build/` e o executável nativo ELF x86-64 `medivia`; o manifesto usa `launch.runner: "native"` e `launch.executable: "medivia"`.
- Validação real do Medivia: o usuário clicou em `Baixar e instalar`; o launcher baixou aproximadamente 90 MB, extraiu os arquivos, registrou `/home/kaiquelb/.local/share/dev.kaiquelb.2d-mmo-launcher/games/medivia` no SQLite e abriu o cliente automaticamente. O `runner.log` confirmou `action=launch_after_archive_install`, runner `native`, working dir da instalação, PID e inicialização do Medivia x86-64. `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml` e `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` passaram.
- O Archlight agora possui instalação portátil gerenciada pelo manifesto usando `https://dw.archlightonline.com/abaldar.zip`. O ZIP oficial tem aproximadamente 274 MB, não possui pasta superior e contém `abaldar.exe`, `libEGL.dll` e `libGLESv2.dll`; portanto usa método `archive` sem `stripTopLevelDir`, mantém `launch.runner: "proton"`, define `launch.executable: "abaldar.exe"` e inicia automaticamente após extrair, sem executar instalador Windows.
- Validação real do Archlight: o usuário confirmou download, extração e abertura do jogo. A instalação foi registrada em `/home/kaiquelb/.local/share/dev.kaiquelb.2d-mmo-launcher/games/archlight`; o `runner.log` confirmou resolução para `system-umu-run` (`/usr/bin/umu-run`), prefixo isolado em `compat-data/archlight/proton`, execução com `UMU-Proton-10.0-4`, PID iniciado e reabertura posterior pelo botão `Jogar`. `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml` e `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` passaram.
- A entrada de catálogo do Zezenia foi removida e substituída pelo PokeMMO (`id: "pokemmo"`). O manifesto usa o endpoint oficial `https://pokemmo.com/download_file/1/`, que redireciona para o ZIP Linux atual de aproximadamente 96 MB. O pacote não possui pasta superior e usa `PokeMMO.sh` como entrada nativa; o script exige o working directory dos arquivos e inicia `PokeMMO.exe` como classpath Java. O método `archive` inicia automaticamente após instalar e o backend garante permissão executável ao script.
- Validação real do PokeMMO: o usuário confirmou que ele apareceu no catálogo, baixou, extraiu e abriu. A instalação foi registrada em `/home/kaiquelb/.local/share/dev.kaiquelb.2d-mmo-launcher/games/pokemmo`; o `runner.log` confirmou JVM com `-Xmx384M`, OpenGL/Mesa inicializado, carregamento da interface e encerramento normal com código `0`. O sistema testado tinha `/usr/bin/java` disponível. `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml` e `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` passaram.
- A ação `Verificar arquivos` agora é funcional e genérica. O manifesto aceita `verification.requiredFiles` opcional; o comando Tauri `verify_game_install(gameId)` confere a pasta registrada, resolve o executável efetivo e verifica os caminhos obrigatórios relativos à instalação, retornando resultado estruturado para a UI.
- O executável efetivo respeita o fluxo real de lançamento: em jogos comuns usa `launch.executable`; quando `launch.battlEye.launchMode: "main"`, como no RavenQuest, verifica o executável do BattlEye resolvido pela base configurada (`compatPrefix`), portanto valida `ravenquest_dx_BE.exe` em vez de `launcher.exe`.
- A verificação não baixa, apaga nem substitui arquivos. Ela apenas diagnostica e informa `repairStrategy`: `remoteManifest` quando há reparo remoto disponível, depois `archive`, `windowsInstaller` ou `existing` conforme os métodos declarados. A reparação permanece uma ação explícita separada para evitar efeitos destrutivos inesperados.
- Arquivos obrigatórios mínimos foram declarados nos manifestos atuais: PokeMMO (`PokeMMO.sh`, `PokeMMO.exe`, `data`, `roms`), PokeXGames (`pxgme-linux`), Grand Line Adventures (`glaclient-linux`), Medivia (`medivia`) e Archlight (`abaldar.exe`, `libEGL.dll`, `libGLESv2.dll`). O RavenQuest usa lista adicional vazia porque o executável BattlEye efetivo já é verificado e o manifesto remoto completo continua sendo a fonte de integridade aprofundada.
- O drawer mostra estado íntegro ou de atenção, pasta, executável efetivo, estratégia de reparo, problemas e arquivos ausentes. Teste interativo no Tauri confirmou estado íntegro no PokeMMO e no RavenQuest, incluindo o caminho terminado em `ravenquest_dx_BE.exe`. Dois testes Rust cobrem arquivo obrigatório ausente e pasta de instalação ausente. Validações: `cargo test --manifest-path src-tauri/Cargo.toml` (2 testes), `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo fmt` e `git diff --check` passaram.
- O primeiro fluxo de reparo explícito foi conectado para instalações com `repairStrategy: "remoteManifest"`. Quando a verificação encontra problema, a pasta registrada ainda existe e o manifesto suporta update remoto, o drawer mostra `Reparar arquivos pelo manifesto`; jogos com estratégias `archive`, `windowsInstaller`, `existing` ou manuais continuam apenas com diagnóstico para evitar reinstalações destrutivas inesperadas.
- Update explícito e reparo compartilham uma única função no frontend e o mesmo `run_game_remote_update` transacional do backend, incluindo staging, validação, aplicação, eventos Tauri e fallback por `runner.log`. Ao terminar, o frontend chama `verify_game_install` novamente e substitui o diagnóstico antigo pelo estado atual.
- Teste real controlado do reparo no RavenQuest: `ravenquest_dx_BE.exe` foi movido temporariamente para backup, `Verificar arquivos` mostrou `Instalação requer atenção` e o CTA de reparo, o reparo remoto recriou o executável e a reverificação automática mudou para `Instalação íntegra`. `cmp` confirmou que o arquivo baixado era binariamente idêntico ao backup original; o backup temporário foi removido e a instalação ficou restaurada.
- A verificação genérica agora aceita `verification.checksums` opcional no manifesto, inicialmente com algoritmo `crc32`. Cada entrada declara `path`, `algorithm` e `value`; caminhos absolutos, travessia para fora da instalação, valores CRC inválidos e algoritmos desconhecidos são rejeitados pelo backend.
- `verify_game_install` calcula os checksums configurados sem alterar arquivos e retorna `checksumResults` com caminho, algoritmo, valor esperado, valor obtido opcional e validade. Um checksum ausente ou divergente torna a instalação não íntegra e aparece na lista de problemas, mas não habilita reparo remoto quando o manifesto não oferece `remoteManifest`.
- O drawer de verificação mostra uma seção `Checksums` com estado válido/divergente/ausente e os valores esperado/obtido. O PokeMMO declara o CRC32 real de `PokeMMO.sh` (`4a98704b`) como primeiro uso do recurso; os demais jogos continuam compatíveis sem declarar checksums.
- Testes Rust cobrem CRC32 válido, arquivo divergente, arquivo ausente, caminho inseguro/absoluto, algoritmo desconhecido e valor malformado. Validações da etapa: `cargo test --manifest-path src-tauri/Cargo.toml` (5 testes), `npm run build`, `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, validação de todos os JSONs e `git diff --check` passaram. No Tauri real, o usuário confirmou `Instalação íntegra` e `CRC32 válido` para o PokeMMO instalado.
- Configurações por jogo agora são persistidas na tabela SQLite `game_settings`, separada de `installs`, com `game_id`, `runner_override`, `env_overrides_json` e timestamps. Os comandos Tauri `get_game_settings`, `save_game_settings` e `reset_game_settings` validam o jogo pelo catálogo; overrides vazios restauram o fallback do manifesto e nomes de variáveis de ambiente são validados antes da persistência.
- A precedência de runner no launch é `game_settings.runner_override` → `installs.runner_override` legado → `launch.runner`; no updater externo, a configuração local também vence `update.runner`. O resolver aceita tanto categorias genéricas (`proton`, `wine`, `native`) quanto o ID exato de um runner detectado, permitindo selecionar uma instalação específica na UI sem quebrar os contratos antigos.
- Overrides locais de ambiente são mesclados por último em `launch.env` e `update.env`, portanto vencem os defaults declarados no manifesto e continuam usando a expansão existente de `~/`, `$HOME/` e `${HOME}/`. O mesmo manifesto efetivo é usado no launch normal, auto-launch pós-instalação, update externo e update remoto; jogos sem registro em `game_settings` mantêm o comportamento anterior.
- O drawer `⋯` ganhou uma seção funcional `Configurações locais`, aberta por `Configurar` ou `Configurar runner`. Ela oferece o runner padrão mais runners detectados disponíveis, gera inputs genericamente a partir de `launch.env`, mostra os valores do manifesto como defaults e permite salvar ou restaurar padrões. No RavenQuest isso expõe, entre outras variáveis, `PROTONPATH`, `PROTON_BATTLEYE_RUNTIME` e `PROTON_EAC_RUNTIME`, sem regra condicionada ao ID do jogo.
- Validação real das configurações por jogo: no Tauri recompilado, o usuário confirmou que o formulário do RavenQuest abriu, persistiu um override após fechar/reabrir e restaurou corretamente os defaults. `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml` (5 testes), `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` e `git diff --check` passaram.
- O seletor de runner das configurações locais usa esquema de cores escuro também no menu nativo aberto (`color-scheme: dark` e cores explícitas nas opções), corrigindo o baixo contraste visto no WebKitGTK. O usuário confirmou no Tauri real que a lista ficou escura e legível; `npm run build` e `git diff --check` passaram após o ajuste.
- A camada SQLite foi extraída de `src-tauri/src/lib.rs` para `src-tauri/src/database.rs`. O módulo concentra abertura do banco, modelos persistidos, queries de `installs`/`game_settings` e migrations, enquanto os comandos Tauri e regras de catálogo/launch permanecem em `lib.rs`, sem alterar os contratos expostos ao frontend.
- O schema agora é versionado por `PRAGMA user_version`, atualmente na versão `3`: migration 1 cria/reconcilia `installs`, migration 2 cria/reconcilia `game_settings` e migration 3 cria `runners` para versões gerenciadas pelo launcher. Cada migration roda em transação e usa `CREATE TABLE IF NOT EXISTS`, permitindo que bancos legados na versão `0`, já contendo as tabelas anteriores, sejam adotados sem apagar ou recriar dados. Bancos com versão futura desconhecida são rejeitados para evitar escrita incompatível.
- Testes Rust do módulo de banco cobrem criação de banco vazio, adoção de schema legado preservando instalação e ciclo de persistência/reset de configurações. A suíte passou com 8 testes no total, além de `cargo check`, `cargo fmt`, `npm run build` e `git diff --check`. No banco real, a abertura do Tauri migrou `user_version` de `0` para `2`, preservou 6 instalações e 1 registro de settings; o usuário confirmou visualmente que RavenQuest continuou instalado e com configurações locais preenchidas.
- O launcher agora gerencia a release mais recente do Proton-GE. `src-tauri/src/managed_runners.rs` consulta a API oficial de releases do `GloriousEggroll/proton-ge-custom`, seleciona o asset `.tar.gz`, baixa em uma tarefa bloqueante separada e emite `runner-install-progress` com catálogo, download, extração, aplicação, conclusão ou erro.
- A instalação do Proton-GE usa staging em `app_data/runners/.staging`, valida caminhos do TAR contra absolutos/travessia, exige encontrar o executável `proton` e somente então move a pasta validada para `app_data/runners/proton-ge/<versão>` e registra o runner na tabela `runners`. O backend valida o tamanho informado pelo asset; verificação criptográfica do asset ainda pode ser adicionada futuramente.
- A descoberta em `runners.rs` passou a consumir os registros persistidos, validar se o executável continua disponível e expor `managed`, `version` e `canRemove`. IDs gerenciados são estáveis (`managed-proton-ge-<versão-normalizada>`) e já funcionam no resolver exato e no seletor existente por jogo, sem hardcode por game ID.
- O drawer `⋯` ganhou uma seção compacta `Runners gerenciados`: consulta a versão mais recente, mostra tamanho/estado, instala com progresso e lista/remove versões gerenciadas. A remoção exige confirmação, aceita somente registros de origem `Launcher` e valida por canonicalização que o caminho está dentro de `app_data/runners` antes de apagar.
- Validação real do Proton-GE gerenciado: o Tauri compilou e abriu, o catálogo exibiu corretamente a release mais recente, o usuário confirmou o download/extração/aplicação e a versão instalada apareceu tanto na lista `Instalados` quanto no seletor de runner das configurações por jogo. O ciclo de remoção também foi validado no drawer: após confirmação, a versão sumiu da lista e do seletor; uma reinstalação completa subsequente terminou com sucesso e restaurou ambas as entradas. Validações automatizadas: `cargo test --manifest-path src-tauri/Cargo.toml` passou com 11 testes, `npm run build`, `cargo fmt` e `git diff --check` passaram.
- Ainda existem metadados visuais temporários por jogo no frontend, como abreviação, gradiente e categoria curta; eles não devem conter regra de negócio.

## Onde prosseguir daqui

Próximo passo recomendado para desenvolvimento:

1. **Completar dados de execução nos manifestos**
   - Definir `launch.executable` para outros jogos nativos quando houver executável conhecido.
   - Avaliar se o manifesto precisa de campos adicionais para validação de pasta, executáveis alternativos ou argumentos por plataforma.

2. **Evoluir reparo das instalações**
   - O reparo explícito por `remoteManifest` já está implementado e validado no RavenQuest, sem disparar downloads automaticamente durante a verificação.
   - Checksums CRC32 opcionais por manifesto já estão implementados para jogos sem manifesto remoto; manter `requiredFiles` como checagem estrutural rápida e declarar checksums adicionais apenas quando houver valores estáveis/confiáveis para a versão distribuída.
   - Definir fluxos não destrutivos antes de habilitar reparo para `archive` ou `windowsInstaller`; até lá, essas estratégias permanecem apenas como orientação diagnóstica.

3. **Camada de runners**
   - Validar o botão `Baixar instalador Windows` do RavenQuest em ambiente com Proton/UMU disponível.
   - Se o instalador não abrir, consultar `logs/ravenquest/runner.log` no diretório de dados do app para analisar stdout/stderr do Wine/Proton.
   - Validar o auto-launch pós-instalação do RavenQuest (`launchAfterInstall`) após uma instalação limpa e após uma instalação reconciliada de prefixo antigo.
   - Validar o RavenQuest em execução real com BattlEye e conferir no `runner.log` se aparecem `main_executable_replaced_by_battl_eye=true`, `battl_eye_launch_mode=main`, `env.PROTONPATH=...GE-Proton11-1`, `env.PROTON_BATTLEYE_RUNTIME=...battleye_runtime`, `unset_env.GAMEID=true` e `unset_env.STORE=true`.
   - Retestar o botão `Jogar` após reiniciar/recompilar o Tauri e conferir que uma nova `action=launch_game` aparece imediatamente no `runner.log`, sem `action=run_game_remote_update` automático antes dela. A atualização completa deve ocorrer apenas ao clicar em `Atualizar arquivos do jogo`.
   - Testar a ação explícita de update/verificação remota do RavenQuest instalado e conferir se a UI mostra o painel compacto sempre visível e o drawer detalhado com barra, porcentagem, stage atual, último evento, fonte do progresso (`evento Tauri` ou fallback `runner.log`), pasta alvo, log e arquivo atual sem congelar. No `runner.log`, conferir `action=run_game_remote_update`, `update_strategy=remoteManifest`, `remote_update_stage=...`, `remote_binary_file=Some("/ravenquest_dx.exe")` ou caminho equivalente vindo do manifesto remoto, `remote_update_progress=checking`, `planned_update_files=...`, `remote_update_parallel_downloads=true`, `remote_update_download_workers=8`, `remote_update_staging_dir=...`, `remote_update_progress=downloading`, `remote_update_download_validation=parallel_worker`, checkpoints agregados de 100 em 100 arquivos quando a UI estiver na fonte `runner.log`, stages `validateStagedFiles`/`applyStagedFiles`, checkpoints `remote_update_progress=applying` de 50 em 50 arquivos durante a aplicação, `remote_update_staging_validation=completed_during_parallel_download`, URLs com caracteres especiais escapados (ex.: `%23` para `#`) e retry quando houver erro transitório.
   - Se o jogo ainda reclamar anti-cheat, conferir se os runtimes do Lutris existem nos caminhos efetivos (defaults do manifesto ou overrides locais) e se o runner resolvido é `system-umu-run`; os caminhos e o runner já podem ser ajustados em `⋯ → Configurar`.
   - Testar execução via Wine quando houver jogo/instalador Windows simples e Wine disponível.
   - Validar RavenQuest com Proton usando o prefixo gerenciado criado em `compat-data/ravenquest/proton`.
   - Evoluir `game_settings` somente quando necessário para prefixo ou outros campos além de runner e ambiente; runner e caminhos locais de Proton/BattlEye/EAC já são persistidos pela UI.
   - Ajustar variáveis de ambiente adicionais de Proton/UMU conforme necessário após teste real.
   - Instalar/testar `umu-run` (`umu-launcher` no Arch/AUR conforme disponibilidade) para validar RavenQuest com ambiente Proton mais adequado fora da Steam.
   - O fluxo inicial para instalar/registrar/remover o Proton-GE mais recente já está implementado; evoluir depois para múltiplas versões do catálogo, verificação criptográfica do pacote e Wine gerenciado.
   - Implementar suporte progressivo a Wine/Proton para RavenQuest e Archlight.
   - Usar o instalador Windows do RavenQuest como base de teste para Proton/Wine.
   - Registrar sessão para futuro tempo jogado quando o spawn for bem-sucedido.

4. **Evoluir persistência modular**
   - O módulo `database` e migrations via `PRAGMA user_version` já estão implementados e validados com banco legado real.
   - Adicionar migrations incrementais no array `MIGRATIONS` de `src-tauri/src/database.rs`; nunca editar retroativamente uma migration já distribuída.
   - Usar o módulo para futuras tabelas de runners gerenciados, sessões e downloads, mantendo SQL e modelos persistidos fora de `lib.rs`.

5. **Depois avançar para download/instalação automática**
   - Só iniciar depois que catálogo, instalações existentes e execução básica estiverem bem definidos.

Critério de arquitetura: sempre que uma funcionalidade parecer específica demais para um jogo, tentar modelar como manifesto, runner, método de instalação ou configuração persistida.

## Preferências de colaboração

- Responder em **pt-BR**.
- Explicar mudanças de forma direta e prática.
- Antes de editar arquivos importantes, conferir padrões existentes do projeto.
- Manter arquitetura extensível e evitar acoplamento desnecessário.
- Ao validar visual, lembrar que o usuário quer ver no **Tauri**, não só no navegador.
- Durante mudanças funcionais, executar testes intermediários no Tauri real sempre que possível, pedir explicitamente ao usuário o resultado observado e usar esse feedback antes de avançar para a próxima correção ou criar commit. Não acumular várias suposições sem checkpoints interativos quando o comportamento depender da janela nativa, processos externos, Wine/Proton/UMU ou jogo real.

## Fluxo obrigatório de etapas Git

- Ao concluir cada etapa funcional aprovada pelo usuário, atualizar este próprio `AGENTS.md` com o estado recente, decisões importantes, próximos passos e qualquer nova regra operacional definida durante a etapa.
- A atualização do `AGENTS.md` deve acontecer antes do commit da etapa, para que o contexto versionado acompanhe a evolução real do projeto.
- Ao concluir cada etapa funcional aprovada pelo usuário, criar um commit Git específico para aquela etapa.
- Depois do commit local, subir as alterações para o remoto configurado com `git push` antes de iniciar a próxima etapa.
- Antes de commitar, revisar `git status` e, quando útil, o diff para evitar incluir mudanças acidentais.
- Mensagens de commit devem ser curtas, descritivas e em português ou inglês técnico consistente com o histórico do projeto.
- Se `git push` falhar por credenciais, rede ou divergência com o remoto, informar o erro e aguardar orientação antes de prosseguir para a próxima etapa.