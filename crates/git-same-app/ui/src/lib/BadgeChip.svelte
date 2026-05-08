<script lang="ts">
  import type { Badge } from './types';
  import { badgeLabel } from './utils';

  export let badge: Badge;
  export let count: number | null = null;
</script>

<span class={`chip ${badge}`}>
  <i aria-hidden="true"></i>
  <span>{badgeLabel(badge)}</span>
  {#if count !== null}
    <strong>{count}</strong>
  {/if}
</span>

<style>
  .chip {
    width: fit-content;
    min-width: 82px;
    display: inline-grid;
    grid-template-columns: auto minmax(0, auto) auto;
    align-items: center;
    gap: 6px;
    border-radius: 999px;
    padding: 4px 8px;
    font-size: 12px;
    font-weight: 700;
    line-height: 1;
    border: 1px solid transparent;
  }

  i {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: currentColor;
  }

  strong {
    color: inherit;
    font-size: 11px;
    opacity: 0.78;
  }

  .green {
    background: color-mix(in srgb, var(--ok) 13%, transparent);
    color: var(--ok);
    border-color: color-mix(in srgb, var(--ok) 24%, transparent);
  }

  .blue {
    background: color-mix(in srgb, var(--blue) 13%, transparent);
    color: var(--blue);
    border-color: color-mix(in srgb, var(--blue) 24%, transparent);
  }

  .orange {
    background: color-mix(in srgb, var(--warning) 15%, transparent);
    color: var(--warning);
    border-color: color-mix(in srgb, var(--warning) 26%, transparent);
  }

  .red {
    background: color-mix(in srgb, var(--danger) 13%, transparent);
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 24%, transparent);
  }

  .gray {
    background: var(--panel-alt);
    color: var(--muted);
    border-color: var(--line);
  }
</style>
