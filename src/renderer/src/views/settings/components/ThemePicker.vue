<script setup lang="ts">
import { computed, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useSettingsStore } from '../../../stores/settings'
import {
  buildThemeColors,
  customToThemeDef,
  dedupeThemeIdentity,
  findTheme,
  themeDefToFileConfig,
  THEMES,
  type ThemeDef
} from '../../../theme/themes'
import AutoThemeDialog from './AutoThemeDialog.vue'
import ThemeEditDialog from './ThemeEditDialog.vue'
import ContextMenu, { type MenuItem } from '../../../components/ContextMenu.vue'

const store = useSettingsStore()
const { settings } = storeToRefs(store)

interface ThemeItem {
  def: ThemeDef
  styles: Record<string, string>
}

// ---- 轻提示（与 AlbumView/ArtistView 同一套实现） ----
const toast = ref('')
let toastTimer: ReturnType<typeof setTimeout> | null = null
function showToast(msg: string): void {
  toast.value = msg
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => (toast.value = ''), 3600)
}

const customDefs = computed<ThemeDef[]>(() =>
  settings.value.appearance.customThemes.map(customToThemeDef)
)

function themePreviewStyles(theme: ThemeDef): Record<string, string> {
  return {
    '--color-primary-theme': buildThemeColors(theme)['--color-theme'] ?? theme.primary,
    '--background-image-theme': theme.ext['--background-image'] ?? 'none'
  }
}

const themeItems = computed<ThemeItem[]>(() =>
  [...THEMES, ...customDefs.value].map((def) => ({ def, styles: themePreviewStyles(def) }))
)

const autoStyles = computed<Record<string, string>>(() => {
  const light =
    findTheme(settings.value.appearance.lightThemeId, customDefs.value) ??
    findTheme('green', []) ??
    THEMES[0]
  const dark =
    findTheme(settings.value.appearance.darkThemeId, customDefs.value) ??
    findTheme('black', []) ??
    THEMES[0]
  return {
    '--color-primary-theme-light': buildThemeColors(light)['--color-theme'] ?? light.primary,
    '--background-image-theme-light': light.ext['--background-image'] ?? 'none',
    '--color-primary-theme-dark': buildThemeColors(dark)['--color-theme'] ?? dark.primary,
    '--background-image-theme-dark': dark.ext['--background-image'] ?? 'none'
  }
})

const themeId = computed(() => settings.value.appearance.themeId)

function toggleTheme(id: string): void {
  if (themeId.value === id) return
  void store.update({ appearance: { themeId: id } })
}

function themeKind(theme: ThemeDef): string {
  if (theme.isDark) return '深色'
  return theme.ext['--background-image'] !== 'none' ? '插画' : '浅色'
}

const showAutoDialog = ref(false)
const showEdit = ref(false)
const editThemeId = ref('')

function handleEditTheme(theme?: ThemeDef): void {
  if (theme && !theme.isCustom) return
  editThemeId.value = theme?.id ?? ''
  showEdit.value = true
}

// ---------- 右键菜单：编辑 / 导出 / 删除 ----------
const menu = ref<{ x: number; y: number; theme: ThemeDef } | null>(null)

const menuItems = computed<MenuItem[]>(() => {
  const theme = menu.value?.theme
  if (!theme) return []
  return [
    { key: 'edit', label: '编辑主题', icon: 'edit', disabled: !theme.isCustom },
    { key: 'export', label: '导出主题', icon: 'download' },
    {
      key: 'delete',
      label: '删除主题',
      icon: 'trash',
      danger: true,
      divider: true,
      disabled: !theme.isCustom
    }
  ]
})

function openMenu(e: MouseEvent, theme: ThemeDef): void {
  menu.value = { x: e.clientX, y: e.clientY, theme }
}

function onMenuSelect(key: string): void {
  const theme = menu.value?.theme
  if (!theme) return
  if (key === 'edit') handleEditTheme(theme)
  else if (key === 'export') void exportTheme(theme)
  else if (key === 'delete') void removeTheme(theme)
}

