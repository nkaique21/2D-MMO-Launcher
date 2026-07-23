# ADR 0005 — Contexto documental em camadas

## Status

Aceita.

## Contexto

O antigo `AGENTS.md` acumulou regras, roadmap, histórico, diagnósticos e planos.
Isso consumia contexto excessivo em modelos locais e dificultava identificar a
verdade atual.

## Decisão

Separar documentação em:

- `AGENTS.md`: regras permanentes;
- `PROJECT.md`: estado atual;
- `STATE.md`: etapa ativa;
- `MEMORY.md`: fatos duráveis;
- `docs/`: detalhes por domínio;
- `docs/decisions/`: decisões;
- `docs/archive/`: histórico fora do contexto padrão.

## Consequências

- agentes carregam menos tokens;
- documentação precisa ser atualizada no arquivo correto;
- histórico antigo continua recuperável;
- duplicações devem ser removidas;
- Codex e agentes locais seguem o mesmo protocolo.
