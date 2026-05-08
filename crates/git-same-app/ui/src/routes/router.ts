import type { RouteDefinition } from 'svelte-spa-router';
import BadgeBrowser from './BadgeBrowser.svelte';
import Dashboard from './Dashboard.svelte';
import FinderBadges from './FinderBadges.svelte';
import Requirements from './Requirements.svelte';
import Settings from './Settings.svelte';
import Workspace from './Workspace.svelte';

export const routes: RouteDefinition = {
  '/': Dashboard,
  '/dashboard': Dashboard,
  '/finder-badges': FinderBadges,
  '/badge-browser': BadgeBrowser,
  '/workspace': Workspace,
  '/settings': Settings,
  '/requirements': Requirements,
  '*': Dashboard,
};
