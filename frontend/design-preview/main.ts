import { mount } from 'svelte';
import App from './App.svelte';

const target = document.getElementById('design-preview');
if (!target) throw new Error('design-preview root element missing');

const app = mount(App, { target });

export default app;
