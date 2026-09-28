// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

declare module 'monaco-core' {
	export * from 'monaco-editor';
}

declare module 'monaco-editor/esm/vs/editor/editor.api' {
	export * from 'monaco-editor';
}

declare module 'monaco-editor/esm/vs/editor/editor.api.js' {
	export * from 'monaco-editor';
}

declare module 'monaco-editor/esm/vs/editor/editor.worker?worker' {
	export default class EditorWorker extends Worker {
		constructor();
	}
}

declare module 'monaco-editor/esm/vs/language/typescript/ts.worker?worker' {
	export default class TsWorker extends Worker {
		constructor();
	}
}

export {};
