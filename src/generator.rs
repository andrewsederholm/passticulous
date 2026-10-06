//! Password, passphrase and pattern generation.
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
/// Every printable ASCII symbol. Pattern mode lets the user pick any subset.
pub const ALL_SYMBOLS: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
pub const DEFAULT_PATTERN_SYMBOLS: &str = "!@#$%&*?";
/// Characters that are easily confused with one another in many fonts.
pub const LOOK_ALIKES: &str = "Il1|O0o";

/// At least one character per class, so all four classes can always be satisfied.
pub const MIN_LENGTH: usize = 4;
pub const MAX_LENGTH: usize = 256;
pub const MIN_WORDS: usize = 3;
pub const MAX_WORDS: usize = 20;
pub const MAX_SEPARATOR_LEN: usize = 8;
pub const MIN_PATTERN_WORDS: usize = 1;
pub const MAX_PATTERN_WORDS: usize = 5;
pub const MAX_PATTERN_DIGITS: usize = 16;
pub const MAX_PATTERN_SYMBOLS: usize = 16;
/// Simple mode: symbols that are easy to say out loud.
pub const SIMPLE_SYMBOLS: &str = "!@#$?";
pub const MIN_SIMPLE_LENGTH: usize = 10;
pub const MAX_SIMPLE_LENGTH: usize = 16;
pub const DEFAULT_SIMPLE_LENGTH: usize = 12;
pub const MAX_SIMPLE_WORD_LEN: usize = 32;
/// Simple passwords always have at least this many digits...
const MIN_SIMPLE_DIGITS: usize = 2;
/// ...and random words are long enough to need at most this many.
const MAX_SIMPLE_DIGITS: usize = 5;
/// Random words are at least this long, so they don't feel like filler.
const MIN_SIMPLE_WORD_LEN: usize = 6;
/// Themed lists whose words are easy to say and spell, for simple mode.
pub const SIMPLE_CATEGORIES: [WordCategory; 5] = [
    WordCategory::Animals,
    WordCategory::Colors,
    WordCategory::Foods,
    WordCategory::Nature,
    WordCategory::Space,
];

/// The main wordlist (14,014 common lowercase words of 6–10 letters), one word per line.
pub const WORDLIST_RAW: &str = include_str!("../assets/wordlist.txt");

pub static WORDLIST: LazyLock<Vec<&'static str>> = LazyLock::new(|| parse_wordlist(WORDLIST_RAW));

fn parse_wordlist(raw: &'static str) -> Vec<&'static str> {
    raw.lines().filter(|w| !w.is_empty()).collect()
}

/// Themed word lists for pattern mode. `Any` is the full main wordlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordCategory {
    Any,
    Animals,
    Colors,
    Foods,
    Nature,
    Space,
    Elements,
}

static CATEGORY_WORDS: LazyLock<Vec<Vec<&'static str>>> = LazyLock::new(|| {
    WordCategory::ALL
        .iter()
        .map(|c| parse_wordlist(c.raw()))
        .collect()
});

impl WordCategory {
    pub const ALL: [WordCategory; 7] = [
        Self::Any,
        Self::Animals,
        Self::Colors,
        Self::Foods,
        Self::Nature,
        Self::Space,
        Self::Elements,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Animals => "animals",
            Self::Colors => "colors",
            Self::Foods => "foods",
            Self::Nature => "nature",
            Self::Space => "space",
            Self::Elements => "elements",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }

