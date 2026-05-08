import type { RouteDefinition } from 'svelte-spa-router';
import Dashboard from './Dashboard.svelte';
import Settings from './Settings.svelte';

export const routes: RouteDefinition = {
  '/': Dashboard,
  '/settings': Settings,
  '*': Dashboard,
};
