<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import {
		EditorView,
		keymap,
		hoverTooltip,
		lineNumbers,
		highlightActiveLine,
		type Tooltip,
	} from "@codemirror/view";
	import { EditorState } from "@codemirror/state";
	import { linter, type Diagnostic } from "@codemirror/lint";
	import {
		defaultKeymap,
		history,
		historyKeymap,
	} from "@codemirror/commands";
	import { bracketMatching } from "@codemirror/language";
	import { getWasm, type JsDiagnostic } from "$lib/wasm";
	import { toyLangSupport } from "$lib/toy-lang";

	interface Props {
		value: string;
		onchange?: (value: string) => void;
	}

	let { value = $bindable(), onchange }: Props = $props();

	let container: HTMLDivElement;
	let view: EditorView | null = null;

	// Toy language linter
	const toyLinter = linter(
		(view) => {
			const wasm = getWasm();
			if (!wasm) return [];

			const source = view.state.doc.toString();
			const diagnostics: JsDiagnostic[] = wasm.analyze(source);

			return diagnostics.map((d): Diagnostic => {
				const from =
					view.state.doc.line(d.start_line + 1).from + d.start_col;
				const to = view.state.doc.line(d.end_line + 1).from + d.end_col;
				return {
					from,
					to,
					severity:
						d.severity === "error"
							? "error"
							: d.severity === "warning"
								? "warning"
								: "info",
					message: d.message,
				};
			});
		},
		{ delay: 300 },
	);

	// Hover tooltip for types
	const toyHover = hoverTooltip((view, pos): Tooltip | null => {
		const wasm = getWasm();
		if (!wasm) return null;

		const source = view.state.doc.toString();
		const line = view.state.doc.lineAt(pos);
		const lineNum = line.number - 1;
		const col = pos - line.from;

		const info = wasm.hover(source, lineNum, col);
		if (!info.found || !info.type_info) return null;

		return {
			pos,
			above: true,
			create() {
				const dom = document.createElement("div");
				dom.className = "cm-tooltip-hover";
				dom.style.padding = "4px 8px";
				dom.style.fontFamily = "var(--font-mono)";
				dom.style.fontSize = "13px";
				dom.textContent = `${info.name}: ${info.type_info}`;
				return { dom };
			},
		};
	});

	onMount(() => {
		const state = EditorState.create({
			doc: value,
			extensions: [
				lineNumbers(),
				highlightActiveLine(),
				history(),
				bracketMatching(),
				keymap.of([...defaultKeymap, ...historyKeymap]),
				toyLangSupport,
				toyLinter,
				toyHover,
				EditorView.updateListener.of((update) => {
					if (update.docChanged) {
						const newValue = update.state.doc.toString();
						value = newValue;
						onchange?.(newValue);
					}
				}),
				EditorView.theme({
					"&": { height: "100%" },
					".cm-scroller": {
						overflow: "auto",
						fontFamily: "var(--font-mono)",
					},
					".cm-content": { padding: "12px 0" },
					".cm-line": { padding: "0 12px" },
				}),
			],
		});

		view = new EditorView({
			state,
			parent: container,
		});
	});

	onDestroy(() => {
		view?.destroy();
	});

	$effect(() => {
		if (view && view.state.doc.toString() !== value) {
			view.dispatch({
				changes: { from: 0, to: view.state.doc.length, insert: value },
			});
		}
	});
</script>

<div
	bind:this={container}
	class="h-full w-full border border-border rounded-md overflow-hidden bg-card"
></div>
