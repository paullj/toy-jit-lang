import { StreamLanguage } from "@codemirror/language";
import { tags } from "@lezer/highlight";

// Simple tokenizer for toy language
const toyLanguage = StreamLanguage.define({
	token(stream) {
		// Skip whitespace
		if (stream.eatSpace()) return null;

		// Comments
		if (stream.match("//")) {
			stream.skipToEnd();
			return "comment";
		}

		// Strings
		if (stream.match('"')) {
			while (!stream.eol()) {
				if (stream.next() === '"') break;
			}
			return "string";
		}

		// Numbers (floats and integers)
		if (stream.match(/^-?\d+\.?\d*/)) {
			return "number";
		}

		// Definition operator :=
		if (stream.match(":=")) {
			return "definitionOperator";
		}

		// Type annotation :
		if (stream.match(":")) {
			return "punctuation";
		}

		// Float operators
		if (stream.match(/^[+\-*/%]=?\./) || stream.match(/^[<>=!]=?\./)) {
			return "operator";
		}

		// Regular operators
		if (stream.match(/^[+\-*/%]=?/) || stream.match(/^[<>=!]=?/) || stream.match("&&") || stream.match("||")) {
			return "operator";
		}

		// Parentheses and brackets
		if (stream.match(/^[()[\]{}]/)) {
			return "bracket";
		}

		// Keywords and identifiers
		if (stream.match(/^[a-zA-Z_][a-zA-Z0-9_]*/)) {
			const word = stream.current();
			if (["true", "false"].includes(word)) {
				return "bool";
			}
			if (["if", "else", "while", "for", "fn", "return", "let", "mut"].includes(word)) {
				return "keyword";
			}
			return "variableName";
		}

		// Skip unknown characters
		stream.next();
		return null;
	},
});

// Highlighting style
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";

const toyHighlightStyle = HighlightStyle.define([
	{ tag: tags.keyword, class: "tok-keyword" },
	{ tag: tags.operator, class: "tok-operator" },
	{ tag: tags.number, class: "tok-number" },
	{ tag: tags.string, class: "tok-string" },
	{ tag: tags.comment, class: "tok-comment" },
	{ tag: tags.variableName, class: "tok-variableName" },
	{ tag: tags.definition(tags.variableName), class: "tok-definition" },
	{ tag: tags.bool, class: "tok-bool" },
]);

export const toyLangSupport = [
	toyLanguage,
	syntaxHighlighting(toyHighlightStyle),
];
