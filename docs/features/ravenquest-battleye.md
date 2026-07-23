# RavenQuest, BattlEye e UMU

## Estado atual

O fluxo funcional confirmado usa:

- executável principal efetivo: `ravenquest_dx_BE.exe`;
- runner: Proton por meio de `umu-run`;
- prefixo do jogo em `compat-data/ravenquest/proton`;
- ambiente compatível com runtimes BattlEye/EAC;
- update e reparo por manifesto remoto.

## Configuração conceitual

O manifesto usa `launch.battlEye` opcional.

Quando:

```json
{
  "launchMode": "main"
}
```

o executável do BattlEye substitui `launch.executable` como processo principal.

Consequências:

- `launch_game` inicia BattlEye diretamente;
- não inicia o launcher comum em paralelo;
- verificação usa o executável efetivo;
- reconciliação não procura recursivamente o executável comum;
- logs registram que o processo principal foi substituído.

## Caminhos conhecidos

BattlEye/jogo dentro do prefixo:

```text
drive_c/Program Files (x86)/Tavernlight Games/RavenQuest/
```

Executável efetivo:

```text
ravenquest_dx_BE.exe
```

O `belauncher.exe` de `system32` foi testado como hipótese e não é a entrada
principal apropriada.

## Ambiente confirmado

Variáveis usadas no teste funcional:

- `PROTONPATH`
- `PROTON_BATTLEYE_RUNTIME`
- `PROTON_EAC_RUNTIME`
- `WINEESYNC=1`
- `WINEFSYNC=1`
- `WINEARCH=win64`
- `WINEDEBUG=-all`

Variáveis removidas:

- `GAMEID`
- `STORE`

Os caminhos específicos da máquina devem permanecer configuráveis por
`game_settings`, não hardcoded no backend.

## Instalador

O instalador Windows pode ser aberto com Wine, usando prefixo compatível com o
prefixo Proton do launch. Instalação e execução podem usar runners diferentes.

Após o instalador:

- monitorar processo;
- reconciliar instalação;
- atualizar SQLite;
- emitir `install-updated`;
- executar auto-launch somente quando declarado.

## Update remoto

Fonte configurada no manifesto:

```text
https://dw.ravenquest.io/ravenquest/checksums.txt.gz
```

Formato conhecido:

```text
ravenquestZlib
```

Alvo:

```text
drive_c/Program Files (x86)/Tavernlight Games/RavenQuest
```

O update deve incluir `files` e `binary.file`.

## Logs esperados

Em um launch funcional, procurar chaves equivalentes a:

```text
action=launch_game
main_executable_replaced_by_battl_eye=true
battl_eye_launch_mode=main
unset_env.GAMEID=true
unset_env.STORE=true
```

Também confirmar:

- runner resolvido para UMU;
- `PROTONPATH`;
- runtime BattlEye;
- PID iniciado;
- ausência de erro pré-spawn.

## Fatos descartados

- `ravenquest_dx_BE.exe 1 0` não foi o método funcional confirmado.
- Reinstalar o jogo não é a primeira resposta para erro de serviço BattlEye.
- `belauncher.exe` isolado não deve substituir o fluxo confirmado.
