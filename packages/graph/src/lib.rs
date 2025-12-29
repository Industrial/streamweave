#![doc = include_str!("../README.md")]

pub mod connection;
pub mod execution;
#[allow(clippy::module_inception)]
pub mod graph;
pub mod node;
pub mod port;
pub mod router;
pub mod routers;
pub mod serialization;
pub mod stateful;
pub mod subgraph;
pub mod traits;
pub mod windowing;

pub use connection::{CompatibleWith, Connection, ConnectionError, HasInputPort, HasOutputPort};
pub use execution::{ExecutionError, ExecutionState, GraphExecution, GraphExecutor};
pub use graph::{
  AppendNode, ConnectionInfo, ContainsNodeType, Graph, GraphBuilder, GraphError, HasConnections,
  HasNodes, RuntimeGraphBuilder,
};
pub use node::{ConsumerNode, ProducerNode, TransformerNode};
pub use port::*;
pub use router::{InputRouter, OutputRouter, RouterError};
pub use routers::*;
pub use serialization::{SerializationError, deserialize, serialize};
pub use stateful::*;
pub use subgraph::*;
pub use traits::*;
pub use windowing::*;
