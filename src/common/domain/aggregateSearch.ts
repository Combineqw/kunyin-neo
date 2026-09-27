export interface SourceSearchCallbacks<TSource, TResult> {
  resolved(source: TSource, result: TResult): void
  rejected(source: TSource, error: unknown): void
  settled(source: TSource): void
}

/** Runs independent provider searches and publishes each result as soon as it settles. */
export async function runSourceSearches<TSource, TResult>(
  sources: readonly TSource[],
  search: (source: TSource) => Promise<TResult>,
  isCurrent: () => boolean,
  callbacks: SourceSearchCallbacks<TSource, TResult>
): Promise<void> {
  await Promise.all(
    sources.map(async (source) => {
      try {
        const result = await search(source)
        if (isCurrent()) callbacks.resolved(source, result)
      } catch (error) {
        if (isCurrent()) callbacks.rejected(source, error)
      } finally {
        if (isCurrent()) callbacks.settled(source)
      }
    })
  )
}
