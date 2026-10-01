<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useSettingsStore } from '../../stores/settings'
import { useApi } from '../../composables/useApi'
import { listSystemFonts } from '../../composables/useFonts'
import { FONT_SIZE_LIST, WINDOW_SIZE_LIST, type ComfortLevel } from '@common'
import { COMFORT_LEVELS } from '../../theme/comfort'
import BaseCheckbox from '../../components/BaseCheckbox.vue'
import BaseSelect from '../../components/BaseSelect.vue'
import ThemePicker from './components/ThemePicker.vue'

const store = useSettingsStore()
const { settings } = storeToRefs(store)
const api = useApi()

const fontList = ref<{ id: string; label: string }[]>([{ id: '', label: '默认' }])
onMounted(async () => {
  const platform = await api.app.getPlatform()
  const fonts = await listSystemFonts(platform)
  fontList.value = [
    { id: '', label: '默认' },
    ...fonts.map((f) => ({ id: f, label: f.replace(/(^"|"$)/g, '') }))
  ]
})

function setAppFont(font: string): void {
  void store.update({ appearance: { appFont: font } })
}
</script>

<template>
  <dt id="basic">外观与界面</dt>
  <dd>
    <h3 id="basic_theme">主题外观 <span class="hint">右键自定义主题可编辑</span></h3>
    <div>
      <ThemePicker />
    </div>
  </dd>
  <dd class="glass-setting">
    <h3 id="basic_glass">玻璃材质</h3>
    <div class="glass-state">
      <span class="glass-state-dot" aria-hidden="true" />
      <strong>已应用</strong>
      <span>主窗口、设置卡片和播放条会根据当前主题使用分层玻璃表面。</span>
    </div>
    <div class="glass-preview" aria-label="玻璃材质预览">
      <div class="glass-sample glass-thin">
        <strong>薄</strong>
        <span>侧栏</span>
      </div>
      <div class="glass-sample glass-regular">
        <strong>中</strong>
        <span>主区</span>
      </div>
      <div class="glass-sample glass-thick">
        <strong>厚</strong>
        <span>浮层</span>
      </div>
    </div>
  </dd>
  <dd>
    <h3 id="basic_behavior">动画与窗口</h3>
    <div class="behavior-options">
      <BaseCheckbox
        id="setting_show_animation"
        :model-value="settings.behavior.showAnimation"
        label="显示动画效果"
        @update:model-value="store.update({ behavior: { showAnimation: $event as boolean } })"
      />
      <BaseCheckbox
        id="setting_random_animation"
        :disabled="!settings.behavior.showAnimation"
        :model-value="settings.behavior.randomAnimation"
        label="弹出层随机动画"
        @update:model-value="store.update({ behavior: { randomAnimation: $event as boolean } })"
      />
      <BaseCheckbox
        id="setting_start_in_fullscreen"
        :model-value="settings.behavior.startInFullscreen"
        label="以全屏模式启动"
        @update:model-value="store.update({ behavior: { startInFullscreen: $event as boolean } })"
      />
      <BaseCheckbox
        id="setting_close_to_tray"
        :model-value="settings.behavior.closeToTray"
        label="关闭窗口时不退出软件，将其最小化到系统托盘"
        @update:model-value="store.update({ behavior: { closeToTray: $event as boolean } })"
      />
    </div>
  </dd>
  <dd>
    <h3 id="basic_window_size">窗口尺寸</h3>
    <div>
      <BaseCheckbox
        v-for="item in WINDOW_SIZE_LIST"
        :id="`setting_window_size_${item.id}`"
        :key="item.id"
        class="gap-left"
        name="setting_window_size"
        need
        :model-value="settings.appearance.windowSizeId"
        :value="item.id"
        :label="item.name"
        @update:model-value="store.update({ appearance: { windowSizeId: $event as number } })"
      />
    </div>
  </dd>
  <dd>
    <h3 id="basic_font_size">字体大小</h3>
    <div>
      <BaseCheckbox
        v-for="item in FONT_SIZE_LIST"
        :id="`setting_font_size_${item.id}`"
        :key="item.id"
        class="gap-left"
        name="setting_font_size"
        need
        :model-value="settings.appearance.fontSize"
        :value="item.id"
        :label="item.name"
        @update:model-value="store.update({ appearance: { fontSize: $event as number } })"
      />
    </div>
  </dd>
  <dd>
    <h3 id="basic_comfort">界面亮度</h3>
    <div>
      <BaseCheckbox
        v-for="item in COMFORT_LEVELS"
        :id="`setting_comfort_${item.id}`"
        :key="item.id"
        class="gap-left"
        name="setting_comfort"
        need
        :model-value="settings.appearance.comfortLevel"
        :value="item.id"
        :label="item.name"
        @update:model-value="store.update({ appearance: { comfortLevel: $event as ComfortLevel } })"
      />
    </div>
    <p class="hint">压暗浅色主题的大面积亮面，不改变文字对比度。深色主题不受影响。</p>
  </dd>
  <dd>
    <h3 id="basic_font">软件字体</h3>
    <div>
      <BaseSelect
        :model-value="settings.appearance.appFont"
        :list="fontList"
        @update:model-value="setAppFont"
      />
    </div>
  </dd>
  <dd>
    <h3 id="basic_list">列表</h3>
    <div>
      <BaseCheckbox
        id="setting_list_show_operation_buttons"
        :model-value="settings.list.showOperationButtons"
        label="显示列表操作按钮"
        @update:model-value="store.update({ list: { showOperationButtons: $event as boolean } })"
      />
    </div>
  </dd>
</template>

<style scoped>
.behavior-options {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
}
.glass-setting {
  overflow: hidden;
}
.glass-state {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--color-font-label);
  font-size: 12px;
  line-height: 1.45;
}
.glass-state strong {
  color: var(--color-primary-font);
  font-weight: 650;
}
.glass-state-dot {
  width: 7px;
  height: 7px;
  flex: none;
  border-radius: 50%;
  background: var(--color-primary);
  box-shadow: 0 0 0 4px var(--color-primary-alpha-100);
}
.glass-preview {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  margin-top: 14px;
  padding: 9px;
  border: 1px solid var(--glass-border);
  border-radius: 10px;
  background: var(--color-primary-background);
}
.glass-sample {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 4px;
  min-height: 52px;
  padding: 9px 11px;
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  color: var(--color-font);
  box-shadow: none;
}
.glass-sample strong {
  font-size: 12px;
  font-weight: 650;
}
.glass-sample span {
  color: var(--color-font-label);
  font-size: 10px;
}
@media (max-width: 560px) {
  .glass-state {
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .glass-preview {
    grid-template-columns: 1fr;
  }
  .glass-sample {
    min-height: 42px;
  }
}
</style>
