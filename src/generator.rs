//! Password and passphrase generation.
//!
//! All randomness comes from the operating system CSPRNG (`OsRng`) in
//! production. Indices are drawn with rejection sampling so every character
//! or word is chosen with exactly equal probability (no modulo bias).

use rand::RngCore;
use std::fmt;
use std::sync::LazyLock;

pub const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
pub const DIGITS: &str = "0123456789";
/// Symbols that are safe to paste into most shells and forms (no quotes,
/// backslashes or backticks).
pub const SYMBOLS: &str = "!#$%&()*+,-./:;<=>?@[]^_{|}~";
/// Characters that are easily confused with one another in many fonts.
pub const LOOK_ALIKES: &str = "Il1|O0o";

/// At least one character per class, so all four classes can always be satisfied.
pub const MIN_LENGTH: usize = 4;
pub const MAX_LENGTH: usize = 256;
pub const MIN_WORDS: usize = 3;
pub const MAX_WORDS: usize = 20;
pub const MAX_SEPARATOR_LEN: usize = 8;

/// The EFF large wordlist (7776 words), one word per line.
pub const WORDLIST_RAW: &str = include_str!("../assets/wordlist.txt");

pub static WORDLIST: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| WORDLIST_RAW.lines().filter(|w| !w.is_empty()).collect());

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerateError {
    NoCharacterClasses,
    LengthOutOfRange,
    WordCountOutOfRange,
    SeparatorTooLong,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCharacterClasses => write!(f, "at least one character class must be enabled"),
            Self::LengthOutOfRange => {
                write!(f, "length must be between {MIN_LENGTH} and {MAX_LENGTH}")
            }
            Self::WordCountOutOfRange => {
                write!(f, "words must be between {MIN_WORDS} and {MAX_WORDS}")
            }
            Self::SeparatorTooLong => {
                write!(
                    f,
                    "separator must be at most {MAX_SEPARATOR_LEN} characters"
                )
            }
        }
    }
}

impl std::error::Error for GenerateError {}

