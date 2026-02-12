/**
 * CSS sanitizer for console %c styled segments.
 * Whitelists safe properties to prevent XSS via CSS injection.
 */

const ALLOWED_PROPERTIES = new Set([
	'color',
	'background',
	'background-color',
	'font-weight',
	'font-style',
	'font-size',
	'text-decoration',
	'padding',
	'padding-left',
	'padding-right',
	'padding-top',
	'padding-bottom',
	'border-radius',
	'margin',
	'margin-left',
	'margin-right'
]);

const DANGEROUS_VALUES = /url\s*\(|expression\s*\(|javascript:|@import|behavior\s*:/i;

/**
 * Sanitize a CSS string from a console %c directive.
 * Only allows whitelisted properties and filters dangerous values.
 */
export function sanitizeCSS(css: string): string {
	if (!css) return '';

	return css
		.split(';')
		.map((decl) => decl.trim())
		.filter((decl) => {
			if (!decl) return false;
			const colonIdx = decl.indexOf(':');
			if (colonIdx === -1) return false;

			const prop = decl.slice(0, colonIdx).trim().toLowerCase();
			const value = decl.slice(colonIdx + 1).trim();

			if (!ALLOWED_PROPERTIES.has(prop)) return false;
			if (DANGEROUS_VALUES.test(value)) return false;

			return true;
		})
		.join('; ');
}
