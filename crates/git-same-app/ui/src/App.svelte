<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Router from 'svelte-spa-router';
  import Sidebar from './lib/Sidebar.svelte';
  import StatusBanner from './lib/StatusBanner.svelte';
  import TitleBar from './lib/TitleBar.svelte';
  import { loadMonitorStatus, subscribeMonitor } from './stores/monitor';
  import { errorMessage, loading, refresh, subscribePush } from './stores/status';
  import { routes } from './routes/router';

  let unsubscribe: (() => void) | undefined;
  let unsubscribeMonitor: (() => void) | undefined;

  onMount(() => {
    void (async () => {
      try {
        // Listen before fetching: an update emitted in between (the app's
        // own startup recovery, for one) would otherwise be lost.
        unsubscribe = await subscribePush();
        unsubscribeMonitor = await subscribeMonitor();
        await Promise.all([refresh(), loadMonitorStatus()]);
      } catch (err) {
        errorMessage.set(String(err));
      } finally {
        loading.set(false);
      }
    })();
  });

  onDestroy(() => {
    unsubscribe?.();
    unsubscribeMonitor?.();
  });
</script>

<svelte:head>
  <title>Git-Same</title>
</svelte:head>

<main class="shell">
  <Sidebar />
  <section class="content">
    <TitleBar />
    <div class="content-scroll">
      <StatusBanner />
      <Router {routes} />
    </div>
  </section>
</main>

<style>
  .shell {
    display: grid;
    grid-template-columns: 248px minmax(0, 1fr);
    height: 100dvh;
    overflow: hidden;
    color: var(--text);
    border-top: 1px solid var(--line);
  }

  .content {
    min-width: 0;
    height: 100%;
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
  }

  .content-scroll {
    min-width: 0;
    min-height: 0;
    overflow: auto;
    padding: 18px 22px 28px;
  }

  @media (max-width: 860px) {
    .shell {
      grid-template-columns: 1fr;
      height: auto;
      min-height: 100dvh;
      overflow: visible;
    }
  }
</style>
