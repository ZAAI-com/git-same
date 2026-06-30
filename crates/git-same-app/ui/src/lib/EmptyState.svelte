<script lang="ts">
  export let title = 'No data';
  export let detail = '';
</script>

<section class="empty-state">
  <strong>{title}</strong>
  {#if detail}
    <p>{detail}</p>
  {/if}
  <slot />
</section>

<style>
  .empty-state {
    display: grid;
    justify-items: start;
    gap: 8px;
    min-height: 132px;
    align-content: center;
    border: 1px dashed var(--line-strong);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel) 70%, transparent);
    padding: 22px;
  }

  strong {
    font-size: 15px;
  }

  p {
    max-width: 560px;
    margin: 0;
    color: var(--muted);
  }
</style>
