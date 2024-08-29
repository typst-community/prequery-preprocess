use std::future::Future;

use tokio::task::{JoinError, JoinSet};

pub async fn spawn_set<I, F, E>(futures: I) -> Vec<E>
where
    I: Iterator<Item = F>,
    F: Future<Output = Result<(), E>> + Send + 'static,
    E: From<JoinError> + Send + 'static,
{
    let mut set = JoinSet::new();
    for future in futures {
        set.spawn(future);
    }

    let mut errors = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Err(error) => errors.push(error.into()),
            Ok(Err(error)) => errors.push(error),
            Ok(Ok(())) => {}
        }
    }
    errors
}
