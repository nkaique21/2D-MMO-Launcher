# Como instalar esta estrutura no repositório

## Conteúdo

Este pacote contém:

```text
AGENTS.md
PROJECT.md
MEMORY.md
STATE.md
docs/
```

O antigo contexto completo foi preservado em:

```text
docs/archive/AGENTS-legacy-2026-07.md
docs/archive/battleye-plan-legacy-2026-07.md
```

## Cópia segura

Abra o terminal na raiz do projeto e copie os arquivos do pacote para lá.

Antes, confira o estado atual:

```fish
git status --short
```

Se já existir um `AGENTS.md`, salve uma cópia:

```fish
cp AGENTS.md AGENTS.before-context-refactor.md
```

Depois copie a nova estrutura mantendo `docs/`:

```fish
cp -r /caminho/2d-mmo-launcher-ai-context/. .
```

Revise:

```fish
git status --short
git diff --stat
git diff -- AGENTS.md PROJECT.md MEMORY.md STATE.md docs
```

## Primeira mensagem para o Devstral/Codex

Use algo como:

```text
Leia AGENTS.md, PROJECT.md e STATE.md. Depois leia apenas a documentação
temática relacionada à tarefa. Não use docs/archive como contexto padrão.
Siga o protocolo de atualização documental antes de concluir a etapa.
```

## Regra prática

Para uma tarefa de runner:

```text
AGENTS.md + PROJECT.md + STATE.md + docs/runners.md
```

Para update:

```text
AGENTS.md + PROJECT.md + STATE.md + docs/installation-update-repair.md
```

Para RavenQuest/BattlEye:

```text
AGENTS.md + PROJECT.md + STATE.md
+ docs/runners.md
+ docs/features/ravenquest-battleye.md
```

## Commit sugerido

Depois de revisar:

```fish
git add AGENTS.md PROJECT.md MEMORY.md STATE.md docs MIGRATION.md
git commit -m "docs: reorganiza contexto dos agentes"
```

Faça push somente conforme o fluxo autorizado do projeto.
