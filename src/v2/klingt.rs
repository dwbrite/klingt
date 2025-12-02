//! Klingt - High-level audio engine API
//!
//! Provides a simple interface for audio playback with automatic
//! sample rate conversion and scheduling.

use core::marker::PhantomData;

use hashbrown::HashMap;
use rtrb::RingBuffer;

use crate::v2::graph::AudioGraph;
use crate::v2::node::{AudioNode, NodeId};
use crate::v2::nodes::{ResamplingSource, RtrbSink};

#[cfg(feature = "cpal_sink")]
use crate::v2::device::CpalDevice;

/// Handle for sending messages to a node
pub struct Handle<M: Send + 'static> {
    pub(crate) node_id: NodeId,
    #[allow(dead_code)]
    pub(crate) graph_id: usize,
    pub(crate) sender: rtrb::Producer<M>,
    pub(crate) _marker: PhantomData<M>,
}

impl<M: Send + 'static> Handle<M> {
    /// Send a message to the node
    pub fn send(&mut self, msg: M) -> Result<(), M> {
        self.sender.push(msg).map_err(|rtrb::PushError::Full(m)| m)
    }
}

/// Internal tracking for sub-graphs that need resampling
struct SubGraph {
    graph: AudioGraph,
    /// Sample rate of this sub-graph
    #[allow(dead_code)]
    sample_rate: u32,
    /// Node ID of the RtrbSink in this sub-graph (terminal that feeds main graph)
    sink_node: NodeId,
    /// Node ID of the ResamplingSource in the main graph
    resampler_node: NodeId,
    /// How many blocks we've processed
    blocks_processed: u64,
}

/// The main Klingt audio engine
pub struct Klingt {
    /// Main output graph at device/output sample rate
    main_graph: AudioGraph,
    /// Output sample rate
    sample_rate: u32,
    /// Number of output channels
    channels: usize,
    
    /// Sub-graphs for nodes at different sample rates
    /// Key: the sample rate of the sub-graph
    sub_graphs: HashMap<u32, SubGraph>,
    
    /// The output sink node in main graph (e.g., CpalSink)
    sink_node: Option<NodeId>,
    
    /// Blocks processed on main graph (for scheduling)
    main_blocks_processed: u64,
}

