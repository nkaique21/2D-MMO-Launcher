# STATE.md — Estado de trabalho atual

## Etapa atual

### Objetivo
Separar o catálogo oficial em um repositório remoto, mantendo cache local e fallback embutido.

### Estado
- implementação concluída; aguardando validação no Tauri real

### Implementado
- módulo `src-tauri/src/catalog.rs`;
- endpoint oficial `nkaique21/2D-MMO-Launcher-Catalog`;
- cache em staging e ativação transacional;
- fallback para manifestos empacotados;
- background refresh no startup;
- atualização manual e status no drawer;
- eventos `catalog-updated` e `catalog-update-failed`;
- `schemaVersion: 1` para manifestos remotos;
- manifestos embutidos incluídos como resources do bundle;
- documentação e ADR do catálogo.

### Validação automatizada
- [ ] `npm run build`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `git diff --check`

### Checkpoint manual
1. Criar e publicar o repositório `2D-MMO-Launcher-Catalog`.
2. Abrir o Tauri com internet e confirmar `Cache remoto oficial`.
3. Alterar `catalogVersion`, fazer push e clicar `Atualizar`.
4. Desconectar a internet e confirmar que o catálogo cacheado continua abrindo.
5. Remover temporariamente o cache e confirmar fallback embutido offline.

### Próximo passo
Após validação, fechar a estabilização do MVP e criar a tag `v0.1.0`.
