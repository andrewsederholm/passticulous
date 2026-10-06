// Passticulous client-side generator.
//
// Mirrors src/generator.rs: randomness comes from crypto.getRandomValues and
// indices are drawn with rejection sampling to avoid modulo bias. Generated
// values never leave the browser; only the public word lists are fetched.
"use strict";

const UPPERCASE = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE = "abcdefghijklmnopqrstuvwxyz";
const DIGITS = "0123456789";
const SYMBOLS = "!#$%&()*+,-./:;<=>?@[]^_{|}~";
const ALL_SYMBOLS = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
const DEFAULT_PATTERN_SYMBOLS = "!@#$%&*?";
const LOOK_ALIKES = "Il1|O0o";

const MIN_LENGTH = 4;
const MAX_LENGTH = 256;
const MIN_WORDS = 3;
const MAX_WORDS = 20;
const MIN_PATTERN_WORDS = 1;
const MAX_PATTERN_WORDS = 5;
const MAX_PATTERN_DIGITS = 16;
const MAX_PATTERN_SYMBOLS = 16;
const MAX_SEPARATOR_LEN = 8;
const SIMPLE_SYMBOLS = "!@#$?";
const MIN_SIMPLE_LENGTH = 10;
const MAX_SIMPLE_LENGTH = 16;
const DEFAULT_SIMPLE_LENGTH = 12;
const MAX_SIMPLE_WORD_LEN = 32;
const MIN_SIMPLE_DIGITS = 2;
const MAX_SIMPLE_DIGITS = 5;
const MIN_SIMPLE_WORD_LEN = 6;
const SIMPLE_CATEGORIES = ["animals", "colors", "foods", "nature", "space"];
const CATEGORIES = ["animals", "colors", "foods", "nature", "space", "elements", "any"];
const ORDERS = [
  "word,digits,symbols",
  "word,symbols,digits",
  "digits,word,symbols",
  "symbols,word,digits",
  "digits,symbols,word",
  "symbols,digits,word",
];
const MODES = ["password", "passphrase", "pattern", "simple"];
const U32_MAX = 0xffffffff;
const STORAGE_KEY = "passticulous:options";

// ---------- randomness ----------

const randomBuffer = new Uint32Array(256);
let randomOffset = randomBuffer.length;

function nextU32() {
  if (randomOffset >= randomBuffer.length) {
    crypto.getRandomValues(randomBuffer);
    randomOffset = 0;
  }
  const value = randomBuffer[randomOffset];
  randomBuffer[randomOffset++] = 0;
  return value;
}

/** Uniform integer in [0, n) without modulo bias. */
function uniformIndex(n) {
  if (!Number.isInteger(n) || n <= 0 || n > U32_MAX) throw new RangeError("bad range");
  const zone = U32_MAX - (U32_MAX % n);
  for (;;) {
    const v = nextU32();
    if (v < zone) return v % n;
  }
}

function pick(list) {
  return list[uniformIndex(list.length)];
}

// ---------- generators ----------

function generatePassword(opts) {
  if (!(opts.length >= MIN_LENGTH && opts.length <= MAX_LENGTH)) {
    throw new Error(`Length must be between ${MIN_LENGTH} and ${MAX_LENGTH}.`);
  }
  const classes = [
    [opts.uppercase, UPPERCASE],
    [opts.lowercase, LOWERCASE],
    [opts.digits, DIGITS],
    [opts.symbols, SYMBOLS],
  ]
    .filter(([enabled]) => enabled)
    .map(([, set]) => [...set].filter((c) => !(opts.excludeLookAlikes && LOOK_ALIKES.includes(c))))
    .filter((set) => set.length > 0);
  if (classes.length === 0) throw new Error("Enable at least one character type.");

  const pool = classes.flat();
  // Draw whole candidates and reject any missing a class, so the result is
  // uniform over all passwords that contain every enabled class.
  let candidate;
  do {
    candidate = Array.from({ length: opts.length }, () => pick(pool));
  } while (!classes.every((set) => candidate.some((c) => set.includes(c))));

  return { value: candidate.join(""), entropyBits: opts.length * Math.log2(pool.length) };
}

function capitalize(word) {
  return word.charAt(0).toUpperCase() + word.slice(1);
}

