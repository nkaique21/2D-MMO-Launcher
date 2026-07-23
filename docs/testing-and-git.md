# Testes, checkpoints e Git

## Validação padrão

Escolha os comandos aplicáveis:

```fish
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Também:

```fish
git status --short
git diff --stat
git diff --name-status
```

## Testes específicos

### Manifestos

- validar JSON;
- garantir paridade Rust/TypeScript;
- testar manifesto sem os novos campos;
- testar valores inválidos e paths inseguros.

### Banco

- banco vazio;
- upgrade de banco anterior;
- preservação de dados;
- versão futura;
- CRUD do novo modelo.

### Download e extração

- timeout;
- retry;
- nome com caractere reservado;
- path traversal;
- arquivo ausente;
- checksum divergente;
- staging inválido;
- falha antes da aplicação.

### Runners

- categoria genérica;
- ID concreto;
- runner ausente;
- ambiente aplicado;
- ambiente removido;
- prefixo;
- working directory;
- erro pré-spawn.

## Checkpoint manual

Quando necessário, dê ao usuário um teste curto:

1. abrir o Tauri recompilado;
2. executar uma ação concreta;
3. observar um resultado;
4. verificar uma chave específica no log;
5. devolver apenas o resultado relevante.

Não pedir testes vagos como “vê se funcionou”.

## Conclusão de etapa

Antes do commit:

1. revisar código;
2. executar validações;
3. obter aprovação quando houver teste real;
4. atualizar documentação;
5. conferir `STATE.md`;
6. conferir `git status`;
7. criar commit específico;
8. fazer push quando autorizado pelo fluxo.

## Documentação antes do commit

- estado atual mudou → `PROJECT.md`;
- fato durável surgiu → `MEMORY.md`;
- detalhe de domínio mudou → documento temático;
- decisão arquitetural → ADR;
- etapa atual → `STATE.md`;
- nunca registrar changelog em `AGENTS.md`.

## Push

- não usar force;
- parar em erro de credencial, rede ou divergência;
- informar o erro;
- não iniciar nova etapa antes de resolver, quando o fluxo exigir push.
