// Translation infrastructure for PortFinder.
//
// Layout:
//   src/i18n/index.ts       — registers messages, picks the initial locale.
//   src/i18n/locales/*.json — one bundle per supported language.
//
// Locale resolution order:
//   1. user override saved in localStorage (set by the picker)
//   2. system locale reported by tauri-plugin-os, mapped to a 2-letter code
//   3. 'en' fallback
//
// To add a new language: drop a `<code>.json` next to en.json (same key
// shape), import + addMessages it below, and append to LOCALES.

import { addMessages, init, locale } from 'svelte-i18n';
import { locale as osLocale } from '@tauri-apps/plugin-os';

import en from './locales/en.json';
import es from './locales/es.json';
import fr from './locales/fr.json';
import de from './locales/de.json';

export const LOCALES = ['en', 'es', 'fr', 'de'] as const;
export type LocaleCode = (typeof LOCALES)[number];

const STORAGE_KEY = 'portfinder.locale';

addMessages('en', en);
addMessages('es', es);
addMessages('fr', fr);
addMessages('de', de);

init({
    fallbackLocale: 'en',
    initialLocale: 'en',
});

/** Pick the initial locale from user override → OS locale → fallback. */
export async function applyInitialLocale(): Promise<void> {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && (LOCALES as readonly string[]).includes(saved)) {
        locale.set(saved);
        return;
    }
    try {
        const sys = await osLocale();
        if (sys) {
            const short = sys.slice(0, 2).toLowerCase();
            if ((LOCALES as readonly string[]).includes(short)) {
                locale.set(short);
            }
        }
    } catch {
        // tauri-plugin-os not loaded (e.g., browser dev). Keep fallback.
    }
}

/** Persist a user-chosen locale and switch the live store. */
export function setLocale(code: LocaleCode): void {
    localStorage.setItem(STORAGE_KEY, code);
    locale.set(code);
}