function generatePassphrase(opts, wordlist) {
  if (!(opts.words >= MIN_WORDS && opts.words <= MAX_WORDS)) {
    throw new Error(`Words must be between ${MIN_WORDS} and ${MAX_WORDS}.`);
  }
  const words = Array.from({ length: opts.words }, () => {
    const word = pick(wordlist);
    return opts.capitalize ? capitalize(word) : word;
  });
  return { value: words.join(opts.separator), entropyBits: opts.words * Math.log2(wordlist.length) };
}

const CASES = {
  title: capitalize,
  lower: (w) => w.toLowerCase(),
  upper: (w) => w.toUpperCase(),
};

function generatePattern(opts, wordlist) {
  if (!(opts.words >= MIN_PATTERN_WORDS && opts.words <= MAX_PATTERN_WORDS)) {
    throw new Error(`Words must be between ${MIN_PATTERN_WORDS} and ${MAX_PATTERN_WORDS}.`);
  }
  if (!(opts.digits >= 0 && opts.digits <= MAX_PATTERN_DIGITS)) {
    throw new Error(`Numbers must be between 0 and ${MAX_PATTERN_DIGITS}.`);
  }
  // Fixed symbols are used verbatim (repeats allowed) instead of random ones.
  const fixed = opts.fixed ? [...opts.fixedSymbols] : null;
  if (fixed) {
    if (fixed.length === 0 || fixed.length > MAX_PATTERN_SYMBOLS) {
      throw new Error(`Type 1 to ${MAX_PATTERN_SYMBOLS} symbols to use every time.`);
    }
    const bad = fixed.find((c) => !ALL_SYMBOLS.includes(c));
    if (bad !== undefined) throw new Error(`"${bad}" is not a symbol. Use punctuation like #@!`);
  } else if (!(opts.symbols >= 0 && opts.symbols <= MAX_PATTERN_SYMBOLS)) {
    throw new Error(`Symbols must be between 0 and ${MAX_PATTERN_SYMBOLS}.`);
  }
  const symbolSet = [...new Set(opts.symbolSet)].filter((c) => ALL_SYMBOLS.includes(c));
  if (!fixed && opts.symbols > 0 && symbolSet.length === 0) {
    throw new Error("Pick at least one allowed symbol, or set Symbols to 0.");
  }
  const applyCase = CASES[opts.case] ?? CASES.title;
  const words = wordlist.filter((w) => w.length >= opts.minWordLength);
  if (words.length === 0) {
    const longest = Math.max(...wordlist.map((w) => w.length));
    throw new Error(`Minimum word length is too long: the longest word in this list is ${longest} letters.`);
  }

  const segments = [];
  for (const part of opts.order.split(",")) {
    if (part === "word") {
      for (let i = 0; i < opts.words; i++) segments.push(applyCase(pick(words)));
    } else if (part === "digits" && opts.digits > 0) {
      segments.push(Array.from({ length: opts.digits }, () => pick(DIGITS)).join(""));
    } else if (part === "symbols" && fixed) {
      segments.push(fixed.join(""));
    } else if (part === "symbols" && opts.symbols > 0) {
      segments.push(Array.from({ length: opts.symbols }, () => pick(symbolSet)).join(""));
    }
  }

  let entropyBits = opts.words * Math.log2(words.length) + opts.digits * Math.log2(10);
  // Fixed symbols are known to an attacker, so they add no entropy.
  if (!fixed && opts.symbols > 0) entropyBits += opts.symbols * Math.log2(symbolSet.length);
  return { value: segments.join(opts.separator), entropyBits };
}

/** Digits needed after a word to reach minLength, counting the one symbol. */
function simpleDigitCount(wordLength, minLength) {
  return Math.max(MIN_SIMPLE_DIGITS, minLength - wordLength - 1);
}

