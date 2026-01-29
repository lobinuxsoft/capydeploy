import { writable } from 'svelte/store';
import type { GameSetup, UploadProgress } from '$lib/types';

function createGameSetupsStore() {
	const { subscribe, set, update } = writable<GameSetup[]>([]);

	return {
		subscribe,
		set,
		update,
		add: (setup: GameSetup) => update(setups => [...setups, setup]),
		remove: (id: string) => update(setups => setups.filter(s => s.id !== id)),
		updateSetup: (id: string, setup: GameSetup) => update(setups =>
			setups.map(s => s.id === id ? setup : s)
		)
	};
}

export const gameSetups = createGameSetupsStore();

export const uploadProgress = writable<UploadProgress | null>(null);
