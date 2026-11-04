import { createEffect, onCleanup, onMount } from "solid-js"
import { EditorState } from "@codemirror/state"
import { EditorView, GutterMarker, gutter, keymap } from "@codemirror/view"
import { acceptCompletion } from "@codemirror/autocomplete"
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language"
import { basicSetup } from "codemirror"
import { sql } from "@codemirror/lang-sql"
import { tags } from "@lezer/highlight"

interface StatementSpan {
  start: number
  end: number
}

/** Statement ranges, ignoring `;` inside strings and comments. */
function statementSpans(text: string): StatementSpan[] {
  const spans: StatementSpan[] = []
  let start = 0
  let i = 0
  while (i < text.length) {
    const char = text[i]
    if (char === "'" || char === '"' || char === "`") {
      const quote = char
      i += 1
      while (i < text.length) {
        if (text[i] === quote) {
          if (text[i + 1] === quote) {
            i += 2
            continue
          }
          i += 1
          break
        }
        i += 1
      }
      continue
    }
    if (char === "-" && text[i + 1] === "-") {
      const newline = text.indexOf("\n", i)
      i = newline === -1 ? text.length : newline + 1
      continue
    }
    if (char === "/" && text[i + 1] === "*") {
      const close = text.indexOf("*/", i + 2)
      i = close === -1 ? text.length : close + 2
      continue
    }
    if (char === ";") {
      spans.push({ start, end: i })
      start = i + 1
    }
    i += 1
  }
  spans.push({ start, end: text.length })
  return spans
}

/**
 * The statement under the cursor, so a buffer with several statements runs the
 * one the user is looking at.
 */
function statementAt(text: string, position: number): string {
  const spans = statementSpans(text)
  const hit = spans.find((span) => position >= span.start && position <= span.end)
  const picked = hit ? text.slice(hit.start, hit.end).trim() : ""
  if (picked) return picked
  // cursor sits in whitespace (e.g. right after the final `;`) — use the statement before it
  for (let index = spans.length - 1; index >= 0; index -= 1) {
    if (spans[index].end >= position) continue
    const candidate = text.slice(spans[index].start, spans[index].end).trim()
    if (candidate) return candidate
  }
  return text.trim()
}

const PLAY_ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" width="10" height="10" aria-hidden="true"><path d="M8 5v14l11-7z"/></svg>'

/** One play button in the gutter, pinned to the first line of a statement. */
class RunStatementMarker extends GutterMarker {
  constructor(private readonly statement: string, private readonly run: (sql: string) => void) {
    super()
  }

  toDOM(): HTMLElement {
    const button = document.createElement("button")
    button.type = "button"
    button.className = "cm-run-btn"
    button.title = "执行这条语句"
    button.setAttribute("aria-label", "执行这条语句")
    button.innerHTML = PLAY_ICON
    button.addEventListener("mousedown", (event) => event.preventDefault())
    button.addEventListener("click", (event) => {
      event.preventDefault()
      event.stopPropagation()
      this.run(this.statement)
    })
    return button
  }
}

class RunMarkerSpacer extends GutterMarker {
  toDOM(): HTMLElement {
    const span = document.createElement("span")
    span.className = "cm-run-spacer"
    return span
  }
}

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
  /** run one specific statement, from its gutter button */
  onRunStatement: (sql: string) => void
  /** hands back an imperative handle for the toolbar and the run action */
  register?: (api: { insert: (text: string) => void; statement: () => string }) => void
}) {
  let host!: HTMLDivElement
  let view: EditorView | null = null

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: props.value,
        extensions: [
          // Tab accepts the highlighted completion; otherwise it stays in the editor
          keymap.of([
            { key: "Tab", run: acceptCompletion },
            { key: "Tab", run: () => true },
          ]),
          gutter({
            class: "cm-run-gutter",
            lineMarker: (view, line) => {
              const doc = view.state.doc.toString()
              for (const span of statementSpans(doc)) {
                const text = doc.slice(span.start, span.end)
                const offset = text.search(/\S/)
                if (offset < 0) continue
                const anchor = span.start + offset
                if (anchor < line.from || anchor > line.to) continue
                return new RunStatementMarker(text.trim(), props.onRunStatement)
              }
              return null
            },
            lineMarkerChange: (update) => update.docChanged,
            initialSpacer: () => new RunMarkerSpacer(),
          }),
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
    props.register?.({
      insert: (text) => {
        const current = view
        if (!current) return
        const selection = current.state.selection.main
        const prefix = selection.from === 0 || current.state.sliceDoc(selection.from - 1, selection.from) === " " ? "" : " "
        current.dispatch({
          changes: { from: selection.from, to: selection.to, insert: prefix + text },
          selection: { anchor: selection.from + prefix.length + text.length },
        })
        current.focus()
      },
      statement: () => {
        const current = view
        if (!current) return ""
        const { from, to } = current.state.selection.main
        if (from !== to) return current.state.sliceDoc(from, to)
        return statementAt(current.state.doc.toString(), from)
      },
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
