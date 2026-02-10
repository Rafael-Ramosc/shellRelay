# Shell Relay

Esse README vai ser usado para lembrar os comandos do spacetimeDB por enquanto

Lembrete rápido dos comandos de publish no SpacetimeDB para este projeto.

Versão usada no projeto:

- `spacetime 1.12.0`

Pré-requisito:

- estar logado no CLI quando publicar em servidor remoto: `spacetime login`

Publicar o módulo `server` com um nome novo de banco:

```bash
spacetime publish shell-relay-test -p server
```

Atualizar um banco já existente (usando o nome ou identity):

```bash
spacetime publish <name-ou-identity> -p server
```

Forçar mudanças que quebram clientes (sem apagar dados):

```bash
spacetime publish <name-ou-identity> -p server --break-clients
```

Permitir apagar dados só quando houver conflito de schema:

```bash
spacetime publish <name-ou-identity> -p server --break-clients --delete-data=on-conflict
```

Apagar todos os dados antes de publicar:

```bash
spacetime publish <name-ou-identity> -p server --delete-data=always --yes
```

Publicar usando um `.wasm` já compilado:

```bash
spacetime publish <name-ou-identity> -b <caminho-do-wasm>
```

Ver opções completas do comando:

```bash
spacetime publish --help
```

Observações:

- O argumento `<name-ou-identity>` aceita nome do banco ou identity.
- Nome de banco: letras minúsculas, números e `-` (ex.: `shell-relay-test`).
