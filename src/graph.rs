//! Audio graph - owns nodes and message queues

use alloc::boxed::Box;
use core::marker::PhantomData;

use dasp_graph::{Buffer, Input, NodeData, Processor};
use hashbrown::HashMap;
use petgraph::graph::NodeIndex;
use rtrb::{Consumer, Producer, RingBuffer};

use crate::node::{AudioNode, NodeId, ProcessContext};

/// Internal handle to send messages to a node in an AudioGraph
pub(crate) struct NodeHandle<M: Send + 'static> {
    pub(crate) id: NodeId,
    pub(crate) sender: Producer<M>,
    pub(crate) _marker: PhantomData<M>,
}

impl<M: Send + 'static> NodeHandle<M> {
    /// Send a message to the node (applied next process cycle)
    /// 
    /// Returns Err if the queue is full (message dropped)
    #[allow(dead_code)]
    pub fn send(&mut self, msg: M) -> Result<(), M> {
        self.sender.push(msg).map_err(|rtrb::PushError::Full(v)| v)
    }
    
    pub fn id(&self) -> NodeId {
        self.id
    }
}

// Type-erased wrapper so we can store heterogeneous nodes
trait ErasedNode: Send {
    fn process_erased(&mut self, ctx: &ProcessContext, inputs: &[Input], outputs: &mut [Buffer]);
}

struct NodeWrapper<N: AudioNode> {
    node: N,
    receiver: Consumer<N::Message>,
}

impl<N: AudioNode> ErasedNode for NodeWrapper<N> {
    fn process_erased(&mut self, ctx: &ProcessContext, inputs: &[Input], outputs: &mut [Buffer]) {
        // Split borrow to avoid conflict between receiver and node
        let receiver = &mut self.receiver;
        let node = &mut self.node;
        
        // Create a draining iterator directly from the consumer - no allocation!
        let messages = core::iter::from_fn(|| receiver.pop().ok());
        node.process(ctx, messages, inputs, outputs);
    }
}

// Adapter for dasp_graph
struct DaspAdapter {
    node: Box<dyn ErasedNode>,
    ctx: ProcessContext,
}

impl dasp_graph::Node for DaspAdapter {
    fn process(&mut self, inputs: &[Input], outputs: &mut [Buffer]) {
        self.node.process_erased(&self.ctx, inputs, outputs);
    }
}

type InnerGraph = petgraph::graph::Graph<NodeData<DaspAdapter>, ()>;

/// An audio processing graph at a fixed sample rate
pub(crate) struct AudioGraph {
    graph: InnerGraph,
    processor: Processor<InnerGraph>,
    ctx: ProcessContext,
    
    node_indices: HashMap<NodeId, NodeIndex>,
    next_node_id: u32,
    
    terminal: Option<NodeIndex>,
}

impl AudioGraph {
    /// Create a new graph with the given sample rate
    pub fn new(sample_rate: u32) -> Self {
        Self {
            graph: InnerGraph::with_capacity(64, 64),
            processor: Processor::with_capacity(64),
            ctx: ProcessContext {
                sample_rate,
                buffer_size: 64, // dasp_graph default
            },
            node_indices: HashMap::new(),
            next_node_id: 0,
            terminal: None,
        }
    }
    
    #[allow(dead_code)]
    pub fn sample_rate(&self) -> u32 {
        self.ctx.sample_rate
    }

    /// Add a node, returns a handle for sending messages
    pub fn add<N: AudioNode>(&mut self, node: N) -> NodeHandle<N::Message> {
        self.add_with_queue_size(node, 64)
    }
    
    /// Add a node with a custom message queue size
    pub fn add_with_queue_size<N: AudioNode>(&mut self, node: N, queue_size: usize) -> NodeHandle<N::Message> {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        
        let (producer, consumer) = RingBuffer::new(queue_size);
        
        let num_outputs = node.num_outputs();
        let wrapper = NodeWrapper { node, receiver: consumer };
        let adapter = DaspAdapter {
            node: Box::new(wrapper),
            ctx: self.ctx,
        };
        
        let node_data = match num_outputs {
            1 => NodeData::new1(adapter),
            2 => NodeData::new2(adapter),
            // 0 outputs = sink, but dasp_graph still needs a buffer for inputs
            _ => NodeData::new1(adapter),
        };
        
        let idx = self.graph.add_node(node_data);
        self.node_indices.insert(id, idx);
        
        NodeHandle {
            id,
            sender: producer,
            _marker: PhantomData,
        }
    }

    /// Connect output of `from` to input of `to`
    pub fn connect<M1, M2>(&mut self, from: &NodeHandle<M1>, to: &NodeHandle<M2>) 
    where
        M1: Send + 'static,
        M2: Send + 'static,
    {
        let from_idx = self.node_indices[&from.id];
        let to_idx = self.node_indices[&to.id];
        self.graph.add_edge(from_idx, to_idx, ());
    }
    
    /// Set which node to process to (typically a sink)
    pub fn set_terminal<M: Send + 'static>(&mut self, handle: &NodeHandle<M>) {
        self.terminal = Some(self.node_indices[&handle.id]);
    }
    
    /// Process one block of audio through the graph
    pub fn process(&mut self) {
        if let Some(terminal) = self.terminal {
            self.processor.process(&mut self.graph, terminal);
        }
    }
}
