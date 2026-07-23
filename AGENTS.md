# AGENTS.md — Regras para agentes de desenvolvimento

Este arquivo define **como Codex, Devstral, Cline e outros agentes devem trabalhar neste repositório**.
Ele contém regras permanentes, não histórico do projeto.

> Nunca transforme `AGENTS.md` em changelog, plano de tarefa, diário ou depósito de logs.

## 1. Ordem de leitura

Antes de alterar algo:

1. Leia `AGENTS.md`.
2. Leia `PROJECT.md`.
3. Leia `STATE.md`.
4. Leia somente os documentos de `docs/` relacionados à tarefa.
5. Consulte `MEMORY.md` quando precisar de decisões ou fatos duráveis.
6. Inspecione o código relacionado.
7. Execute `git status --short`.

Não leia `docs/archive/` por padrão. Ele serve apenas para recuperar histórico.

## 2. Prioridades

1. Pedido atual do usuário.
2. Estado e regras específicas do projeto.
3. Este arquivo.
4. Convenções existentes no código.
5. Suposições do agente.

Quando faltar informação, não invente. Procure no código/documentação ou diga exatamente o que precisa ser confirmado.

## 3. Ambiente e comunicação

- Sistema principal: CachyOS/Arch Linux.
- Shell: `fish`.
- Responda em pt-BR.
- Prefira comandos curtos, copiáveis e compatíveis com Fish.
- Evite heredocs, Bash avançado e scripts inline extensos.
- Para lógica longa, crie ou edite um script real.
- Não use `sudo`, instale pacotes ou altere configuração global sem necessidade explicada.
- Seja direto, prático e transparente.
- Não esconda erros, limitações ou validações pendentes.
- Não peça novamente informação já fornecida.

## 4. Antes de editar

- Localize os módulos envolvidos e onde são usados.
- Identifique contratos públicos, efeitos colaterais e testes próximos.
- Leia o documento temático correspondente em `docs/`.
- Confirme quais arquivos realmente precisam mudar.
- Não assuma comportamento de API, biblioteca ou código não inspecionado.
- Não faça reescrita ampla quando uma alteração pequena resolver.

## 5. Forma de trabalhar

- Faça mudanças pequenas e incrementais.
- Resolva uma responsabilidade por etapa.
- Preserve comportamento fora do escopo.
- Evite refatorações e dependências não solicitadas.
- Prefira configuração a hardcode.
- Evite condicionais por ID de jogo.
- Separe UI, regra de negócio, persistência e integrações.
- Não crie abstração complexa sem necessidade real.
- Preserve estilo, nomes e padrões do projeto.
- Não edite arquivos gerados, vendorizados ou de build.

Quando algo parecer específico de um jogo, tente representá-lo como:

- campo de manifesto;
- runner;
- método de instalação;
- estratégia de update/reparo;
- configuração persistida;
- módulo reutilizável.

Exceções hardcoded devem ser pequenas, explícitas e registradas como dívida.

## 6. Segurança

- Não execute comando destrutivo sem autorização explícita.
- Evite `rm -rf`, `git reset --hard`, `git clean -fd` e reescrita de histórico.
- Desvincular instalação nunca deve apagar arquivos do jogo.
- Valide e canonicalize caminhos antes de extrair, mover ou remover.
- Rejeite caminho absoluto ou travessia em conteúdo externo.
- Nunca grave segredo, token ou senha no repositório.
- Não esconda falhas com catch vazio ou retorno silencioso.

## 7. Comandos e validação

Prefira comandos curtos:

```fish
git status --short
git diff --stat
git diff --name-status
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Após mudar código:

1. Revise o diff.
2. Execute o teste mais próximo.
3. Rode build/type-check/lint/testes aplicáveis.
4. Confira warnings relevantes e arquivos não relacionados.
5. Para UI, valide no Tauri nativo.
6. Para runners, updates, instaladores ou anti-cheat, confira logs reais.

Comandos padrão:

```fish
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Não declare sucesso sem evidência. Não repita indefinidamente um comando que falhou; investigue a causa.

## 8. Checkpoints na máquina do usuário

Quando a validação depender de Tauri, seletor nativo, processo externo, Wine, Proton, UMU, jogo real ou anti-cheat:

1. faça primeiro toda validação automatizada possível;
2. peça um teste curto e objetivo;
3. informe ação, resultado esperado e trecho de log a verificar;
4. use o retorno antes de avançar ou commitar.

Não acumule várias suposições sem checkpoint real.

## 9. Protocolo obrigatório de documentação

### `AGENTS.md`

Somente regras permanentes. Não adicionar histórico, tarefa atual ou logs.

### `PROJECT.md`

Verdade atual resumida: objetivo, arquitetura, funcionalidades, limites e roadmap. Substitua informação antiga; não narre a cronologia.

### `STATE.md`

Somente a etapa ativa: objetivo, estado, arquivos, critérios, validação, bloqueios e próximo passo. Substitua o conteúdo ao trocar de etapa.

### `MEMORY.md`

Somente fatos duráveis, decisões confirmadas, invariantes, armadilhas e comportamentos validados. Atualize ou remova fatos obsoletos.

### `docs/*.md`

Detalhes por domínio. Leia e atualize apenas o documento afetado.

### `docs/decisions/*.md`

Decisões arquiteturais duráveis. Não criar ADR para correção trivial.

### `docs/archive/`

Histórico encerrado e investigações longas. Fora do contexto padrão.

## 10. Limites de contexto

- `AGENTS.md`: até ~220 linhas.
- `PROJECT.md`: até ~220 linhas.
- `MEMORY.md`: até ~180 linhas.
- `STATE.md`: até ~70 linhas.
- Um tema por documento em `docs/`.
- Não repetir o mesmo fato em vários arquivos.
- Não colar logs extensos; registrar caminho, erro principal e conclusão.
- Quando um arquivo exceder seu papel, compacte-o antes de continuar.

## 11. Fechamento de uma etapa

Antes de concluir:

1. Atualize `PROJECT.md` se o produto atual mudou.
2. Atualize `MEMORY.md` somente se surgiu conhecimento durável.
3. Atualize o documento temático afetado.
4. Crie ADR apenas para decisão arquitetural.
5. Atualize `STATE.md` com resultado e próximo foco.
6. Não escreva histórico em `AGENTS.md`.
7. Revise contradições entre código e documentação.
8. Execute validações e revise `git status`/diff.

## 12. Git

- Não inclua arquivos não relacionados.
- Use commit curto e específico.
- Não use force push nem reescreva histórico sem pedido.
- Só commite após validação e aprovação do usuário quando houver teste real.
- Atualize documentação antes do commit.
- Faça push quando o fluxo estiver autorizado e antes da próxima etapa.
- Se commit/push não estiver autorizado, deixe os comandos prontos.
- Se push falhar, informe o erro e pare; não contorne credenciais ou divergências de forma arriscada.

## 13. Resumo final

Informe somente:

- arquivos alterados;
- comportamento implementado/corrigido;
- documentação atualizada;
- testes executados e resultados;
- checkpoint manual necessário;
- pendências reais.
