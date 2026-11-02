import {
  Activity,
  ArrowLeft,
  Bell,
  Boxes,
  ChevronDown,
  FlaskConical,
  Folder,
  Info,
  Keyboard,
  Link2,
  Palette,
  Puzzle,
  Search,
  Server,
  SlidersHorizontal,
  Sparkles,
  Store,
  Workflow,
} from "lucide-solid"
import { For, Show, createMemo, createSignal, type JSX } from "solid-js"

interface SettingRowProps {
  label: string
  description: string
  children: JSX.Element
  block?: boolean
}

function SettingRow(props: SettingRowProps) {
  return (
    <div class="setting-row" classList={{ block: props.block }}>
      <div class="setting-copy">
        <div class="setting-label">{props.label}</div>
        <div class="setting-desc">{props.description}</div>
      </div>
      <div class="setting-control">{props.children}</div>
    </div>
  )
}

function Toggle(props: { checked: boolean; onChange: () => void; label: string }) {
  return (
    <button class="toggle" classList={{ on: props.checked }} role="switch" aria-checked={props.checked} aria-label={props.label} onClick={props.onChange}>
      <span class="toggle-knob" />
    </button>
  )
}

function Select(props: { label: string }) {
  return (
    <button class="setting-select">
      {props.label}
      <ChevronDown size={13} />
    </button>
  )
}

const sections = [
  { id: "providers", label: "提供商", icon: () => <Boxes size={16} /> },
  { id: "models", label: "模型", icon: () => <Sparkles size={16} /> },
  { id: "extensions", label: "扩展", icon: () => <Puzzle size={16} /> },
  { id: "projects", label: "项目", icon: () => <Folder size={16} /> },
  { id: "workspace", label: "工作区", icon: () => <Workflow size={16} /> },
  { id: "preferences", label: "偏好设置", icon: () => <SlidersHorizontal size={16} /> },
  { id: "appearance", label: "外观", icon: () => <Palette size={16} /> },
  { id: "notifications", label: "通知", icon: () => <Bell size={16} /> },
  { id: "shortcuts", label: "快捷键", icon: () => <Keyboard size={16} /> },
  { id: "pairing", label: "Pairing", icon: () => <Link2 size={16} /> },
  { id: "marketplace", label: "插件市场", icon: () => <Store size={16} /> },
  { id: "run-meter", label: "Run meter", icon: () => <Activity size={16} /> },
  { id: "servers", label: "服务器", icon: () => <Server size={16} />, divider: true },
  { id: "experiments", label: "实验", icon: () => <FlaskConical size={16} /> },
  { id: "about", label: "关于", icon: () => <Info size={16} /> },
]

export default function SettingsView(props: { onClose: () => void }) {
  const [active, setActive] = createSignal("preferences")
  const [query, setQuery] = createSignal("")
  const [prefs, setPrefs] = createSignal({
    autoApprove: true,
    showAgents: false,
    pinch: false,
    spellcheck: true,
    wrap: true,
  })

  const filtered = createMemo(() => {
    const q = query().trim().toLowerCase()
    return q ? sections.filter((item) => item.label.toLowerCase().includes(q)) : sections
  })

  const activeSection = createMemo(() => sections.find((item) => item.id === active()))

  function toggle(key: keyof ReturnType<typeof prefs>) {
    setPrefs({ ...prefs(), [key]: !prefs()[key] })
  }

  return (
    <div class="settings-page">
      <aside class="settings-nav">
        <button class="settings-back" onClick={props.onClose}>
          <ArrowLeft size={15} />
          返回应用
        </button>
        <label class="settings-search">
          <Search size={15} />
          <input
            placeholder="搜索"
            aria-label="搜索设置"
            value={query()}
            onInput={(event) => setQuery(event.currentTarget.value)}
          />
        </label>
        <div class="settings-nav-list">
          <For each={filtered()}>
            {(item) => (
              <button
                class="settings-nav-item"
                classList={{ active: active() === item.id, divider: "divider" in item && item.divider }}
                onClick={() => setActive(item.id)}
              >
                {item.icon()}
                {item.label}
              </button>
            )}
          </For>
          <Show when={filtered().length === 0}>
            <div class="settings-nav-empty">没有找到设置</div>
          </Show>
        </div>
      </aside>

      <div class="settings-scroll">
        <div class="settings-content">
          <Show
            when={active() === "preferences"}
            fallback={
              <>
                <div class="settings-heading">
                  <h1>{activeSection()?.label}</h1>
                  <p>这个设置在原型中暂未展开。</p>
                </div>
                <div class="settings-empty">
                  <div class="settings-empty-title">{activeSection()?.label}</div>
                  <p>返回「偏好设置」查看当前可交互的设置项。</p>
                </div>
              </>
            }
          >
            <div class="settings-heading">
              <h1>偏好设置</h1>
              <p>自定义偏好、主题和默认行为</p>
            </div>

            <section class="settings-card">
              <SettingRow label="语言" description="更改 OMDBX 的显示语言">
                <Select label="简体中文" />
              </SettingRow>
              <SettingRow label="默认环境" description="选择新会话的启动位置">
                <Select label="各项目上次使用的位置" />
              </SettingRow>
              <SettingRow label="自动接受权限" description="无需询问即可批准权限请求，项目可在自己的设置中覆盖。">
                <Toggle checked={prefs().autoApprove} label="自动接受权限" onChange={() => toggle("autoApprove")} />
              </SettingRow>
              <SettingRow label="显示智能体" description="在输入框中切换智能体。隐藏时默认使用 Build 智能体。">
                <Toggle checked={prefs().showAgents} label="显示智能体" onChange={() => toggle("showAgents")} />
              </SettingRow>
              <SettingRow label="后续消息行为" description="选择将后续消息加入队列还是引导当前轮次。">
                <Select label="加入队列" />
              </SettingRow>
              <SettingRow label="双指缩放" description="允许使用触控板双指捏合和 Ctrl+滚轮手势缩放">
                <Toggle checked={prefs().pinch} label="双指缩放" onChange={() => toggle("pinch")} />
              </SettingRow>
              <SettingRow label="拼写检查" description="输入时给拼写错误的词加下划线。">
                <Toggle checked={prefs().spellcheck} label="拼写检查" onChange={() => toggle("spellcheck")} />
              </SettingRow>
              <SettingRow label="换行" description="在移动设备上换行，而不是水平滚动">
                <Toggle checked={prefs().wrap} label="换行" onChange={() => toggle("wrap")} />
              </SettingRow>
            </section>

            <h2 class="settings-section-title">时间表</h2>
            <section class="settings-card">
              <SettingRow
                label="时间轴详细信息"
                description="选择时间轴上显示的活动量。消息始终可见。"
                block
              >
                <input class="settings-slider" type="range" min="0" max="4" value="2" aria-label="时间轴详细信息" />
              </SettingRow>
              <div class="settings-note">
                <strong>紧凑型：</strong> 单独显示子智能体，其他活动折叠在详情里。
              </div>
              <div class="settings-subhead">高级</div>
              <SettingRow label="Timeline width" description="更窄的线条更易扫读，更宽的线条一次展示更多。">
                <Select label="Wide" />
              </SettingRow>
            </section>
          </Show>
        </div>
      </div>
    </div>
  )
}
