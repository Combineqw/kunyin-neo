export const SEASONAL_AURORA_AUTO_ID = 'aurora_seasonal_auto'

export type AuroraSeason = 'spring' | 'summer' | 'autumn' | 'winter'

/** Meteorological month groups keep the seasonal switch predictable at boundaries. */
export function getAuroraSeason(date = new Date()): AuroraSeason {
  const month = date.getMonth() + 1
  if (month >= 3 && month <= 5) return 'spring'
  if (month >= 6 && month <= 8) return 'summer'
  if (month >= 9 && month <= 11) return 'autumn'
  return 'winter'
}

export function getSeasonalAuroraThemeId(date = new Date()): string {
  return `aurora_${getAuroraSeason(date)}`
}
