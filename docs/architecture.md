# Arquitetura

## Objetivo

Manter um launcher genérico e orientado a dados, com frontend focado em
apresentação e backend responsável por sistema de arquivos, processos, rede e
persistência.

## Camadas

### React

Responsável por:

- catálogo e biblioteca;
- seleção do jogo;
- ações do usuário;
- feedback de progresso;
- configurações;
- apresentação de erros e diagnósticos.

Não deve:

- decidir regras por ID de jogo;
- duplicar estado persistido;
- manipular diretamente arquivos e processos;
- reproduzir lógica do backend.

### Ponte Tauri

`src/lib/tauri.ts` deve concentrar contratos de comandos e tipos de eventos.
Evite chamadas Tauri dispersas e inconsistentes.

### Backend Tauri/Rust

Responsável por:

- leitura e validação de manifestos;
- persistência;
- download e extração;
- resolução de runners;
- montagem e spawn de processos;
- logs;
- verificação, update e reparo;
- emissão de eventos.

### SQLite

Armazena estado local, nunca a definição canônica do jogo.

## Direção de modularização

- `catalog`: manifestos e catálogo.
- `installation`: instalação registrada e reconciliação.
- `downloader`: HTTP, retry e progresso.
- `extractor`: formatos e segurança de caminhos.
- `launcher`: resolução da ação de jogar.
- `process`: spawn, logs e acompanhamento.
- `settings`: configurações globais e por jogo.
- `database`: schema, migrations e queries.
- `runners`: detecção, resolução e comando.
- `managed_runners`: catálogo e lifecycle de runners baixados.

A extração de módulos deve acontecer quando reduzir responsabilidade real de
`lib.rs`, não apenas para aumentar o número de arquivos.

## Contratos

- O manifesto descreve capacidades e defaults.
- Configuração local pode sobrescrever defaults permitidos.
- O backend produz resultados estruturados.
- A UI não interpreta logs como primeira fonte quando existe evento estruturado.
- O log permanece fallback e ferramenta diagnóstica.
- Tarefas longas devem sair da thread que atende a UI.
- O processo principal é acompanhado pelo `ProcessManager` no estado Tauri.
- Sessões SQLite são a fonte única do tempo jogado acumulado.
- `Child::wait()` deve ocorrer fora de locks e fora da thread da UI.