// A title-case word, then digits, then one symbol, at least minLength long.
// `words` is every themed word of at least MIN_SIMPLE_WORD_LEN letters.
function generateSimple(opts, words) {
  if (!(opts.minLength >= MIN_SIMPLE_LENGTH && opts.minLength <= MAX_SIMPLE_LENGTH)) {
    throw new Error(`Minimum length must be between ${MIN_SIMPLE_LENGTH} and ${MAX_SIMPLE_LENGTH}.`);
  }
  const symbolBits = Math.log2(SIMPLE_SYMBOLS.length);
  let word;
  let entropyBits;
  if (opts.word) {
    const length = [...opts.word].length;
    if (length > MAX_SIMPLE_WORD_LEN || /\s/.test(opts.word)) {
      throw new Error(`Your word must be at most ${MAX_SIMPLE_WORD_LEN} characters with no spaces.`);
    }
    word = opts.word;
    // The word is known to anyone who knows the setup, so only the digits
    // and symbol count.
    entropyBits = simpleDigitCount(length, opts.minLength) * Math.log2(10) + symbolBits;
  } else {
    // Only words that need at most MAX_SIMPLE_DIGITS digits.
    const minWordLength = Math.max(MIN_SIMPLE_WORD_LEN, opts.minLength - MAX_SIMPLE_DIGITS - 1);
    const list = words.filter((w) => w.length >= minWordLength);
    // Shorter words get more digits, so count every possible result.
    const outcomes = list.reduce((sum, w) => sum + 10 ** simpleDigitCount(w.length, opts.minLength), 0);
    word = capitalize(pick(list));
    entropyBits = Math.log2(outcomes) + symbolBits;
  }
  const digits = Array.from({ length: simpleDigitCount([...word].length, opts.minLength) }, () => pick(DIGITS));
  return { value: word + digits.join("") + pick(SIMPLE_SYMBOLS), entropyBits };
}

let simpleWords;
function loadSimpleWords() {
  simpleWords ??= Promise.all(SIMPLE_CATEGORIES.map(loadWordlist))
    .then((lists) => [...new Set(lists.flat())].filter((w) => w.length >= MIN_SIMPLE_WORD_LEN))
    .catch((err) => {
      simpleWords = undefined;
      throw err;
    });
  return simpleWords;
}

const wordlists = new Map();
function loadWordlist(category) {
  if (!wordlists.has(category)) {
    const promise = fetch(`/wordlists/${category}.txt`)
      .then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.text();
      })
      .then((text) => text.split("\n").map((w) => w.trim()).filter(Boolean))
      .catch((err) => {
        wordlists.delete(category);
        throw new Error(`Could not load the word list (${err.message}).`);
      });
    wordlists.set(category, promise);
  }
  return wordlists.get(category);
}

// ---------- UI ----------

const $ = (id) => document.getElementById(id);
const el = {
  tabs: Object.fromEntries(MODES.map((m) => [m, $(`tab-${m}`)])),
  panels: Object.fromEntries(MODES.map((m) => [m, $(`panel-${m}`)])),
  result: $("result"),
  copy: $("copy"),
  regenerate: $("regenerate"),
  meterFill: $("meter-fill"),
  strengthLabel: $("strength-label"),
  error: $("error"),
  length: $("length"),
  lengthNumber: $("length-number"),
  lengthValue: $("length-value"),
  uppercase: $("uppercase"),
  lowercase: $("lowercase"),
  digits: $("digits"),
  symbols: $("symbols"),
  excludeLookAlikes: $("exclude-look-alikes"),
  words: $("words"),
  wordsValue: $("words-value"),
  separator: $("separator"),
  capitalize: $("capitalize"),
  patternCategory: $("pattern-category"),
  patternCategorySize: $("pattern-category-size"),
  patternCase: $("pattern-case"),
  patternWords: $("pattern-words"),
  patternWordsValue: $("pattern-words-value"),
  patternMinWordLength: $("pattern-min-word-length"),
  patternMinWordLengthValue: $("pattern-min-word-length-value"),
  patternDigits: $("pattern-digits"),
  patternDigitsValue: $("pattern-digits-value"),
  patternFixed: $("pattern-fixed"),
  patternFixedField: $("pattern-fixed-field"),
  patternFixedSymbols: $("pattern-fixed-symbols"),
  patternSymbolsField: $("pattern-symbols-field"),
  patternSymbols: $("pattern-symbols"),
  symbolPicker: $("symbol-picker"),
  patternSymbolsValue: $("pattern-symbols-value"),
  symbolChips: $("symbol-chips"),
  symbolPicked: $("symbol-picked"),
  patternOrder: $("pattern-order"),
  patternSeparator: $("pattern-separator"),
  simpleLength: $("simple-length"),
  simpleLengthValue: $("simple-length-value"),
  simpleWord: $("simple-word"),
  themeMode: $("theme-mode"),
  themeColors: [...$("theme-colors").querySelectorAll("input[type=color]")],
  themeReset: $("theme-reset"),
  themePresets: $("theme-presets"),
};