    /// The list as stored on disk, one word per line.
    pub fn raw(self) -> &'static str {
        match self {
            Self::Any => WORDLIST_RAW,
            Self::Animals => include_str!("../assets/categories/animals.txt"),
            Self::Colors => include_str!("../assets/categories/colors.txt"),
            Self::Foods => include_str!("../assets/categories/foods.txt"),
            Self::Nature => include_str!("../assets/categories/nature.txt"),
            Self::Space => include_str!("../assets/categories/space.txt"),
            Self::Elements => include_str!("../assets/categories/elements.txt"),
        }
    }

    pub fn words(self) -> &'static [&'static str] {
        &CATEGORY_WORDS[self as usize]
    }

    /// Length of the longest word in the list.
    pub fn longest_word(self) -> usize {
        self.words().iter().map(|w| w.len()).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordCase {
    /// `Giraffe`
    Title,
    /// `giraffe`
    Lower,
    /// `GIRAFFE`
    Upper,
}

impl WordCase {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "title" => Some(Self::Title),
            "lower" => Some(Self::Lower),
            "upper" => Some(Self::Upper),
            _ => None,
        }
    }

    fn apply(self, word: &str) -> String {
        match self {
            Self::Title => capitalize(word),
            Self::Lower => word.to_lowercase(),
            Self::Upper => word.to_uppercase(),
        }
    }
}

/// One section of a pattern password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Word,
    Digits,
    Symbols,
}

impl Part {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "word" | "words" => Some(Self::Word),
            "digits" | "numbers" => Some(Self::Digits),
            "symbols" => Some(Self::Symbols),
            _ => None,
        }
    }

    /// Parses an order like `word,digits,symbols`. Each part must appear once.
    pub fn parse_order(s: &str) -> Option<[Part; 3]> {
        let parts: Vec<Part> = s
            .split(',')
            .map(|p| Part::from_name(p.trim()))
            .collect::<Option<_>>()?;
        let order: [Part; 3] = parts.try_into().ok()?;
        is_permutation(&order).then_some(order)
    }
}

