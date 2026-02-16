import { writable } from 'svelte/store';

export interface Toast {
	id: string;
	type: 'success' | 'error' | 'warning' | 'info';
	title: string;
	message?: string;
	duration?: number;
}

function createToastStore() {
	const { subscribe, update } = writable<Toast[]>([]);
	// Track timeouts to allow cleanup
	const timeouts = new Map<string, ReturnType<typeof setTimeout>>();

	function add(toast: Omit<Toast, 'id'>) {
		const id = crypto.randomUUID();
		const duration = toast.duration ?? 4000;

		update((toasts) => [...toasts, { ...toast, id }]);

		if (duration > 0) {
			const timeoutId = setTimeout(() => remove(id), duration);
			timeouts.set(id, timeoutId);
		}

		return id;
	}

	function remove(id: string) {
		// Clear timeout if exists
		const timeoutId = timeouts.get(id);
		if (timeoutId) {
			clearTimeout(timeoutId);
			timeouts.delete(id);
		}
		update((toasts) => toasts.filter((t) => t.id !== id));
	}

	function clear() {
		// Clear all pending timeouts
		timeouts.forEach((timeoutId) => clearTimeout(timeoutId));
		timeouts.clear();
		update(() => []);
	}

	function success(title: string, message?: string) {
		return add({ type: 'success', title, message });
	}

	function error(title: string, message?: string) {
		return add({ type: 'error', title, message, duration: 6000 });
	}

	function warning(title: string, message?: string) {
		return add({ type: 'warning', title, message });
	}

	function info(title: string, message?: string) {
		return add({ type: 'info', title, message });
	}

	return {
		subscribe,
		add,
		remove,
		clear,
		success,
		error,
		warning,
		info
	};
}

export const toast = createToastStore();
