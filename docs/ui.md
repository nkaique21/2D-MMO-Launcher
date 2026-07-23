# Interface e experiência

## Direção

- visual dark;
- glass/blur com moderação;
- foco no jogo selecionado;
- janela estática para ações comuns;
- hierarquia clara;
- uma ação principal dominante.

## Composição atual

- sidebar estreita com jogos instalados;
- hero/banner ocupando a área principal;
- informações sobre o banner;
- ação principal no canto inferior direito;
- ações secundárias no drawer `⋯`;
- faixa compacta de progresso acima da ação;
- diagnóstico completo no drawer.

## Ação principal

- jogo instalado → `Jogar`;
- jogo disponível → `Baixar e instalar`;
- estado transitório → texto correspondente, como `Baixando...`.

Não executar update remoto completo automaticamente antes de todo launch.

Durante uma execução rastreada:

- o botão mostra `Iniciando...` ou `Jogando`;
- um segundo launch do mesmo jogo fica bloqueado;
- o hero mostra tempo acumulado e estado ativo;
- o drawer mostra tempo, sessões encerradas e PID.

## Ações secundárias

Exemplos:

- localizar instalação;
- abrir pasta;
- verificar arquivos;
- reparar;
- atualizar;
- desvincular;
- configurar;
- gerenciar runners.

Elas não devem competir visualmente com a ação principal.

## Progresso

Mostrar:

- fase;
- porcentagem;
- arquivo atual quando útil;
- contadores;
- erro claro;
- fonte de progresso no diagnóstico.

O estado local deve aparecer imediatamente após o clique, antes do primeiro
evento do backend.

## WebKitGTK

- selects e opções precisam de contraste explícito;
- `color-scheme: dark` pode ser necessário;
- validar menus nativos no Tauri real.

## Validação

Browser serve para inspeção rápida.
A aprovação visual exige `npm run tauri dev` e observação na janela nativa.

Para interações que dependem do sistema, pedir retorno objetivo do usuário antes
de concluir a etapa.
