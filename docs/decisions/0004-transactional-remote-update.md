# ADR 0004 — Update remoto transacional

## Status

Aceita.

## Contexto

Aplicar arquivos enquanto ainda são baixados pode deixar instalações
parcialmente atualizadas. O RavenQuest possui um manifesto remoto muito grande.

## Decisão

Verificar primeiro, criar plano, baixar divergências em staging, validar e
somente então aplicar. Usar concorrência limitada, retry e progresso estruturado.

## Consequências

- mais uso temporário de disco;
- menor risco de instalação quebrada;
- falhas são conhecidas antes da aplicação;
- tarefas longas precisam rodar fora da thread da UI;
- logs devem ser agregados;
- UI recebe eventos e possui fallback por log.
