# Runners

## Responsabilidades

A camada de runners deve:

1. detectar runners disponíveis;
2. listar opções instaláveis;
3. resolver uma categoria ou ID concreto;
4. montar comando e ambiente;
5. definir prefixo;
6. fornecer diagnóstico;
7. permitir runners gerenciados.

Detecção e instalação são responsabilidades separadas.

## Tipos

- nativo;
- Wine;
- Proton;
- UMU;
- Steam;
- personalizado;
- runner gerenciado pelo launcher.

## Precedência

Para launch:

1. `game_settings.runner_override`;
2. `installs.runner_override` legado;
3. `launch.runner`.

Um override pode ser categoria genérica ou ID exato detectado.

## Prefixos

Direção padrão:

```text
compat-data/<game_id>/<runner_kind>
```

- Wine usa `WINEPREFIX`.
- Proton usa `STEAM_COMPAT_DATA_PATH`.
- Instalador e jogo podem compartilhar um prefixo compatível.
- Caminho não deve ser fixado a uma versão específica sem necessidade.

## UMU

Para Proton fora da Steam, UMU é preferido quando disponível.

O comando deve permitir:

- `PROTONPATH`;
- runtimes BattlEye/EAC;
- variáveis Wine;
- remoção de variáveis incompatíveis;
- working directory correto.

## RunnerCommand

O comando resolvido deve carregar de forma estruturada:

- programa;
- argumentos;
- working directory;
- variáveis a aplicar;
- variáveis a remover;
- prefixo;
- metadados úteis para log.

## Logs

Toda tentativa relevante deve registrar desde antes do spawn:

- ação;
- runner solicitado e resolvido;
- comando;
- working directory;
- ambiente relevante;
- variáveis removidas;
- PID;
- exit status;
- erro pré-spawn ou de spawn.

Não registrar segredos.

## Runners gerenciados

Fluxo seguro:

1. consultar catálogo;
2. baixar;
3. validar metadados disponíveis;
4. extrair em staging;
5. bloquear caminhos inseguros;
6. validar executável esperado;
7. mover para pasta final;
8. registrar no SQLite;
9. expor por ID estável.

Remoção:

- somente runners de origem gerenciada;
- canonicalizar;
- confirmar que o caminho está dentro de `app_data/runners`;
- remover registro e arquivos;
- nunca aceitar caminho arbitrário vindo da UI.

## Próximas evoluções

- múltiplas versões de Proton-GE;
- verificação criptográfica;
- Wine gerenciado;
- política de versão por jogo;
- limpeza de versões não utilizadas.
