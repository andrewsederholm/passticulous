// Passticulous theme.
//
// Loaded before the page renders so a visitor's saved theme applies without
// a flash of the default one. The server sets the instance default mode as
// data-theme on <html> and its colors in /theme.css. A visitor's own colors
// are set inline on <html>, which wins over both. app.js wires up the
// Appearance controls.
"use strict";

const THEME_STORAGE_KEY = "passticulous:theme";
const THEME_MODES = ["system", "light", "dark"];
const THEME_COLORS = ["bg", "surface", "text", "muted", "border", "accent"];
const DEFAULT_THEME_MODE = THEME_MODES.includes(document.documentElement.dataset.theme)
  ? document.documentElement.dataset.theme
  : "system";

function isHexColor(value) {
  return typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value);
}

/** Black or white, whichever contrasts more. Mirrors readable_text_on in src/server.rs. */
function readableTextOn(hex) {
  const channel = (i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
  return 1.05 / (luminance + 0.05) >= (luminance + 0.05) / 0.05 ? "#ffffff" : "#000000";
}

/** The visitor's overrides: a mode (or null for the default) and any colors. */
function loadTheme() {
  let saved;
  try {
    saved = JSON.parse(localStorage.getItem(THEME_STORAGE_KEY) || "null");
  } catch {
    saved = null;
  }
  const theme = { mode: null, colors: {} };
  if (!saved || typeof saved !== "object") return theme;
  if (THEME_MODES.includes(saved.mode)) theme.mode = saved.mode;
  if (saved.colors && typeof saved.colors === "object") {
    for (const name of THEME_COLORS) {
      if (isHexColor(saved.colors[name])) theme.colors[name] = saved.colors[name].toLowerCase();
    }
  }
  return theme;
}

function saveTheme(theme) {
  try {
    if (theme.mode === null && Object.keys(theme.colors).length === 0) {
      localStorage.removeItem(THEME_STORAGE_KEY);
    } else {
      localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify(theme));
    }
  } catch {
    /* storage unavailable; ignore */
  }
}

function applyTheme(theme) {
  const root = document.documentElement;
  root.dataset.theme = theme.mode ?? DEFAULT_THEME_MODE;
  for (const name of THEME_COLORS) {
    if (theme.colors[name]) root.style.setProperty(`--${name}`, theme.colors[name]);
    else root.style.removeProperty(`--${name}`);
  }
  if (theme.colors.accent) root.style.setProperty("--accent-text", readableTextOn(theme.colors.accent));
  else root.style.removeProperty("--accent-text");
}

applyTheme(loadTheme());
