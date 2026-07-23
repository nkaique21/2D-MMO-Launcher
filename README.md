<div align="center">

# 2D MMO Launcher

Um launcher open source para instalar, configurar, atualizar e executar MMORPGs 2D no Linux.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-backend-000000?logo=rust)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-frontend-61DAFB?logo=react&logoColor=111)](https://react.dev/)
[![Linux](https://img.shields.io/badge/Linux-supported-FCC624?logo=linux&logoColor=111)](#compatibilidade)

[Catálogo oficial](https://github.com/nkaique21/2D-MMO-Launcher-Catalog) · [Reportar problema](https://github.com/nkaique21/2D-MMO-Launcher/issues)

</div>

## Sobre o projeto

O **2D MMO Launcher** reúne jogos 2D em uma interface única e permite que novos títulos e correções sejam publicados por meio de um catálogo remoto, sem exigir uma nova versão do aplicativo para cada alteração de manifesto.

O projeto nasceu com foco em Linux e em jogos que normalmente exigem procedimentos diferentes de instalação e execução: binários nativos, Java, Wine, Proton, UMU e mecanismos auxiliares como BattlEye.

> Este projeto não é afiliado, patrocinado ou aprovado pelos desenvolvedores e publicadores dos jogos presentes no catálogo. Marcas, nomes e artes pertencem aos seus respectivos proprietários.

## Funcionalidades do MVP

- catálogo remoto com cache local e fallback embutido;
- instalação por arquivos compactados e localização de instalações existentes;
- suporte a ZIP, TAR, TAR.GZ/TGZ e TAR.BZ2/TBZ2;
- execução nativa e por runners compatíveis;
- comandos externos, incluindo jogos Java;
- integração com Wine, Proton e UMU;
- fluxos de instalação, atualização, reparo e verificação definidos por manifesto;
- rastreamento do processo principal do jogo;
- sessões persistidas e tempo jogado acumulado;
- recuperação conservadora de sessões após encerramento inesperado;
- preferências e instalações armazenadas em SQLite.

## Jogos

A lista de jogos não fica presa ao código do launcher. Ela é publicada no repositório separado:

**[2D-MMO-Launcher-Catalog](https://github.com/nkaique21/2D-MMO-Launcher-Catalog)**

O catálogo pode adicionar ou ajustar jogos por JSON, desde que utilize recursos já suportados pelo launcher. Jogos que exigem um novo tipo de instalação ou runner ainda podem demandar uma evolução no aplicativo.

## Compatibilidade

O MVP é desenvolvido e testado principalmente em:

- Linux x86_64;
- Arch Linux e derivados, incluindo CachyOS;
- ambientes gráficos com suporte ao WebKitGTK exigido pelo Tauri.

Outras distribuições Linux podem funcionar, mas ainda não fazem parte da matriz oficial de testes. Windows e macOS não são alvos suportados neste momento.

## Executando em desenvolvimento

### Dependências principais

- Node.js e npm;
- Rust e Cargo;
- dependências de desenvolvimento do Tauri 2 para sua distribuição;
- WebKitGTK 4.1.

No Arch Linux/CachyOS, uma base comum é:

```bash
sudo pacman -S --needed base-devel rust nodejs npm webkit2gtk-4.1
```

Clone o repositório e instale as dependências:

```bash
git clone https://github.com/nkaique21/2D-MMO-Launcher.git
cd 2D-MMO-Launcher
npm ci
```

Inicie o aplicativo:

```bash
npm run tauri dev
```

### Validação

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

### Build distribuível

```bash
npm run tauri build
```

Os pacotes gerados ficam sob `src-tauri/target/release/bundle/`.

## Dados locais

Por padrão, os dados ficam no diretório de aplicação definido pelo Tauri. No ambiente atual do projeto:

```text
~/.local/share/dev.kaiquelb.2d-mmo-launcher/
```

Esse diretório pode conter banco SQLite, cache do catálogo, jogos, runners, prefixes e arquivos temporários. Não o inclua em relatórios públicos sem revisar possíveis dados pessoais ou credenciais.

## Arquitetura resumida

```text
React + TypeScript + Tailwind
            │
         Tauri 2
            │
 Rust: catálogo, downloads, arquivos,
 runners, processos, SQLite e comandos
            │
 catálogo JSON remoto + cache local
```

Documentação técnica adicional está em [`docs/`](docs/).

## Contribuindo

Contribuições são bem-vindas em duas frentes:

- **launcher:** funcionalidades, correções, interface, runners e infraestrutura;
- **catálogo:** manifests, assets e correções específicas de jogos.

Antes de abrir um pull request:

1. mantenha alterações fora do escopo separadas;
2. execute os comandos de validação aplicáveis;
3. descreva como a mudança foi testada;
4. não envie cookies, tokens, bancos locais, prefixes ou arquivos proprietários de jogos.

Para adicionar ou corrigir um jogo sem alterar o launcher, use o [repositório do catálogo](https://github.com/nkaique21/2D-MMO-Launcher-Catalog).

## Segurança e downloads

Os manifests podem definir URLs, argumentos e arquivos esperados. Por isso:

- prefira sempre fontes oficiais e HTTPS;
- não publique cookies, tokens ou URLs temporárias de sessão;
- não redistribua clientes proprietários sem autorização;
- revise alterações de catálogo antes de publicá-las.

Falhas de segurança devem ser relatadas de forma privada ao mantenedor antes da abertura de uma issue pública.

## Estado do projeto

O projeto está em estágio **MVP / `0.1.x`**. A estrutura principal já funciona, mas APIs de manifesto e detalhes de comportamento ainda podem mudar antes de uma versão estável.

## Licença

O código do launcher é distribuído sob a [licença MIT](LICENSE).

A licença do launcher não cobre jogos, logotipos, banners, ícones ou outros materiais de terceiros. Cada item continua sujeito aos termos de seu respectivo proprietário.
