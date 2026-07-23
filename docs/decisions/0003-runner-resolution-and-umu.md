# ADR 0003 — Resolução de runners e preferência por UMU

## Status

Aceita.

## Contexto

Jogos Windows podem precisar de Wine, Proton ou versões concretas instaladas no
sistema ou gerenciadas pelo launcher.

## Decisão

Separar detecção, resolução e instalação. Permitir categorias e IDs concretos.
Preferir UMU para executar Proton fora da Steam quando disponível.

## Consequências

- manifestos não fixam necessariamente um caminho de runner;
- configuração local pode escolher versão concreta;
- prefixos são isolados;
- ambiente e variáveis removidas fazem parte do comando resolvido;
- jogos com anti-cheat podem configurar runtimes sem hardcode por ID.
