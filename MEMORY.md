# MEMORY.md — Fatos duráveis e decisões confirmadas

Este arquivo registra somente conhecimento que deve sobreviver entre tarefas.
Não é um changelog.

## Invariantes arquiteturais

- O launcher é orientado a manifestos.
- Estado descritivo do jogo pertence ao manifesto.
- Estado local do usuário pertence ao SQLite.
- Casos específicos devem virar configuração, não condicionais por ID.
- Frontend não deve duplicar dados que vêm de manifestos ou SQLite.
- Desvincular instalação nunca deve apagar arquivos do jogo.
- Verificação é diagnóstica; reparo é uma ação explícita separada.
- Update completo não deve bloquear todo clique em `Jogar`.
- Processamento externo longo não deve bloquear a thread da UI.
- Downloads e aplicações destrutivas devem usar staging e validação.

## Ambiente real

- Sistema principal: CachyOS/Arch Linux.
- Shell: Fish.
- Sessão usada nos testes: KDE Plasma, historicamente X11 em parte dos testes.
- A aparência final deve ser validada no Tauri, não apenas no navegador.
- Diretório de dados esperado:
  `~/.local/share/dev.kaiquelb.2d-mmo-launcher`.

## Manifestos

- `launch.runner` define a categoria padrão.
- Configuração local pode escolher um runner concreto por ID.
- `launch.env` e `launch.unsetEnv` permitem ambiente por jogo.
- Caminhos iniciados por `~/`, `$HOME/` ou `${HOME}/` são expandidos.
- Métodos de instalação podem declarar runner e prefixo próprios.
- O instalador pode usar runner diferente do jogo.
- `launchAfterInstall` é opcional.
- `verification.requiredFiles` serve para integridade estrutural.
- `verification.checksums` aceita CRC32 validado pelo backend.
- `update.strategy` suporta `externalLauncher` e `remoteManifest`.

## Catálogo oficial

- O catálogo oficial vive no repositório separado `2D-MMO-Launcher-Catalog`.
- Endpoint padrão usa `raw.githubusercontent.com` na branch `main`.
- Cache remoto válido vence os manifestos embutidos.
- Manifestos embutidos permanecem fallback offline e de recuperação.
- Atualização baixa o conjunto completo para staging e ativa de forma transacional.
- Falha remota preserva o último cache válido e registra o erro em metadata.
- Catálogo e manifestos remotos usam `schemaVersion: 1`.
- URLs externas precisam usar HTTPS; paths absolutos e travessia são rejeitados.
- Assets remotos não são cacheados nesta etapa.
- O repositório de catálogo é uma superfície de segurança crítica.

## Banco

- Persistência foi extraída para `src-tauri/src/database.rs`.
- O schema usa `PRAGMA user_version`.
- Migrations são incrementais e transacionais.
- Migration já distribuída não deve ser alterada.
- Bancos futuros desconhecidos são rejeitados.
- A adoção de banco legado foi validada preservando instalações e settings.
- Tabelas atuais: `installs`, `game_settings`, `runners`, `playtime_sessions`.

## Runners

- Descoberta e resolução ficam em `runners.rs`.
- Runners concretos podem ser selecionados pelo ID detectado.
- Prefixos são isolados por jogo em `compat-data/<game_id>/<runner_kind>`.
- UMU é preferível para Proton fora da Steam quando disponível.
- Comandos finais carregam variáveis a aplicar e variáveis a remover.
- stdout e stderr dos processos vão para `logs/<game_id>/runner.log`.
- Proton-GE gerenciado usa staging e validação antes de ser registrado.
- Remoção de runner gerenciado valida que o caminho está dentro de `app_data/runners`.

## RavenQuest e BattlEye

- RavenQuest deve executar via Proton/UMU.
- O método funcional confirmado usa `ravenquest_dx_BE.exe` via `umu-run`.
- `ravenquest_dx_BE.exe` substitui o processo principal quando
  `launch.battlEye.launchMode` é `main`.
