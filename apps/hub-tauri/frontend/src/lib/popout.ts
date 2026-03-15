import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { emit } from '@tauri-apps/api/event';

export const POPOUT_LABELS = {
	telemetry: 'popout-telemetry',
	consolelog: 'popout-consolelog',
	filebrowser: 'popout-filebrowser'
} as const;

interface PopoutConfig {
	label: string;
	title: string;
	url: string;
	width?: number;
	height?: number;
}

export async function openPopout(config: PopoutConfig): Promise<void> {
	const existing = await WebviewWindow.getByLabel(config.label);
	if (existing) {
		await existing.setFocus();
		return;
	}

	new WebviewWindow(config.label, {
		url: config.url,
		title: config.title,
		width: config.width ?? 600,
		height: config.height ?? 500,
		minWidth: 400,
		minHeight: 300,
		resizable: true,
		decorations: true
	});
}

export async function closePopout(label: string): Promise<void> {
	const win = await WebviewWindow.getByLabel(label);
	if (win) {
		await win.close();
	}
}

export async function closeAllPopouts(): Promise<void> {
	await Promise.allSettled(
		Object.values(POPOUT_LABELS).map((label) => closePopout(label))
	);
}

export async function requestStateSync(): Promise<void> {
	await emit('popout:request-state');
}
