---
name: frontend_dev
description: Svelte 5 + Tailwind + shadcn-svelte frontend development.
---
# Frontend Stack

- **Framework**: Svelte 5 (runes mode)
- **Styling**: Tailwind CSS v4
- **Components**: shadcn-svelte
- **Build**: Vite
- **Language**: TypeScript

## Svelte 5 Runes

```svelte
<script lang="ts">
  // Reactive state
  let count = $state(0);

  // Derived values
  let doubled = $derived(count * 2);

  // Effects
  $effect(() => {
    console.log('Count changed:', count);
  });

  // Props
  let { title, onSubmit }: { title: string; onSubmit: () => void } = $props();
</script>
```

## Component Structure
```
frontend/src/
├── lib/
│   ├── components/
│   │   ├── ui/           # shadcn-svelte components
│   │   ├── DeviceList.svelte
│   │   ├── ArtworkGrid.svelte
│   │   └── UploadProgress.svelte
│   ├── stores/           # Svelte stores for global state
│   ├── api.ts            # Wails binding wrappers
│   └── types.ts          # TypeScript interfaces
├── App.svelte            # Main app component
└── main.ts               # Entry point
```

## shadcn-svelte Usage
```bash
npx shadcn-svelte@latest add button
npx shadcn-svelte@latest add card
npx shadcn-svelte@latest add tabs
npx shadcn-svelte@latest add dialog
```

```svelte
<script>
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
</script>

<Card.Root>
  <Card.Header>
    <Card.Title>Device</Card.Title>
  </Card.Header>
  <Card.Content>
    <Button on:click={connect}>Connect</Button>
  </Card.Content>
</Card.Root>
```

## Tailwind Best Practices
- Use utility classes directly, avoid `@apply` in most cases
- Group related utilities: `class="flex items-center gap-2"`
- Use CSS variables from shadcn theme: `text-primary`, `bg-card`
- Responsive: `md:flex-row`, `lg:grid-cols-3`

## Wails Integration
```typescript
// lib/api.ts - Typed wrappers for Wails bindings
import { ScanDevices, ConnectDevice } from '../../wailsjs/go/main/App';
import { EventsOn, EventsOff } from '../../wailsjs/runtime/runtime';
import type { Device } from './types';

export async function scanDevices(): Promise<Device[]> {
  return await ScanDevices();
}

export function onUploadProgress(callback: (progress: number) => void) {
  return EventsOn('upload:progress', callback);
}
```

## Image Handling (solves the original issue)
```svelte
<!-- Native browser support - animated WebP/GIF work! -->
<img
  src={artworkUrl}
  alt={gameName}
  class="rounded-lg object-cover"
  loading="lazy"
/>

<!-- With error fallback -->
<img
  src={artworkUrl}
  alt={gameName}
  onerror={(e) => e.target.src = '/placeholder.png'}
/>
```

## State Management
```typescript
// stores/devices.ts
import { writable } from 'svelte/store';
import type { Device } from '$lib/types';

export const devices = writable<Device[]>([]);
export const connectedDevice = writable<Device | null>(null);
```
