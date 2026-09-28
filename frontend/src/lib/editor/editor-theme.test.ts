import { describe, it, expect } from 'vitest';
import { getEditorTheme } from './editor-theme';

function hexToRgb(hex: string): [number, number, number] {
	const clean = hex.replace('#', '');
	const num = parseInt(clean, 16);
	return [(num >> 16) & 255, (num >> 8) & 255, num & 255];
}

function relativeLuminance([r, g, b]: [number, number, number]): number {
	const [rs, gs, bs] = [r, g, b].map((c) => {
		const s = c / 255;
		return s <= 0.04045 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
	});
	return 0.2126 * rs! + 0.7152 * gs! + 0.0722 * bs!;
}

function contrastRatio(hex1: string, hex2: string): number {
	const l1 = relativeLuminance(hexToRgb(hex1));
	const l2 = relativeLuminance(hexToRgb(hex2));
	const lighter = Math.max(l1, l2);
	const darker = Math.min(l1, l2);
	return (lighter + 0.05) / (darker + 0.05);
}

describe('Editor Theme Contrast Audit against #111113', () => {
	const theme = getEditorTheme();
	const bg = (theme.colors && theme.colors['editor.background']) || '#111113';

	it('asserts all theme syntax token rules meet WCAG AA (>= 4.5:1)', () => {
		for (const rule of theme.rules) {
			if (!rule.foreground) continue;
			const ratio = contrastRatio(rule.foreground, bg);
			expect(
				ratio,
				`Token '${rule.token}' with color #${rule.foreground} against ${bg} should be >= 4.5:1 (actual: ${ratio.toFixed(2)}:1)`
			).toBeGreaterThanOrEqual(4.5);
		}
	});

	it('asserts foreground text and line active number meet WCAG AA against editor background', () => {
		const fg = theme.colors!['editor.foreground']!;
		const activeLine = theme.colors!['editorLineNumber.activeForeground']!;

		expect(contrastRatio(fg, bg)).toBeGreaterThanOrEqual(4.5);
		expect(contrastRatio(activeLine, bg)).toBeGreaterThanOrEqual(4.5);
	});
});