impl Klingt {
    /// Create a new Klingt instance with explicit sample rate
    /// 
    /// Use `with_output()` to set the output sink, or add one manually.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            main_graph: AudioGraph::new(sample_rate),
            sample_rate,
            channels: 2,
            sub_graphs: HashMap::new(),
            sink_node: None,
            main_blocks_processed: 0,
        }
    }

    /// Create Klingt with the default audio output device
    #[cfg(feature = "cpal_sink")]
    pub fn default_output() -> Option<Self> {
        let device = CpalDevice::default_output()?;
        let sample_rate = device.sample_rate();
        let channels = device.channels() as usize;
        
        let mut klingt = Self {
            main_graph: AudioGraph::new(sample_rate),
            sample_rate,
            channels,
            sub_graphs: HashMap::new(),
            sink_node: None,
            main_blocks_processed: 0,
        };
        
        // Add the CPAL sink as the output
        let sink = device.create_sink();
        let handle = klingt.main_graph.add(sink);
        klingt.sink_node = Some(handle.id());
        klingt.main_graph.set_terminal(&handle);
        
        Some(klingt)
    }

    /// Set the number of output channels
    pub fn with_channels(mut self, channels: usize) -> Self {
        self.channels = channels;
        self
    }

    /// Add a custom output sink
    pub fn with_output<S: AudioNode<Message = ()>>(mut self, sink: S) -> Self {
        let handle = self.main_graph.add(sink);
        self.sink_node = Some(handle.id());
        self.main_graph.set_terminal(&handle);
        self
    }

    /// Get the output sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Add a node to the graph
    /// 
    /// If the node has a different sample rate than the output, it will
    /// automatically be placed in a sub-graph with resampling.
    pub fn add<N: AudioNode>(&mut self, node: N) -> Handle<N::Message> {
        let node_rate = node.native_sample_rate();
        
        if let Some(rate) = node_rate {
            if rate != self.sample_rate {
                // Node needs its own sub-graph with resampling
                return self.add_to_subgraph(node, rate);
            }
        }
        
        // Node matches output rate (or has no preference) - add to main graph
        let handle = self.main_graph.add(node);
        let node_id = handle.id();
        
        Handle {
            node_id,
            graph_id: 0,
            sender: handle.sender,
            _marker: PhantomData,
        }
    }

    /// Add a node to a sub-graph at a specific sample rate
    fn add_to_subgraph<N: AudioNode>(&mut self, node: N, rate: u32) -> Handle<N::Message> {
        let channels = node.num_outputs().max(self.channels);
        
        // Get or create sub-graph for this sample rate
        if !self.sub_graphs.contains_key(&rate) {
            self.create_subgraph(rate, channels);
        }
        
        let sub = self.sub_graphs.get_mut(&rate).unwrap();
        let handle = sub.graph.add(node);
        let node_id = handle.id();
        
        Handle {
            node_id,
            graph_id: rate as usize,
            sender: handle.sender,
            _marker: PhantomData,
        }
    }

    /// Create a new sub-graph with resampling bridge to main graph
    fn create_subgraph(&mut self, rate: u32, channels: usize) {
        // Ring buffer between sub-graph and main graph
        let buffer_size = ((rate as f32 * 0.1) as usize * channels).next_power_of_two().max(8192);
        let (producer, consumer) = RingBuffer::<f32>::new(buffer_size);
        
        // Create sub-graph
        let mut sub_graph = AudioGraph::new(rate);
        
        // Add RtrbSink to sub-graph (this is the terminal that feeds main graph)
        let sink = RtrbSink::new(producer, channels);
        let sink_handle = sub_graph.add(sink);
        let sink_node = sink_handle.id();
        sub_graph.set_terminal(&sink_handle);
        
        // Add resampling source to main graph
        let resampler = ResamplingSource::new(consumer, channels, rate);
        let resampler_handle = self.main_graph.add(resampler);
        let resampler_node = resampler_handle.id();
        
        self.sub_graphs.insert(rate, SubGraph {
            graph: sub_graph,
            sample_rate: rate,
            sink_node,
            resampler_node,
            blocks_processed: 0,
        });
    }

    /// Connect two nodes
    /// 
    /// If nodes are in different sub-graphs, this connects through the resampling bridge.
    pub fn connect<M1, M2>(&mut self, from: &Handle<M1>, to: &Handle<M2>)
    where
        M1: Send + 'static,
        M2: Send + 'static,
    {
        // graph_id: 0 = main graph, otherwise it's the sample rate of a sub-graph
        let from_graph_id = from.graph_id;
        let to_graph_id = to.graph_id;
        
        // Create internal dasp_graph handles
        let from_h = Self::make_handle::<M1>(from.node_id);
        let to_h = Self::make_handle::<M2>(to.node_id);
        
        match (from_graph_id, to_graph_id) {
            // Both in main graph
            (0, 0) => {
                self.main_graph.connect(&from_h, &to_h);
            }
            // Both in same sub-graph
            (r1, r2) if r1 == r2 && r1 != 0 => {
                let rate = r1 as u32;
                let sub = self.sub_graphs.get_mut(&rate).unwrap();
                sub.graph.connect(&from_h, &to_h);
            }
            // From sub-graph to main graph - connect through resampler bridge
            (rate_usize, 0) if rate_usize != 0 => {
                let rate = rate_usize as u32;
                let sub = self.sub_graphs.get_mut(&rate).unwrap();
                
                // Connect source node to the RtrbSink in sub-graph
                let sink_handle = Self::make_handle::<()>(sub.sink_node);
                sub.graph.connect(&from_h, &sink_handle);
                
                // Connect ResamplingSource to destination in main graph
                let resampler_handle = Self::make_handle::<()>(sub.resampler_node);
                self.main_graph.connect(&resampler_handle, &to_h);
            }
            // Other cases not yet supported
            _ => {
                panic!("Cannot connect nodes across different sub-graphs directly (from graph {} to graph {})", from_graph_id, to_graph_id);
            }
        }
    }

    /// Connect a node to the output sink
    /// 
    /// This connects the given node to the audio output (e.g., speakers).
    /// If the node is in a sub-graph (different sample rate), it will be
    /// routed through the resampler automatically.
    pub fn output<M: Send + 'static>(&mut self, handle: &Handle<M>) {
        let sink_id = self.sink_node.expect("No output sink configured. Use default_output() or with_output().");
        
        if handle.graph_id == 0 {
            // Node is in main graph - connect directly to sink
            let from_h = Self::make_handle::<M>(handle.node_id);
            let to_h = Self::make_handle::<()>(sink_id);
            self.main_graph.connect(&from_h, &to_h);
        } else {
            // Node is in a sub-graph - connect through resampler bridge
            let rate = handle.graph_id as u32;
            let sub = self.sub_graphs.get_mut(&rate)
                .expect("Sub-graph not found for handle's graph_id");
            
            // Connect node to RtrbSink in sub-graph
            let from_h = Self::make_handle::<M>(handle.node_id);
            let sink_handle = Self::make_handle::<()>(sub.sink_node);
            sub.graph.connect(&from_h, &sink_handle);
            
            // Connect ResamplingSource to output sink in main graph
            let resampler_handle = Self::make_handle::<()>(sub.resampler_node);
            let to_h = Self::make_handle::<()>(sink_id);
            self.main_graph.connect(&resampler_handle, &to_h);
        }
    }

    /// Process one block of audio
    /// 
    /// Internally handles scheduling sub-graphs to keep resamplers fed.
    pub fn process(&mut self) {
        // First, process sub-graphs enough to feed their resamplers
        let main_rate = self.sample_rate as f64;
        let main_blocks = self.main_blocks_processed + 1;
        
        for (rate, sub) in self.sub_graphs.iter_mut() {
            let rate_ratio = *rate as f64 / main_rate;
            // How many sub-graph blocks needed to feed main_blocks of output
            let blocks_needed = ((main_blocks as f64) * rate_ratio).ceil() as u64 + 4;
            
            while sub.blocks_processed < blocks_needed {
                sub.graph.process();
                sub.blocks_processed += 1;
            }
        }
        
        // Then process main graph
        self.main_graph.process();
        self.main_blocks_processed += 1;
    }

    // Helper to create internal handle (static - no borrow needed)
    fn make_handle<M: Send + 'static>(node_id: NodeId) -> crate::v2::graph::NodeHandle<M> {
        crate::v2::graph::NodeHandle {
            id: node_id,
            sender: rtrb::RingBuffer::new(1).0, // dummy, not used for connect
            _marker: PhantomData,
        }
    }
}