#[derive(Debug, Clone)]
pub struct PasswordOptions {
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_look_alikes: bool,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 20,
            uppercase: true,
            lowercase: true,
            digits: true,
            symbols: true,
            exclude_look_alikes: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PassphraseOptions {
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        Self {
            words: 6,
            separator: "-".to_string(),
            capitalize: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Generated {
    pub value: String,
    /// Estimated entropy in bits, assuming the attacker knows the options used.
    pub entropy_bits: f64,
}

/// Returns a uniformly distributed integer in `0..n` using rejection sampling.
///
/// A plain `rng.next_u32() % n` favours small results whenever 2^32 is not a
/// multiple of `n`. Discarding draws at or above the largest multiple of `n`
/// that fits in a `u32` removes that bias.
pub fn uniform_index<R: RngCore + ?Sized>(rng: &mut R, n: usize) -> usize {
    assert!(n > 0, "cannot sample from an empty range");
    let n = u32::try_from(n).expect("range too large");
    let zone = u32::MAX - (u32::MAX % n);
    loop {
        let v = rng.next_u32();
        if v < zone {
            return (v % n) as usize;
        }
    }
}

/// Builds the enabled character classes, with look-alikes removed if requested.
fn character_classes(opts: &PasswordOptions) -> Vec<Vec<char>> {
    [
        (opts.uppercase, UPPERCASE),
        (opts.lowercase, LOWERCASE),
        (opts.digits, DIGITS),
        (opts.symbols, SYMBOLS),
    ]
    .into_iter()
    .filter(|(enabled, _)| *enabled)
    .map(|(_, set)| {
        set.chars()
            .filter(|c| !(opts.exclude_look_alikes && LOOK_ALIKES.contains(*c)))
            .collect::<Vec<char>>()
    })
    .filter(|class| !class.is_empty())
    .collect()
}

/// Generates a random password.
///
/// Every enabled character class is guaranteed to appear at least once.
/// This is done by rejection: whole candidates are drawn uniformly from the
/// combined pool and discarded if a class is missing, so the result is
/// uniform over all passwords that satisfy the constraint.
pub fn generate_password<R: RngCore + ?Sized>(
    rng: &mut R,
    opts: &PasswordOptions,
) -> Result<Generated, GenerateError> {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&opts.length) {
        return Err(GenerateError::LengthOutOfRange);
    }
    let classes = character_classes(opts);
    if classes.is_empty() {
        return Err(GenerateError::NoCharacterClasses);
    }

    let pool: Vec<char> = classes.iter().flatten().copied().collect();
    let mut candidate: Vec<char> = Vec::with_capacity(opts.length);
    loop {
        candidate.clear();
        candidate.extend((0..opts.length).map(|_| pool[uniform_index(rng, pool.len())]));
        if classes
            .iter()
            .all(|class| candidate.iter().any(|c| class.contains(c)))
        {
            break;
        }
    }

    Ok(Generated {
        value: candidate.into_iter().collect(),
        entropy_bits: opts.length as f64 * (pool.len() as f64).log2(),
    })
}

/// Generates a passphrase of words chosen uniformly from the EFF wordlist.
pub fn generate_passphrase<R: RngCore + ?Sized>(
    rng: &mut R,
    opts: &PassphraseOptions,
) -> Result<Generated, GenerateError> {
    if !(MIN_WORDS..=MAX_WORDS).contains(&opts.words) {
        return Err(GenerateError::WordCountOutOfRange);
    }
    if opts.separator.chars().count() > MAX_SEPARATOR_LEN {
        return Err(GenerateError::SeparatorTooLong);
    }

    let list = &*WORDLIST;
    let words: Vec<String> = (0..opts.words)
        .map(|_| {
            let word = list[uniform_index(rng, list.len())];
            if opts.capitalize {
                capitalize(word)
            } else {
                word.to_string()
            }
        })
        .collect();

    Ok(Generated {
        value: words.join(&opts.separator),
        entropy_bits: opts.words as f64 * (list.len() as f64).log2(),
    })
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::{OsRng, StdRng};
    use std::collections::HashSet;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(0x5eed)
    }

    /// An RNG that replays a fixed sequence, for testing rejection sampling.
    struct Scripted(Vec<u32>);

    impl RngCore for Scripted {
        fn next_u32(&mut self) -> u32 {
            self.0.remove(0)
        }
        fn next_u64(&mut self) -> u64 {
            unimplemented!()
        }
        fn fill_bytes(&mut self, _: &mut [u8]) {
            unimplemented!()
        }
        fn try_fill_bytes(&mut self, _: &mut [u8]) -> Result<(), rand::Error> {
            unimplemented!()
        }
    }

    #[test]
    fn uniform_index_rejects_values_in_the_biased_tail() {
        // For n = 10 the zone is 4294967290; anything at or above is rejected.
        let mut rng = Scripted(vec![u32::MAX, 4_294_967_290, 4_294_967_289]);
        assert_eq!(uniform_index(&mut rng, 10), 4_294_967_289 % 10);
        assert!(rng.0.is_empty());
    }

    #[test]
    fn uniform_index_stays_in_range() {
        let mut rng = rng();
        for n in [1, 2, 3, 7, 10, 62, 7776] {
            for _ in 0..1000 {
                assert!(uniform_index(&mut rng, n) < n);
            }
        }
    }

    #[test]
    fn uniform_index_is_roughly_uniform() {
        let mut rng = rng();
        let n = 10;
        let draws = 100_000;
        let mut counts = vec![0usize; n];
        for _ in 0..draws {
            counts[uniform_index(&mut rng, n)] += 1;
        }
        let expected = draws as f64 / n as f64;
        let chi_squared: f64 = counts
            .iter()
            .map(|&c| (c as f64 - expected).powi(2) / expected)
            .sum();
        // 9 degrees of freedom; 27.88 is the p = 0.001 critical value.
        assert!(
            chi_squared < 27.88,
            "chi-squared {chi_squared} too high: {counts:?}"
        );
    }

    #[test]
    fn password_has_requested_length() {
        let mut rng = rng();
        for length in [MIN_LENGTH, 12, 64, MAX_LENGTH] {
            let opts = PasswordOptions {
                length,
                ..Default::default()
            };
            let pw = generate_password(&mut rng, &opts).unwrap();
            assert_eq!(pw.value.chars().count(), length);
        }
    }

    #[test]
    fn password_contains_every_enabled_class() {
        let mut rng = rng();
        let opts = PasswordOptions {
            length: MIN_LENGTH,
            ..Default::default()
        };
        for _ in 0..500 {
            let pw = generate_password(&mut rng, &opts).unwrap().value;
            assert!(pw.chars().any(|c| UPPERCASE.contains(c)), "{pw}");
            assert!(pw.chars().any(|c| LOWERCASE.contains(c)), "{pw}");
            assert!(pw.chars().any(|c| DIGITS.contains(c)), "{pw}");
            assert!(pw.chars().any(|c| SYMBOLS.contains(c)), "{pw}");
        }
    }

    #[test]
    fn password_only_uses_enabled_classes() {
        let mut rng = rng();
        let opts = PasswordOptions {
            length: 100,
            uppercase: false,
            lowercase: false,
            digits: true,
            symbols: false,
            exclude_look_alikes: false,
        };
        let pw = generate_password(&mut rng, &opts).unwrap().value;
        assert!(pw.chars().all(|c| c.is_ascii_digit()), "{pw}");
    }

    #[test]
    fn password_excludes_look_alikes() {
        let mut rng = rng();
        let opts = PasswordOptions {
            length: MAX_LENGTH,
            exclude_look_alikes: true,
            ..Default::default()
        };
        for _ in 0..50 {
            let pw = generate_password(&mut rng, &opts).unwrap().value;
            assert!(!pw.chars().any(|c| LOOK_ALIKES.contains(c)), "{pw}");
        }
    }

    #[test]
    fn password_rejects_invalid_options() {
        let mut rng = rng();
        let none = PasswordOptions {
            uppercase: false,
            lowercase: false,
            digits: false,
            symbols: false,
            ..Default::default()
        };
        assert_eq!(
            generate_password(&mut rng, &none).unwrap_err(),
            GenerateError::NoCharacterClasses
        );
        for length in [0, MIN_LENGTH - 1, MAX_LENGTH + 1] {
            let opts = PasswordOptions {
                length,
                ..Default::default()
            };
            assert_eq!(
                generate_password(&mut rng, &opts).unwrap_err(),
                GenerateError::LengthOutOfRange
            );
        }
    }

    #[test]
    fn password_entropy_matches_pool_size() {
        let mut rng = rng();
        let opts = PasswordOptions {
            length: 10,
            uppercase: false,
            lowercase: false,
            digits: true,
            symbols: false,
            exclude_look_alikes: true,
        };
        // Digits without 0 and 1 leaves 8 characters = 3 bits each.
        let pw = generate_password(&mut rng, &opts).unwrap();
        assert!((pw.entropy_bits - 30.0).abs() < 1e-9);
    }

    #[test]
    fn passwords_are_not_repeated() {
        let opts = PasswordOptions::default();
        let set: HashSet<String> = (0..1000)
            .map(|_| generate_password(&mut OsRng, &opts).unwrap().value)
            .collect();
        assert_eq!(set.len(), 1000);
    }

    #[test]
    fn wordlist_is_the_eff_large_list() {
        assert_eq!(WORDLIST.len(), 7776);
        let unique: HashSet<_> = WORDLIST.iter().collect();
        assert_eq!(unique.len(), 7776);
        assert!(
            WORDLIST
                .iter()
                .all(|w| w.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
        );
    }

    #[test]
    fn passphrase_has_requested_words_and_separator() {
        let mut rng = rng();
        let opts = PassphraseOptions {
            words: 7,
            separator: " + ".to_string(),
            capitalize: false,
        };
        let pp = generate_passphrase(&mut rng, &opts).unwrap();
        let words: Vec<&str> = pp.value.split(" + ").collect();
        assert_eq!(words.len(), 7);
        assert!(words.iter().all(|w| WORDLIST.contains(w)), "{}", pp.value);
        assert!((pp.entropy_bits - 7.0 * 7776f64.log2()).abs() < 1e-9);
    }

    #[test]
    fn passphrase_capitalizes_words() {
        let mut rng = rng();
        let opts = PassphraseOptions {
            words: 5,
            separator: ".".to_string(),
            capitalize: true,
        };
        let pp = generate_passphrase(&mut rng, &opts).unwrap().value;
        for word in pp.split('.') {
            assert!(word.chars().next().unwrap().is_ascii_uppercase(), "{pp}");
            assert!(WORDLIST.contains(&word.to_lowercase().as_str()), "{pp}");
        }
    }

    #[test]
    fn passphrase_allows_empty_separator() {
        let mut rng = rng();
        let opts = PassphraseOptions {
            words: MIN_WORDS,
            separator: String::new(),
            capitalize: true,
        };
        let pp = generate_passphrase(&mut rng, &opts).unwrap().value;
        assert_eq!(
            pp.chars().filter(|c| c.is_ascii_uppercase()).count(),
            MIN_WORDS
        );
    }

    #[test]
    fn passphrase_rejects_invalid_options() {
        let mut rng = rng();
        for words in [0, MIN_WORDS - 1, MAX_WORDS + 1] {
            let opts = PassphraseOptions {
                words,
                ..Default::default()
            };
            assert_eq!(
                generate_passphrase(&mut rng, &opts).unwrap_err(),
                GenerateError::WordCountOutOfRange
            );
        }
        let opts = PassphraseOptions {
            separator: "x".repeat(MAX_SEPARATOR_LEN + 1),
            ..Default::default()
        };
        assert_eq!(
            generate_passphrase(&mut rng, &opts).unwrap_err(),
            GenerateError::SeparatorTooLong
        );
    }
}
