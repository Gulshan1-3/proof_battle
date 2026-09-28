import { describe, it, expect } from 'vitest';

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

describe('Design Tokens WCAG AA Contrast Audit', () => {
	const backgrounds = [
		{ name: '--bg-base', hex: '#0d0d0f' },
		{ name: '--bg-surface', hex: '#141417' },
		{ name: '--bg-elevated', hex: '#1c1c21' },
		{ name: '--bg-overlay', hex: '#242429' }
	];

	const texts = [
		{ name: '--text-primary', hex: '#f0f0f5' },
		{ name: '--text-secondary', hex: '#8b8b9e' },
		{ name: '--text-muted', hex: '#9494a8' }
	];

	it('asserts all text tokens meet WCAG AA (4.5:1) against primary dark backgrounds', () => {
		const results: Array<{ text: string; bg: string; ratio: number; pass: boolean }> = [];

		for (const bg of backgrounds) {
			for (const text of texts) {
				const ratio = contrastRatio(text.hex, bg.hex);
				const pass = ratio >= 4.5;
				results.push({ text: text.name, bg: bg.name, ratio: Math.round(ratio * 100) / 100, pass });
				expect(
					ratio,
					`${text.name} (${text.hex}) on ${bg.name} (${bg.hex}) should meet 4.5:1`
				).toBeGreaterThanOrEqual(4.5);
			}
		}
	});
});
