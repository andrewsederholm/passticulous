// Passticulous client-side generator.
//
// Mirrors src/generator.rs: randomness comes from crypto.getRandomValues and
// indices are drawn with rejection sampling to avoid modulo bias. Generated
// values never leave the browser; only the public wordlist is fetched.
"use strict";

const UPPERCASE = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE = "abcdefghijklmnopqrstuvwxyz";
const DIGITS = "0123456789";
const SYMBOLS = "!#$%&()*+,-./:;<=>?@[]^_{|}~";
const LOOK_ALIKES = "Il1|O0o";

const MIN_LENGTH = 4;
const MAX_LENGTH = 256;
const MIN_WORDS = 3;
const MAX_WORDS = 20;
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
    candidate = Array.from({ length: opts.length }, () => pool[uniformIndex(pool.length)]);
  } while (!classes.every((set) => candidate.some((c) => set.includes(c))));

  return { value: candidate.join(""), entropyBits: opts.length * Math.log2(pool.length) };
}

function generatePassphrase(opts, wordlist) {
  if (!(opts.words >= MIN_WORDS && opts.words <= MAX_WORDS)) {
    throw new Error(`Words must be between ${MIN_WORDS} and ${MAX_WORDS}.`);
  }
  const words = Array.from({ length: opts.words }, () => {
    const word = wordlist[uniformIndex(wordlist.length)];
    return opts.capitalize ? word[0].toUpperCase() + word.slice(1) : word;
  });
  return { value: words.join(opts.separator), entropyBits: opts.words * Math.log2(wordlist.length) };
}

let wordlistPromise = null;
function loadWordlist() {
  wordlistPromise ??= fetch("/wordlist.txt")
    .then((res) => {
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return res.text();
    })
    .then((text) => text.split("\n").map((w) => w.trim()).filter(Boolean))
    .catch((err) => {
      wordlistPromise = null;
      throw new Error(`Could not load the wordlist (${err.message}).`);
    });
  return wordlistPromise;
}

// ---------- UI ----------

const $ = (id) => document.getElementById(id);
const el = {
  tabs: { password: $("tab-password"), passphrase: $("tab-passphrase") },
  panels: { password: $("panel-password"), passphrase: $("panel-passphrase") },
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
};

let mode = "password";
let generation = 0;

function readOptions() {
  return {
    mode,
    length: clamp(parseInt(el.lengthNumber.value, 10) || 20, MIN_LENGTH, MAX_LENGTH),
    uppercase: el.uppercase.checked,
    lowercase: el.lowercase.checked,
    digits: el.digits.checked,
    symbols: el.symbols.checked,
    excludeLookAlikes: el.excludeLookAlikes.checked,
    words: clamp(parseInt(el.words.value, 10) || 6, MIN_WORDS, MAX_WORDS),
    separator: el.separator.value,
    capitalize: el.capitalize.checked,
  };
}

function clamp(n, lo, hi) {
  return Math.min(hi, Math.max(lo, n));
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
  if (typeof saved.separator === "string") el.separator.value = saved.separator.slice(0, 8);
  if (saved.mode === "passphrase") mode = "passphrase";
}

function setLength(n) {
  const length = clamp(n, MIN_LENGTH, MAX_LENGTH);
  el.lengthNumber.value = length;
  el.length.value = Math.min(length, Number(el.length.max));
}

function setMode(next) {
  mode = next;
  for (const [name, tab] of Object.entries(el.tabs)) {
    const selected = name === mode;
    tab.setAttribute("aria-selected", String(selected));
    tab.tabIndex = selected ? 0 : -1;
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

async function generate() {
  const opts = readOptions();
  el.lengthValue.textContent = `(${opts.length})`;
  el.wordsValue.textContent = `(${opts.words})`;
  saveOptions(opts);
  const id = ++generation;
  try {
    const result =
      opts.mode === "password"
        ? generatePassword(opts)
        : generatePassphrase(opts, await loadWordlist());
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

function init() {
  restoreOptions();
  setMode(mode);
  $("host").textContent = location.host || "localhost:8080";

  for (const [name, tab] of Object.entries(el.tabs)) {
    tab.addEventListener("click", () => {
      setMode(name);
      generate();
    });
    tab.addEventListener("keydown", (e) => {
      if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
        const other = name === "password" ? "passphrase" : "password";
        setMode(other);
        el.tabs[other].focus();
        generate();
      }
    });
  }

  el.length.addEventListener("input", () => {
    el.lengthNumber.value = el.length.value;
    generate();
  });
  el.lengthNumber.addEventListener("change", () => {
    setLength(parseInt(el.lengthNumber.value, 10) || 20);
    generate();
  });
  for (const input of [el.uppercase, el.lowercase, el.digits, el.symbols, el.excludeLookAlikes, el.words, el.separator, el.capitalize]) {
    input.addEventListener("input", generate);
  }

  el.regenerate.addEventListener("click", generate);
  el.copy.addEventListener("click", copyResult);
  document.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement || e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === "r" || e.key === "R") generate();
    if (e.key === "c" || e.key === "C") copyResult();
  });

  generate();
}

init();
