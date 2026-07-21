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
- O RavenQuest declara BattlEye apontando para `drive_c/windows/system32/belauncher.exe`, com `workingDir` na pasta `RavenQuest/BattlEye`, ambos resolvidos a partir do prefixo Proton (`compatPrefix`).
- `launch_game` e o auto-launch pós-instalação iniciam o BattlEye antes do processo principal quando o manifesto exigir.
- O backend registra no `runner.log` o comando, PID e erros do BattlEye para facilitar diagnóstico.
- Validações executadas: `npm run build` e `cargo check --manifest-path src-tauri/Cargo.toml` passaram.

### Próximos passos de teste real

- Rodar o RavenQuest pelo launcher e conferir `logs/ravenquest/runner.log` para confirmar `battl_eye_process_started=true`.
- Se o BattlEye abrir mas o jogo ainda reclamar anti-cheat, testar se o fluxo correto deve iniciar apenas `belauncher.exe` ou se também precisa ajustar o executável principal para `ravenquest_dx.exe`.
