<script lang="ts">
  import DOMPurify from "dompurify";
  import { marked } from "marked";
  import { Eye, PencilSimple } from "phosphor-svelte";
  import Button from "./Button.svelte";

  let {
    value = $bindable(""),
    defaultView = "rendered",
    editable = false,
    disabled = false,
    rows = 12,
    maxHeight = "460px",
  }: {
    value?: string;
    defaultView?: "rendered" | "raw";
    editable?: boolean;
    disabled?: boolean;
    rows?: number;
    maxHeight?: string;
  } = $props();

  let view = $state<"rendered" | "raw" | undefined>();

  $effect(() => {
    if (view === undefined) view = defaultView;
  });

  function renderMarkdown(source: string): string {
    return DOMPurify.sanitize(marked.parse(source, { async: false }));
  }
</script>

<div class="markdown-view">
  <div class="view-toolbar">
    <Button variant="ghost" size="sm" type="button" disabled={disabled} aria-pressed={view === "rendered"} onclick={() => (view = view === "rendered" ? "raw" : "rendered")}>
      {#if view === "rendered"}<PencilSimple size={15} /> Edit source{:else}<Eye size={15} /> Preview{/if}
    </Button>
  </div>
  {#if view === "rendered"}
    <div class="markdown-preview" style:max-height={maxHeight}>{@html renderMarkdown(value)}</div>
  {:else if editable}
    <textarea aria-label="Raw Markdown source" rows={rows} bind:value disabled={disabled}></textarea>
  {:else}
    <pre class="raw-markdown" style:max-height={maxHeight}>{value}</pre>
  {/if}
</div>

<style>
  .markdown-view { display:grid; gap:12px; }
  .view-toolbar { display:flex; justify-content:flex-end; }
  textarea { width:100%; box-sizing:border-box; resize:vertical; color:var(--color-text); background:var(--color-surface); border:1px solid var(--color-border); padding:10px; font:12px/1.5 var(--font-mono); }
  .markdown-preview,.raw-markdown { min-height:260px; max-height:460px; overflow:auto; padding:12px; border:1px solid var(--color-border); background:var(--color-surface-raised); }
  .raw-markdown { margin:0; white-space:pre-wrap; overflow-wrap:anywhere; font:12px/1.5 var(--font-mono); }
  .markdown-preview :global(h1),.markdown-preview :global(h2),.markdown-preview :global(h3),.markdown-preview :global(h4),.markdown-preview :global(h5),.markdown-preview :global(h6) { margin:0 0 10px; }
  .markdown-preview :global(p) { margin:0 0 10px; line-height:1.6; }
  .markdown-preview :global(ul),.markdown-preview :global(ol) { padding-left:24px; }
  .markdown-preview :global(li) { line-height:1.6; }
  .markdown-preview :global(pre),.markdown-preview :global(code) { font-family:var(--font-mono); }
  .markdown-preview :global(pre) { overflow:auto; padding:10px; background:var(--color-console); color:var(--color-console-text); }
  .markdown-preview :global(blockquote) { margin:0 0 10px; padding-left:12px; border-left:3px solid var(--color-border); color:var(--color-text-muted); }
  .markdown-preview :global(table) { width:100%; border-collapse:collapse; }
  .markdown-preview :global(th),.markdown-preview :global(td) { padding:7px 9px; border:1px solid var(--color-border); text-align:left; }
</style>
