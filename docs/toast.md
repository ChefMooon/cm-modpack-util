# Toast Notifications

The renderer exposes a global toast system through `toast()` and `useToast()`. The viewport is mounted by `src/routes/+layout.svelte`, so notifications remain visible across route changes.

Toasts appear in the bottom-right corner, show up to four at once, and queue additional notifications in order. The default duration is five seconds. Use `duration: 0` for a persistent toast, and update or dismiss a toast by its returned ID. New toasts stack below previous toasts by default. Use `stackDirection="above"` on `ToastViewport` to place new toasts above previous ones.

```svelte
<script lang="ts">
  import { useToast } from "../../components/ui/toast/toast.svelte";

  const { toast, update } = useToast();

  async function handleSave() {
    const id = toast({
      title: "Saving settings",
      description: "Please wait...",
      duration: 0
    });

    try {
      await saveSettings();
      update(id, {
        title: "Settings saved",
        description: "Your changes are now stored.",
        severity: "success",
        duration: 5000
      });
    } catch {
      update(id, {
        title: "Could not save settings",
        severity: "error",
        duration: 5000
      });
    }
  }
</script>

<button onclick={handleSave}>Save</button>
```

Supported options include `title`, optional `description`, `severity` (`info`, `success`, `warning`, or `error`), `duration`, and an optional action with a label and click handler. Toast timers pause while a toast is hovered or focused, and every toast includes an accessible dismiss button. Set an action's `dismiss` option to `false` when the toast should remain after the action runs.

The public API is:

```ts
const { toast, update, dismiss, dismissAll } = useToast();
const id = toast({ title: "Hello" });
update(id, { description: "Updated" });
dismiss(id);
dismissAll();
```