let mode = "password";
let generation = 0;

function clamp(n, lo, hi) {
  return Math.min(hi, Math.max(lo, n));
}

function intValue(input, fallback, lo, hi) {
  const n = parseInt(input.value, 10);
  return clamp(Number.isNaN(n) ? fallback : n, lo, hi);
}

// The slider runs from the list's shortest word ("any") to its longest. At
// its minimum there is no limit, saved as 0.
function minWordLength() {
  const input = el.patternMinWordLength;
  return Number(input.value) > Number(input.min) ? Number(input.value) : 0;
}

// Fits the slider to a newly loaded list, keeping "any" if it was set.
function fitMinWordLength(list) {
  const input = el.patternMinWordLength;
  const lengths = list.map((w) => w.length);
  const wasAny = minWordLength() === 0;
  input.min = Math.min(...lengths);
  input.max = Math.max(...lengths);
  if (wasAny) input.value = input.min;
}

function selectedSymbols() {
  return [...el.symbolChips.querySelectorAll("input:checked")].map((i) => i.value).join("");
}

function setSelectedSymbols(set) {
  for (const input of el.symbolChips.querySelectorAll("input")) {
    input.checked = set.includes(input.value);
  }
}

function buildSymbolChips() {
  for (const symbol of ALL_SYMBOLS) {
    const label = document.createElement("label");
    label.className = "chip";
    label.title = `Allow ${symbol}`;
    const input = document.createElement("input");
    input.type = "checkbox";
    input.value = symbol;
    input.setAttribute("aria-label", `Allow ${symbol}`);
    const span = document.createElement("span");
    span.textContent = symbol;
    label.append(input, span);
    el.symbolChips.append(label);
  }
  setSelectedSymbols(DEFAULT_PATTERN_SYMBOLS);
}

function readOptions() {
  return {
    mode,
    length: intValue(el.lengthNumber, 20, MIN_LENGTH, MAX_LENGTH),
    uppercase: el.uppercase.checked,
    lowercase: el.lowercase.checked,
    digits: el.digits.checked,
    symbols: el.symbols.checked,
    excludeLookAlikes: el.excludeLookAlikes.checked,
    words: intValue(el.words, 6, MIN_WORDS, MAX_WORDS),
    separator: el.separator.value,
    capitalize: el.capitalize.checked,
    pattern: {
      category: el.patternCategory.value,
      case: el.patternCase.value,
      words: intValue(el.patternWords, 1, MIN_PATTERN_WORDS, MAX_PATTERN_WORDS),
      minWordLength: minWordLength(),
      digits: intValue(el.patternDigits, 4, 0, MAX_PATTERN_DIGITS),
      symbols: intValue(el.patternSymbols, 3, 0, MAX_PATTERN_SYMBOLS),
      symbolSet: selectedSymbols(),
      fixed: el.patternFixed.checked,
      fixedSymbols: el.patternFixedSymbols.value,
      order: el.patternOrder.value,
      separator: el.patternSeparator.value,
    },
    simple: {
      minLength: intValue(el.simpleLength, DEFAULT_SIMPLE_LENGTH, MIN_SIMPLE_LENGTH, MAX_SIMPLE_LENGTH),
      word: el.simpleWord.value.trim(),
    },
  };
}

// Only options are persisted, never generated values.
function saveOptions(opts) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(opts));
  } catch {
    /* storage unavailable; ignore */
  }
}

