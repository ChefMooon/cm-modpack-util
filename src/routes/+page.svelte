<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Header from "../components/header/Header.svelte";
  import ArrowRightIcon from "phosphor-svelte/lib/ArrowRightIcon";
  import ArrowUpRightIcon from "phosphor-svelte/lib/ArrowUpRightIcon";
  import { useToast } from "../components/ui/toast/toast.svelte";
  import Button from "../components/ui/Button.svelte";

  let name = $state("");
  let greeting = $state("");
  let error = $state("");
  let isLoading = $state(false);
  const { toast, update, dismissAll } = useToast();

  function showToast(severity: "info" | "success" | "warning" | "error") {
    const messages = {
      info: ["Here is some information", "This is an informational toast."],
      success: ["Everything went well", "The operation completed successfully."],
      warning: ["Something needs attention", "You may want to review this action."],
      error: ["Something went wrong", "This error toast is announced urgently."]
    } as const;
    const [title, description] = messages[severity];
    toast({ title, description, severity });
  }

  function showPersistentToast() {
    let id = "";
    id = toast({
      title: "Persistent toast",
      description: "This stays open until dismissed or updated.",
      severity: "info",
      duration: 0,
      action: {
        label: "Mark complete",
        dismiss: false,
        onclick: () => update(id, {
          title: "Marked complete",
          description: "The persistent toast was updated in place.",
          severity: "success",
          duration: 5000,
          action: undefined
        })
      }
    });
  }

  function showActionToast() {
    toast({
      title: "Action toast",
      description: "Click the action to dismiss this notification.",
      severity: "warning",
      action: { label: "Got it", onclick: () => undefined }
    });
  }

  async function greet(event: SubmitEvent) {
    event.preventDefault();
    if (!name.trim()) return;
    isLoading = true;
    error = "";
    try {
      greeting = await invoke<string>("greet", { name: name.trim() });
    } catch (cause) {
      error = `The greeting could not be loaded: ${String(cause)}`;
    } finally {
      isLoading = false;
    }
  }
</script>

<svelte:head>
  <title>Tauri Svelte Template</title>
  <meta name="description" content="A minimal desktop app starter built with Tauri, SvelteKit, and TypeScript." />
</svelte:head>

<main class="shell">
  <Header />

  <section class="hero">
    <p class="eyebrow">Desktop app starter</p>
    <h1>Build something <span>remarkable.</span></h1>
    <p class="intro">A clean starting point for cross-platform desktop apps with the web technologies you already know.</p>
    <div class="stack" aria-label="Technology stack"><span>Tauri 2</span><span>SvelteKit</span><span>TypeScript</span><span>SQLite</span></div>
  </section>

  <section class="workspace" aria-labelledby="workspace-title">
    <div class="section-heading"><div><p class="eyebrow">Try the bridge</p><h2 id="workspace-title">Call Rust from the frontend</h2></div><span class="status"><i aria-hidden="true"></i> Ready</span></div>
    <form onsubmit={greet}>
      <label for="name">Your name</label>
      <div class="form-row"><input id="name" bind:value={name} placeholder="Enter your name" autocomplete="name" aria-describedby="name-hint" /><Button type="submit" disabled={isLoading || !name.trim()}>{isLoading ? "Calling…" : "Say hello"}<ArrowRightIcon size={16} weight="bold" aria-hidden="true" /></Button></div>
      <p id="name-hint" class="hint">This invokes a typed Rust command through the Tauri API.</p>
    </form>
    {#if greeting}<p class="result" role="status">{greeting}</p>{/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </section>

  <section class="toast-playground" aria-labelledby="toast-playground-title">
    <div class="section-heading">
      <div><p class="eyebrow">Component preview</p><h2 id="toast-playground-title">Test toast notifications</h2></div>
    </div>
    <p class="playground-copy">Try each severity, create a persistent notification, or fill the queue to preview the global toast system.</p>
    <div class="toast-controls" aria-label="Toast examples">
      <Button variant="info" size="sm" onclick={() => showToast("info")}>Info</Button>
      <Button variant="success" size="sm" onclick={() => showToast("success")}>Success</Button>
      <Button variant="warning" size="sm" onclick={() => showToast("warning")}>Warning</Button>
      <Button variant="danger" size="sm" onclick={() => showToast("error")}>Error</Button>
      <Button variant="secondary" size="sm" onclick={showPersistentToast}>Persistent with action</Button>
      <Button variant="secondary" size="sm" onclick={showActionToast}>Toast with action</Button>
      <Button variant="quiet" size="sm" onclick={dismissAll}>Dismiss all</Button>
    </div>
  </section>

  <footer><span>Ready to make it yours.</span><a href="https://tauri.app/" target="_blank" rel="noreferrer">Read the Tauri docs <ArrowUpRightIcon size={14} weight="bold" aria-hidden="true" /></a></footer>
</main>

<style>
  .shell { max-width: 940px; min-height: 100vh; margin: 0 auto; padding: 0 42px 28px; display: flex; flex-direction: column; }
  .stack, .form-row, .section-heading, footer { display: flex; align-items: center; }footer { justify-content: space-between; gap: 20px; }footer { color: var(--color-text-subtle); font-size: 12px; }footer a { color: var(--color-text-muted); text-decoration: none; }footer a:hover { color: var(--color-accent-strong); }
  .hero { padding: 115px 0 76px; }.eyebrow { margin: 0 0 16px; color: var(--color-accent-strong); font-size: 11px; font-weight: 750; letter-spacing: .14em; text-transform: uppercase; }h1 { max-width: 690px; margin: 0; font-size: clamp(43px, 7vw, 76px); line-height: .98; letter-spacing: -.065em; }h1 span { color: var(--color-accent); }.intro { max-width: 490px; margin: 26px 0 27px; color: var(--color-text-muted); font-size: 16px; line-height: 1.6; }.stack { gap: 9px; flex-wrap: wrap; }.stack span { padding: 7px 11px; border: 1px solid var(--color-border); border-radius: 100px; color: var(--color-text-muted); font-size: 11px; }
  .workspace { padding: 30px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); }.section-heading { justify-content: space-between; gap: 20px; margin-bottom: 32px; }.section-heading .eyebrow { margin-bottom: 9px; }h2 { margin: 0; font-size: 20px; }.status { display: flex; align-items: center; gap: 7px; color: var(--color-text-muted); font-size: 12px; }.status i { width: 7px; height: 7px; border-radius: 50%; background: var(--color-accent-strong); }label { display: block; margin-bottom: 10px; color: var(--color-text-muted); font-size: 12px; }.form-row { gap: 10px; }input { min-width: 0; flex: 1; padding: 13px 15px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); color: var(--color-text); background: var(--color-bg); }input:focus-visible { border-color: var(--color-accent-strong); }.hint, .result, .error { margin: 18px 0 0; color: var(--color-text-subtle); font-size: 12px; }.result { color: var(--color-accent-strong); }.error { color: var(--color-danger); }
  .toast-playground { margin-top: 24px; padding: 30px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); }.toast-playground .section-heading { margin-bottom: 10px; }.playground-copy { margin: 0 0 20px; color: var(--color-text-muted); font-size: 13px; line-height: 1.55; }.toast-controls { display: flex; flex-wrap: wrap; gap: 9px; }footer { margin-top: auto; padding-top: 48px; }
  @media (max-width: 600px) { .shell { padding: 0 20px 22px; }.hero { padding: 85px 0 55px; }.workspace, .toast-playground { padding: 22px; }.form-row { align-items: stretch; flex-direction: column; }footer { align-items: flex-start; flex-direction: column; } }
</style>
