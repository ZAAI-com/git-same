<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Router from 'svelte-spa-router';
  import Banner from './lib/Banner.svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import Topbar from './lib/Topbar.svelte';
  import { errorMessage, loading, refresh, subscribePush } from './stores/status';
  import { routes } from './routes/router';

  let unsubscribe: (() => void) | undefined;

  onMount(() => {
    void (async () => {
      try {
        await refresh();
        unsubscribe = await subscribePush();
      } catch (err) {
        errorMessage.set(String(err));
      } finally {
        loading.set(false);
      }
    })();
  });

  onDestroy(() => {
    unsubscribe?.();
  });
</script>

<svelte:head>
  <title>git-Same</title>
</svelte:head>

<main class="shell">
  <Sidebar />
  <section class="content">
    <Topbar />
    <Banner />
    <Router {routes} />
  </section>
</main>

<style>
  .shell {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    min-height: 100vh;
    color: var(--text);
  }

  .content {
    min-width: 0;
    padding: 22px;
  }

  @media (max-width: 860px) {
    .shell {
      grid-template-columns: 1fr;
    }
  }
</style>
