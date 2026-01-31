use crate::types::{BotError, BotResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use topgg::{Client as TopGgClient, Stats};
use tracing::{error, info};
use twilight_cache_inmemory::InMemoryCache;

pub struct TopGgPoster {
    client: TopGgClient,
}

impl TopGgPoster {
    pub fn new(token: String) -> BotResult<Self> {
        let client = TopGgClient::new(token);
        Ok(Self { client })
    }

    pub async fn start_auto_posting(
        &self,
        cache: Arc<InMemoryCache>,
        shard_count: u32,
    ) -> BotResult<()> {
        info!("[Top.gg] Starting auto poster with {} shards", shard_count);

        tokio::time::sleep(Duration::from_secs(5)).await;
        if let Err(e) = self.post_stats(&cache, shard_count).await {
            error!("[Top.gg] Failed to post initial stats: {}", e);
        }

        let mut ticker = interval(Duration::from_secs(30 * 60));
        loop {
            ticker.tick().await;
            if let Err(e) = self.post_stats(&cache, shard_count).await {
                error!("[Top.gg] Failed to post stats: {}", e);
            }
        }
    }

    async fn post_stats(&self, cache: &InMemoryCache, shard_count: u32) -> BotResult<()> {
        let stats = cache.stats();
        let guild_count = stats.guilds();

        let payload = Stats::from_count(guild_count, Some(shard_count as usize));

        let result: Result<(), _> = self
            .client
            .post_stats(payload)
            .await
            .map_err(|e| BotError::Other(format!("[Top.gg] {}", e)));
        result?;

        info!(
            "[Top.gg] Stats posted | Servers: {} | Shards: {}",
            guild_count, shard_count
        );

        Ok(())
    }
}
