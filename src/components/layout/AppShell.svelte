<script lang="ts">
  import type { Snippet } from "svelte";
  import Header from "../header/Header.svelte";

  type ShellVariant = "standard" | "workspace";

  let { children, variant = "standard" }: { children: Snippet; variant?: ShellVariant } = $props();
</script>

<main class="app-shell">
  <div class="app-content">
    <Header />
  </div>
  <div class="app-content" class:workspace={variant === "workspace"}>
    {@render children()}
  </div>
</main>

<style>
  .app-shell {
    min-height: 100vh;
    padding-bottom: 42px;
    background: var(--color-bg);
  }

  .app-content {
    max-width: 1144px;
    margin: 0 auto;
    padding: 0 42px;
  }

  .app-content.workspace {
    max-width: 1264px;
  }

  @media (max-width: 760px) {
    .app-shell {
      padding-bottom: 28px;
    }

    .app-content {
      padding: 0 20px;
    }
  }
</style>
