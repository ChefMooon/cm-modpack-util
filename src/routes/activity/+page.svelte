<script lang="ts">
  import ClockCounterClockwiseIcon from "phosphor-svelte/lib/ClockCounterClockwiseIcon";
  import Header from "../../components/header/Header.svelte";
</script>

<svelte:head>
  <title>Activity · CM Modpack Util</title>
  <meta name="description" content="Review CM Modpack Util activity." />
</svelte:head>

<main class="activity-shell">
  <Header />
  <section class="page-heading" aria-labelledby="activity-title">
    <p class="eyebrow">Activity</p>
    <h1 id="activity-title">Operation history</h1>
    <p>Activity will appear here when registered project operations are available.</p>
  </section>
  <section class="empty-state" aria-labelledby="empty-title">
    <div class="empty-icon" aria-hidden="true"><ClockCounterClockwiseIcon size={34} weight="duotone" /></div>
    <p class="eyebrow">No operations yet</p>
    <h2 id="empty-title">Activity is unavailable in v0.0.2</h2>
    <p>There is no project registration, Packwiz process, filesystem scan, network request, or update operation to report.</p>
  </section>
</main>

<style>
  .activity-shell { min-height: 100vh; padding: 0 42px 42px; background: var(--color-bg); }
  .page-heading, .empty-state { max-width: 1060px; margin-right: auto; margin-left: auto; }.page-heading { margin-top: 58px; margin-bottom: 34px; }.eyebrow { margin: 0 0 10px; color: var(--color-accent-strong); font: 700 10px var(--font-mono); letter-spacing: .12em; text-transform: uppercase; }h1, h2 { margin: 0; }h1 { font-size: clamp(28px, 5vw, 44px); line-height: 1.05; }.page-heading > p:last-child { margin: 14px 0 0; color: var(--color-text-muted); font-size: 15px; line-height: 1.55; }
  .empty-state { display: grid; justify-items: center; padding: 72px 28px; border: 1px solid var(--color-border); background: var(--color-surface); box-shadow: 5px 5px 0 var(--color-text); text-align: center; }.empty-icon { display: grid; width: 68px; height: 68px; margin-bottom: 24px; place-items: center; border: 1px solid var(--color-accent); color: var(--color-accent-strong); background: color-mix(in srgb, var(--color-accent) 12%, var(--color-surface)); }.empty-state h2 { margin-bottom: 12px; font-size: 20px; }.empty-state > p:last-child { max-width: 580px; margin: 0; color: var(--color-text-muted); line-height: 1.6; }
  @media (max-width: 700px) { .activity-shell { padding: 0 20px 28px; }.page-heading { margin-top: 36px; }.empty-state { padding: 52px 20px; } }
</style>