function restoreOptions() {
  let saved;
  try {
    saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || "null");
  } catch {
    saved = null;
  }
  if (!saved || typeof saved !== "object") return;
  if (Number.isInteger(saved.length)) setLength(saved.length);
  for (const key of ["uppercase", "lowercase", "digits", "symbols", "excludeLookAlikes", "capitalize"]) {
    if (typeof saved[key] === "boolean") el[key].checked = saved[key];
  }
  if (Number.isInteger(saved.words)) el.words.value = clamp(saved.words, MIN_WORDS, MAX_WORDS);
  if (typeof saved.separator === "string") el.separator.value = saved.separator.slice(0, MAX_SEPARATOR_LEN);
  if (MODES.includes(saved.mode)) mode = saved.mode;

  const s = saved.simple;
  if (s && typeof s === "object") {
    if (Number.isInteger(s.minLength)) el.simpleLength.value = clamp(s.minLength, MIN_SIMPLE_LENGTH, MAX_SIMPLE_LENGTH);
    if (typeof s.word === "string") el.simpleWord.value = s.word.slice(0, MAX_SIMPLE_WORD_LEN);
  }

  const p = saved.pattern;
  if (!p || typeof p !== "object") return;
  if (CATEGORIES.includes(p.category)) el.patternCategory.value = p.category;
  if (p.case in CASES) el.patternCase.value = p.case;
  if (Number.isInteger(p.words)) el.patternWords.value = clamp(p.words, MIN_PATTERN_WORDS, MAX_PATTERN_WORDS);
  // Fitted to the word list once it loads.
  if (Number.isInteger(p.minWordLength)) el.patternMinWordLength.value = p.minWordLength;
  if (Number.isInteger(p.digits)) el.patternDigits.value = clamp(p.digits, 0, MAX_PATTERN_DIGITS);
  if (Number.isInteger(p.symbols)) el.patternSymbols.value = clamp(p.symbols, 0, MAX_PATTERN_SYMBOLS);
  if (typeof p.symbolSet === "string") setSelectedSymbols(p.symbolSet);
  if (typeof p.fixed === "boolean") el.patternFixed.checked = p.fixed;
  if (typeof p.fixedSymbols === "string") {
    el.patternFixedSymbols.value = [...p.fixedSymbols].slice(0, MAX_PATTERN_SYMBOLS).join("");
  }
  if (ORDERS.includes(p.order)) el.patternOrder.value = p.order;
  if (typeof p.separator === "string") el.patternSeparator.value = p.separator.slice(0, MAX_SEPARATOR_LEN);
}

function setLength(n) {
  const length = clamp(n, MIN_LENGTH, MAX_LENGTH);
  el.lengthNumber.value = length;
  el.length.value = Math.min(length, Number(el.length.max));
}

function setMode(next) {
  mode = next;
  // Keep the mode in the URL (e.g. /#pattern) so it can be bookmarked.
  if (location.hash !== `#${mode}`) history.replaceState(null, "", `#${mode}`);
  for (const name of MODES) {
    const selected = name === mode;
    el.tabs[name].setAttribute("aria-selected", String(selected));
    el.tabs[name].tabIndex = selected ? 0 : -1;
    el.panels[name].hidden = !selected;
  }
}

function strength(bits) {
  if (bits < 40) return { label: "Weak", color: "var(--weak)" };
  if (bits < 60) return { label: "Fair", color: "var(--fair)" };
  if (bits < 80) return { label: "Strong", color: "var(--strong)" };
  return { label: "Very strong", color: "var(--very-strong)" };
}

function showError(message) {
  el.error.textContent = message;
  el.error.hidden = false;
  el.result.textContent = "";
  el.copy.disabled = true;
  el.meterFill.style.width = "0";
  el.strengthLabel.textContent = "";
}

function showResult({ value, entropyBits }) {
  el.error.hidden = true;
  el.copy.disabled = false;
  el.result.textContent = value;
  const s = strength(entropyBits);
  el.meterFill.style.width = `${Math.min(100, (entropyBits / 128) * 100)}%`;
  el.meterFill.style.backgroundColor = s.color;
  el.strengthLabel.textContent = `${s.label} · ~${Math.round(entropyBits)} bits of entropy`;
}

function updateLabels(opts) {
  const p = opts.pattern;
  el.lengthValue.textContent = `(${opts.length})`;
  el.wordsValue.textContent = `(${opts.words})`;
  el.patternWordsValue.textContent = `(${p.words})`;
  el.patternMinWordLengthValue.textContent = p.minWordLength ? `(${p.minWordLength}+ letters)` : "(any)";
  el.patternDigitsValue.textContent = `(${p.digits})`;
  el.patternSymbolsValue.textContent = `(${p.symbols})`;
  el.simpleLengthValue.textContent = `(${opts.simple.minLength})`;
  const picked = new Set(p.symbolSet).size;
  el.symbolPicked.textContent = `(${picked} selected)`;
  // Fixed symbols replace the random count and picker.
  el.patternFixedField.hidden = !p.fixed;
  el.patternSymbolsField.hidden = p.fixed;
  el.symbolPicker.hidden = p.fixed;
}

