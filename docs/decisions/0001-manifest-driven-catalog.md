# ADR 0001 — Catálogo orientado a manifestos

## Status

Aceita.

## Contexto

O launcher precisa suportar vários MMORPGs com instalação, execução e update
diferentes sem espalhar regras por jogo no código.

## Decisão

Manifestos JSON são a fonte de descrição e capacidades dos jogos. O backend e a
UI interpretam campos genéricos. Estado local fica fora do manifesto.

## Consequências

- adicionar jogo tende a exigir manifesto e assets;
- schema precisa evoluir de forma compatível;
- tipos Rust e TypeScript precisam permanecer alinhados;
- exceções por ID são dívida técnica;
- comportamentos novos devem ser modelados como campos reutilizáveis.
