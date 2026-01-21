pub struct MnemonicProfile {
    pub name: &'static str,
    pub path: &'static str,
}

const MNEMONIC_PROFILES: &[MnemonicProfile] = &[
    MnemonicProfile {
        name: "trustwallet",
        path: "m/44'/501'/0'",
    },
    MnemonicProfile {
        name: "phantom",
        path: "m/44'/501'/0'",
    },
    MnemonicProfile {
        name: "solflare",
        path: "m/44'/501'/0'",
    },
    MnemonicProfile {
        name: "solana_cli",
        path: "m/44'/501'/0'/0'",
    },
];

pub fn get_mnemonic_profile(name: &str) -> &'static str {
    let key = name.trim().to_lowercase();
    MNEMONIC_PROFILES
        .iter()
        .find(|profile| profile.name == key)
        .map(|profile| profile.path)
        .unwrap_or(MNEMONIC_PROFILES[0].path)
}

pub fn list_mnemonic_profiles() -> Vec<&'static str> {
    MNEMONIC_PROFILES.iter().map(|p| p.name).collect()
}
