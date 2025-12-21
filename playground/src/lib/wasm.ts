// WASM module interface
export interface JsDiagnostic {
	start_line: number;
	start_col: number;
	end_line: number;
	end_col: number;
	severity: 'error' | 'warning' | 'info' | 'hint';
	message: string;
}

export interface RunResult {
	success: boolean;
	value?: string;
	value_type?: string;
	error?: string;
}

export interface HoverInfo {
	found: boolean;
	name?: string;
	type_info?: string;
}

export interface GotoDefResult {
	found: boolean;
	line?: number;
	col?: number;
}

interface WasmModule {
	analyze(source: string): JsDiagnostic[];
	run(source: string): RunResult;
	hover(source: string, line: number, col: number): HoverInfo;
	goto_def(source: string, line: number, col: number): GotoDefResult;
}

let wasmModule: WasmModule | null = null;
let initPromise: Promise<WasmModule> | null = null;

export async function initWasm(): Promise<WasmModule> {
	if (wasmModule) return wasmModule;
	if (initPromise) return initPromise;

	initPromise = (async () => {
		const wasm = await import('../wasm/playground_wasm.js');
		await wasm.default();
		wasmModule = wasm as unknown as WasmModule;
		return wasmModule;
	})();

	return initPromise;
}

export function getWasm(): WasmModule | null {
	return wasmModule;
}
