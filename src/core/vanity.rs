use anyhow::{bail, Result};

const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const ALL_DIGITS: u64 = (1 << 58) - 1;

/// A conjunction of prefix and suffix constraints on a Solana Base58 address.
pub struct AddressPattern {
    prefix: String,
    suffix: String,
    ignore_case: bool,
}

impl AddressPattern {
    pub fn new(prefix: &str, suffix: &str, ignore_case: bool) -> Result<Self> {
        if prefix.is_empty() && suffix.is_empty() {
            bail!("Provide a nonempty --prefix or --suffix");
        }
        for value in [prefix, suffix] {
            if value.len() > 44 {
                bail!("Solana address patterns cannot exceed 44 characters");
            }
            if !value.bytes().all(|c| {
                BASE58.contains(&c)
                    || (ignore_case && BASE58.iter().any(|b| b.eq_ignore_ascii_case(&c)))
            }) {
                bail!("Pattern contains characters unavailable in Solana Base58 addresses");
            }
        }
        let pattern = Self {
            prefix: prefix.to_owned(),
            suffix: suffix.to_owned(),
            ignore_case,
        };
        // Both constraints may overlap. At least one possible address length must agree.
        if !(32..=44).any(|length| pattern.compatible_at_length(length)) {
            bail!("Prefix and suffix conflict for every possible Solana address length");
        }
        if !pattern.has_possible_address() {
            bail!("Pattern cannot match a generated Solana signing address");
        }
        Ok(pattern)
    }

    fn equal(&self, left: &[u8], right: &[u8]) -> bool {
        if self.ignore_case {
            left.eq_ignore_ascii_case(right)
        } else {
            left == right
        }
    }

    fn compatible_at_length(&self, length: usize) -> bool {
        if self.prefix.len().max(self.suffix.len()) > length {
            return false;
        }
        let suffix_start = length - self.suffix.len();
        if suffix_start >= self.prefix.len() {
            return true;
        }
        self.equal(
            &self.prefix.as_bytes()[suffix_start..],
            &self.suffix.as_bytes()[..self.prefix.len() - suffix_start],
        )
    }

    fn has_possible_address(&self) -> bool {
        // Every leading zero byte contributes exactly one '1'. Exclude the
        // all-zero encoding: that small-order point cannot be an Ed25519
        // signing public key, so searching for it would never succeed.
        for zero_bytes in 0..32 {
            let mut lower_bytes = [0u8; 32];
            lower_bytes[zero_bytes] = 1;
            let mut upper_bytes = [0u8; 32];
            upper_bytes[zero_bytes..].fill(255);
            // For exactly k leading zero bytes, the remaining integer ranges
            // from 256^(31-k) through 256^(32-k)-1, inclusive.
            let lower = bs58::encode(lower_bytes).into_string();
            let upper = bs58::encode(upper_bytes).into_string();
            for length in lower.len()..=upper.len() {
                if self.compatible_at_length(length)
                    && self.matches_numeric_interval(zero_bytes, length, &lower, &upper)
                {
                    return true;
                }
            }
        }
        false
    }

    fn allowed_digits(&self, position: usize, length: usize) -> u64 {
        let mut allowed = ALL_DIGITS;
        if let Some(&character) = self.prefix.as_bytes().get(position) {
            allowed &= self.character_digits(character);
        }
        let suffix_start = length - self.suffix.len();
        if position >= suffix_start {
            allowed &= self.character_digits(self.suffix.as_bytes()[position - suffix_start]);
        }
        allowed
    }

    fn character_digits(&self, character: u8) -> u64 {
        BASE58.iter().enumerate().fold(0, |mask, (digit, byte)| {
            if *byte == character || (self.ignore_case && byte.eq_ignore_ascii_case(&character)) {
                mask | (1 << digit)
            } else {
                mask
            }
        })
    }

    fn matches_numeric_interval(
        &self,
        zero_bytes: usize,
        length: usize,
        lower: &str,
        upper: &str,
    ) -> bool {
        for position in 0..zero_bytes {
            if self.allowed_digits(position, length) & 1 == 0 {
                return false;
            }
        }

        // State bits indicate equality to the lower/upper bound so far. When
        // the candidate has more/fewer digits than a bound, it is already
        // strictly above/below it. Four states suffice for exact digit DP.
        let initial =
            usize::from(length == lower.len()) | (usize::from(length == upper.len()) << 1);
        let mut states = [false; 4];
        states[initial] = true;
        for position in zero_bytes..length {
            let mut allowed = self.allowed_digits(position, length);
            if position == zero_bytes {
                // A numeric body's first digit cannot add another leading zero.
                allowed &= !1;
            }
            let lower_digit = lower
                .as_bytes()
                .get(position)
                .and_then(|byte| BASE58.iter().position(|candidate| byte == candidate))
                .unwrap_or(0);
            let upper_digit = upper
                .as_bytes()
                .get(position)
                .and_then(|byte| BASE58.iter().position(|candidate| byte == candidate))
                .unwrap_or(57);
            let mut next = [false; 4];
            for (state, active) in states.into_iter().enumerate() {
                if !active {
                    continue;
                }
                let min = if state & 1 != 0 { lower_digit } else { 0 };
                let max = if state & 2 != 0 { upper_digit } else { 57 };
                if min > max {
                    continue;
                }
                for digit in min..=max {
                    if allowed & (1 << digit) == 0 {
                        continue;
                    }
                    let lower_tight = state & 1 != 0 && digit == lower_digit;
                    let upper_tight = state & 2 != 0 && digit == upper_digit;
                    next[usize::from(lower_tight) | (usize::from(upper_tight) << 1)] = true;
                }
            }
            states = next;
            if !states.into_iter().any(|active| active) {
                return false;
            }
        }
        states.into_iter().any(|active| active)
    }

    pub fn matches(&self, address: &str) -> bool {
        let bytes = address.as_bytes();
        bytes.len() >= self.prefix.len().max(self.suffix.len())
            && self.equal(&bytes[..self.prefix.len()], self.prefix.as_bytes())
            && self.equal(
                &bytes[bytes.len() - self.suffix.len()..],
                self.suffix.as_bytes(),
            )
    }
}
