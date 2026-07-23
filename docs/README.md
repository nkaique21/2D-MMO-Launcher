# Mapa da documentação

Leia somente o necessário para a tarefa.

| Documento | Quando ler |
|---|---|
| `architecture.md` | limites entre frontend, Tauri, banco e serviços |
| `manifests.md` | alterar schema, catálogo ou comportamento configurável |
| `runners.md` | launch, Wine, Proton, UMU ou runners gerenciados |
| `processes-playtime.md` | estado de execução, sessões e tempo acumulado |
| `installation-update-repair.md` | instalação, download, update, verificação e reparo |
| `database.md` | tabelas, queries ou migrations |
| `ui.md` | layout, fluxos, feedback e validação visual |
| `testing-and-git.md` | concluir etapa, validar, commitar ou publicar |
| `features/ravenquest-battleye.md` | RavenQuest, BattlEye e ambiente UMU |
| `decisions/` | entender por que escolhas arquiteturais existem |
| `archive/` | recuperar histórico antigo; não ler por padrão |

## Contexto mínimo recomendado

Para uma tarefa comum:

1. `AGENTS.md`
2. `PROJECT.md`
3. `STATE.md`
4. um documento temático
5. arquivos de código envolvidos

`MEMORY.md` deve ser consultado quando a tarefa tocar decisões ou problemas já
validados.

## Regra contra duplicação

Um fato deve ter uma fonte principal:

- regra de trabalho → `AGENTS.md`;
- estado atual do produto → `PROJECT.md`;
- tarefa em andamento → `STATE.md`;
- fato durável → `MEMORY.md`;
- detalhe de domínio → documento temático;
- justificativa de decisão → ADR;
- histórico encerrado → `archive/`.
