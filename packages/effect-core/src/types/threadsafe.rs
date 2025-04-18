pub trait ThreadSafe: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> ThreadSafe for T {}

pub trait CloneableThreadSafe: Clone + ThreadSafe {}
impl<T: Clone + ThreadSafe> CloneableThreadSafe for T {}
