pub trait ThreadSafe: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> ThreadSafe for T {}

pub trait CloneableThreadSafe: Clone + ThreadSafe {}
impl<T: Clone + ThreadSafe> CloneableThreadSafe for T {}

/// A trait for types that can be used in a single-threaded context
pub trait LocalSafe: 'static {}
impl<T: 'static> LocalSafe for T {}

/// A trait for types that can be cloned and used in a single-threaded context
pub trait CloneableLocalSafe: Clone + LocalSafe {}
impl<T: Clone + LocalSafe> CloneableLocalSafe for T {}
