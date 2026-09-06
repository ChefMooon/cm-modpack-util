<script lang="ts">
  import FolderSimpleIcon from "phosphor-svelte/lib/FolderSimpleIcon";
  import PlusIcon from "phosphor-svelte/lib/PlusIcon";
  import ArrowRightIcon from "phosphor-svelte/lib/ArrowRightIcon";
  import Header from "../components/header/Header.svelte";
  import Button from "../components/ui/Button.svelte";
  import Tooltip from "../components/ui/Tooltip.svelte";
</script>

<svelte:head>
  <title>Projects · CM Modpack Util</title>
  <meta name="description" content="Manage registered CM modpack projects." />
</svelte:head>

<main class="projects-shell">
  <Header />

  <section class="hero" aria-labelledby="projects-title">
    <div>
      <p class="eyebrow">Projects</p>
      <h1 id="projects-title">Your local modpack workspace</h1>
      <p class="lede">Registered projects will appear here once project registration is available.</p>
    </div>
    <Tooltip text="Project registration arrives in v0.0.2">
      <Button variant="secondary" type="button" disabled><PlusIcon size={17} weight="bold" /> Register project</Button>
    </Tooltip>
  </section>

  <section class="empty-state" aria-labelledby="empty-title">
    <div class="empty-icon" aria-hidden="true"><FolderSimpleIcon size={34} weight="duotone" /></div>
    <p class="eyebrow">No registered projects</p>
    <h2 id="empty-title">Start with a project registration</h2>
    <p>CM Modpack Util does not access external project folders until you explicitly register one. That workflow is planned for v0.0.2.</p>
    <div class="state-note" role="status"><span class="status-mark" aria-hidden="true"></span><span>Foundation state: project scanning, Packwiz evidence, and updates are unavailable.</span></div>
  </section>

  <section class="next-panel" aria-labelledby="next-title">
    <div><p class="eyebrow">What is available now</p><h2 id="next-title">A stable foundation for local work</h2><p>Settings, accessibility preferences, window behavior, and the application shell are ready. No project files or network services are touched in v0.0.1.</p></div>
    <a class="next-link" href="/activity">View activity <ArrowRightIcon size={16} weight="bold" /></a>
  </section>
</main>

<style>
  .projects-shell { min-height: 100vh; padding: 0 42px 42px; background: var(--color-bg); }
  .hero { display: flex; align-items: flex-end; justify-content: space-between; gap: 24px; max-width: 1060px; margin: 58px auto 34px; }
  .eyebrow { margin: 0 0 10px; color: var(--color-accent-strong); font: 700 10px var(--font-mono); letter-spacing: .12em; text-transform: uppercase; }
  h1, h2 { margin: 0; } h1 { max-width: 680px; font-size: clamp(28px, 5vw, 46px); line-height: 1.04; } h2 { font-size: 20px; }
  .lede { max-width: 620px; margin: 14px 0 0; color: var(--color-text-muted); font-size: 15px; line-height: 1.55; }
  .empty-state, .next-panel { max-width: 1060px; margin: 0 auto; border: 1px solid var(--color-border); background: var(--color-surface); }
  .empty-state { display: grid; justify-items: center; padding: 72px 28px; text-align: center; box-shadow: 5px 5px 0 var(--color-text); }
  .empty-icon { display: grid; width: 68px; height: 68px; margin-bottom: 24px; place-items: center; border: 1px solid var(--color-accent); color: var(--color-accent-strong); background: color-mix(in srgb, var(--color-accent) 12%, var(--color-surface)); }
  .empty-state h2 { margin-bottom: 12px; } .empty-state > p:not(.eyebrow) { max-width: 540px; margin: 0; color: var(--color-text-muted); line-height: 1.6; }
  .state-note { display: flex; align-items: center; gap: 9px; max-width: 600px; margin-top: 26px; padding: 10px 13px; border: 1px solid var(--color-border); color: var(--color-text-muted); font: 11px var(--font-mono); text-align: left; }
  .status-mark { width: 7px; height: 7px; flex: 0 0 7px; background: var(--color-yellow); }
  .next-panel { display: flex; align-items: center; justify-content: space-between; gap: 24px; margin-top: 28px; padding: 24px; }
  .next-panel p:not(.eyebrow) { max-width: 680px; margin: 9px 0 0; color: var(--color-text-muted); font-size: 13px; line-height: 1.55; }
  .next-link { display: inline-flex; align-items: center; gap: 8px; flex: 0 0 auto; color: var(--color-accent-strong); font: 11px var(--font-mono); text-decoration: none; }.next-link:hover { color: var(--color-text); }
  @media (max-width: 700px) { .projects-shell { padding: 0 20px 28px; }.hero, .next-panel { align-items: flex-start; flex-direction: column; }.hero { margin-top: 36px; }.empty-state { padding: 52px 20px; }.next-link { margin-top: 6px; } }
</style>
