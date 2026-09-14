<script lang="ts">
  import { commitTagInput, normalizeTags, removeTagAt } from "../../lib/tags";

  type Props = {
    value?: string[];
    disabled?: boolean;
    label: string;
    description?: string;
    inputId?: string;
  };

  let {
    value = $bindable<string[]>([]),
    disabled = false,
    label,
    description,
    inputId = "tag-editor-input",
  }: Props = $props();

  let inputValue = $state("");
  let statusMessage = $state("");
  let inputElement = $state<HTMLInputElement | null>(null);
  const descriptionId = $derived(`${inputId}-description`);
  const helperId = $derived(`${inputId}-helper`);
  const statusId = $derived(`${inputId}-status`);
  const normalizedValue = $derived(normalizeTags(value));

  function refocusInput() {
    queueMicrotask(() => inputElement?.focus());
  }

  function commit(trigger: "comma" | "enter") {
    const result = commitTagInput(value, inputValue, trigger);
    value = result.tags;
    inputValue = result.input;
    statusMessage = result.duplicates.length
      ? `Already added: ${result.duplicates.join(", ")}.`
      : "";
    refocusInput();
  }

  function handleInput(event: Event) {
    const nextValue = (event.currentTarget as HTMLInputElement).value;
    if (nextValue.includes(",")) {
      inputValue = nextValue;
      commit("comma");
      return;
    }

    inputValue = nextValue;
    statusMessage = "";
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      commit("enter");
      return;
    }

    if (event.key === "Backspace" && inputValue === "" && normalizedValue.length > 0) {
      event.preventDefault();
      value = removeTagAt(value, normalizedValue.length - 1);
      statusMessage = "";
      refocusInput();
    }
  }

  function removeTag(index: number) {
    value = removeTagAt(value, index);
    statusMessage = "";
    refocusInput();
  }
</script>

<div class="tag-editor" class:disabled>
  <div class="field-label">
    <label for={inputId}>{label}</label>
    {#if description}<p id={descriptionId}>{description}</p>{/if}
    <p id={helperId}>Press Enter or comma to add a tag. Spaces are allowed inside a tag.</p>
  </div>

  <div class="tag-input">
    {#if normalizedValue.length > 0}
      <ul class="tag-list" aria-label={`${label} values`}>
        {#each normalizedValue as tag, index (tag.toLowerCase())}
          <li class="tag">
            <span>{tag}</span>
            <button
              type="button"
              class="remove-tag"
              title={`Remove ${tag}`}
              aria-label={`Remove tag ${tag}`}
              disabled={disabled}
              onclick={() => removeTag(index)}
            >
              <span aria-hidden="true">×</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <input
      bind:this={inputElement}
      id={inputId}
      value={inputValue}
      disabled={disabled}
      aria-describedby={[description ? descriptionId : "", helperId].filter(Boolean).join(" ")}
      oninput={handleInput}
      onkeydown={handleKeydown}
    />
  </div>

  <p id={statusId} class="status" role="status" aria-live="polite" aria-atomic="true">{statusMessage}</p>
</div>

<style>
  .tag-editor { display: grid; gap: 7px; color: var(--color-text); font-size: 12px; }
  .field-label { display: grid; gap: 5px; }
  .field-label label { line-height: 18px; }
  .field-label p, .status { margin: 0; color: var(--color-text-muted); font-size: 11px; line-height: 1.4; }
  .tag-input { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; min-height: 40px; padding: 5px 8px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); color: var(--color-text); background: var(--color-bg); }
  .tag-input:focus-within { box-shadow: var(--focus-ring); }
  .tag-list { display: flex; flex: 0 1 auto; flex-wrap: wrap; gap: 6px; min-width: 0; list-style: none; margin: 0; padding: 0; }
  .tag { display: inline-flex; align-items: center; gap: 4px; max-width: 100%; padding: 3px 4px 3px 8px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); color: var(--color-text); background: var(--color-surface-raised); overflow-wrap: anywhere; }
  .remove-tag { display: inline-grid; place-items: center; width: 24px; height: 24px; padding: 0; border: 0; border-radius: var(--radius-sm); color: var(--color-text); background: transparent; font-size: 17px; line-height: 1; cursor: pointer; }
  .remove-tag:hover:not(:disabled) { color: var(--color-danger); background: var(--color-bg); }
  .remove-tag:disabled { cursor: not-allowed; opacity: .5; }
  .tag-input input { flex: 1 1 120px; min-width: min(120px, 100%); height: 28px; padding: 0 2px; border: 0; outline: none; color: inherit; background: transparent; font: inherit; }
  .tag-input input:disabled { cursor: not-allowed; }
  .tag-editor.disabled { opacity: .7; }
  .status:empty { min-height: 0; }
  @media (prefers-reduced-motion: reduce) { .remove-tag { transition: none; } }
</style>
