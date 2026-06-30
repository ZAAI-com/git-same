<div class="brand" aria-label="GIT-SAME">
  <pre class="banner" aria-hidden="true">{` ██████╗ ██╗████████╗   ███████╗ █████╗ ███╗   ███╗███████╗
██╔════╝ ██║╚══██╔══╝   ██╔════╝██╔══██╗████╗ ████║██╔════╝
██║  ███╗██║   ██║█████╗███████╗███████║██╔████╔██║█████╗
██║   ██║██║   ██║╚════╝╚════██║██╔══██║██║╚██╔╝██║██╔══╝
╚██████╔╝██║   ██║      ███████║██║  ██║██║ ╚═╝ ██║███████╗
 ╚═════╝ ╚═╝   ╚═╝      ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝`}</pre>
  <p class="subhead">Mirror GitHub structure to local disk.</p>
</div>

<style>
  .brand {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .banner {
    margin: 0;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 5.5px;
    font-weight: 900;
    line-height: 1;
    letter-spacing: 0;
    white-space: pre;
    overflow: hidden;
    background: linear-gradient(90deg, #3b82f6 0%, #06b6d4 50%, #22c55e 100%);
    color: #3b82f6;
    background-clip: text;
    -webkit-background-clip: text;
  }

  @supports (-webkit-background-clip: text) {
    .banner {
      color: transparent;
      -webkit-text-fill-color: transparent;
    }
  }

  .subhead {
    margin: 0;
    color: var(--muted);
    font-size: 10px;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
  }
</style>
