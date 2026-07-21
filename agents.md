## Plano de trabalho — BattlEye no launcher

Contexto atual: o launcher Tauri já consegue abrir o jogo corretamente. O próximo objetivo é fazer o componente BattlEye/anti-cheat também iniciar quando o jogo exigir, mantendo o comportamento atual dos jogos que não usam BattlEye.

### Objetivo

- Integrar a execução do BattlEye ao fluxo de lançamento do cliente, sem quebrar os runners já existentes.

### Entregáveis previstos

- Mapear como os manifests descrevem executáveis/argumentos atuais e onde o BattlEye deve ser declarado.
- Ajustar o modelo Rust/TypeScript de manifest, se necessário, para suportar configuração opcional de BattlEye.
- Atualizar o runner Tauri responsável por iniciar o jogo para também iniciar o BattlEye quando configurado.
- Atualizar o(s) manifest(s) do(s) jogo(s) que dependem de BattlEye.
- Validar com build/checks locais e revisar compatibilidade com os jogos sem BattlEye.

### Critérios de sucesso

- Jogos sem BattlEye continuam abrindo como antes.
- Jogos com BattlEye conseguem iniciar o executável/launcher do BattlEye junto ao fluxo do jogo.
- O projeto compila sem erros de TypeScript/Rust.
- A alteração fica documentada neste arquivo para retomada caso o contexto seja perdido.

### Checklist de execução

- [x] Inspecionar `src-tauri/src/runners.rs`, `src-tauri/src/lib.rs` e manifests para entender o fluxo atual.
- [x] Identificar qual jogo/manifest precisa de BattlEye e qual executável/argumentos devem ser usados.
- [x] Modelar configuração opcional de BattlEye no manifest.
- [x] Implementar o start do BattlEye no runner com fallback seguro.
- [x] Atualizar manifest(s) necessários.
- [x] Rodar checks/builds possíveis.
- [x] Registrar achados finais e próximos passos.

### Resultado implementado

- O manifesto agora aceita `launch.battlEye` opcional. Sem esse bloco, os jogos continuam usando o fluxo antigo.
- O manifesto agora também aceita `launch.env` e `launch.unsetEnv` opcionais para ajustes de ambiente por jogo, sem hardcode no backend. Valores iniciados por `~/`, `$HOME/` ou `${HOME}/` são expandidos para o diretório home do usuário.
- O manifesto agora aceita `update.strategy: "externalLauncher"` com `runner`, `compatPrefix`, `executable`, `args`, `pathBase`, `workingDir`, `workingDirBase`, `env` e `unsetEnv` opcionais para delegar atualização a um launcher/updater externo por jogo.
- O manifesto também aceita `update.strategy: "remoteManifest"` com `manifestUrl`, `manifestFormat`, `targetDir` e `targetDirBase` para verificar/baixar arquivos a partir de um manifesto remoto.
- O RavenQuest declara BattlEye apontando para `drive_c/Program Files (x86)/Tavernlight Games/RavenQuest/ravenquest_dx_BE.exe`, com `workingDir` na pasta `RavenQuest`, ambos resolvidos a partir do prefixo Proton (`compatPrefix`). O `belauncher.exe` de `system32` foi descartado como entrada principal porque encerra rapidamente sem abrir o jogo.
- O RavenQuest agora declara `update.strategy: "remoteManifest"`, lendo `https://dw.ravenquest.io/ravenquest/checksums.txt.gz` no formato `ravenquestZlib` e aplicando arquivos em `drive_c/Program Files (x86)/Tavernlight Games/RavenQuest` a partir do prefixo Proton (`compatPrefix`).
- `launch_game` e o auto-launch pós-instalação suportam dois modos de BattlEye:
  - modo padrão/anterior: iniciar BattlEye antes do processo principal;
  - `launch.battlEye.launchMode: "main"`: iniciar o BattlEye como processo principal, sem abrir `launch.executable` em paralelo.
