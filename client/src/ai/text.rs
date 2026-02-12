use super::MAX_REPLY_CHARS;

pub(super) fn short_identity(identity: &str) -> String {
    const MAX: usize = 18;
    if identity.len() <= MAX {
        return identity.to_string();
    }

    let head = &identity[..10.min(identity.len())];
    let tail = &identity[identity.len().saturating_sub(6)..];
    format!("{head}..{tail}")
}

pub(super) fn truncate_for_context(text: &str, max_chars: usize) -> String {
    let clean = text.replace('\n', " ").trim().to_string();
    let char_count = clean.chars().count();
    if char_count <= max_chars {
        return clean;
    }

    let truncated: String = clean.chars().take(max_chars).collect();
    format!("{truncated}...")
}

pub(super) fn normalize_reply(text: &str) -> String {
    // Remove quebras e múltiplos espaços para não estourar layout no terminal.
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return String::new();
    }

    // Mantém no máximo 2 frases para resposta parecer conversa natural.
    let mut end_idx = compact.len();
    let mut sentence_count = 0usize;
    for (idx, ch) in compact.char_indices() {
        if matches!(ch, '.' | '!' | '?') {
            sentence_count += 1;
            if sentence_count >= 2 {
                end_idx = idx + ch.len_utf8();
                break;
            }
        }
    }
    let two_sentences = compact[..end_idx].trim().to_string();

    // Limite final de caracteres como fallback forte contra respostas longas.
    let mut out = if two_sentences.chars().count() > MAX_REPLY_CHARS {
        let mut cut: String = two_sentences.chars().take(MAX_REPLY_CHARS).collect();
        if !cut.ends_with('.') && !cut.ends_with('!') && !cut.ends_with('?') {
            cut.push_str("...");
        }
        cut
    } else {
        two_sentences
    };

    if out.is_empty() {
        out = compact
            .chars()
            .take(MAX_REPLY_CHARS.min(compact.chars().count()))
            .collect();
    }
    out
}
