<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { PluginRepoStatus, PluginStoreEvent, PluginStoreItem } from '@common'
import AppIcon from '../components/AppIcon.vue'
import BaseBtn from '../components/BaseBtn.vue'

const plugins = ref<PluginStoreItem[]>([])
const loading = ref(false)
const actionId = ref('')
const error = ref('')
const notice = ref('')
/** 当前数据源展示名（本地源为「本地安装」）；不是地址 */
const sourceLabel = ref('')
const keyword = ref('')
let unsubscribe: (() => void) | undefined

const filteredPlugins = computed(() => {
  const query = keyword.value.trim().toLowerCase()
  if (!query) return plugins.value
  return plugins.value.filter((plugin) =>
    [plugin.name, plugin.description, plugin.author, plugin.kind].some((value) =>
      value.toLowerCase().includes(query)
    )
  )
})

function formatSize(sizeBytes?: number): string {
  if (!sizeBytes) return ''
  if (sizeBytes < 1024 * 1024) return `${Math.ceil(sizeBytes / 1024)} KB`
  return `${(sizeBytes / 1024 / 1024).toFixed(1)} MB`
}

function kindLabel(kind: PluginStoreItem['kind']): string {
  return { lyrics: '歌词', theme: '主题', visual: '视觉', other: '其他' }[kind]
}

function mergePlugin(next: PluginStoreItem): void {
  const index = plugins.value.findIndex((item) => item.id === next.id)
  if (index >= 0) plugins.value[index] = next
}

function handleChange(event: PluginStoreEvent): void {
  mergePlugin(event.plugin)
}

async function refresh(): Promise<void> {
  loading.value = true
  error.value = ''
  notice.value = ''
  try {
    const result = await window.api.pluginStore.list()
    plugins.value = result.plugins
    sourceLabel.value = result.sourceLabel
    if (!plugins.value.length) notice.value = '还没有插件，拖入 .zip 插件包即可安装。'
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '插件列表读取失败。'
  } finally {
    loading.value = false
  }
}

// ---------- 本地 zip 安装（拖入 / 选择文件） ----------
const dragging = ref(false)

/** 从拖放事件取本地路径。Electron 39 已移除 File.path，统一走 webUtils。 */
function pathsFromDrop(event: DragEvent): string[] {
  const files = [...(event.dataTransfer?.files ?? [])]
  return files
    .map((file) => window.electron?.webUtils?.getPathForFile?.(file) ?? '')
    .filter((path) => !!path)
}

async function stagePaths(paths: string[]): Promise<void> {
  const zips = paths.filter((path) => /\.zip$/i.test(path))
  if (!zips.length) {
    error.value = '请拖入 .zip 格式的插件包。'
    return
  }
  loading.value = true
  error.value = ''
  notice.value = ''
  try {
    for (const path of zips) {
      const result = await window.api.pluginStore.stageLocal(path)
      notice.value = result.message
    }
    await refresh()
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '插件包读取失败。'
  } finally {
    loading.value = false
  }
}

function onDrop(event: DragEvent): void {
  dragging.value = false
  void stagePaths(pathsFromDrop(event))
}

async function pickLocal(): Promise<void> {
  error.value = ''
  notice.value = ''
  try {
    const result = await window.api.pluginStore.pickLocal()
    if (!result) return
    notice.value = result.message
    await refresh()
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '插件包读取失败。'
  }
}

async function install(plugin: PluginStoreItem): Promise<void> {
  actionId.value = plugin.id
  error.value = ''
  notice.value = ''
  try {
    const result = await window.api.pluginStore.install(plugin.id)
    mergePlugin(result.plugin)
    notice.value = result.message
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '插件安装失败。'
  } finally {
    actionId.value = ''
  }
}

async function uninstall(plugin: PluginStoreItem): Promise<void> {
  if (!window.confirm(`确定卸载「${plugin.name}」吗？`)) return
  actionId.value = plugin.id
  error.value = ''
  notice.value = ''
  try {
    const result = await window.api.pluginStore.uninstall(plugin.id)
    mergePlugin(result.plugin)
    notice.value = result.message
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '插件卸载失败。'
  } finally {
    actionId.value = ''
  }
}

