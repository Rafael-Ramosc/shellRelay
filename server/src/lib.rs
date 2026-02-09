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
    // Cria um usuário padrão se ele não existir
    if let None = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().insert(User {
            identity: ctx.sender,
            name: "Anônimo".to_string(),
            online: true,
        });
    } else {
        // Se já existe, marca como online
        if let Some(mut user) = ctx.db.user().identity().find(ctx.sender) {
            user.online = true;
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
    if let Some(mut user) = ctx.db.user().identity().find(ctx.sender) {
        user.name = new_name;
        ctx.db.user().identity().update(user);
    }
}
