# 2D MMO Launcher — Contexto para IA

Este arquivo serve como contexto permanente para qualquer IA, agente de código ou novo chat que venha trabalhar neste projeto. Leia antes de propor mudanças.

## Ambiente do usuário

- Sistema operacional principal: **Arch Linux**.
- Shell padrão: **fish**.
- Preferir comandos simples, curtos e compatíveis com fish.
- Evitar comandos longos com sintaxe específica de bash, especialmente heredocs complexos, pipelines grandes e substituições avançadas.
- Quando uma operação precisar de lógica mais longa, prefira:
  - usar ferramentas de patch/edição de arquivo;
  - criar um script temporário simples;
  - dividir em comandos menores.
- Não assumir que comandos copiados de bash funcionarão diretamente no fish.

## Projeto

- Nome: **2D MMO Launcher**.
- Direcionamento: desenvolver um launcher próprio inspirado em conceitos de arquitetura, organização e experiência do **Twintail Launcher**, mas voltado para MMORPGs 2D/Tibia-like.
- Importante: usar o Twintail apenas como **referência conceitual**. Não copiar código, implementações específicas, assets ou estrutura proprietária.
- Stack atual:
  - Tauri 2;
  - React;
  - Vite;
  - TypeScript;
  - Tailwind CSS;
  - Rust no backend Tauri;
  - SQLite planejado para persistência local.
- Objetivo do projeto: criar um launcher desktop genérico para MMORPGs 2D, baseado em manifestos JSON, evitando lógica específica hardcoded por jogo sempre que possível.

## Filosofia central

Todo jogo deve ser descrito por um **manifesto**.

Adicionar um novo jogo deve exigir, idealmente, apenas:

- criar um manifesto JSON;
- adicionar imagens/assets;
- informar como instalar;
- informar como executar;
- informar estratégia de update, quando existir.

O launcher não deve exigir alteração de código para cada novo jogo. Sempre que surgir uma necessidade específica, primeiro avaliar se ela pode virar configuração de manifesto, configuração de runner ou configuração persistida no banco.

Evite criar `if game.id === "..."` no frontend ou backend. Exceções temporárias precisam ser documentadas e tratadas como dívida técnica.

## Estrutura relevante

- `src/App.tsx`: shell visual principal do launcher.
- `src/styles.css`: estilos globais e base Tailwind.
- `src/types/manifest.ts`: tipos TypeScript dos manifestos.
- `src/lib/tauri.ts`: ponte frontend para comandos Tauri.
- `src-tauri/src/lib.rs`: comandos/backend Tauri.
- `src-tauri/manifests/*.json`: manifestos locais dos jogos.
- `src-tauri/tauri.conf.json`: configuração principal do Tauri.

## Arquitetura alvo

Arquitetura conceitual desejada:

```text
Launcher UI (React)
        │
        │ invoke()
        ▼
Backend (Rust/Tauri)
        │
        ├── SQLite
        ├── Downloader
        ├── Gerenciador de Manifestos
        ├── Gerenciador de Instalações
        ├── Gerenciador de Runners
        ├── Processo de Execução
        └── Configurações
```

### Backend Rust/Tauri

Organizar gradualmente o backend em serviços/módulos:

- `catalog`: leitura, validação e exposição dos manifestos disponíveis.
- `installation`: registro, localização e estado de instalações existentes.
- `downloader`: fila e execução de downloads.
- `extractor`: extração de `.zip`, `.tar.gz` e outros formatos suportados.
- `launcher`: resolução de comando final para executar um jogo.
- `process`: spawn, monitoramento e encerramento de processos de jogo.
- `settings`: configurações globais e por jogo.
- `database`: conexão SQLite, migrations e queries.

### Frontend React

Organizar gradualmente o frontend por domínios/telas:

- `Library`: biblioteca/lista de jogos instalados.
- `Game Details`: hero/banner, informações, ação principal e ações secundárias.
- `Downloads`: progresso, fila e histórico de downloads.
- `Settings`: configurações globais, runners e preferências.

O frontend deve consumir dados via comandos Tauri (`invoke`) e evitar duplicar estado que deveria vir de manifestos ou SQLite.

## Direção de UX/UI

A interface desejada deve seguir esta direção:

- Visual moderno, dark, com aparência glass/blur e foco visual forte no jogo selecionado.
- Barra lateral esquerda: jogos já instalados.
- Faixa superior: jogos disponíveis por manifesto, com possibilidade de baixar/instalar.
- Área principal: banner/hero grande do jogo selecionado.
- Botão principal deve ser claro e destacado:
  - `Jogar` para jogos instalados;
  - `Baixar e instalar` para jogos disponíveis.
