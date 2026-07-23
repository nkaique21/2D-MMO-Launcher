# Instalação, update, verificação e reparo

## Princípios

- Instalação e update devem ser observáveis.
- Tarefas longas não podem bloquear a UI.
- Arquivos externos devem ser tratados como não confiáveis.
- Aplicação deve ser transacional quando possível.
- Verificação não modifica arquivos.
- Reparo é explícito.
- Desvinculação nunca remove arquivos.

## Instalação existente

- Usuário seleciona uma pasta.
- Backend registra ou atualiza `installs`.
- Reconciliação pode localizar executável real.
- Busca recursiva ampla deve ser evitada quando o executável efetivo já é
  conhecido por configuração.
- Abrir pasta e desvincular são ações separadas.

## Instalador Windows

- Download preserva extensão.
- Método pode usar runner diferente do launch.
- Prefixo compatível pode ser compartilhado.
- stdout/stderr devem ir para o log do jogo.
- Ao terminar, pode reconciliar instalação e emitir `install-updated`.
- `launchAfterInstall` é opcional.

## Archive

Formatos atuais:

- `zip`;
- `tar`;
- `tar.gz` e alias `tgz`;
- `tar.bz2` e aliases `tbz2`/`tbz`.

O campo `format` é recomendado. Quando ausente, o backend tenta inferir pela
extensão da URL, ignorando query string e fragmento. A resolução e a extração
ficam centralizadas em `archive.rs`, para que formatos futuros não criem
condicionais espalhadas pelo instalador.

Arquivos TAR aceitam somente diretórios e arquivos regulares. Links simbólicos,
hard links e arquivos especiais são recusados por segurança.

Fluxo:

1. baixar em background;
2. aplicar retry e headers configurados;
3. extrair em staging;
4. rejeitar paths absolutos e travessia;
5. remover diretório superior quando declarado;
6. validar executável e arquivos essenciais;
7. ajustar permissão executável no Linux;
8. mover/aplicar para destino final;
9. registrar no SQLite;
10. auto-launch opcional.

## Verificação

Retorna resultado estruturado:

- pasta;
- executável efetivo;
- integridade;
- problemas;
- arquivos ausentes;
- checksums;
- estratégia de reparo.

Verificação não deve baixar, apagar ou substituir arquivos.

## Reparo

- Só mostrar CTA quando há estratégia segura.
- `remoteManifest` já possui fluxo explícito.
- `archive` e `windowsInstaller` exigem desenho não destrutivo antes de ativação.
- Após reparo, executar verificação novamente e substituir diagnóstico antigo.

## Update remoto transacional

Fluxo atual:

1. carregar instalação e manifesto local;
2. reconciliar instalação;
3. resolver alvo;
4. baixar e decodificar manifesto remoto;
5. unir lista comum e binário principal;
6. verificar arquivos locais;
7. criar plano de divergências;
8. preparar staging;
9. baixar em paralelo;
10. validar tamanho/CRC no worker;
11. impedir aplicação se houver falhas;
12. aplicar arquivos;
13. emitir conclusão;
14. limpar ou preservar staging conforme política diagnóstica.

## HTTP

- Cliente reutilizado durante a operação.
- Timeout finito.
- Retry com backoff para falhas transitórias.
- Percent-encoding por segmento do caminho remoto.
- Não interpretar `#` como fragmento de URL.
- Mensagens de erro devem informar arquivo e tentativa.

## Concorrência

- Respeitar `maxConcurrentDownloads` com limite interno.
- Distribuição de trabalho não deve ser serializada por um receiver travado.
- Índice atômico compartilhado foi a solução validada.
- Falha de um arquivo não deve necessariamente abortar a fila imediatamente.
- Reportar o conjunto de falhas antes da aplicação.

## Progresso

Fonte primária:

- eventos Tauri estruturados.

Fallback:

- leitura agregada de `runner.log`.

Etapas devem incluir preparação, verificação, plano, download, validação,
aplicação, conclusão e erro.

Logs por arquivo bem-sucedido devem ser evitados em catálogos grandes.
Manter checkpoints agregados e detalhes para falhas/retries.

### Downloads de arquivos grandes

Downloads de instaladores e archives usam um cliente HTTP separado, com timeout apenas para o estabelecimento da conexão. Não existe timeout total fixo para o corpo do arquivo, porque pacotes grandes podem levar mais de 60 segundos em conexões válidas. O cliente envia um `User-Agent` identificável e aceita conteúdo binário genérico. Cabeçalhos adicionais específicos de um provedor continuam podendo ser definidos em `installation.methods[].headers` no manifesto.