- Executar `ravenquest_dx_BE.exe 1 0` não foi o método funcional confirmado.
- O `belauncher.exe` de `system32` não é a entrada principal adequada.
- O ambiente que funcionou incluiu:
  - `PROTONPATH`;
  - `PROTON_BATTLEYE_RUNTIME`;
  - `PROTON_EAC_RUNTIME`;
  - `WINEESYNC=1`;
  - `WINEFSYNC=1`;
  - `WINEARCH=win64`;
  - `WINEDEBUG=-all`;
  - remoção de `GAMEID` e `STORE`.
- Os caminhos de runtime podem ser ajustados pelas configurações locais.
- O instalador pode usar Wine e compartilhar o prefixo compatível com Proton.
- A reconciliação não deve procurar recursivamente `launcher.exe` quando
  BattlEye é o executável efetivo.
- O botão `Jogar` deve chamar launch diretamente; update remoto é explícito.

## Update remoto

- O manifesto remoto do RavenQuest contém dezenas de milhares de arquivos.
- A verificação síncrona bloqueava a UI e foi movida para trabalho bloqueante
  separado.
- O fluxo atual é:
  1. resolver alvo;
  2. baixar e decodificar manifesto;
  3. verificar arquivos;
  4. montar plano;
  5. baixar em staging;
  6. validar durante download;
  7. aplicar somente após sucesso;
  8. emitir conclusão ou erro.
- `files` e o arquivo de `binary` entram no mesmo mapa.
- Segmentos de URL precisam de percent-encoding; `#` deve virar `%23`.
- Downloads reutilizam cliente HTTP e possuem retry/backoff.
- Concorrência usa índice atômico compartilhado, não receiver serializado.
- Erros são acumulados para não cancelar toda a fila no primeiro problema.
- Progresso pode vir por evento Tauri ou fallback do `runner.log`.
- Logs devem ser agregados, não uma linha de sucesso por arquivo.
- Aplicação e download precisam expor progresso visível.

## Instalação por archive

- Extração deve rejeitar caminho absoluto e travessia.
- Pacotes podem declarar remoção de diretório superior.
- Executável precisa ser validado antes da aplicação.
- Em Linux, a permissão de execução deve ser garantida.
- Aplicação deve ocorrer depois de staging válido.

## Validações reais confirmadas

- RavenQuest abriu pelo botão `Jogar` com o fluxo BattlEye/UMU.
- Update e reparo remoto do RavenQuest foram testados em arquivo removido.
- O arquivo reparado foi binariamente idêntico ao backup no teste controlado.
- Medivia instalou e abriu pelo fluxo archive nativo.
- Archlight instalou e abriu via UMU/Proton.
- PokeMMO instalou, abriu e encerrou normalmente.
- CRC32 do `PokeMMO.sh` foi validado na instalação real.
- Configurações locais do RavenQuest persistiram e restauraram defaults.
- Menu escuro de seleção ficou legível no WebKitGTK.
- Proton-GE gerenciado foi instalado, removido e reinstalado com sucesso.

## Processos e tempo jogado

- Somente o processo principal efetivo conta tempo jogado.
- Instaladores, updaters e processos auxiliares não criam sessão.
- RavenQuest em `launchMode: main` conta o processo BattlEye efetivo.
- Sessões recebem checkpoints periódicos; recuperação após crash preserva apenas o último tempo confirmado.
- O usuário validou o rastreamento com os runners reais do projeto.

## Armadilhas conhecidas

- Um processo pode falhar antes do spawn; o log deve começar antes dessa etapa.
- Procurar recursivamente em prefixos grandes pode aparentar congelamento.
- Evento Tauri pode não chegar; manter fallback de progresso por log.
- Muitos logs por arquivo degradam updates com milhares de arquivos pequenos.
- `Mutex<Receiver>` pode serializar distribuição de trabalho e destruir a
  concorrência esperada.
- Aplicar arquivos antes de validar o conjunto deixa instalação parcialmente
  atualizada.
- Browser não representa fielmente comportamento visual e integração Tauri.
