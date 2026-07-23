# Catálogo remoto oficial

## Objetivo

Separar o catálogo de jogos do ciclo de release do launcher. Correções de
manifesto, URLs, argumentos, assets e estratégias já suportadas podem ser
publicadas no repositório de catálogo sem recompilar o aplicativo.

Repositório esperado:

```text
https://github.com/nkaique21/2D-MMO-Launcher-Catalog
```

Endpoint padrão:

```text
https://raw.githubusercontent.com/nkaique21/2D-MMO-Launcher-Catalog/main/catalog.json
```

## Fontes e prioridade

1. Cache remoto oficial validado.
2. Manifestos embutidos no bundle do launcher.

O SQLite não participa do catálogo e continua armazenando somente estado local
do usuário.

## Inicialização

1. O launcher abre imediatamente com o cache remoto válido, quando existir.
2. Sem cache, usa os manifestos embutidos.
3. Em background, tenta atualizar o catálogo oficial.
4. Ao concluir, emite `catalog-updated` e a UI recarrega os jogos.
5. Em falha, emite `catalog-update-failed` e preserva a fonte ativa.

A atualização manual usa o mesmo fluxo pelo comando `refresh_catalog`.

## Cache

Diretório esperado:

```text
~/.local/share/dev.kaiquelb.2d-mmo-launcher/catalog/
├── official/
│   ├── catalog.json
│   └── manifests/*.json
├── staging/
├── backup/
└── metadata.json
```

A atualização é transacional:

1. baixa o índice;
2. valida schema, IDs e paths;
3. baixa todos os manifestos habilitados;
4. valida cada manifesto;
5. normaliza assets relativos para URLs absolutas;
6. grava o conjunto em staging;
7. valida o staging completo;
8. troca o diretório ativo;
9. remove o backup após sucesso.

Um download parcial nunca substitui o último catálogo válido.

## Contrato do índice

```json
{
  "schemaVersion": 1,
  "catalogVersion": "0.1.0",
  "generatedAt": "2026-07-23T00:00:00Z",
  "games": [
    {
      "id": "medivia",
      "manifest": "manifests/medivia.json",
      "enabled": true
    }
  ]
}
```

## Manifestos

Manifestos remotos declaram `schemaVersion: 1`. Manifestos embutidos antigos
que não possuem o campo usam versão 1 por compatibilidade.

Assets remotos podem ser:

- URL HTTPS absoluta;
- path relativo à raiz do catálogo;
- path iniciado por `/`, também resolvido contra a raiz do catálogo.

O launcher não baixa assets para o cache nesta etapa. Sem internet, o catálogo e
suas funcionalidades continuam disponíveis, mas a imagem remota pode não ser
carregada; o gradiente da UI permanece como fallback visual.

## Validações de segurança

- catálogo e manifestos somente por HTTPS;
- limite de tamanho para índice e manifesto;
- schema conhecido;
- IDs únicos e sanitizados;
- manifesto precisa declarar o mesmo ID do índice;
- rejeição de path absoluto, `..`, prefixo de drive e NUL;
- URLs de instalação e update precisam usar HTTPS;
- todos os manifestos precisam passar antes da ativação;
- cache anterior é preservado em falha.

O repositório remoto continua sendo uma fonte confiável capaz de alterar
instalação e execução. Assinatura criptográfica do índice permanece evolução
futura.

## Comandos e eventos

Comandos Tauri:

```text
list_games
get_catalog_status
refresh_catalog
```

Eventos:

```text
catalog-updated
catalog-update-failed
```

`get_catalog_status` informa fonte ativa, URL oficial, versão, timestamps, erro
mais recente e quantidade de jogos.

## Publicação

No repositório de catálogo:

1. altere manifesto/assets;
2. incremente `catalogVersion`;
3. atualize `generatedAt` em UTC;
4. execute `python scripts/validate_catalog.py`;
5. faça commit e push na branch `main`.

O GitHub Actions valida catálogo, manifestos, paths, URLs e assets.
