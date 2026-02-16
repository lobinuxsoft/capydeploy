import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number): string {
	if (bytes === 0) return '0 B';
	const k = 1024;
	const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export function truncatePath(path: string, maxLen: number): string {
	if (path.length <= maxLen) return path;
	return '...' + path.slice(-maxLen + 3);
}

export function isAnimatedImage(mime: string, url: string): boolean {
	const mimeLower = (mime || '').toLowerCase();
	const urlLower = (url || '').toLowerCase();

	// GIF is always animated (or treated as such)
	if (mimeLower === 'image/gif' || urlLower.endsWith('.gif')) {
		return true;
	}

	// APNG
	if (mimeLower === 'image/apng' || urlLower.endsWith('.apng')) {
		return true;
	}

	// WebP can be animated - check URL patterns from SteamGridDB
	// SteamGridDB URL patterns for animated images often include these indicators
	if (urlLower.endsWith('.webp') || mimeLower === 'image/webp') {
		// SteamGridDB serves animated WebPs from specific paths or with specific naming
		if (urlLower.includes('/animated') ||
		    urlLower.includes('animated/') ||
		    urlLower.includes('_animated') ||
		    urlLower.includes('-animated') ||
		    urlLower.includes('animated_') ||
		    urlLower.includes('animated.')) {
			return true;
		}
		// Some animated webps have 'anim' in the filename
		const filename = urlLower.split('/').pop() || '';
		if (filename.includes('anim')) {
			return true;
		}
	}

	// Generic animation indicators in any URL
	if (urlLower.includes('/anim/') || urlLower.includes('_anim_') || urlLower.includes('-anim-')) {
		return true;
	}

	return false;
}
