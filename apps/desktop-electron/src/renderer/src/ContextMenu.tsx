import { For, Show, createSignal } from "solid-js"
import { Check, ChevronRight } from "lucide-solid"
import type { JSX } from "solid-js"

export interface MenuItem {
  label?: string
  separator?: boolean
  checked?: boolean
  disabled?: boolean
  icon?: JSX.Element
  /** right-aligned muted text, like a count */
  hint?: string
  submenu?: MenuItem[]
  onSelect?: () => void
}

const MENU_WIDTH = 236
const MENU_HEIGHT = 460

export default function ContextMenu(props: {
  x: number
  y: number
  items: MenuItem[]
  title?: string
  onClose: () => void
}) {
  const [openSub, setOpenSub] = createSignal<number | null>(null)
  const left = () => Math.max(8, Math.min(props.x, window.innerWidth - MENU_WIDTH - 8))
  const top = () => Math.max(8, Math.min(props.y, window.innerHeight - MENU_HEIGHT))

  function pick(item: MenuItem) {
    if (item.disabled || item.separator || item.submenu) return
    item.onSelect?.()
    props.onClose()
  }

  return (
    <div class="menu-backdrop" onClick={props.onClose} onContextMenu={(event) => event.preventDefault()}>
      <div
        class="menu"
        role="menu"
        style={{ left: `${left()}px`, top: `${top()}px` }}
        onClick={(event) => event.stopPropagation()}
      >
        <Show when={props.title}>
          <div class="menu-title">{props.title}</div>
        </Show>
        <For each={props.items}>
          {(item, index) => (
            <Show when={!item.separator} fallback={<div class="menu-sep" />}>
              <div class="menu-wrap" onMouseEnter={() => setOpenSub(item.submenu ? index() : null)}>
                <button
                  class="menu-item"
                  classList={{
                    disabled: item.disabled,
                    checked: item.checked,
                    active: !!item.submenu && openSub() === index(),
                  }}
                  role="menuitem"
                  disabled={item.disabled}
                  onClick={() => pick(item)}
                >
                  <span class="menu-icon">
                    <Show when={item.icon} fallback={<Show when={item.checked}><Check size={15} /></Show>}>
                      {item.icon}
                    </Show>
                  </span>
                  <span class="menu-label">{item.label}</span>
                  <Show when={item.hint}>
                    <span class="menu-hint">{item.hint}</span>
                  </Show>
                  <Show when={item.submenu}>
                    <ChevronRight size={15} class="menu-arrow" />
                  </Show>
                </button>
                <Show when={item.submenu && openSub() === index()}>
                  <div class="menu menu-sub" role="menu">
                    <For each={item.submenu}>
                      {(sub) => (
                        <Show when={!sub.separator} fallback={<div class="menu-sep" />}>
                          <button
                            class="menu-item"
                            classList={{ disabled: sub.disabled, checked: sub.checked }}
                            role="menuitem"
                            disabled={sub.disabled}
                            onClick={() => pick(sub)}
                          >
                            <span class="menu-icon">
                              <Show when={sub.icon} fallback={<Show when={sub.checked}><Check size={15} /></Show>}>
                                {sub.icon}
                              </Show>
                            </span>
                            <span class="menu-label">{sub.label}</span>
                          </button>
                        </Show>
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            </Show>
          )}
        </For>
      </div>
    </div>
  )
}
