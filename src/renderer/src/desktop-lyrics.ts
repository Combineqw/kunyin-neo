/** 独立桌面歌词渲染进程入口：只挂载窗口专用视图。 */
import { createApp } from 'vue'
import DesktopLyrics from './desktop-lyrics/DesktopLyrics.vue'

createApp(DesktopLyrics).mount('#app')