async function toggleEnabled(plugin: PluginStoreItem): Promise<void> {
  actionId.value = plugin.id
  error.value = ''
  notice.value = ''
  try {
    const result = await window.api.pluginStore.setEnabled(plugin.id, !plugin.enabled)
    mergePlugin(result.plugin)
    notice.value = result.message
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '插件状态更新失败。'
  } finally {
    actionId.value = ''
  }
}

// ---------- 自定义源（参考 Sileo 的仓库模型：粘地址 → 拉索引 → 列包 → 装） ----------
const repos = ref<PluginRepoStatus[]>([])
const repoPanelOpen = ref(false)
const repoUrl = ref('')
const repoName = ref('')
const repoBusy = ref(false)

async function loadRepos(): Promise<void> {
  try {
    repos.value = await window.api.pluginRepo.list()
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '源列表读取失败。'
  }
}

/** 判断输入的地址是否为明文 http，用于提交前提示。 */
const urlIsInsecure = computed(() => {
  const raw = repoUrl.value.trim()
  if (!raw) return false
  return /^http:\/\//i.test(raw) || !/^[a-z][a-z0-9+.-]*:\/\//i.test(raw)
})

async function addRepo(): Promise<void> {
  const url = repoUrl.value.trim()
  if (!url) return
  // 明文源要用户明确确认一次：插件是应用内运行的代码，HTTP 下可被中途替换，
  // 而本项目没有签名体系，sha256 挡不住能同时改写索引与包的攻击者。
  if (
    urlIsInsecure.value &&
    !window.confirm(
      '这个源使用 HTTP 明文传输。\n\n' +
        '插件是应用内运行的代码，明文传输时网络中的人可以替换插件包，' +
        '本应用没有签名校验，无法识别这种替换。\n\n' +
        '仅在你信任该网络与该仓库时继续。要添加吗？'
    )
  ) {
    return
  }
  repoBusy.value = true
  error.value = ''
  notice.value = ''
  try {
    const added = await window.api.pluginRepo.add(url, repoName.value.trim())
    repoUrl.value = ''
    repoName.value = ''
    await loadRepos()
    notice.value = added.lastError
      ? `已添加「${added.name || added.url}」，但拉取失败：${added.lastError}`
      : `已添加「${added.name || added.url}」，找到 ${added.pluginCount ?? 0} 个插件。`
    await refresh()
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '添加源失败。'
  } finally {
    repoBusy.value = false
  }
}

async function removeRepo(repo: PluginRepoStatus): Promise<void> {
  if (!window.confirm(`确定移除源「${repo.name || repo.url}」吗？已安装的插件不会被卸载。`)) return
  repoBusy.value = true
  error.value = ''
  try {
    await window.api.pluginRepo.remove(repo.id)
    await loadRepos()
    await refresh()
    notice.value = '源已移除'
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '移除源失败。'
  } finally {
    repoBusy.value = false
  }
}

async function toggleRepo(repo: PluginRepoStatus): Promise<void> {
  repoBusy.value = true
  error.value = ''
  try {
    await window.api.pluginRepo.setEnabled(repo.id, !repo.enabled)
    await loadRepos()
    await refresh()
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : '源状态更新失败。'
  } finally {
    repoBusy.value = false
  }
}

onMounted(() => {
  unsubscribe = window.api.pluginStore.onChange(handleChange)
  void loadRepos()
  void refresh()
})

onBeforeUnmount(() => unsubscribe?.())
</script>

