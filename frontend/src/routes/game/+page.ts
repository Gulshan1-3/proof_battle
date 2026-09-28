// The game page is a fully dynamic real-time application page.
// It depends entirely on WebSocket state that is not available during SSR.
// Disabling SSR prevents hydration mismatches from Svelte 5 $derived runes
// that use Date.now() or other runtime-only values.
export const ssr = false;
