use std::collections::HashMap;
use std::future::Future;

use tokio::task::{JoinError, JoinSet};

pub async fn spawn_set<I, F, E>(futures: I) -> Vec<E>
where
    I: Iterator<Item = F>,
    F: Future<Output = Result<(), E>> + Send + 'static,
    E: From<JoinError> + Send + 'static,
{
    spawn_set_with_id(futures.map(|f| ((), f)), |_, error| E::from(error)).await
}

pub async fn spawn_set_with_id<I, Id, F, E>(futures: I, to_error: fn(Id, JoinError) -> E) -> Vec<E>
where
    I: Iterator<Item = (Id, F)>,
    F: Future<Output = Result<(), E>> + Send + 'static,
    E: Send + 'static,
{
    let mut set = JoinSet::new();
    let mut ids = HashMap::new();
    for (id, future) in futures {
        let handle = set.spawn(future);
        ids.insert(handle.id(), id);
    }

    let mut errors = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Err(error) => {
                let id = ids.remove(&error.id()).expect("id was previously inserted");
                errors.push(to_error(id, error));
            }
            Ok(Err(error)) => errors.push(error),
            Ok(Ok(())) => {}
        }
    }
    errors
}
