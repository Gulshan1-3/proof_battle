import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, fireEvent, cleanup } from '@testing-library/svelte';
import Button from './Button.svelte';
import Badge from './Badge.svelte';
import Spinner from './Spinner.svelte';
import Stars from './Stars.svelte';
import Tooltip from './Tooltip.svelte';
import Modal from './Modal.svelte';
import Timer from './Timer.svelte';

afterEach(() => {
	cleanup();
});

describe('UI Primitives Test Suite', () => {
	describe('Button.svelte', () => {
		it('renders with default primary variant', () => {
			const { getByRole } = render(Button);
			const button = getByRole('button');
			expect(button.className).toContain('bg-[var(--accent)]');
			expect(button.hasAttribute('disabled')).toBe(false);
		});

		it('handles secondary, ghost, and danger variants', () => {
			const r1 = render(Button, { props: { variant: 'secondary' } });
			expect(r1.getByRole('button').className).toContain('bg-[var(--bg-surface)]');
			cleanup();

			const r2 = render(Button, { props: { variant: 'ghost' } });
			expect(r2.getByRole('button').className).toContain('bg-transparent');
			cleanup();

			const r3 = render(Button, { props: { variant: 'danger' } });
			expect(r3.getByRole('button').className).toContain('bg-[var(--error)]');
		});

		it('renders loading spinner and disables button', () => {
			const { getByRole } = render(Button, { props: { loading: true } });
			const button = getByRole('button');
			expect(button.hasAttribute('disabled')).toBe(true);
			expect(button.querySelector('svg')).not.toBeNull();
		});

		it('respects disabled state and prevents click handler', async () => {
			const onclick = vi.fn();
			const { getByRole } = render(Button, { props: { disabled: true, onclick } });
			const button = getByRole('button');
			expect(button.hasAttribute('disabled')).toBe(true);
			await fireEvent.click(button);
			expect(onclick).not.toHaveBeenCalled();
		});
	});

	describe('Badge.svelte', () => {
		it('renders different variants with appropriate tokens', () => {
			const r1 = render(Badge, { props: { variant: 'success' } });
			expect(r1.container.querySelector('span')?.className).toContain('text-[var(--success)]');

			const r2 = render(Badge, { props: { variant: 'error' } });
			expect(r2.container.querySelector('span')?.className).toContain('text-[var(--error)]');

			const r3 = render(Badge, { props: { variant: 'warning' } });
			expect(r3.container.querySelector('span')?.className).toContain('text-[var(--warning)]');

			const r4 = render(Badge, { props: { variant: 'info' } });
			expect(r4.container.querySelector('span')?.className).toContain('text-[var(--info)]');

			const r5 = render(Badge, { props: { variant: 'accent' } });
			expect(r5.container.querySelector('span')?.className).toContain(
				'text-[var(--accent-bright)]'
			);
		});
	});

	describe('Spinner.svelte', () => {
		it('renders accessible svg spinner with custom size', () => {
			const { getByRole } = render(Spinner, { props: { size: 24 } });
			const spinner = getByRole('status');
			expect(spinner.getAttribute('width')).toBe('24');
			expect(spinner.getAttribute('height')).toBe('24');
			expect(spinner.getAttribute('aria-label')).toBe('Loading');
		});
	});

	describe('Stars.svelte', () => {
		it('renders 5 star SVGs with filled status matching difficulty', () => {
			const { container, getByLabelText } = render(Stars, { props: { difficulty: 3, max: 5 } });
			expect(getByLabelText('Difficulty: 3 of 5')).toBeDefined();
			const svgs = container.querySelectorAll('svg');
			expect(svgs.length).toBe(5);
			expect(svgs[0]?.getAttribute('fill')).toBe('var(--warning)');
			expect(svgs[1]?.getAttribute('fill')).toBe('var(--warning)');
			expect(svgs[2]?.getAttribute('fill')).toBe('var(--warning)');
			expect(svgs[3]?.getAttribute('fill')).toBe('none');
			expect(svgs[4]?.getAttribute('fill')).toBe('none');
		});
	});

	describe('Tooltip.svelte', () => {
		it('shows tooltip content on mouse enter and hides on mouse leave', async () => {
			const { container, queryByRole } = render(Tooltip, {
				props: { content: 'Tactic hint: try simp' }
			});
			expect(queryByRole('tooltip')).toBeNull();

			const region = container.querySelector('[role="region"]');
			expect(region).not.toBeNull();

			await fireEvent.mouseEnter(region!);
			expect(queryByRole('tooltip')?.textContent?.trim()).toBe('Tactic hint: try simp');

			await fireEvent.mouseLeave(region!);
			expect(queryByRole('tooltip')).toBeNull();
		});
	});

	describe('Modal.svelte', () => {
		it('renders when open is true and responds to escape', async () => {
			const onclose = vi.fn();
			const { getByRole } = render(Modal, {
				props: { open: true, title: 'Round Finished', onclose }
			});

			const dialog = getByRole('dialog');
			expect(dialog).toBeDefined();
			expect(dialog.getAttribute('aria-modal')).toBe('true');

			await fireEvent.keyDown(window, { key: 'Escape' });
			expect(onclose).toHaveBeenCalledTimes(1);
		});

		it('does not render when open is false', () => {
			const { queryByRole } = render(Modal, { props: { open: false } });
			expect(queryByRole('dialog')).toBeNull();
		});
	});

	describe('Timer.svelte', () => {
		it('formats remaining time correctly and applies warning styles', () => {
			const now = Date.now();
			// 45 seconds remaining -> urgent (< 60s)
			const { getByRole } = render(Timer, {
				props: { endsAtMs: now + 45000, serverTimeOffsetMs: 0 }
			});

			const timer = getByRole('timer');
			expect(timer.textContent).toContain('00:45');
			expect(timer.className).toContain('text-[var(--warning)]');
		});

		it('applies critical styles when remaining time <= 10s', () => {
			const now = Date.now();
			// 8 seconds remaining -> critical (< 10s)
			const { getByRole } = render(Timer, {
				props: { endsAtMs: now + 8000, serverTimeOffsetMs: 0 }
			});

			const timer = getByRole('timer');
			expect(timer.textContent).toContain('00:08');
			expect(timer.className).toContain('timer-critical');
			expect(timer.className).toContain('text-[var(--error)]');
		});
	});
});
