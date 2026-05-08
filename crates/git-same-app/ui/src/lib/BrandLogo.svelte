<div class="brand-logo" aria-label="Git-Same">
  <div class="mark" aria-hidden="true">
    <span>GIT</span>
    <span>SAME</span>
  </div>
  <div class="copy">
    <strong>Git-Same</strong>
    <small>macOS utility</small>
  </div>
</div>

<style>
  .brand-logo {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .mark {
    width: 44px;
    height: 38px;
    display: grid;
    align-content: center;
    border-radius: 8px;
    border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--line));
    background:
      linear-gradient(135deg, rgba(45, 125, 210, 0.16), rgba(38, 166, 91, 0.14)),
      var(--panel);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.45);
    color: var(--accent);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 10px;
    font-weight: 800;
    line-height: 1.05;
    text-align: center;
  }

  .mark span:last-child {
    color: var(--ok);
  }

  .copy {
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  .copy strong,
  .copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .copy strong {
    font-size: 15px;
    line-height: 1.1;
  }

  .copy small {
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
  }
</style>
