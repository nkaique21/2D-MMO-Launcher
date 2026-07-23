# Manifestos

## Papel

Cada manifesto descreve um jogo e suas possibilidades. A fonte principal pode
vir do catálogo oficial remoto; `src-tauri/manifests/*.json` permanece como
fallback embutido no bundle. Adicionar um jogo deve exigir principalmente
manifesto e assets.

## Áreas conceituais

```json
{
  "schemaVersion": 1,
  "id": "...",
  "name": "...",
  "description": "...",
  "assets": {},
  "installation": {},
  "launch": {},
  "update": {},
  "verification": {}
}
```

## Launch

Campos usados ou suportados conceitualmente:

- `runner`
- `executable`
- `args`
- `workingDir`
- `env`
- `unsetEnv`
- `battlEye`

O runner pode ser uma categoria genérica. A configuração local pode selecionar
um runner concreto detectado.

## BattlEye

É opcional. Jogos sem o bloco mantêm o fluxo comum.

Pode declarar:

- executável;
- argumentos;
- base do caminho;
- working directory;
- modo de lançamento.

Quando `launchMode` for `main`, o executável do BattlEye substitui o processo
principal e deve ser considerado pela verificação e reconciliação.

## Instalação

Métodos previstos ou atuais:

- `existing`
- `archive`
- `windowsInstaller`
- AppImage
- launcher externo
- Steam

Campos dependem do método, incluindo:

- URL;
- formato (`zip`, `tar`, `tar.gz`/`tgz` ou `tar.bz2`/`tbz2`);
- headers;
- runner;
- prefixo compatível;
- diretório alvo;
- remoção de pasta superior;
- auto-launch.

Quando `format` não é informado, o launcher tenta inferi-lo pela extensão da
URL. Declarar o formato explicitamente continua preferível para URLs sem nome de
arquivo estável.

## Update

Estratégias atuais:

### `externalLauncher`

Executa um updater/launcher oficial com runner, caminho, argumentos e ambiente
declarados no manifesto.

### `remoteManifest`

Baixa um manifesto remoto, verifica arquivos e aplica divergências em fluxo
transacional.

Pode declarar concorrência máxima, formato remoto, alvo e bases de caminho.

## Verification

### `requiredFiles`

Checagem estrutural rápida de arquivos e diretórios relativos à instalação.

### `checksums`

Checagem opcional de integridade. O algoritmo inicial suportado é CRC32.

O backend deve rejeitar:

- caminhos absolutos;
- travessia;
- algoritmo desconhecido;
- valor malformado.

## Regras

- Tipos Rust e TypeScript devem permanecer compatíveis.
- Campo novo deve ser opcional quando jogos existentes não o exigirem.
- Um campo deve representar comportamento reutilizável.
- Valores específicos de máquina devem ser override local quando possível.
- Não mover configuração local do usuário para o manifesto.


## Fonte remota

O índice remoto fica no repositório `2D-MMO-Launcher-Catalog`. Cada entrada
aponta para um manifesto e precisa declarar o mesmo `id`.

O launcher aceita somente `schemaVersion: 1`, valida paths e URLs, grava todos
os manifestos em staging e só ativa o conjunto completo após sucesso.

Manifestos embutidos sem `schemaVersion` usam versão 1 por compatibilidade.
Detalhes do fluxo estão em `docs/catalog.md`.