async function generate() {
  const opts = readOptions();
  updateLabels(opts);
  saveOptions(opts);
  const id = ++generation;
  try {
    let result;
    if (opts.mode === "password") {
      result = generatePassword(opts);
    } else if (opts.mode === "passphrase") {
      result = generatePassphrase(opts, await loadWordlist("any"));
    } else if (opts.mode === "simple") {
      result = generateSimple(opts.simple, await loadSimpleWords());
    } else {
      const list = await loadWordlist(opts.pattern.category);
      if (id !== generation) return;
      fitMinWordLength(list);
      opts.pattern.minWordLength = minWordLength();
      updateLabels(opts);
      saveOptions(opts);
      const matching = list.filter((w) => w.length >= opts.pattern.minWordLength).length;
      el.patternCategorySize.textContent =
        matching === list.length
          ? `${list.length.toLocaleString()} words to choose from`
          : `${matching.toLocaleString()} of ${list.length.toLocaleString()} words ${matching === 1 ? "is" : "are"} long enough`;
      result = generatePattern(opts.pattern, list);
    }
    if (id === generation) showResult(result);
  } catch (err) {
    if (id === generation) showError(err.message);
  }
}

async function copyResult() {
  const text = el.result.textContent;
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    flash(el.copy, "Copied!");
  } catch {
    // Clipboard API unavailable (e.g. plain HTTP): select it for manual copy.
    const range = document.createRange();
    range.selectNodeContents(el.result);
    const selection = getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    flash(el.copy, "Press ⌘/Ctrl+C");
  }
}

function flash(button, text) {
  clearTimeout(button.flashTimer);
  button.textContent = text;
  button.flashTimer = setTimeout(() => (button.textContent = "Copy"), 1500);
}

// ---------- appearance (see theme.js) ----------

// Ready-made looks. Each sets the mode and all six colors; text, secondary
// text and the accent all meet WCAG AA contrast on the background and panels.
const THEME_PRESETS = [
  { name: "Ocean", mode: "light", colors: { bg: "#eef5fa", surface: "#ffffff", text: "#0f2a3d", muted: "#46606f", border: "#cddfea", accent: "#0a6aa8" } },
  { name: "Forest", mode: "light", colors: { bg: "#f0f5ef", surface: "#ffffff", text: "#1a2e1f", muted: "#4d6454", border: "#d0e0d1", accent: "#2a7343" } },
  { name: "Fall", mode: "light", colors: { bg: "#fbf3ea", surface: "#fffaf4", text: "#3a2414", muted: "#73533c", border: "#ead6c2", accent: "#b4400c" } },
  { name: "Rose", mode: "light", colors: { bg: "#fcf1f4", surface: "#ffffff", text: "#3b1d29", muted: "#77505f", border: "#efd1dc", accent: "#b0175a" } },
  { name: "Sand", mode: "light", colors: { bg: "#fdf6e3", surface: "#fffcf2", text: "#073642", muted: "#52666d", border: "#e6dcc0", accent: "#1f6fa8" } },
  { name: "Midnight", mode: "dark", colors: { bg: "#0b1220", surface: "#131c2e", text: "#e2e8f5", muted: "#93a0ba", border: "#26334d", accent: "#60a5fa" } },
  { name: "Arctic", mode: "dark", colors: { bg: "#2e3440", surface: "#3b4252", text: "#eceff4", muted: "#b3bccb", border: "#4c566a", accent: "#88c0d0" } },
  { name: "Grape", mode: "dark", colors: { bg: "#282a36", surface: "#343746", text: "#f8f8f2", muted: "#b4b9d4", border: "#4a4e63", accent: "#bd93f9" } },
  { name: "Graphite", mode: "dark", colors: { bg: "#151515", surface: "#202020", text: "#ececec", muted: "#a3a3a3", border: "#363636", accent: "#d4d4d4" } },
];

function isPreset(theme, preset) {
  return theme.mode === preset.mode && THEME_COLORS.every((name) => theme.colors[name] === preset.colors[name]);
}

function buildPresetButtons() {
  for (const preset of THEME_PRESETS) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "preset";
    button.setAttribute("aria-pressed", "false");
    const swatch = document.createElement("span");
    swatch.className = "preset-swatch";
    swatch.setAttribute("aria-hidden", "true");
    swatch.style.setProperty("--swatch-bg", preset.colors.bg);
    swatch.style.setProperty("--swatch-accent", preset.colors.accent);
    button.append(swatch, preset.name);
    button.preset = preset;
    el.themePresets.append(button);
  }
}

