use std::sync::Arc;

/// The context for executing a WebResource job. Defines how downloading and saving files work, and
/// thus allows mocking.
pub trait World: Send + Sync + 'static {
    type MainWorld: crate::world::World;

    fn new(main: Arc<Self::MainWorld>) -> Self;

    fn main(&self) -> &Arc<Self::MainWorld>;
}

/// The default context, accessing the real web and filesystem.
#[derive(Clone)]
pub struct DefaultWorld {
    main: Arc<crate::world::DefaultWorld>,
}

impl World for DefaultWorld {
    type MainWorld = crate::world::DefaultWorld;

    fn new(main: Arc<Self::MainWorld>) -> Self {
        Self { main }
    }

    fn main(&self) -> &Arc<Self::MainWorld> {
        &self.main
    }
}
