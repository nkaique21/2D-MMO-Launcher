# STATE.md — Estado de trabalho atual

## Etapa atual

### Objetivo
Validar no Tauri real a camada genérica de atividade e tempo jogado.

### Estado
- implementação concluída;
- frontend TypeScript/Vite validado;
- validação Rust pendente no ambiente do usuário porque o sandbox não possui
  `cargo`/`rustc`.

### Implementado
- migration 4 e tabela `playtime_sessions`;
- criação, finalização, recuperação, listagem e resumo de sessões;
- `ProcessManager` no estado Tauri;
- estados `starting`, `running`, `exited` e `failed`;
- bloqueio de launch duplicado;
- monitoramento do processo principal fora de locks;
- eventos `game-process-state` e `game-activity-updated`;
- comandos de atividade e histórico;
- tempo acumulado e sessão ativa na interface.

### Validação concluída
- [x] `tsc`
- [x] build Vite
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] teste no Tauri com jogo nativo
- [ ] teste no Tauri com UMU/Proton

### Checkpoint manual
1. Rodar os três comandos Cargo.
2. Abrir `npm run tauri dev`.
3. Iniciar Medivia ou PokeMMO e confirmar `Em execução`/`Jogando`.
4. Tentar clicar novamente e confirmar bloqueio do launch duplicado.
5. Fechar o jogo e confirmar aumento do tempo acumulado.
6. Repetir com RavenQuest ou Archlight para validar o lifecycle do runner.

### Próximo passo
Após validação real, corrigir qualquer runner que se desacople do `Child` ou
avançar para histórico detalhado de sessões na UI.