async function exportTheme(theme: ThemeDef): Promise<void> {
  try {
    // 内置主题也可导出：改个色再导入即成为自定义主题
    const name = await window.api.theme.exportTheme(themeDefToFileConfig(theme))
    if (name) showToast(`已导出主题：${name}`)
  } catch (err) {
    showToast(`导出失败：${(err as Error).message}`)
  }
}

async function importTheme(): Promise<void> {
  let config: Awaited<ReturnType<typeof window.api.theme.importTheme>>
  try {
    config = await window.api.theme.importTheme()
  } catch (err) {
    // 校验失败（缺字段 / 格式错 / 非法色值 / 携带远程地址）在此收敛为 toast，不崩溃
    showToast(`导入失败：${(err as Error).message}`)
    return
  }
  if (!config) return
  const customThemes = [...settings.value.appearance.customThemes]
  if (customThemes.length >= 10) {
    showToast('自定义主题数量已达上限（10 个）')
    return
  }
  // id 与内置/已有主题查重，冲突自动加后缀
  const safe = dedupeThemeIdentity(config, customThemes)
  customThemes.push(safe)
  // store.update 内部会调用 setCustomThemes + applyTheme，切到新主题即时生效
  await store.update({ appearance: { customThemes, themeId: safe.id } })
  showToast(
    safe.id === config.id
      ? `已导入主题：${safe.name}`
      : `已导入主题：${safe.name}（id 冲突，已重命名）`
  )
}

async function removeTheme(theme: ThemeDef): Promise<void> {
  if (!theme.isCustom) return
  const customThemes = settings.value.appearance.customThemes.filter((t) => t.id !== theme.id)
  const patch: Parameters<typeof store.update>[0] = { appearance: { customThemes } }
  // 删除的是当前正在用的主题时回落默认主题，避免解析不到主题
  const { themeId: cur, lightThemeId, darkThemeId } = settings.value.appearance
  if (cur === theme.id) patch.appearance!.themeId = theme.isDark ? 'black' : 'green'
  if (lightThemeId === theme.id) patch.appearance!.lightThemeId = 'green'
  if (darkThemeId === theme.id) patch.appearance!.darkThemeId = 'black'
  await store.update(patch)
  showToast(`已删除主题：${theme.name}`)
}
</script>

<template>
  <div class="theme">
    <button
      v-for="item in themeItems"
      :key="item.def.id"
      class="theme-item pressable"
      :class="{ active: themeId === item.def.id }"
      :style="item.styles"
      :aria-label="item.def.name"
      :aria-pressed="themeId === item.def.id"
      @click="toggleTheme(item.def.id)"
      @contextmenu.prevent="openMenu($event, item.def)"
    >
      <span class="preview">
        <span class="preview-image" />
        <span class="swatch" />
        <span v-if="themeId === item.def.id" class="check">✓</span>
      </span>
      <span class="label">
        <strong>{{ item.def.name }}</strong>
        <small>{{ themeKind(item.def) }}</small>
      </span>
    </button>

    <button
      class="theme-item auto pressable"
      :class="{ active: themeId === 'auto' }"
      :style="autoStyles"
      aria-label="跟随系统"
      :aria-pressed="themeId === 'auto'"
      @click="toggleTheme('auto')"
      @contextmenu.prevent="showAutoDialog = true"
    >
      <span class="preview auto-preview">
        <span class="light" />
        <span class="dark" />
        <span v-if="themeId === 'auto'" class="check">✓</span>
      </span>
      <span class="label">
        <strong>跟随系统</strong>
        <small>右键设置明暗主题</small>
      </span>
    </button>

    <button class="theme-item add pressable" aria-label="添加主题" @click="handleEditTheme()">
      <span class="preview add-preview">
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 5v14M5 12h14" />
        </svg>
      </span>
      <span class="label">
        <strong>添加主题</strong>
        <small>创建自己的配色</small>
      </span>
    </button>

    <button class="theme-item add pressable" aria-label="导入主题" @click="importTheme()">
      <span class="preview add-preview">
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 3v12M7 10l5 5 5-5M4 19h16" />
        </svg>
      </span>
      <span class="label">
        <strong>导入主题</strong>
        <small>从 .json 文件导入</small>
      </span>
    </button>
  </div>
  <ContextMenu
    v-if="menu"
    :x="menu.x"
    :y="menu.y"
    :items="menuItems"
    @select="onMenuSelect"
    @close="menu = null"
  />
  <Teleport to="body">
    <div v-if="toast" class="theme-toast">{{ toast }}</div>
  </Teleport>
  <AutoThemeDialog v-model="showAutoDialog" />
  <ThemeEditDialog v-model="showEdit" :theme-id="editThemeId" />