/** Normalizes "#abc" or "#aabbcc" to "#aabbcc"; anything else gives null. */
function toHex6(value) {
  const v = value.trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(v)) return v;
  if (/^#[0-9a-f]{3}$/.test(v)) return `#${[...v.slice(1)].map((c) => c + c).join("")}`;
  return null;
}

// Shows the colors currently in effect, including defaults the visitor
// hasn't changed.
function showTheme(theme) {
  el.themeMode.value = theme.mode ?? DEFAULT_THEME_MODE;
  const style = getComputedStyle(document.documentElement);
  for (const input of el.themeColors) {
    const color = toHex6(style.getPropertyValue(`--${input.dataset.color}`));
    if (color) input.value = color;
  }
  showActivePreset(theme);
}

function showActivePreset(theme) {
  for (const button of el.themePresets.children) {
    button.setAttribute("aria-pressed", String(isPreset(theme, button.preset)));
  }
}

function initAppearance() {
  let theme = loadTheme();
  const update = () => {
    saveTheme(theme);
    applyTheme(theme);
  };
  buildPresetButtons();
  showTheme(theme);

  for (const button of el.themePresets.children) {
    button.addEventListener("click", () => {
      theme = { mode: button.preset.mode, colors: { ...button.preset.colors } };
      update();
      showTheme(theme);
    });
  }
  el.themeMode.addEventListener("input", () => {
    theme.mode = el.themeMode.value;
    update();
    showTheme(theme);
  });
  for (const input of el.themeColors) {
    input.addEventListener("input", () => {
      theme.colors[input.dataset.color] = input.value.toLowerCase();
      update();
      showActivePreset(theme);
    });
  }
  el.themeReset.addEventListener("click", () => {
    theme = { mode: null, colors: {} };
    update();
    showTheme(theme);
  });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => showTheme(theme));
}

function init() {
  buildSymbolChips();
  restoreOptions();
  initAppearance();
  const fromHash = location.hash.slice(1);
  setMode(MODES.includes(fromHash) ? fromHash : mode);
  $("host").textContent = location.host || "localhost:8080";

  MODES.forEach((name, index) => {
    const tab = el.tabs[name];
    tab.addEventListener("click", () => {
      setMode(name);
      generate();
    });
    tab.addEventListener("keydown", (e) => {
      if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
      const step = e.key === "ArrowRight" ? 1 : -1;
      const next = MODES[(index + step + MODES.length) % MODES.length];
      setMode(next);
      el.tabs[next].focus();
      generate();
    });
  });

  el.length.addEventListener("input", () => {
    el.lengthNumber.value = el.length.value;
    generate();
  });
  el.lengthNumber.addEventListener("change", () => {
    setLength(parseInt(el.lengthNumber.value, 10) || 20);
    generate();
  });
  for (const input of [
    el.uppercase, el.lowercase, el.digits, el.symbols, el.excludeLookAlikes,
    el.words, el.separator, el.capitalize,
    el.patternCategory, el.patternCase, el.patternWords, el.patternMinWordLength, el.patternDigits,
    el.patternSymbols, el.patternFixed, el.patternFixedSymbols, el.patternOrder, el.patternSeparator,
    el.simpleLength, el.simpleWord,
  ]) {
    input.addEventListener("input", generate);
  }
  el.symbolChips.addEventListener("change", generate);
  for (const button of document.querySelectorAll("[data-symbols]")) {
    button.addEventListener("click", () => {
      const preset = button.dataset.symbols;
      setSelectedSymbols(preset === "all" ? ALL_SYMBOLS : preset === "none" ? "" : DEFAULT_PATTERN_SYMBOLS);
      generate();
    });
  }

  el.regenerate.addEventListener("click", generate);
  el.copy.addEventListener("click", copyResult);
  document.addEventListener("keydown", (e) => {
    const t = e.target;
    if (t instanceof HTMLInputElement || t instanceof HTMLSelectElement || t instanceof HTMLTextAreaElement) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === "r" || e.key === "R") generate();
    if (e.key === "c" || e.key === "C") copyResult();
  });

  generate();
}

init();
