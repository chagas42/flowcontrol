<!-- toast/toastStore.ts
     Minimal store for transient toasts. Import { pushToast } anywhere.
-->
<script context="module" lang="ts">
  import { writable } from 'svelte/store';

  export type Toast = {
    id: number;
    title: string;
    desc?: string;
    tone?: 'info' | 'success' | 'warn';
    ttl?: number;
  };

  export const toasts = writable<Toast[]>([]);

  let uid = 0;
  export function pushToast(t: Omit<Toast, 'id'>) {
    const id = ++uid;
    const toast: Toast = { id, ttl: 2400, tone: 'info', ...t };
    toasts.update(list => [...list, toast]);
    setTimeout(() => {
      toasts.update(list => list.filter(x => x.id !== id));
    }, toast.ttl);
  }

  export function dismissToast(id: number) {
    toasts.update(list => list.filter(x => x.id !== id));
  }
</script>
