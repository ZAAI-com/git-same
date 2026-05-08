<script lang="ts">
  import { push } from 'svelte-spa-router';

  export let active = false;
  export let href = '';
  export let label = '';
  export let title = label;
  export let compact = false;
  export let onSelect: (() => void) | undefined = undefined;

  function activate() {
    onSelect?.();
    if (href) void push(href);
  }
</script>

<button class:active class:compact type="button" {title} on:click={activate}>
  <slot name="icon" />
  <span>{label}</span>
  <slot />
</button>

<style>
  button {
    width: 100%;
    min-height: 34px;
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    padding: 0 9px;
    text-align: left;
  }

  button:hover {
    background: var(--hover);
  }

  button.active {
    background: var(--selected);
    color: var(--accent);
    font-weight: 700;
  }

  button.compact {
    min-height: 30px;
    padding-left: 8px;
  }

  span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
