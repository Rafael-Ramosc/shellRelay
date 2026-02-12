use rand::{
    prelude::{IndexedRandom, SliceRandom},
    rng,
};

/// Nomes base disponíveis para seleção no startup.
pub const FANTASY_NAMES: &[&str] = &[
    "Aelric",
    "Branna",
    "Cedric",
    "Darian",
    "Elowen",
    "Faelar",
    "Gareth",
    "Isolde",
    "Kael",
    "Lyria",
    "Morgana",
    "Nimue",
    "Orin",
    "Rowan",
    "Seraphina",
    "Thorin",
    "Valen",
    "Ysolda",
];

/// Profissões clássicas de RPG de fantasia.
pub const RPG_PROFESSIONS: &[&str] = &[
    "Mago",
    "Guerreiro",
    "Ladino",
    "Clerigo",
    "Ranger",
    "Bardo",
    "Paladino",
    "Druida",
    "Feiticeiro",
    "Monge",
];

#[derive(Clone, Debug)]
pub struct AiBotProfile {
    pub name: String,
    pub profession: String,
}

/// Gera perfis de bots com nomes/profissões sorteados no início da aplicação.
pub fn generate_bot_profiles(count: usize) -> Vec<AiBotProfile> {
    if count == 0 {
        return Vec::new();
    }

    let mut rng = rng();
    let mut names_pool: Vec<&str> = FANTASY_NAMES.to_vec();
    names_pool.shuffle(&mut rng);

    (0..count)
        .map(|i| {
            let base_name = names_pool[i % names_pool.len()];
            let name = if i < names_pool.len() {
                base_name.to_string()
            } else {
                format!("{base_name}-{}", (i / names_pool.len()) + 1)
            };

            let profession = RPG_PROFESSIONS
                .choose(&mut rng)
                .copied()
                .unwrap_or("Aventureiro")
                .to_string();

            AiBotProfile { name, profession }
        })
        .collect()
}

pub fn profession_roleplay_style(profession: &str) -> &'static str {
    match profession {
        "Mago" => "Tom curioso e observador, com referencias leves a magia quando couber.",
        "Guerreiro" => "Tom direto e pratico, sem grosseria.",
        "Ladino" => "Tom esperto, com humor seco ocasional.",
        "Clerigo" => "Tom acolhedor e tranquilo, sem sermoes.",
        "Ranger" => "Tom pratico, com exemplos de trilha e natureza quando fizer sentido.",
        "Bardo" => "Tom sociavel e criativo, sem exagero poetico.",
        "Paladino" => "Tom firme e confiavel, sem moralismo.",
        "Druida" => "Tom calmo e equilibrado, com referencias sutis a natureza.",
        "Feiticeiro" => "Tom confiante e espontaneo, com energia leve.",
        "Monge" => "Tom objetivo e centrado, sem rigidez.",
        _ => "Fale como uma pessoa natural, objetiva e respeitosa.",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{FANTASY_NAMES, RPG_PROFESSIONS, generate_bot_profiles};

    #[test]
    fn generate_bot_profiles_respects_count() {
        let bots = generate_bot_profiles(4);
        assert_eq!(bots.len(), 4);
    }

    #[test]
    fn generated_bots_use_known_name_bases_and_professions() {
        let bots = generate_bot_profiles(6);
        let name_bases: HashSet<&str> = FANTASY_NAMES.iter().copied().collect();
        let professions: HashSet<&str> = RPG_PROFESSIONS.iter().copied().collect();

        for bot in bots {
            let base = bot.name.split('-').next().unwrap_or_default();
            assert!(name_bases.contains(base));
            assert!(professions.contains(bot.profession.as_str()));
        }
    }
}
