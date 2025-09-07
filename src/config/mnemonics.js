// Preset mnemonic derivation profiles for Solana (ed25519/SLIP-0010)
// These capture common wallet defaults. Indices may vary by account.

export const MNEMONIC_PROFILES = {
  // Trust Wallet, Phantom and Solflare often use the non-change path at account 0
  trustwallet: { path: "m/44'/501'/0'" },
  phantom: { path: "m/44'/501'/0'" },
  solflare: { path: "m/44'/501'/0'" },
  // Solana CLI-compatible hardened path variant (frequently seen)
  solana_cli: { path: "m/44'/501'/0'/0'" },
};

export function getMnemonicProfile(name = 'trustwallet') {
  const key = String(name || '').toLowerCase();
  return MNEMONIC_PROFILES[key] || MNEMONIC_PROFILES.trustwallet;
}

export function listMnemonicProfiles() {
  return Object.keys(MNEMONIC_PROFILES);
}

