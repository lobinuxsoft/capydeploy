import { writable } from 'svelte/store';
import type { ConsoleLogStatus, ConsoleLogEntry } from '$lib/types';

const MAX_ENTRIES = 1000;

function createConsoleLogStore() {
	const status = writable<ConsoleLogStatus>({ enabled: false });
	const entries = writable<ConsoleLogEntry[]>([]);
	const totalDropped = writable<number>(0);

	return {
		status,
		entries,
		totalDropped,
		addBatch: (newEntries: ConsoleLogEntry[], dropped: number) => {
			entries.update((current) => {
				const merged = [...current, ...newEntries];
				if (merged.length > MAX_ENTRIES) {
					return merged.slice(merged.length - MAX_ENTRIES);
				}
				return merged;
			});
			if (dropped > 0) {
				totalDropped.update((n) => n + dropped);
			}
		},
		clear: () => {
			entries.set([]);
			totalDropped.set(0);
		},
		reset: () => {
			status.set({ enabled: false });
			entries.set([]);
			totalDropped.set(0);
		}
	};
}

export const consolelog = createConsoleLogStore();

// Console log level colors — configurable with localStorage persistence

export interface ConsoleColors {
	error: string;
	warn: string;
	info: string;
	debug: string;
	log: string;
}

const DEFAULT_COLORS: ConsoleColors = {
	error: '#f87171',
	warn: '#facc15',
	info: '#60a5fa',
	debug: '#71717a',
	log: '#f1f5f9'
};

const STORAGE_KEY = 'capydeploy:consolelog-colors';

function loadColors(): ConsoleColors {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored) {
			const parsed = JSON.parse(stored);
			return { ...DEFAULT_COLORS, ...parsed };
		}
	} catch {
		// Ignore parse errors
	}
	return { ...DEFAULT_COLORS };
}

function createConsoleColorsStore() {
	const store = writable<ConsoleColors>(loadColors());

	function persist(colors: ConsoleColors) {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(colors));
		} catch {
			// Ignore storage errors
		}
	}

	return {
		subscribe: store.subscribe,
		set: (v: ConsoleColors) => {
			store.set(v);
			persist(v);
		},
		update: (fn: (c: ConsoleColors) => ConsoleColors) => {
			store.update((c) => {
				const n = fn(c);
				persist(n);
				return n;
			});
		},
		updateColors: (partial: Partial<ConsoleColors>) => {
			store.update((current) => {
				const n = { ...current, ...partial };
				persist(n);
				return n;
			});
		},
		resetColors: () => {
			const d = { ...DEFAULT_COLORS };
			store.set(d);
			persist(d);
		}
	};
}

export const consoleColors = createConsoleColorsStore();
export { DEFAULT_COLORS };