- Ações secundárias devem ser mais discretas, por exemplo:
  - localizar instalação;
  - verificar arquivos;
  - abrir pasta;
  - detalhes do manifesto;
  - configurações do runner.
- Evitar UI muito poluída ou com muitos botões competindo com a ação principal.

## Regras de jogos e runners

- **RavenQuest** deve ser tratado como exclusivo para execução via **Proton**.
- **Archlight** deve ser tratado como exclusivo para execução via **Proton**.
- Os manifestos desses jogos devem manter:

```json
"launch": {
  "runner": "proton"
}
```

- Outros jogos podem usar runner nativo quando o manifesto permitir.

### Runners previstos

O launcher deve evoluir para suportar:

- Linux nativo;
- Wine;
- Proton;
- Steam;
- runner personalizado.

Cada jogo pode usar um runner diferente. A decisão deve vir do manifesto e/ou das configurações persistidas, não de lógica hardcoded espalhada pela UI.

### Jogos iniciais do catálogo

- RavenQuest;
- PokeXGames;
- Grand Line Adventures;
- Archlight;
- Zezenia;
- Medivia;
- WoT posteriormente.

## Manifestos

- Manifestos ficam em `src-tauri/manifests`.
- Cada manifesto descreve:
  - `id`;
  - `name`;
  - `description`;
  - assets como `banner` e `icon`;
  - métodos de instalação;
  - configuração de launch;
  - estratégia de update.
- A intenção é evoluir para carregar a UI a partir dos manifestos reais, não manter tudo duplicado no frontend.

Formato conceitual base:

```json
{
  "id": "...",
  "name": "...",
  "description": "...",
  "assets": {},
  "installation": {},
  "launch": {},
  "update": {}
}
```

### Métodos de instalação previstos

Suportar progressivamente:

- Archive (`.zip`, `.tar.gz` etc.);
- AppImage;
- instalador Windows;
- launcher externo;
- Steam;
- instalação já existente.

O MVP pode começar com `existing`/localizar instalação existente, mas a estrutura deve permitir expansão sem refatorações grandes.

## Banco SQLite planejado

SQLite será usado para persistência local. Tabelas iniciais desejadas:

- `games`: índice local/cache de jogos conhecidos, se necessário.
- `installs`: instalações localizadas ou criadas pelo launcher.
- `game_settings`: configurações individuais por jogo.
- `playtime_sessions`: sessões de tempo jogado.
- `download_tasks`: fila/histórico de downloads.
- `runners`: runners configurados/disponíveis.

Separação conceitual importante:

- Manifesto descreve o jogo e possibilidades.
- SQLite descreve o estado local do usuário: instalado ou não, caminho, configurações, runner escolhido, sessões, downloads etc.

## Funcionalidades do MVP

O MVP deve cobrir:

- biblioteca de jogos;
- banner/hero do jogo;
- informações do jogo;
- instalar;
- localizar instalação existente;
- jogar;
- configurações individuais por jogo;
- SQLite para armazenar instalações e configurações.

## Roadmap

### Fase 1 — UI e catálogo

- Interface inspirada no Twintail em termos de experiência, sem copiar código.
- Biblioteca visual.
- Cards/atalhos de jogos.
- Tela de detalhes com banner, descrição, runner e ação principal.
- Carregar jogos a partir dos manifestos reais.

### Fase 2 — Instalações existentes e jogar

- Detectar/localizar instalações existentes.
- Persistir caminho no SQLite.
- Botão `Jogar` funcionando para runners simples/nativos.
- Configurações individuais por jogo.

### Fase 3 — Download e instalação automática

- Downloader.
- Fila de downloads.
- Extração/instalação automática.
- Prioridade inicial: Zezenia, GLA e PokeXGames, conforme viabilidade dos manifestos.

### Fase 4 — Wine/Proton

- Camada de runners.
- Suporte a Wine.
- Suporte a Proton.
- RavenQuest via Proton.
- Archlight via Proton.

### Fase 5 — Recursos avançados

- Atualizações.
- Reparo/verificação de arquivos.
- Tempo jogado.
- Notícias.
- Discord RPC.
- Integração opcional com Steam.
- Auto update do launcher.

## Comandos comuns

Use comandos simples:

```sh
npm run build
```

Valida TypeScript e build Vite.