<template>
  <section class="plugin-store scroll">
    <header class="head">
      <div>
        <span class="eyebrow">EXTENSIONS</span>
        <h1>插件商店</h1>
        <p>安装并管理坤音neo扩展。支持本地 zip 与自定义源。</p>
      </div>
      <div class="head-actions">
        <BaseBtn min outline :disabled="repoBusy" @click="repoPanelOpen = !repoPanelOpen">
          <AppIcon name="plus" :size="15" />
          管理源
        </BaseBtn>
        <BaseBtn min :disabled="loading" @click="void pickLocal()">
          <AppIcon name="folder" :size="15" />
          选择插件包
        </BaseBtn>
      </div>
    </header>

    <!-- 自定义源面板 -->
    <div v-if="repoPanelOpen" class="repo-panel">
      <div class="repo-add">
        <input
          v-model="repoUrl"
          class="repo-input"
          type="text"
          placeholder="仓库索引地址，例如 example.com/plugins.json"
          :disabled="repoBusy"
          @keydown.enter.prevent="void addRepo()"
        />
        <input
          v-model="repoName"
          class="repo-input repo-input-name"
          type="text"
          placeholder="显示名（可留空）"
          :disabled="repoBusy"
          @keydown.enter.prevent="void addRepo()"
        />
        <BaseBtn min :disabled="repoBusy || !repoUrl.trim()" @click="void addRepo()">添加</BaseBtn>
      </div>
      <p v-if="urlIsInsecure" class="repo-warn">
        <AppIcon name="info" :size="13" />
        这是 HTTP 明文地址。插件是应用内运行的代码，明文传输时可被中途替换，本应用没有签名校验。
      </p>
      <p v-else class="repo-hint">
        索引需为 JSON：
        <code>{ "plugins": [ { id, name, version, kind, downloadUrl, sha256 } ] }</code>
        。每个条目必须带 sha256，下载后逐字节比对。
      </p>

      <ul v-if="repos.length" class="repo-list">
        <li v-for="repo in repos" :key="repo.id" class="repo-item">
          <div class="repo-meta">
            <span class="repo-title">
              {{ repo.name || repo.url }}
              <em v-if="repo.insecure" class="repo-tag">明文</em>
              <em v-if="!repo.enabled" class="repo-tag repo-tag-off">已停用</em>
            </span>
            <small class="repo-url">{{ repo.url }}</small>
            <small v-if="repo.lastError" class="repo-error">拉取失败：{{ repo.lastError }}</small>
            <small v-else-if="repo.pluginCount !== undefined" class="repo-ok">
              {{ repo.pluginCount }} 个插件
            </small>
          </div>
          <div class="repo-ops">
            <BaseBtn min outline :disabled="repoBusy" @click="void toggleRepo(repo)">
              {{ repo.enabled ? '停用' : '启用' }}
            </BaseBtn>
            <BaseBtn min outline :disabled="repoBusy" @click="void removeRepo(repo)">移除</BaseBtn>
          </div>
        </li>
      </ul>
      <p v-else class="repo-empty">还没有自定义源。粘贴一个仓库索引地址即可添加。</p>
    </div>

    <!-- 本地安装入口：拖入 zip 或点击选择 -->
    <div
      class="dropzone pressable"
      :class="{ dragging }"
      role="button"
      tabindex="0"
      @click="void pickLocal()"
      @keydown.enter.prevent="void pickLocal()"
      @dragover.prevent="dragging = true"
      @dragenter.prevent="dragging = true"
      @dragleave="dragging = false"
      @drop.prevent="onDrop"
    >
      <AppIcon name="upload" :size="22" />
      <strong>{{ dragging ? '松手即读取插件包' : '把 .zip 插件包拖到这里' }}</strong>
      <small>会检查最低应用版本；同目录存在 .sha256 文件时校验完整性</small>
    </div>

    <div class="toolbar">
      <label class="search-box">
        <AppIcon name="search" :size="16" />
        <input v-model="keyword" type="search" placeholder="搜索插件、作者或类型" />
      </label>
      <span v-if="sourceLabel" class="source">来源：{{ sourceLabel }}</span>
    </div>

    <p v-if="error" class="state error-state">
      <AppIcon name="info" :size="18" />
      {{ error }}
    </p>
    <p v-else-if="notice" class="state notice-state">{{ notice }}</p>
    <p v-if="loading && !plugins.length" class="state">正在读取插件列表…</p>
    <p v-else-if="!loading && !error && !filteredPlugins.length" class="state">
      {{ keyword ? '没有匹配的插件。' : '暂无可用插件。' }}
    </p>

    <div v-else class="plugin-grid">
      <article v-for="plugin in filteredPlugins" :key="plugin.id" class="plugin-item">
        <div class="plugin-topline">
          <div class="plugin-icon"><AppIcon name="plus" :size="20" /></div>
          <div class="plugin-title">
            <h2>{{ plugin.name }}</h2>
            <p>{{ plugin.author }} · v{{ plugin.version }}</p>
          </div>
          <span class="kind">{{ kindLabel(plugin.kind) }}</span>
        </div>
        <p class="description">{{ plugin.description }}</p>
        <div class="plugin-meta">
          <span v-if="formatSize(plugin.sizeBytes)">{{ formatSize(plugin.sizeBytes) }}</span>
          <span v-if="plugin.minAppVersion">需要 v{{ plugin.minAppVersion }}+</span>
          <span v-if="plugin.installed" class="installed">
            {{ plugin.enabled ? '已启用' : '已停用' }}
          </span>
        </div>
        <div class="actions">
          <template v-if="plugin.installed">
            <BaseBtn
              min
              outline
              :disabled="actionId === plugin.id"
              @click="void toggleEnabled(plugin)"
            >
              {{ plugin.enabled ? '停用' : '启用' }}
            </BaseBtn>
            <BaseBtn min outline :disabled="actionId === plugin.id" @click="void uninstall(plugin)">
              卸载
            </BaseBtn>
          </template>
          <BaseBtn v-else min :disabled="actionId === plugin.id" @click="void install(plugin)">
            {{ actionId === plugin.id ? '安装中…' : '安装' }}
          </BaseBtn>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.plugin-store {
  height: 100%;
  padding: 30px clamp(24px, 5vw, 70px) 52px;
  box-sizing: border-box;
  overflow-y: auto;
}
.head {
  width: min(100%, 980px);
  margin: 0 auto 24px;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
}
.eyebrow {
  color: var(--color-primary-font);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.16em;
}
h1 {
  margin-top: 7px;
  font-size: 27px;
  line-height: 1.15;
  font-weight: 720;
}
.head p {
  margin-top: 8px;
  color: var(--color-font-label);
  font-size: 12px;
}
.head :deep(.btn),
.actions :deep(.btn) {
  display: inline-flex;
  align-items: center;
  gap: 7px;
}
.head-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