fn is_permutation(order: &[Part; 3]) -> bool {
    [Part::Word, Part::Digits, Part::Symbols]
        .iter()
        .all(|p| order.contains(p))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerateError {
    NoCharacterClasses,
    LengthOutOfRange,
    WordCountOutOfRange,
    SeparatorTooLong,
    PatternWordsOutOfRange,
    DigitCountOutOfRange,
    SymbolCountOutOfRange,
    NoSymbolsSelected,
    InvalidSymbol(char),
    FixedSymbolsOutOfRange,
    InvalidOrder,
    SimpleLengthOutOfRange,
    InvalidSimpleWord,
    NoWordsLongEnough(WordCategory),
    NoWordsOfLength(WordCategory, usize),
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
            Self::PatternWordsOutOfRange => write!(
                f,
                "words must be between {MIN_PATTERN_WORDS} and {MAX_PATTERN_WORDS}"
            ),
            Self::DigitCountOutOfRange => {
                write!(f, "digit count must be between 0 and {MAX_PATTERN_DIGITS}")
            }
            Self::SymbolCountOutOfRange => {
                write!(
                    f,
                    "symbol count must be between 0 and {MAX_PATTERN_SYMBOLS}"
                )
            }
            Self::NoSymbolsSelected => write!(f, "choose at least one symbol to use"),
            Self::InvalidSymbol(c) => write!(f, "'{c}' is not an allowed symbol"),
            Self::FixedSymbolsOutOfRange => {
                write!(
                    f,
                    "fixed symbols must be between 1 and {MAX_PATTERN_SYMBOLS} characters"
                )
            }
            Self::InvalidOrder => {
                write!(f, "order must list word, digits and symbols once each")
            }
            Self::SimpleLengthOutOfRange => write!(
                f,
                "min_length must be between {MIN_SIMPLE_LENGTH} and {MAX_SIMPLE_LENGTH}"
            ),
            Self::InvalidSimpleWord => write!(
                f,
                "word must be 1 to {MAX_SIMPLE_WORD_LEN} characters with no spaces"
            ),
            Self::NoWordsLongEnough(category) => write!(
                f,
                "min_word_length is too long: the longest {} word is {} characters",
                category.name(),
                category.longest_word()
            ),
            Self::NoWordsOfLength(category, length) => write!(
                f,
                "no {} words are exactly {length} characters long",
                category.name()
            ),
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

/// Options for pattern passwords such as `Giraffe3287#@!`.
#[derive(Debug, Clone)]
pub struct PatternOptions {
    pub category: WordCategory,
    pub words: usize,
    /// Only words at least this many characters long are used. 0 for any.
    pub min_word_length: usize,
    /// When set, only words exactly this long are used, and
    /// `min_word_length` is ignored.
    pub word_length: Option<usize>,
    pub case: WordCase,
    pub digits: usize,
    pub symbols: usize,
    /// The symbols to choose from. Duplicates are ignored.
    pub symbol_set: String,
    /// When set, this exact run of symbols is used every time instead of
    /// random ones, and `symbols` and `symbol_set` are ignored.
    pub fixed_symbols: Option<String>,
    pub order: [Part; 3],
    /// Placed between every word and section. Empty by default.
    pub separator: String,
}

impl Default for PatternOptions {
    fn default() -> Self {
        Self {
            category: WordCategory::Animals,
            words: 1,
            min_word_length: 0,
            word_length: None,
            case: WordCase::Title,
            digits: 4,
            symbols: 3,
            symbol_set: DEFAULT_PATTERN_SYMBOLS.to_string(),
            fixed_symbols: None,
            order: [Part::Word, Part::Digits, Part::Symbols],
            separator: String::new(),
        }
    }
}

/// Options for simple temporary passwords such as `Giraffe4821!`.
#[derive(Debug, Clone)]
pub struct SimpleOptions {
    /// Digits are added until the password is at least this long.
    pub min_length: usize,
    /// A word to use every time (e.g. `Welcome` or a company name) instead
    /// of a random one. Used exactly as typed.
    pub word: Option<String>,
}

impl Default for SimpleOptions {
    fn default() -> Self {
        Self {
            min_length: DEFAULT_SIMPLE_LENGTH,
            word: None,
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

/// Generates a passphrase of words chosen uniformly from the main wordlist.
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

/// Generates a pattern password: themed words, then a run of digits, then a
/// run of symbols (in the configured order). Each word, digit and symbol is
/// chosen independently and uniformly, unless fixed symbols are given.
pub fn generate_pattern<R: RngCore + ?Sized>(
    rng: &mut R,
    opts: &PatternOptions,
) -> Result<Generated, GenerateError> {
    if !(MIN_PATTERN_WORDS..=MAX_PATTERN_WORDS).contains(&opts.words) {
        return Err(GenerateError::PatternWordsOutOfRange);
    }
    if opts.digits > MAX_PATTERN_DIGITS {
        return Err(GenerateError::DigitCountOutOfRange);
    }
    if opts.separator.chars().count() > MAX_SEPARATOR_LEN {
        return Err(GenerateError::SeparatorTooLong);
    }
    if !is_permutation(&opts.order) {
        return Err(GenerateError::InvalidOrder);
    }
    // Fixed symbols are used verbatim (repeats allowed); otherwise symbols
    // are drawn from the de-duplicated set.
    let symbols = match &opts.fixed_symbols {
        Some(fixed) => {
            if !(1..=MAX_PATTERN_SYMBOLS).contains(&fixed.chars().count()) {
                return Err(GenerateError::FixedSymbolsOutOfRange);
            }
            if let Some(c) = fixed.chars().find(|&c| !ALL_SYMBOLS.contains(c)) {
                return Err(GenerateError::InvalidSymbol(c));
            }
            SymbolSource::Fixed(fixed)
        }
        None => {
            if opts.symbols > MAX_PATTERN_SYMBOLS {
                return Err(GenerateError::SymbolCountOutOfRange);
            }
            let mut set: Vec<char> = Vec::new();
            for c in opts.symbol_set.chars() {
                if !ALL_SYMBOLS.contains(c) {
                    return Err(GenerateError::InvalidSymbol(c));
                }
                if !set.contains(&c) {
                    set.push(c);
                }
            }
            if opts.symbols > 0 && set.is_empty() {
                return Err(GenerateError::NoSymbolsSelected);
            }
            SymbolSource::Random(set)
        }
    };

    let list: Vec<&str> = opts
        .category
        .words()
        .iter()
        .copied()
        .filter(|w| match opts.word_length {
            Some(length) => w.len() == length,
            None => w.len() >= opts.min_word_length,
        })
        .collect();
    if list.is_empty() {
        return Err(match opts.word_length {
            Some(length) => GenerateError::NoWordsOfLength(opts.category, length),
            None => GenerateError::NoWordsLongEnough(opts.category),
        });
    }
    let digits: Vec<char> = DIGITS.chars().collect();
    let mut segments: Vec<String> = Vec::new();
    for part in opts.order {
        match part {
            Part::Word => segments.extend(
                (0..opts.words).map(|_| opts.case.apply(list[uniform_index(rng, list.len())])),
            ),
            Part::Digits if opts.digits > 0 => segments.push(
                (0..opts.digits)
                    .map(|_| digits[uniform_index(rng, digits.len())])
                    .collect(),
            ),
            Part::Symbols => match &symbols {
                SymbolSource::Fixed(fixed) => segments.push(fixed.to_string()),
                SymbolSource::Random(set) if opts.symbols > 0 => segments.push(
                    (0..opts.symbols)
                        .map(|_| set[uniform_index(rng, set.len())])
                        .collect(),
                ),
                SymbolSource::Random(_) => {}
            },
            Part::Digits => {}
        }
    }

    let mut entropy_bits = opts.words as f64 * (list.len() as f64).log2()
        + opts.digits as f64 * (digits.len() as f64).log2();
    // Fixed symbols are known to the attacker, so they add no entropy.
    if let SymbolSource::Random(set) = &symbols
        && opts.symbols > 0
    {
        entropy_bits += opts.symbols as f64 * (set.len() as f64).log2();
    }

    Ok(Generated {
        value: segments.join(&opts.separator),
        entropy_bits,
    })
}

/// The words simple mode picks from: every themed word (except elements) of
/// at least `MIN_SIMPLE_WORD_LEN` letters, de-duplicated.
static SIMPLE_WORDS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut words: Vec<&'static str> = SIMPLE_CATEGORIES
        .iter()
        .flat_map(|c| c.words().iter().copied())
        .filter(|w| w.len() >= MIN_SIMPLE_WORD_LEN)
        .collect();
    words.sort_unstable();
    words.dedup();
    words
});

/// Digits needed after `word` to reach `min_length`, counting the one symbol.
fn simple_digit_count(word_len: usize, min_length: usize) -> usize {
    min_length
        .saturating_sub(word_len + 1)
        .max(MIN_SIMPLE_DIGITS)
}

/// Generates a simple temporary password: a title-case word, then digits,
/// then one symbol, at least `min_length` characters long. Easy to read out
/// to a new hire, but weak, so only for passwords that are changed at first
/// sign-in.
pub fn generate_simple<R: RngCore + ?Sized>(
    rng: &mut R,
    opts: &SimpleOptions,
) -> Result<Generated, GenerateError> {
    if !(MIN_SIMPLE_LENGTH..=MAX_SIMPLE_LENGTH).contains(&opts.min_length) {
        return Err(GenerateError::SimpleLengthOutOfRange);
    }
    let symbols: Vec<char> = SIMPLE_SYMBOLS.chars().collect();
    let symbol_bits = (symbols.len() as f64).log2();

    let (word, entropy_bits) = match &opts.word {
        Some(word) => {
            let len = word.chars().count();
            if !(1..=MAX_SIMPLE_WORD_LEN).contains(&len)
                || word.chars().any(|c| c.is_whitespace() || c.is_control())
            {
                return Err(GenerateError::InvalidSimpleWord);
            }
            // The word is known to anyone who knows the setup, so only the
            // digits and symbol count.
            let digits = simple_digit_count(len, opts.min_length);
            (word.clone(), digits as f64 * 10f64.log2() + symbol_bits)
        }
        None => {
            // Only words that need at most MAX_SIMPLE_DIGITS digits.
            let min_word_len = opts
                .min_length
                .saturating_sub(MAX_SIMPLE_DIGITS + 1)
                .max(MIN_SIMPLE_WORD_LEN);
            let list: Vec<&str> = SIMPLE_WORDS
                .iter()
                .copied()
                .filter(|w| w.len() >= min_word_len)
                .collect();
            // Shorter words get more digits, so count every possible result.
            let outcomes: f64 = list
                .iter()
                .map(|w| 10f64.powi(simple_digit_count(w.len(), opts.min_length) as i32))
                .sum();
            let word = capitalize(list[uniform_index(rng, list.len())]);
            (word, outcomes.log2() + symbol_bits)
        }
    };

    let digit_count = simple_digit_count(word.chars().count(), opts.min_length);
    let digits: Vec<char> = DIGITS.chars().collect();
    let mut value = word;
    value.extend((0..digit_count).map(|_| digits[uniform_index(rng, digits.len())]));
    value.push(symbols[uniform_index(rng, symbols.len())]);

    Ok(Generated {
        value,
        entropy_bits,
    })
}

enum SymbolSource<'a> {
    Fixed(&'a str),
    Random(Vec<char>),
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
    fn wordlist_is_unique_lowercase_words() {
        assert_eq!(WORDLIST.len(), 14_014);
        let unique: HashSet<_> = WORDLIST.iter().collect();
        assert_eq!(unique.len(), 14_014);
        assert!(
            WORDLIST
                .iter()
                .all(|w| (6..=10).contains(&w.len()) && w.chars().all(|c| c.is_ascii_lowercase()))
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
        assert!((pp.entropy_bits - 7.0 * (WORDLIST.len() as f64).log2()).abs() < 1e-9);
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

    #[test]
    fn all_symbols_is_printable_ascii_punctuation() {
        let expected: String = (b'!'..=b'~')
            .map(char::from)
            .filter(|c| c.is_ascii_punctuation())
            .collect();
        let mut actual: Vec<char> = ALL_SYMBOLS.chars().collect();
        actual.sort_unstable();
        assert_eq!(actual.into_iter().collect::<String>(), expected);
        assert!(SYMBOLS.chars().all(|c| ALL_SYMBOLS.contains(c)));
        assert!(
            DEFAULT_PATTERN_SYMBOLS
                .chars()
                .all(|c| ALL_SYMBOLS.contains(c))
        );
    }

    #[test]
    fn category_lists_are_clean() {
        for category in WordCategory::ALL {
            let words = category.words();
            assert!(
                words.len() >= 90,
                "{} has {} words",
                category.name(),
                words.len()
            );
            let unique: HashSet<_> = words.iter().collect();
            assert_eq!(
                unique.len(),
                words.len(),
                "{} has duplicates",
                category.name()
            );
            assert!(
                words.iter().all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c == '-')),
                "{} has an invalid word",
                category.name()
            );
            assert_eq!(WordCategory::from_name(category.name()), Some(category));
        }
        assert_eq!(WordCategory::Any.words().len(), WORDLIST.len());
        assert!(WordCategory::Animals.words().contains(&"giraffe"));
        assert_eq!(WordCategory::Elements.words().len(), 118);
        assert!(WordCategory::Elements.words().contains(&"rhodium"));
        assert_eq!(WordCategory::from_name("dinosaurs"), None);
    }

    #[test]
    fn pattern_default_looks_like_word_digits_symbols() {
        let mut rng = rng();
        let opts = PatternOptions::default();
        for _ in 0..200 {
            let pw = generate_pattern(&mut rng, &opts).unwrap().value;
            let word: String = pw.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            let rest = &pw[word.len()..];
            assert!(
                WordCategory::Animals
                    .words()
                    .contains(&word.to_lowercase().as_str()),
                "{pw}"
            );
            assert!(word.starts_with(|c: char| c.is_ascii_uppercase()), "{pw}");
            assert!(word[1..].chars().all(|c| c.is_ascii_lowercase()), "{pw}");
            assert_eq!(rest.len(), 7, "{pw}");
            assert!(rest[..4].chars().all(|c| c.is_ascii_digit()), "{pw}");
            assert!(
                rest[4..]
                    .chars()
                    .all(|c| DEFAULT_PATTERN_SYMBOLS.contains(c)),
                "{pw}"
            );
        }
    }

    #[test]
    fn pattern_respects_order_case_separator_and_symbol_set() {
        let mut rng = rng();
        let opts = PatternOptions {
            category: WordCategory::Colors,
            words: 2,
            min_word_length: 0,
            word_length: None,
            case: WordCase::Upper,
            digits: 3,
            symbols: 2,
            symbol_set: "##@".to_string(),
            fixed_symbols: None,
            order: [Part::Symbols, Part::Word, Part::Digits],
            separator: ".".to_string(),
        };
        for _ in 0..100 {
            let pp = generate_pattern(&mut rng, &opts).unwrap();
            let segments: Vec<&str> = pp.value.split('.').collect();
            assert_eq!(segments.len(), 4, "{}", pp.value);
            assert!(segments[0].len() == 2 && segments[0].chars().all(|c| c == '#' || c == '@'));
            for word in &segments[1..3] {
                assert!(word.chars().all(|c| c.is_ascii_uppercase()), "{}", pp.value);
                assert!(
                    WordCategory::Colors
                        .words()
                        .contains(&word.to_lowercase().as_str())
                );
            }
            assert!(segments[3].len() == 3 && segments[3].chars().all(|c| c.is_ascii_digit()));
            // Duplicate '#' is ignored: two symbols from a set of 2 = 2 bits.
            let colors = WordCategory::Colors.words().len() as f64;
            let expected = 2.0 * colors.log2() + 3.0 * 10f64.log2() + 2.0;
            assert!((pp.entropy_bits - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn pattern_uses_fixed_symbols_verbatim() {
        let mut rng = rng();
        let opts = PatternOptions {
            symbols: 0,
            symbol_set: String::new(),
            fixed_symbols: Some("#@!!".to_string()),
            order: [Part::Word, Part::Symbols, Part::Digits],
            separator: "-".to_string(),
            ..Default::default()
        };
        for _ in 0..50 {
            let pp = generate_pattern(&mut rng, &opts).unwrap();
            let segments: Vec<&str> = pp.value.split('-').collect();
            assert_eq!(segments.len(), 3, "{}", pp.value);
            assert_eq!(segments[1], "#@!!", "{}", pp.value);
            let animals = WordCategory::Animals.words().len() as f64;
            let expected = animals.log2() + 4.0 * 10f64.log2();
            assert!((pp.entropy_bits - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn pattern_can_skip_digits_and_symbols() {
        let mut rng = rng();
        let opts = PatternOptions {
            words: 3,
            case: WordCase::Lower,
            digits: 0,
            symbols: 0,
            symbol_set: String::new(),
            separator: "-".to_string(),
            ..Default::default()
        };
        let pw = generate_pattern(&mut rng, &opts).unwrap().value;
        let words: Vec<&str> = pw.split('-').collect();
        assert_eq!(words.len(), 3, "{pw}");
        assert!(
            words
                .iter()
                .all(|w| WordCategory::Animals.words().contains(w)),
            "{pw}"
        );
    }

    #[test]
    fn pattern_only_uses_words_of_min_word_length() {
        let mut rng = rng();
        let opts = PatternOptions {
            category: WordCategory::Elements,
            min_word_length: 11,
            case: WordCase::Lower,
            digits: 0,
            symbols: 0,
            ..Default::default()
        };
        let long: Vec<&str> = WordCategory::Elements
            .words()
            .iter()
            .copied()
            .filter(|w| w.len() >= 11)
            .collect();
        assert_eq!(long.len(), 10);
        for _ in 0..200 {
            let pp = generate_pattern(&mut rng, &opts).unwrap();
            assert!(long.contains(&pp.value.as_str()), "{}", pp.value);
            // Only the matching words count toward strength.
            assert!((pp.entropy_bits - 10f64.log2()).abs() < 1e-9);
        }

        // The longest word is always allowed; one character more is not.
        for category in WordCategory::ALL {
            let longest = category.longest_word();
            let at_longest = PatternOptions {
                category,
                min_word_length: longest,
                ..Default::default()
            };
            let pw = generate_pattern(&mut rng, &at_longest).unwrap();
            assert!(pw.value.len() >= longest, "{}", pw.value);
            let too_long = PatternOptions {
                min_word_length: longest + 1,
                ..at_longest
            };
            assert_eq!(
                generate_pattern(&mut rng, &too_long).unwrap_err(),
                GenerateError::NoWordsLongEnough(category)
            );
        }
    }

    #[test]
    fn pattern_only_uses_words_of_exact_word_length() {
        let mut rng = rng();
        let opts = PatternOptions {
            category: WordCategory::Animals,
            // Ignored when word_length is set.
            min_word_length: 11,
            word_length: Some(5),
            case: WordCase::Lower,
            digits: 0,
            symbols: 0,
            ..Default::default()
        };
        for _ in 0..200 {
            let pp = generate_pattern(&mut rng, &opts).unwrap();
            assert_eq!(pp.value.len(), 5, "{}", pp.value);
            assert!(WordCategory::Animals.words().contains(&pp.value.as_str()));
            // 45 animals have 5 letters.
            assert!((pp.entropy_bits - 45f64.log2()).abs() < 1e-9);
        }

        // Space has 10- and 13-letter words but none in between.
        for (length, ok) in [(10, true), (12, false), (13, true), (0, false)] {
            let opts = PatternOptions {
                category: WordCategory::Space,
                word_length: Some(length),
                ..Default::default()
            };
            match generate_pattern(&mut rng, &opts) {
                Ok(_) => assert!(ok, "{length}"),
                Err(err) => {
                    assert!(!ok, "{length}");
                    assert_eq!(err, GenerateError::NoWordsOfLength(WordCategory::Space, length));
                }
            }
        }
    }

    #[test]
    fn pattern_rejects_invalid_options() {
        let mut rng = rng();
        let cases = [
            (
                PatternOptions {
                    words: 0,
                    ..Default::default()
                },
                GenerateError::PatternWordsOutOfRange,
            ),
            (
                PatternOptions {
                    words: MAX_PATTERN_WORDS + 1,
                    ..Default::default()
                },
                GenerateError::PatternWordsOutOfRange,
            ),
            (
                PatternOptions {
                    digits: MAX_PATTERN_DIGITS + 1,
                    ..Default::default()
                },
                GenerateError::DigitCountOutOfRange,
            ),
            (
                PatternOptions {
                    symbols: MAX_PATTERN_SYMBOLS + 1,
                    ..Default::default()
                },
                GenerateError::SymbolCountOutOfRange,
            ),
            (
                PatternOptions {
                    symbol_set: String::new(),
                    ..Default::default()
                },
                GenerateError::NoSymbolsSelected,
            ),
            (
                PatternOptions {
                    symbol_set: "#a".to_string(),
                    ..Default::default()
                },
                GenerateError::InvalidSymbol('a'),
            ),
            (
                PatternOptions {
                    fixed_symbols: Some(String::new()),
                    ..Default::default()
                },
                GenerateError::FixedSymbolsOutOfRange,
            ),
            (
                PatternOptions {
                    fixed_symbols: Some("#".repeat(MAX_PATTERN_SYMBOLS + 1)),
                    ..Default::default()
                },
                GenerateError::FixedSymbolsOutOfRange,
            ),
            (
                PatternOptions {
                    fixed_symbols: Some("#a!".to_string()),
                    ..Default::default()
                },
                GenerateError::InvalidSymbol('a'),
            ),
            (
                PatternOptions {
                    order: [Part::Word, Part::Word, Part::Digits],
                    ..Default::default()
                },
                GenerateError::InvalidOrder,
            ),
            (
                PatternOptions {
                    separator: "x".repeat(MAX_SEPARATOR_LEN + 1),
                    ..Default::default()
                },
                GenerateError::SeparatorTooLong,
            ),
        ];
        for (opts, expected) in cases {
            assert_eq!(
                generate_pattern(&mut rng, &opts).unwrap_err(),
                expected,
                "{opts:?}"
            );
        }
    }

    #[test]
    fn simple_is_word_digits_symbol_of_at_least_min_length() {
        let mut rng = rng();
        for min_length in MIN_SIMPLE_LENGTH..=MAX_SIMPLE_LENGTH {
            let opts = SimpleOptions {
                min_length,
                word: None,
            };
            for _ in 0..200 {
                let g = generate_simple(&mut rng, &opts).unwrap();
                let v = &g.value;
                assert!(v.len() >= min_length, "{v}");
                let word: String = v.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
                assert!(word.len() >= MIN_SIMPLE_WORD_LEN, "{v}");
                assert!(SIMPLE_WORDS.contains(&word.to_lowercase().as_str()), "{v}");
                assert!(word.starts_with(|c: char| c.is_ascii_uppercase()), "{v}");
                let digits = &v[word.len()..v.len() - 1];
                assert!(
                    (MIN_SIMPLE_DIGITS..=MAX_SIMPLE_DIGITS).contains(&digits.len()),
                    "{v}"
                );
                assert!(digits.chars().all(|c| c.is_ascii_digit()), "{v}");
                assert!(SIMPLE_SYMBOLS.contains(v.chars().last().unwrap()), "{v}");
            }
        }
    }

    #[test]
    fn simple_pads_a_custom_word_with_digits() {
        let mut rng = rng();
        let opts = |word: &str| SimpleOptions {
            min_length: 12,
            word: Some(word.to_string()),
        };
        let g = generate_simple(&mut rng, &opts("Welcome")).unwrap();
        assert!(g.value.starts_with("Welcome"), "{}", g.value);
        assert_eq!(g.value.len(), 12, "{}", g.value);
        // 4 digits and one of 5 symbols; the word itself adds nothing.
        assert!((g.entropy_bits - (4.0 * 10f64.log2() + 5f64.log2())).abs() < 1e-9);

        let g = generate_simple(&mut rng, &opts("Acme")).unwrap();
        assert_eq!(g.value.len(), 12, "{}", g.value);
        // Long words still get at least two digits and a symbol.
        let g = generate_simple(&mut rng, &opts("Organization")).unwrap();
        assert_eq!(g.value.len(), 15, "{}", g.value);
    }

    #[test]
    fn simple_rejects_invalid_options() {
        let mut rng = rng();
        for min_length in [MIN_SIMPLE_LENGTH - 1, MAX_SIMPLE_LENGTH + 1] {
            let opts = SimpleOptions {
                min_length,
                word: None,
            };
            assert_eq!(
                generate_simple(&mut rng, &opts).unwrap_err(),
                GenerateError::SimpleLengthOutOfRange
            );
        }
        for word in ["", "two words", &"a".repeat(MAX_SIMPLE_WORD_LEN + 1)] {
            let opts = SimpleOptions {
                min_length: 12,
                word: Some(word.to_string()),
            };
            assert_eq!(
                generate_simple(&mut rng, &opts).unwrap_err(),
                GenerateError::InvalidSimpleWord,
                "{word:?}"
            );
        }
    }

    #[test]
    fn parse_order_accepts_permutations_only() {
        assert_eq!(
            Part::parse_order("symbols, word ,numbers"),
            Some([Part::Symbols, Part::Word, Part::Digits])
        );
        assert_eq!(Part::parse_order("word,digits"), None);
        assert_eq!(Part::parse_order("word,digits,digits"), None);
        assert_eq!(Part::parse_order("word,digits,symbols,word"), None);
        assert_eq!(Part::parse_order("word,letters,symbols"), None);
    }
}
