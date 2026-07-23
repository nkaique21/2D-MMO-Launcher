# ADR 0006 — Catálogo oficial remoto com cache e fallback

## Status

Aceita.

## Contexto

Manifestos embutidos exigem uma nova release do launcher para corrigir URLs,
argumentos, assets e configurações de jogos. O produto é orientado a manifestos
e deve receber novos jogos sem alteração de código quando o schema já suportar
o comportamento.

## Decisão

Manter um repositório público separado chamado
`2D-MMO-Launcher-Catalog`. O launcher consulta `catalog.json`, baixa e valida
todos os manifestos em staging e ativa o conjunto de forma transacional.

O último cache remoto válido é a fonte principal. Manifestos empacotados no
bundle permanecem como fallback offline e de recuperação.

## Consequências

- correções de catálogo deixam de depender de release do aplicativo;
- o launcher abre offline;
- falha remota não invalida o cache anterior;
- manifestos remotos precisam de schema e validações rígidas;
- o repositório oficial vira superfície de segurança crítica;
- assets continuam remotos nesta etapa;
- assinatura criptográfica é recomendada como evolução futura.
