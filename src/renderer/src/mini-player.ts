/** 独立迷你播放器渲染进程入口：只挂载窗口专用视图。 */
import { createApp } from 'vue'
import MiniPlayer from './mini-player/MiniPlayer.vue'

createApp(MiniPlayer).mount('#app')