</template>

<style scoped>
.theme {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(148px, 1fr));
  gap: 10px;
}
.theme-item {
  min-width: 0;
  padding: 0;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  overflow: hidden;
  color: var(--color-font);
  background: color-mix(in srgb, var(--color-main-background) 94%, transparent);
  text-align: left;
  cursor: pointer;
  /* transform/filter 归全局 .pressable；这里只留边框与投影 */
  transition:
    border-color var(--anim-dur-fast) var(--anim-ease-standard),
    box-shadow var(--anim-dur-fast) var(--anim-ease-standard);
}
.theme-item:hover {
  border-color: var(--color-primary-alpha-600);
  box-shadow: 0 7px 18px rgba(0, 0, 0, 0.07);
  --press-lift: -2px;
}
.theme-item.active {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-alpha-800);
}
/* 当前主题不再上浮（仍保留按下缩放） */
.theme-item.active:hover {
  --press-lift: 0px;
}
.preview {
  position: relative;
  display: block;
  height: 72px;
  overflow: hidden;
  background-color: var(--color-primary-theme);
}
.preview-image {
  position: absolute;
  inset: 0;
  background-image: var(--background-image-theme);
  background-position: center 38%;
  background-size: cover;
  background-repeat: no-repeat;
}
.preview::after {
  position: absolute;
  inset: 0;
  content: '';
  background: linear-gradient(180deg, transparent 45%, rgba(0, 0, 0, 0.12));
}
.swatch {
  position: absolute;
  z-index: 1;
  left: 9px;
  bottom: 8px;
  width: 18px;
  height: 6px;
  border-radius: 999px;
  background: var(--color-primary-theme);
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.85);
}
.check {
  position: absolute;
  z-index: 2;
  top: 7px;
  right: 7px;
  display: grid;
  place-items: center;
  width: 21px;
  height: 21px;
  border-radius: 50%;
  color: var(--color-primary-font);
  background: rgba(255, 255, 255, 0.94);
  font-size: 12px;
  font-weight: 800;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.14);
}
.label {
  display: grid;
  gap: 3px;
  min-width: 0;
  padding: 9px 10px 10px;
}
.label strong,
.label small {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.label strong {
  font-size: 11px;
  font-weight: 650;
}
.label small {
  color: var(--color-font-label);
  font-size: 9px;
}
.auto .light,
.auto .dark {
  position: absolute;
  inset: 0;
  background-position: center;
  background-size: cover;
  background-repeat: no-repeat;
}
.auto .light {
  clip-path: polygon(0 0, 100% 0, 0 100%);
  background-color: var(--color-primary-theme-light);
  background-image: var(--background-image-theme-light);
}
.auto .dark {
  clip-path: polygon(0 100%, 100% 0, 100% 100%);
  background-color: var(--color-primary-theme-dark);
  background-image: var(--background-image-theme-dark);
}
.add-preview {
  display: grid;
  place-items: center;
  color: var(--color-primary-font);
  background:
    radial-gradient(circle at center, var(--color-primary-alpha-800), transparent 58%),
    color-mix(in srgb, var(--color-main-background) 92%, var(--color-primary) 8%);
}
.add .icon {
  width: 25px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linecap: round;
}

/* 轻提示：Teleport 到 body，避免被设置页的滚动容器裁切 */
.theme-toast {
  position: fixed;
  left: 50%;
  bottom: 88px;
  transform: translateX(-50%);
  z-index: 2000;
  padding: 9px 18px;
  border-radius: 999px;
  font-size: 13px;
  color: #fff;
  background: rgba(0, 0, 0, 0.72);
  pointer-events: none;
}

@media (prefers-reduced-motion: reduce) {
  .theme-item {
    transition: none;
  }
}
</style>
