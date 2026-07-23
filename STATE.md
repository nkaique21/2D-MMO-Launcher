# STATE.md — Estado de trabalho atual

## Etapa atual

### Objetivo
Generalizar a instalação por archive e validar o Tibia distribuído em TAR.GZ.

### Estado
- implementação concluída; aguardando compilação Rust e teste real no Tauri

### Implementado
- módulo `src-tauri/src/archive.rs` para resolução e extração centralizadas;
- suporte a `zip`, `tar`, `tar.gz`/`tgz` e `tar.bz2`/`tbz2`;
- inferência por extensão quando `format` não é informado;
- aliases normalizados e erro com lista de formatos aceitos;
- staging e `stripTopLevelDir` preservados para todos os formatos;
- permissões Unix restauradas quando disponíveis;
- TAR restrito a arquivos regulares e diretórios;
- recusa de path traversal, paths absolutos, links e arquivos especiais;
- testes unitários da resolução de formatos;
- documentação atualizada.

### Validação automatizada
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `npm run build`
- [ ] `git diff --check`

### Checkpoint manual
1. Publicar o manifesto do Tibia com `format: "tar.gz"`.
2. Atualizar o catálogo no launcher.
3. Baixar e instalar o Tibia.
4. Confirmar que `Tibia` foi encontrado após remover a pasta superior.
5. Abrir e fechar o jogo, conferindo processo e tempo jogado.

### Próximo passo
Após o teste real, registrar o Tibia como validação do fluxo genérico e fechar a estabilização do MVP.
