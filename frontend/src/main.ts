import { mount } from 'svelte';
import './style.css';
import './i18n';
import { applyInitialLocale } from './i18n';
import App from './App.svelte';

// Fire-and-forget — locale resolution is async (it asks tauri-plugin-os
// for the system locale). The store starts on the 'en' fallback so the
// first paint is never blank, and switches over once the OS replies.
applyInitialLocale();

const app = mount(App, { target: document.getElementById('app')! });

export default app;
