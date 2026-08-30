/**
 * 十段图形均衡器的共享常量。
 * 领域层只定义频段与合法增益范围；节点创建、参数调度由渲染层 audio 模块负责。
 */
export const EQ_FREQUENCIES = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000] as const
export const EQ_GAIN_MIN = -12
export const EQ_GAIN_MAX = 12
