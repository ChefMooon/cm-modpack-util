<script lang="ts">
  export let version = "v0.0.2";
</script>

<header class="site-header">
  <a class="brand" href="/" aria-label="CM Modpack Util home">
    <span class="brand-mark" aria-hidden="true">CM</span>
    <span>CM Modpack Util</span>
  </a>

  <nav class="navigation" aria-label="Main navigation">
    <span class="version">{version}</span>
    <a href="/activity">Activity</a>
    <a href="/settings">Settings</a>
  </nav>
</header>

<style>
  .site-header {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    min-height: 78px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent);
    background: color-mix(in srgb, var(--color-bg) 92%, transparent);
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
  }

  .brand,
  .navigation {
    display: flex;
    align-items: center;
  }

  .brand {
    gap: 14px;
    color: var(--color-text);
    font-size: 14px;
    font-weight: 650;
    text-decoration: none;
  }

  .brand-mark {
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
    border: 2px solid var(--color-yellow);
    border-radius: 0;
    color: var(--color-yellow);
    background: transparent;
    font-size: 11px;
    font-weight: 800;
  }

  .navigation {
    gap: 14px;
  }

  .version,
  .navigation a {
    font-size: 12px;
  }

  .version {
    color: var(--color-text-subtle);
  }

  .navigation a {
    color: var(--color-text-muted);
    text-decoration: none;
  }

  .navigation a:hover {
    color: var(--color-accent-strong);
  }

  @media (max-width: 600px) {
    .site-header {
      min-height: 68px;
    }

    .version {
      display: none;
    }
  }
</style>
