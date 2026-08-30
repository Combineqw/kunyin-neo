import { IpcChannels, type PlayEventPayload, type RecommendationSong } from '@common'
import { handle } from '../helpers'
import { recordPlayEvent } from '../../store/playlog'
import { queryRecommendationSongs } from '../../store/recommendation'
import { getDailyRecommendations } from '../../store/daily-recommendation'

export function registerRecommendationHandlers(): void {
  handle(IpcChannels.PLAYLOG_RECORD, (payload: PlayEventPayload): void => recordPlayEvent(payload))
  handle(IpcChannels.RECOMMENDATION_SONGS, (limit?: number): RecommendationSong[] =>
    queryRecommendationSongs(limit)
  )
  handle(IpcChannels.RECOMMENDATION_DAILY, (force?: boolean) => getDailyRecommendations(!!force))
}
