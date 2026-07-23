# ADR 0002 — SQLite para estado local

## Status

Aceita.

## Contexto

Instalações, caminhos, overrides e runners pertencem à máquina do usuário e não
ao catálogo distribuído.

## Decisão

Usar SQLite local com migrations versionadas por `PRAGMA user_version`.

## Consequências

- manifestos permanecem portáveis;
- banco pode evoluir sem apagar dados;
- migrations são incrementais e não podem ser editadas retroativamente;
- backend concentra acesso ao banco em módulo dedicado;
- testes de adoção de banco legado são obrigatórios.