/* ---- 自定义源面板 ---- */
.repo-panel {
  width: min(100%, 980px);
  margin: 0 auto 14px;
  padding: 16px;
  box-sizing: border-box;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-main-background);
}
.repo-add {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
.repo-input {
  flex: 1 1 260px;
  min-width: 0;
  height: 32px;
  padding: 0 10px;
  box-sizing: border-box;
  color: var(--color-font);
  font-size: 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--form-radius);
  background: var(--color-content-background);
  /* 只过渡颜色，形变交给全局 .pressable */
  transition: border-color var(--anim-dur-fast) var(--anim-ease-smooth);
}
.repo-input:focus {
  outline: 0;
  border-color: var(--color-primary);
}
.repo-input-name {
  flex: 0 1 160px;
}
.repo-hint,
.repo-warn,
.repo-empty {
  margin: 10px 0 0;
  font-size: 11px;
  line-height: 1.6;
  color: var(--color-font-label);
}
.repo-hint code {
  padding: 1px 4px;
  font-size: 10px;
  border-radius: 3px;
  background: var(--color-primary-background);
}
.repo-warn {
  display: flex;
  gap: 5px;
  align-items: flex-start;
  color: var(--color-badge-tertiary);
}
.repo-list {
  margin: 12px 0 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.repo-item {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
}
.repo-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.repo-title {
  font-size: 12.5px;
  font-weight: 650;
  color: var(--color-font);
}
.repo-tag {
  margin-left: 6px;
  padding: 1px 5px;
  font-size: 10px;
  font-style: normal;
  border-radius: 3px;
  color: #fff;
  background: var(--color-badge-tertiary);
}
.repo-tag-off {
  background: var(--color-font-label);
}
.repo-url,
.repo-error,
.repo-ok {
  font-size: 10.5px;
  color: var(--color-font-label);
  overflow-wrap: anywhere;
}
.repo-error {
  color: var(--color-badge-tertiary);
}
.repo-ops {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.dropzone {
  width: min(100%, 980px);
  margin: 0 auto 18px;
  padding: 22px 18px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 7px;
  text-align: center;
  cursor: pointer;
  color: var(--color-font-label);
  border: 1px dashed var(--color-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-main-background) 94%, var(--color-primary) 6%);
  /* 仅过渡颜色相关属性，形变交给全局 .pressable 的弹性回弹 */
  transition:
    border-color var(--anim-dur-fast) var(--anim-ease-smooth),
    background-color var(--anim-dur-fast) var(--anim-ease-smooth);
}
.dropzone:hover,
.dropzone:focus-visible {
  outline: 0;
  color: var(--color-primary-font);
  border-color: var(--color-primary);
}
.dropzone.dragging {
  color: var(--color-primary-font);
  border-style: solid;
  border-color: var(--color-primary);
  background: var(--color-primary-background);
}
.dropzone strong {
  color: var(--color-font);
  font-size: 13px;
  font-weight: 650;
}
.dropzone small {
  font-size: 11px;
}
.toolbar {
  width: min(100%, 980px);
  margin: 0 auto 18px;
  display: flex;
  align-items: center;
  gap: 12px;
}
.search-box {
  flex: 1;
  min-width: 0;
  height: 36px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  color: var(--color-font-label);
  background: color-mix(in srgb, var(--color-main-background) 88%, var(--color-primary) 12%);
}
.search-box input {
  width: 100%;
  min-width: 0;
  border: 0;
  outline: 0;
  color: var(--color-font);
  background: transparent;
  font: inherit;
}
.source {
  flex: none;
  color: var(--color-font-label);
  font-size: 11px;
}
.state {
  width: min(100%, 980px);
  margin: 18px auto;
  padding: 15px 16px;
  box-sizing: border-box;
  color: var(--color-font-label);
  border: 1px dashed var(--color-border);
  border-radius: 8px;
  text-align: center;
}
.state.error-state {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 8px;
  color: var(--color-error, #b3261e);
  border-style: solid;
}
.notice-state {
  color: var(--color-primary-font);
}
.plugin-grid {
  width: min(100%, 980px);
  margin: 0 auto;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(270px, 1fr));
  gap: 12px;
}
.plugin-item {
  min-width: 0;
  padding: 17px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--color-main-background) 94%, var(--color-primary) 6%);
}
.plugin-topline {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}
.plugin-icon {
  width: 38px;
  height: 38px;
  display: grid;
  place-items: center;
  flex: none;
  border-radius: 8px;
  color: var(--color-primary-font);
  background: var(--color-primary-background);
}
.plugin-title {
  min-width: 0;
  flex: 1;
}
.plugin-title h2 {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 14px;
  font-weight: 650;
}
.plugin-title p,
.description,
.plugin-meta {
  color: var(--color-font-label);
  font-size: 11px;
}
.plugin-title p {
  margin-top: 5px;
}
.kind {
  flex: none;
  padding: 4px 7px;
  border-radius: 5px;
  color: var(--color-primary-font);
  background: var(--color-primary-background);
  font-size: 10px;
}
.description {
  min-height: 34px;
  margin: 16px 0 14px;
  line-height: 1.55;
}
.plugin-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.installed {
  color: var(--color-primary-font);
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}
@media (max-width: 560px) {
  .head {
    align-items: flex-start;
    flex-direction: column;
  }
  .toolbar {
    align-items: stretch;
    flex-direction: column;
  }
  .source {
    display: none;
  }
}
</style>