```sh
npm run dev -- --host 127.0.0.1
```

Sobe apenas o Vite para debug web. Não é o preview final do app.

```sh
npm run tauri dev
```

Roda o app na janela nativa Tauri. Este é o modo correto para validar o visual real do launcher.

## Observações sobre preview

- Para avaliar o visual final, preferir sempre Tauri nativo.
- Browser/Puppeteer pode servir para inspeção rápida, mas não substitui a janela Tauri.
- Se `npm run tauri dev` falhar no Arch Linux, verificar dependências do Tauri/WebKitGTK e informar exatamente quais pacotes estão faltando.

## Estado recente do projeto

- A UI foi ajustada para separar jogos instalados à esquerda e jogos disponíveis por manifesto no topo.
- A área principal agora usa um hero/banner grande do jogo selecionado.
- RavenQuest e Archlight foram marcados na UI como Proton-only.
- Os manifestos `ravenquest.json` e `archlight.json` foram ajustados para `runner: "proton"`.
- `npm run build` passou com sucesso após esses ajustes.
- `npm run tauri dev` compilou o backend Rust e iniciou `target/debug/two-d-mmo-launcher` com sucesso no ambiente local.
- `src/App.tsx` foi refatorado para carregar o catálogo real via `listGames()`/`list_games`, usando `GameManifest[]` vindo do backend Tauri.
- O frontend agora usa descrição, assets, runner e métodos de instalação vindos dos manifestos locais.
- Ainda existe apenas uma camada temporária de instalação local (`temporaryInstalledGameIds`) até SQLite/tabela `installs` ser implementada.
- Ainda existem metadados visuais temporários por jogo no frontend, como abreviação, gradiente e categoria curta; eles não devem conter regra de negócio.

## Onde prosseguir daqui

Próximo passo recomendado para desenvolvimento:

1. **Separar conceito de catálogo e instalação**
   - Manifestos representam jogos disponíveis.
   - Instalações representam jogos realmente instalados/localizados no computador do usuário.
   - Até existir SQLite, pode haver estado mockado/temporário, mas deve ser fácil remover.

2. **Introduzir SQLite**
   - Escolher crate compatível com Tauri/Rust, por exemplo `rusqlite` ou alternativa adequada.
   - Criar módulo `database`.
   - Criar migrations/tabelas iniciais: `installs`, `game_settings`, `runners` primeiro.

3. **Localizar instalação existente**
   - Criar comando Tauri para selecionar pasta/arquivo.
   - Salvar caminho em SQLite.
   - Atualizar UI para mover jogo para área de instalados quando existir instalação registrada.

4. **Botão Jogar**
   - Resolver runner pelo manifesto/configuração.
   - Montar comando de execução.
   - Fazer spawn pelo backend Rust, não pelo frontend.
   - Registrar sessão para futuro tempo jogado.

5. **Depois avançar para download/instalação automática**
   - Só iniciar depois que catálogo, instalações existentes e execução básica estiverem bem definidos.

Critério de arquitetura: sempre que uma funcionalidade parecer específica demais para um jogo, tentar modelar como manifesto, runner, método de instalação ou configuração persistida.

## Preferências de colaboração

- Responder em **pt-BR**.
- Explicar mudanças de forma direta e prática.
- Antes de editar arquivos importantes, conferir padrões existentes do projeto.
- Manter arquitetura extensível e evitar acoplamento desnecessário.
- Ao validar visual, lembrar que o usuário quer ver no **Tauri**, não só no navegador.

## Fluxo obrigatório de etapas Git

- Ao concluir cada etapa funcional aprovada pelo usuário, atualizar este próprio `AGENTS.md` com o estado recente, decisões importantes, próximos passos e qualquer nova regra operacional definida durante a etapa.
- A atualização do `AGENTS.md` deve acontecer antes do commit da etapa, para que o contexto versionado acompanhe a evolução real do projeto.
- Ao concluir cada etapa funcional aprovada pelo usuário, criar um commit Git específico para aquela etapa.
- Depois do commit local, subir as alterações para o remoto configurado com `git push` antes de iniciar a próxima etapa.
- Antes de commitar, revisar `git status` e, quando útil, o diff para evitar incluir mudanças acidentais.
- Mensagens de commit devem ser curtas, descritivas e em português ou inglês técnico consistente com o histórico do projeto.
- Se `git push` falhar por credenciais, rede ou divergência com o remoto, informar o erro e aguardar orientação antes de prosseguir para a próxima etapa.