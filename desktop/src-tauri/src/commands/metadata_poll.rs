use std::{future::Future, time::Duration};

/// Short, bounded read-after-write schedule for relay metadata.
///
/// Relay acceptance and read-index visibility are separate events. Most
/// metadata is visible on the first read; later attempts cover ordinary index
/// lag without turning a successful write into an unbounded UI spinner.
const METADATA_BACKOFF_MS: [u64; 5] = [0, 100, 200, 500, 700];

pub(crate) async fn poll_metadata<F, Fut, T, E>(mut fetch: F) -> Result<Option<T>, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<T>, E>>,
{
    for delay_ms in METADATA_BACKOFF_MS {
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        if let Some(value) = fetch().await? {
            return Ok(Some(value));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use super::poll_metadata;

    #[tokio::test(start_paused = true)]
    async fn returns_first_visible_value_and_stops_polling() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&attempts);

        let result = poll_metadata(move || {
            let attempt = seen.fetch_add(1, Ordering::SeqCst) + 1;
            async move { Ok::<_, ()>((attempt == 3).then_some("metadata")) }
        })
        .await;

        assert_eq!(result, Ok(Some("metadata")));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_after_the_bounded_schedule() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&attempts);

        let result = poll_metadata(move || {
            seen.fetch_add(1, Ordering::SeqCst);
            async { Ok::<Option<()>, ()>(None) }
        })
        .await;

        assert_eq!(result, Ok(None));
        assert_eq!(attempts.load(Ordering::SeqCst), 5);
    }

    #[tokio::test(start_paused = true)]
    async fn returns_fetch_errors_without_retrying() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&attempts);

        let result = poll_metadata(move || {
            seen.fetch_add(1, Ordering::SeqCst);
            async { Err::<Option<()>, _>("relay unavailable") }
        })
        .await;

        assert_eq!(result, Err("relay unavailable"));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}
