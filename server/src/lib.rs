use spacetimedb::{Identity, ReducerContext, Table, Timestamp, reducer, table};

#[table(name = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity, // ID único da conexão (vem do SpacetimeDB)
    pub name: String, // Nome que o usuário escolher
    pub online: bool,
}

#[table(name = message, public)]
pub struct Message {
    #[primary_key]
    #[auto_inc]
    pub id: u64, // ID automático da mensagem
    pub sender: Identity,   // Quem mandou
    pub text: String,       // O conteúdo
    pub sent_at: Timestamp, // Hora do envio
}

// 2. REDUCERS (A Lógica / API)
// ---------------------------------------------------------

// Chamado automaticamente quando alguém conecta
#[reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    // Cria usuário se não existir, mas não entra no chat ainda.
    // O usuário só fica online depois de escolher nome no set_name.
    if let None = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().insert(User {
            identity: ctx.sender,
            name: "Anônimo".to_string(),
            online: false,
        });
    } else {
        // Se já existe, mantém offline até confirmar nome novamente
        if let Some(mut user) = ctx.db.user().identity().find(ctx.sender) {
            user.online = false;
            ctx.db.user().identity().update(user);
        }
    }
}

// Chamado automaticamente quando alguém desconecta
#[reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(mut user) = ctx.db.user().identity().find(ctx.sender) {
        user.online = false;
        ctx.db.user().identity().update(user);
    }
}

// Função que o Client vai chamar para enviar mensagem
#[reducer]
pub fn send_message(ctx: &ReducerContext, text: String) {
    // Só permite enviar depois de entrar no chat (online=true)
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        if !user.online {
            return;
        }
    } else {
        return;
    }

    // Validação simples: não aceita mensagem vazia
    if text.trim().is_empty() {
        return;
    }

    ctx.db.message().insert(Message {
        id: 0, // O autoinc resolve isso
        sender: ctx.sender,
        text,
        sent_at: ctx.timestamp,
    });
}

// Função para mudar o nome de usuário
#[reducer]
pub fn set_name(ctx: &ReducerContext, new_name: String) {
    let cleaned = new_name.trim().to_string();
    if cleaned.is_empty() {
        return;
    }

    if let Some(mut user) = ctx.db.user().identity().find(ctx.sender) {
        user.name = cleaned;
        user.online = true;
        ctx.db.user().identity().update(user);
    }
}
