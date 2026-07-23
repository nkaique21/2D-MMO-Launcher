# Processos e tempo jogado

## Objetivo

Acompanhar somente o processo principal efetivo de cada jogo e usar sessões
persistidas como fonte única do tempo jogado.

Instaladores, updaters e processos auxiliares não geram sessão. Quando o
BattlEye substitui o executável principal (`launchMode: "main"`), o processo
iniciado pelo comando final continua sendo tratado como o processo do jogo.

## Estado em memória

`ProcessManager` é registrado com `tauri::Builder::manage` e mantém um estado
por `game_id`:

- `executionId`: identidade interna; evita depender apenas de PID;
- `status`: `starting`, `running`, `exited` ou `failed`;
- `processId`;
- runner resolvido;
- sessão SQLite relacionada;
- início e fim em Unix time UTC;
- exit code e erro, quando existirem.

Uma nova tentativa é rejeitada enquanto o jogo estiver `starting` ou `running`.
Estados encerrados podem ser substituídos pela próxima execução.

Nenhum lock permanece adquirido durante `Child::wait()`.

## Fluxo de launch

1. A tentativa registra `starting` antes da preparação do comando.
2. Falha antes do spawn registra `failed` e não cria sessão.
3. Depois de um spawn bem-sucedido, o backend obtém o PID.
4. A sessão é criada em `playtime_sessions`.
5. O estado muda para `running`.
6. No Linux, o comando principal é iniciado em um grupo de processos próprio.
   Uma thread acompanha o `Child` e também os descendentes desse grupo, sem
   bloquear a UI, persistindo heartbeat de duração a cada 15 segundos.
7. No encerramento, a sessão recebe duração monotônica, exit code e motivo.
8. O estado muda para `exited` ou `failed`.
9. Eventos atualizam a UI e o tempo acumulado.

Se a persistência falhar depois do spawn, o jogo continua sendo monitorado em
memória e o erro fica no `runner.log`; o launch não é derrubado depois que o
processo já começou.

## Persistência

A migration 4 cria `playtime_sessions`. A duração normal usa `Instant` para não depender de mudanças no relógio do sistema.
A soma de `duration_seconds` das sessões encerradas é a fonte do tempo acumulado. Não existe contador paralelo.

Motivos atuais:

- `normal`: processo terminou com sucesso;
- `nonzero_exit`: processo terminou com código não zero ou sinal;
- `wait_error`: o backend não conseguiu aguardar o processo;
- `interrupted`: sessão ficou aberta após o launcher anterior desaparecer.

## Recuperação

Durante o setup do Tauri, sessões com `ended_at IS NULL` são encerradas como
`interrupted`. A duração preserva o último heartbeat persistido e `ended_at` é
reconstruído como `started_at + duração conhecida`. Assim, um crash perde no
máximo o intervalo ainda não persistido, sem contar tempo posterior sem evidência.

Essa política é conservadora: o launcher não conta o período posterior ao último
heartbeat conhecido.

## Contratos Tauri

Comandos:

- `get_game_activity(gameId)`;
- `list_game_playtime_sessions(gameId)`.

Eventos:

- `game-process-state`: transição do processo;
- `game-activity-updated`: processo atual + resumo persistido.

## Interface

A tela principal mostra:

- tempo acumulado do jogo selecionado;
- tempo da sessão ativa somado visualmente em tempo real;
- badge `Iniciando` ou `Em execução`;
- botão `Jogando` bloqueado durante uma execução ativa.

O drawer mostra tempo acumulado, quantidade de sessões encerradas, estado e PID.

## Runners que entregam o processo

No Linux, o launcher cria um grupo de processos próprio para cada execução
rastreada. Se UMU, Proton ou Wine encerrarem o processo pai depois de entregar o
jogo a um descendente, a sessão continua ativa enquanto houver membros nesse
grupo.

Um runner que crie uma sessão totalmente nova com `setsid` ainda pode escapar
desse grupo. Esse caso deve ser diagnosticado pelo `runner.log` e pela árvore de
processos antes de adicionar uma estratégia mais ampla de descoberta.
