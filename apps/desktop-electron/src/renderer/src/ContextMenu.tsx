import { For, Show, createSignal } from "solid-js"
import { Check, ChevronRight } from "lucide-solid"

export interface MenuItem {
  label?: string
  separator?: boolean
  checked?: boolean
  disabled?: boolean
  submenu?: MenuItem[]
  onSelect?: () => void
}

const MENU_WIDTH = 208
const MENU_HEIGHT = 420

export default function ContextMenu(props: { x: number; y: number; items: MenuItem[]; onClose: () => void }) {
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
        <For each={props.items}>
          {(item, index) => (
            <Show when={!item.separator} fallback={<div class="menu-sep" />}>
              <div class="menu-wrap" onMouseEnter={() => setOpenSub(item.submenu ? index() : null)}>
                <button
                  class="menu-item"
                  classList={{ disabled: item.disabled }}
                  role="menuitem"
                  disabled={item.disabled}
                  onClick={() => pick(item)}
                >
                  <span class="menu-check">
                    <Show when={item.checked}>
                      <Check size={12} />
                    </Show>
                  </span>
                  <span class="menu-label">{item.label}</span>
                  <Show when={item.submenu}>
                    <ChevronRight size={12} class="menu-arrow" />
                  </Show>
                </button>
                <Show when={item.submenu && openSub() === index()}>
                  <div class="menu menu-sub" role="menu">
                    <For each={item.submenu}>
                      {(sub) => (
                        <Show when={!sub.separator} fallback={<div class="menu-sep" />}>
                          <button
                            class="menu-item"
                            classList={{ disabled: sub.disabled }}
                            role="menuitem"
                            disabled={sub.disabled}
                            onClick={() => pick(sub)}
                          >
                            <span class="menu-check">
                              <Show when={sub.checked}>
                                <Check size={12} />
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
