<script setup lang="ts">
// 移植自 lx-music-desktop（Apache-2.0, © lyswhut）components/base/Btn.vue。
withDefaults(defineProps<{ min?: boolean; outline?: boolean; disabled?: boolean }>(), {
  min: false,
  outline: false,
  disabled: false
})
</script>

<template>
  <button class="btn pressable" :class="{ min, outline }" tabindex="0" :disabled="disabled">
    <slot />
  </button>
</template>

<style scoped>
.btn {
  display: inline-block;
  border: none;
  min-height: 34px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  cursor: pointer;
  padding: 8px 15px;
  color: var(--color-button-font);
  outline: none;
  /* transform/filter 过渡由全局 .pressable 统一负责（含按下与弹回两态），
     这里只保留非形变属性的过渡，避免与 .pressable 争夺 transform。 */
  transition:
    background-color var(--anim-dur-fast) var(--anim-ease-standard),
    box-shadow var(--anim-dur-fast) var(--anim-ease-standard);
  background-color: color-mix(in srgb, var(--color-main-background) 82%, var(--color-primary) 18%);
  font-size: 12px;
}
.btn:disabled {
  opacity: 0.4;
  cursor: default;
}
.btn.outline {
  background-color: transparent;
}
/* hover 上浮改为喂给 .pressable 的 --press-lift：按下时位移与缩放复合，
   不会因为 :active 重写 transform 而把上浮"抹平"。 */
.btn:hover:not(:disabled) {
  --press-lift: -1px;
  background-color: var(--color-button-background-hover);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.06);
}
.btn:active {
  background-color: var(--color-button-background-active);
}
.min {
  min-height: 30px;
  padding: 4px 10px;
  font-size: 12px;
}
</style>
