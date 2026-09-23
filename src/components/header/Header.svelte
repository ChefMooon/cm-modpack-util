<script lang="ts">
  import { page } from "$app/state";
  import { formatAppVersion } from "../../lib/appVersion.svelte";

  export let version: string | null = null;
</script>

<header class="site-header">
  <a class="brand" href="/" aria-label="CM Modpack Util home">
    <img class="brand-mark" src="/cm-modpack-util-icon.png" alt="" aria-hidden="true" />
    <span>CM Modpack Util</span>
  </a>

  <nav class="navigation" aria-label="Main navigation">
    <span class="version">{formatAppVersion(version)}</span>
    <a href="/activity" class:active={page.url.pathname === "/activity"} aria-current={page.url.pathname === "/activity" ? "page" : undefined}>Activity</a>
    <a href="/management" class:active={page.url.pathname === "/management"} aria-current={page.url.pathname === "/management" ? "page" : undefined}>Management</a>
    <a href="/settings" class:active={page.url.pathname === "/settings"} aria-current={page.url.pathname === "/settings" ? "page" : undefined} aria-label="Application settings">Settings</a>
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
    width: 100%;
    min-height: 78px;
    margin: 0 auto;
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
    width: 30px;
    height: 30px;
    object-fit: contain;
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
    min-width: 80px;
    padding: 10px 2px 8px;
    border-bottom: 3px solid transparent;
    color: var(--color-text-muted);
    text-align: center;
    text-decoration: none;
  }

  .navigation a:hover,
  .navigation a.active {
    color: var(--color-accent-strong);
  }

  .navigation a.active {
    border-bottom-color: var(--color-accent-strong);
    font-weight: 650;
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
