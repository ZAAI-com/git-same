<script lang="ts">
  import { snapshot } from '../stores/status';
  import { relativeTime } from '../lib/utils';
</script>

<section class="settings-panel">
  <h2>Finder Badges</h2>
  <dl>
    <div>
      <dt>Status file</dt>
      <dd>{$snapshot?.status_path ?? 'Unavailable'}</dd>
    </div>
    <div>
      <dt>Last update</dt>
      <dd>{relativeTime($snapshot?.updated_at)}</dd>
    </div>
    <div>
      <dt>Monitor PID</dt>
      <dd>{$snapshot?.status?.daemon_pid ?? 'Unavailable'}</dd>
    </div>
  </dl>
</section>

<style>
  .settings-panel {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 18px;
  }

  h2 {
    margin: 0;
  }

  .settings-panel dl {
    display: grid;
    gap: 12px;
    margin: 16px 0 0;
  }

  .settings-panel div {
    display: grid;
    gap: 4px;
  }

  .settings-panel dt {
    color: var(--muted);
    font-size: 12px;
    text-transform: uppercase;
  }

  .settings-panel dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
