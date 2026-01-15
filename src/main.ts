import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { performanceService } from '@/lib/services/performanceService';

// Initialize performance tracking (dev only)
performanceService.init();
performanceService.markStartupPhase('main-start');

const app = mount(App, {
  target: document.getElementById('app')!,
});

performanceService.markStartupPhase('main-mounted');

export default app;