- O backend expõe `run_game_update`, que usa a configuração `update` do manifesto, registra `action=run_game_update` no `runner.log` e inicia o updater externo com o runner/env configurados quando a estratégia é `externalLauncher`.
- O backend expõe `run_game_remote_update`, que baixa/decodifica o manifesto remoto, verifica CRC32/tamanho dos arquivos locais e baixa apenas o que estiver ausente ou divergente quando a estratégia é `remoteManifest`.
- No update remoto, o backend inclui tanto `files` quanto `binary` do manifesto remoto na verificação/download. O campo `binary.file` é inserido no mesmo mapa de arquivos, sem duplicar caminho se ele também aparecer em `files`, para garantir que executáveis principais como `ravenquest_dx.exe` não fiquem fora do update. O `runner.log` registra `remote_binary_file=...`.
- O update remoto agora roda em tarefa bloqueante separada via `tauri::async_runtime::spawn_blocking`, evitando congelar a UI durante a verificação/download de muitos arquivos. Downloads HTTP usam timeout de 60s.
- O update remoto emite progresso durante a fase de verificação a cada 100 arquivos e registra checkpoints no `runner.log` a cada 1000 arquivos, além de registrar cada arquivo em download com `remote_update_progress=downloading`.
- O update remoto agora emite e registra fases detalhadas (`remote_update_stage=...`) desde o clique até a verificação/download: abrir banco, carregar instalação, carregar manifesto local, reconciliar instalação, mover para background, resolver manifesto remoto, resolver/preparar pasta alvo, baixar/decodificar manifesto remoto, montar lista, verificar arquivos, baixar divergentes, validar/aplicar arquivos e concluir/erro.
- O frontend exibe ações secundárias de update conforme a estratégia declarada no manifesto, preservando `Atualizar pelo launcher oficial` para `externalLauncher` e a verificação/update remoto para `remoteManifest` quando disponível.
- O frontend agora exibe um painel visual de atualização no hero, com barra progressiva, porcentagem, fase atual, arquivo em processamento e contadores de arquivos verificados/baixados, para evitar a sensação de travamento em updates grandes.
- O painel visual de atualização agora inclui um stepper/timeline de diagnóstico com estados `✓`, `●`, `○` e `!`, além de bloco com stage bruto, último evento recebido, status, pasta alvo e caminho do log. Ao clicar em update remoto, o frontend cria imediatamente um progresso local `preparing/start` para não ficar preso apenas na mensagem “Preparando atualização dos arquivos...”.
- O painel visual de atualização agora também mostra a fonte do progresso (`local`, `evento Tauri` ou `runner.log`) e faz fallback por polling de `get_game_update_progress(gameId)` quando eventos `game-update-progress` não chegam ou ficam parados. Esse comando reconstrói o estado lendo o `runner.log`, permitindo que a UI avance mesmo quando o listener de eventos falhar.
- O update remoto agora percent-encode segmentos de URL dos arquivos remotos, evitando falhas em nomes com `#` e outros caracteres reservados. Downloads de arquivos também usam retry com backoff, registram tentativas no `runner.log` e reutilizam um único cliente HTTP durante a execução para reduzir overhead em milhares de arquivos pequenos.
- O backend registra no `runner.log` o comando, PID, variáveis aplicadas e variáveis removidas (`unset_env.*=true`) para facilitar diagnóstico.
- O RavenQuest foi ajustado para reproduzir o ambiente confirmado manualmente com UMU/Lutris: `PROTONPATH=~/.local/share/Steam/compatibilitytools.d/GE-Proton11-1`, runtimes `PROTON_BATTLEYE_RUNTIME`/`PROTON_EAC_RUNTIME` do Lutris, `WINEESYNC=1`, `WINEFSYNC=1`, `WINEARCH=win64`, `WINEDEBUG=-all`, e remoção de `GAMEID`/`STORE` para deixar o UMU usar `umu-default`.
- Validações executadas: `cargo fmt --manifest-path src-tauri/Cargo.toml`, `npm run build` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram, incluindo o ajuste que mescla `binary` + `files` no update remoto, o progresso visual/não-bloqueante do update e a instrumentação visual por etapas.
- Diagnóstico do erro real “BattlEye service is not running”: o `BELauncher.ini` do RavenQuest declara `64BitExe=ravenquest_dx.exe`, e os scripts oficiais `Install_BattlEye.bat` chamam `..\\ravenquest_dx_BE.exe`. Portanto o fluxo mais provável é iniciar `ravenquest_dx_BE.exe` como entrada principal. Não deve exigir reinstalação de início; primeiro testar com o launcher recompilado/reiniciado.
- Diagnóstico posterior confirmou que executar `ravenquest_dx_BE.exe 1 0` não foi o método funcional no Linux. O método funcional foi iniciar `ravenquest_dx_BE.exe` via `umu-run` com o ambiente equivalente ao Lutris descrito acima.

### Próximos passos de teste real

- Rodar o RavenQuest pelo launcher recompilado/reiniciado e conferir `logs/ravenquest/runner.log` para confirmar `main_executable_replaced_by_battl_eye=true`, `battl_eye_launch_mode=main`, `env.PROTONPATH=...GE-Proton11-1`, `env.PROTON_BATTLEYE_RUNTIME=...battleye_runtime`, `unset_env.GAMEID=true` e `unset_env.STORE=true`.
- Testar a ação de update/verificação remota no RavenQuest instalado e conferir se a UI mostra a barra/porcentagem/arquivo atual sem congelar. No `runner.log`, conferir `action=run_game_remote_update`, `update_strategy=remoteManifest`, `remote_binary_file=Some("/ravenquest_dx.exe")` ou caminho equivalente vindo do manifesto remoto, `remote_update_progress=checking` e `remote_update_progress=downloading` quando houver arquivos divergentes.
- No mesmo teste de update remoto, conferir no painel de diagnóstico se a fonte muda de `local` para `evento Tauri` quando os eventos chegam, ou para `runner.log` quando o fallback por log assumir o progresso.
- Se o update falhar em algum arquivo específico, conferir no `runner.log` `download_url=...`, `download_attempt=...` e `download_error=...`; caminhos com `#` devem aparecer escapados como `%23` na URL.
- Se a máquina não tiver os runtimes do Lutris nos caminhos esperados, instalar/baixar os runtimes ou tornar esses caminhos configuráveis pela UI/SQLite em etapa futura.
