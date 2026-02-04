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

	function add(toast: Omit<Toast, 'id'>) {
		const id = crypto.randomUUID();
		const duration = toast.duration ?? 4000;

		update((toasts) => [...toasts, { ...toast, id }]);

		if (duration > 0) {
			setTimeout(() => remove(id), duration);
		}

		return id;
	}

	function remove(id: string) {
		update((toasts) => toasts.filter((t) => t.id !== id));
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
		success,
		error,
		warning,
		info
	};
}

export const toast = createToastStore();
