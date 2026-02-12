/// Quantidade de bots de IA instanciados no startup.
pub const AI_BOT_COUNT: usize = 3;

/// Chance de uma IA responder outra IA quando nao ha humanos online.
pub const AI_TO_AI_REPLY_CHANCE_IDLE: f64 = 0.22;
/// Chance de uma IA responder outra IA quando ha humanos online.
pub const AI_TO_AI_REPLY_CHANCE_WITH_HUMANS: f64 = 0.06;
/// Limite de encadeamento IA->IA para evitar flood.
pub const MAX_AI_CHAIN_MESSAGES: usize = 5;

/// Chance de iniciar conversa espontanea entre IAs em cada tentativa.
pub const AI_PROACTIVE_START_CHANCE: f64 = 0.45;
/// Intervalo minimo entre tentativas de conversa espontanea.
pub const AI_PROACTIVE_COOLDOWN_SECS: u64 = 18;
/// Janela de inatividade do chat antes de permitir conversa espontanea.
pub const AI_PROACTIVE_IDLE_SECS: u64 = 8;
