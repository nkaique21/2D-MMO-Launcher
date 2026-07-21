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
- O RavenQuest declara BattlEye apontando para `drive_c/Program Files (x86)/Tavernlight Games/RavenQuest/ravenquest_dx_BE.exe`, com `workingDir` na pasta `RavenQuest`, ambos resolvidos a partir do prefixo Proton (`compatPrefix`). O `belauncher.exe` de `system32` foi descartado como entrada principal porque encerra rapidamente sem abrir o jogo.
- `launch_game` e o auto-launch pós-instalação suportam dois modos de BattlEye:
  - modo padrão/anterior: iniciar BattlEye antes do processo principal;
  - `launch.battlEye.launchMode: "main"`: iniciar o BattlEye como processo principal, sem abrir `launch.executable` em paralelo.
- O backend registra no `runner.log` o comando, PID, variáveis aplicadas e variáveis removidas (`unset_env.*=true`) para facilitar diagnóstico.
- O RavenQuest foi ajustado para reproduzir o ambiente confirmado manualmente com UMU/Lutris: `PROTONPATH=~/.local/share/Steam/compatibilitytools.d/GE-Proton11-1`, runtimes `PROTON_BATTLEYE_RUNTIME`/`PROTON_EAC_RUNTIME` do Lutris, `WINEESYNC=1`, `WINEFSYNC=1`, `WINEARCH=win64`, `WINEDEBUG=-all`, e remoção de `GAMEID`/`STORE` para deixar o UMU usar `umu-default`.
- Validações executadas: `npm run build` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram.
- Diagnóstico do erro real “BattlEye service is not running”: o `BELauncher.ini` do RavenQuest declara `64BitExe=ravenquest_dx.exe`, e os scripts oficiais `Install_BattlEye.bat` chamam `..\\ravenquest_dx_BE.exe`. Portanto o fluxo mais provável é iniciar `ravenquest_dx_BE.exe` como entrada principal. Não deve exigir reinstalação de início; primeiro testar com o launcher recompilado/reiniciado.
- Diagnóstico posterior confirmou que executar `ravenquest_dx_BE.exe 1 0` não foi o método funcional no Linux. O método funcional foi iniciar `ravenquest_dx_BE.exe` via `umu-run` com o ambiente equivalente ao Lutris descrito acima.

### Próximos passos de teste real

- Rodar o RavenQuest pelo launcher recompilado/reiniciado e conferir `logs/ravenquest/runner.log` para confirmar `main_executable_replaced_by_battl_eye=true`, `battl_eye_launch_mode=main`, `env.PROTONPATH=...GE-Proton11-1`, `env.PROTON_BATTLEYE_RUNTIME=...battleye_runtime`, `unset_env.GAMEID=true` e `unset_env.STORE=true`.
- Se a máquina não tiver os runtimes do Lutris nos caminhos esperados, instalar/baixar os runtimes ou tornar esses caminhos configuráveis pela UI/SQLite em etapa futura.
