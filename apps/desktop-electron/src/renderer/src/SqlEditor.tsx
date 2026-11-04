import { createEffect, onCleanup, onMount } from "solid-js"
import { EditorState } from "@codemirror/state"
import { EditorView, keymap } from "@codemirror/view"
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language"
import { basicSetup } from "codemirror"
import { sql } from "@codemirror/lang-sql"
import { tags } from "@lezer/highlight"

/**
 * Syntax colours read from CSS variables, so light/dark follow the app theme
 * without rebuilding the editor state.
 */
const highlight = HighlightStyle.define([
  { tag: tags.keyword, color: "var(--sql-keyword)" },
  { tag: [tags.typeName, tags.className, tags.number, tags.bool, tags.null], color: "var(--sql-number)" },
  { tag: [tags.string, tags.special(tags.string)], color: "var(--sql-string)" },
  { tag: [tags.comment, tags.meta], color: "var(--sql-comment)", fontStyle: "italic" },
  { tag: [tags.function(tags.variableName), tags.labelName], color: "var(--sql-function)" },
  { tag: [tags.operator, tags.operatorKeyword, tags.punctuation, tags.separator], color: "var(--sql-operator)" },
  { tag: [tags.variableName, tags.propertyName], color: "var(--ink)" },
  { tag: tags.invalid, color: "var(--danger)" },
])

const appTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "13px",
    backgroundColor: "var(--surface)",
    color: "var(--ink)",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "var(--grid-font, ui-monospace, SFMono-Regular, Menlo, monospace)",
    lineHeight: "1.65",
  },
  ".cm-content": { padding: "10px 0", caretColor: "var(--ink)" },
  ".cm-gutters": {
    backgroundColor: "var(--surface)",
    color: "var(--faint)",
    border: "none",
    paddingRight: "6px",
  },
  ".cm-activeLine": { backgroundColor: "transparent" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--muted)" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--ink)", borderLeftWidth: "2px" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
    backgroundColor: "var(--accent-soft)",
  },
  ".cm-selectionMatch": { backgroundColor: "var(--surface-3)" },
  ".cm-panels": { backgroundColor: "var(--surface-2)", color: "var(--ink)" },
  ".cm-tooltip": {
    border: "1px solid var(--line)",
    borderRadius: "6px",
    backgroundColor: "var(--surface)",
    boxShadow: "0 8px 20px -10px rgba(20, 20, 16, 0.3)",
  },
  ".cm-tooltip-autocomplete ul li[aria-selected]": {
    backgroundColor: "var(--accent-soft)",
    color: "var(--ink)",
  },
})

export default function SqlEditor(props: {
  value: string
  onChange: (value: string) => void
  onRun: () => void
}) {
  let host!: HTMLDivElement
  let view: EditorView | null = null

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: props.value,
        extensions: [
          basicSetup,
          sql(),
          syntaxHighlighting(highlight),
          appTheme,
          EditorView.lineWrapping,
          keymap.of([
            {
              key: "Mod-Enter",
              preventDefault: true,
              run: () => {
                props.onRun()
                return true
              },
            },
          ]),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) props.onChange(update.state.doc.toString())
          }),
        ],
      }),
    })
  })

  // adopt values that changed elsewhere (e.g. "insert to editor")
  createEffect(() => {
    const next = props.value
    const current = view
    if (!current || next === current.state.doc.toString()) return
    current.dispatch({ changes: { from: 0, to: current.state.doc.length, insert: next } })
  })

  onCleanup(() => {
    view?.destroy()
    view = null
  })

  return <div class="sql-cm" ref={host} />
}
