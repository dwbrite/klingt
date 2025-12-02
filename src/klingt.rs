//! High-level audio engine API

use core::marker::PhantomData;

use hashbrown::HashMap;
use rtrb::RingBuffer;

use crate::graph::AudioGraph;
use crate::node::{AudioNode, NodeId};
use crate::nodes::{ResamplingSource, RtrbSink};

#[cfg(feature = "cpal_sink")]
use crate::device::CpalDevice;

/// A handle for sending messages to a node in the audio graph.
///
/// Handles are returned when you add a node to [`Klingt`] and provide two capabilities:
/// 1. **Connections** - Pass handles to [`Klingt::connect`] or [`Klingt::output`]
/// 2. **Messages** - Send parameter updates via [`Handle::send`]
///
/// # Example
///
/// ```no_run
/// # use klingt::{Klingt, nodes::{Sine, SineMessage}};
/// # let mut klingt = Klingt::default_output().unwrap();
/// let mut sine = klingt.add(Sine::new(440.0));
///
/// // Change frequency (processed next audio block)
/// sine.send(SineMessage::SetFrequency(880.0)).ok();
/// ```
///
/// # Message Delivery
///
/// Messages are buffered in a lock-free ring buffer and processed at the start
/// of each audio block. If the buffer is full, [`Handle::send`] returns `Err(msg)`
/// with the message that couldn't be sent.
pub struct Handle<M: Send + 'static> {
    pub(crate) node_id: NodeId,
    #[allow(dead_code)]
    pub(crate) graph_id: usize,
    pub(crate) sender: rtrb::Producer<M>,
    pub(crate) _marker: PhantomData<M>,
}

impl<M: Send + 'static> Handle<M> {
    /// Send a message to the node.
    ///
    /// The message will be processed at the start of the next audio block.
    /// This is lock-free and safe to call from any thread.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the message was queued successfully
    /// - `Err(msg)` if the queue is full (message dropped)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::{Klingt, nodes::{Sine, SineMessage}};
    /// # let mut klingt = Klingt::default_output().unwrap();
    /// let mut sine = klingt.add(Sine::new(440.0));
    ///
    /// // Fire-and-forget style (ignore if queue full)
    /// sine.send(SineMessage::SetFrequency(880.0)).ok();
    ///
    /// // Or handle the error
    /// if sine.send(SineMessage::SetAmplitude(0.5)).is_err() {
    ///     eprintln!("Message queue full!");
    /// }
    /// ```
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

/// The main audio engine - manages nodes, connections, and audio processing.
///
/// `Klingt` provides a high-level API for building audio graphs. It handles:
/// - Adding nodes and connecting them together
/// - Automatic sample rate conversion via sub-graphs
/// - Scheduling and processing audio blocks
///
/// # Creating an Instance
///
/// The easiest way is [`Klingt::default_output`] which uses the system's default audio device:
///
/// ```no_run
/// # use klingt::Klingt;
/// let mut klingt = Klingt::default_output().expect("No audio device found");
/// ```
///
/// For more control, use [`Klingt::new`] with a specific sample rate and add your own output:
///
/// ```no_run
/// # use klingt::{Klingt, CpalDevice};
/// let device = CpalDevice::list_outputs().into_iter().next().unwrap();
/// let mut klingt = Klingt::new(device.sample_rate())
///     .with_output(device.create_sink());
/// ```
///
/// # Building the Graph
///
/// 1. Add nodes with [`add`](Self::add) - returns a [`Handle`] for connections and messages
/// 2. Connect nodes with [`connect`](Self::connect)
/// 3. Connect final node(s) to output with [`output`](Self::output)
///
/// ```no_run
/// # use klingt::{Klingt, nodes::{Sine, Gain, Mixer}};
/// # let mut klingt = Klingt::default_output().unwrap();
/// // Create nodes
/// let sine1 = klingt.add(Sine::new(440.0));
/// let sine2 = klingt.add(Sine::new(880.0));
/// let mixer = klingt.add(Mixer::stereo());
/// let gain = klingt.add(Gain::new(0.5));
///
/// // Build the graph
/// klingt.connect(&sine1, &mixer);
/// klingt.connect(&sine2, &mixer);
/// klingt.connect(&mixer, &gain);
/// klingt.output(&gain);
/// ```
///
/// # Processing Audio
///
/// Call [`process`](Self::process) repeatedly to generate audio. This is typically done
/// in a loop, paced to match real-time:
///
/// ```no_run
/// # use klingt::Klingt;
/// # let mut klingt = Klingt::default_output().unwrap();
/// use std::time::{Duration, Instant};
///
/// let start = Instant::now();
/// let rate = klingt.sample_rate() as f64;
/// let mut blocks = 0u64;
///
/// loop {
///     // Calculate how many blocks should have been processed by now
///     let target = (start.elapsed().as_secs_f64() * rate / 64.0) as u64 + 4;
///     
///     while blocks < target {
///         klingt.process();
///         blocks += 1;
///     }
///     
///     std::thread::sleep(Duration::from_micros(500));
/// }
/// ```
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
    /// Create a new Klingt instance with an explicit sample rate.
    /// 
    /// This creates an engine without an output sink. Use [`with_output`](Self::with_output)
    /// to add one, or add a sink node manually.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::{Klingt, CpalDevice};
    /// let device = CpalDevice::default_output().unwrap();
    /// let mut klingt = Klingt::new(device.sample_rate())
    ///     .with_output(device.create_sink());
    /// ```
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

    /// Create Klingt with the system's default audio output device.
    ///
    /// This is the easiest way to get started. Returns `None` if no audio device is available.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::Klingt;
    /// let mut klingt = Klingt::default_output().expect("No audio device");
    /// ```
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

    /// Set the number of output channels (builder pattern).
    ///
    /// Default is 2 (stereo). This affects sub-graph creation for sample rate conversion.
    pub fn with_channels(mut self, channels: usize) -> Self {
        self.channels = channels;
        self
    }

    /// Add a custom output sink (builder pattern).
    ///
    /// Use this when you need control over which audio device to use,
    /// or to use a custom sink implementation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::{Klingt, CpalDevice};
    /// // Select a specific device
    /// let devices = CpalDevice::list_outputs();
    /// let device = &devices[0]; // Pick the first one
    ///
    /// let mut klingt = Klingt::new(device.sample_rate())
    ///     .with_output(device.create_sink());
    /// ```
    pub fn with_output<S: AudioNode<Message = ()>>(mut self, sink: S) -> Self {
        let handle = self.main_graph.add(sink);
        self.sink_node = Some(handle.id());
        self.main_graph.set_terminal(&handle);
        self
    }

    /// Get the output sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Add a node to the audio graph.
    ///
    /// Returns a [`Handle`] for connecting the node and sending messages to it.
    ///
    /// # Automatic Sample Rate Conversion
    ///
    /// If the node reports a [`native_sample_rate`](AudioNode::native_sample_rate)
    /// different from the output, Klingt automatically:
    /// 1. Creates a sub-graph at the node's native rate
    /// 2. Adds a resampler to bridge to the main graph
    ///
    /// This means you can add audio files at their native sample rate
    /// without manual conversion.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::{Klingt, nodes::Sine};
    /// # let mut klingt = Klingt::default_output().unwrap();
    /// let sine = klingt.add(Sine::new(440.0));
    /// klingt.output(&sine);
    /// ```
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

    /// Connect two nodes together.
    ///
    /// Audio flows from `from` to `to`. You can connect multiple sources to one
    /// destination (they'll be summed if the destination accepts multiple inputs,
    /// like [`Mixer`](crate::nodes::Mixer)).
    ///
    /// # Cross-Sample-Rate Connections
    ///
    /// If nodes are in different sub-graphs (different sample rates), the connection
    /// automatically routes through the resampling bridge. However, connecting
    /// *into* a sub-graph from the main graph is not supported.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::{Klingt, nodes::{Sine, Gain}};
    /// # let mut klingt = Klingt::default_output().unwrap();
    /// let sine = klingt.add(Sine::new(440.0));
    /// let gain = klingt.add(Gain::new(0.5));
    ///
    /// klingt.connect(&sine, &gain);
    /// klingt.output(&gain);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if attempting to connect across sub-graphs in an unsupported direction.
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

    /// Connect a node directly to the audio output.
    ///
    /// This is a convenience method equivalent to connecting to whatever sink
    /// was configured via [`default_output`](Self::default_output) or
    /// [`with_output`](Self::with_output).
    ///
    /// If the node is in a sub-graph (different sample rate), it will be
    /// automatically routed through the resampler.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::{Klingt, nodes::Sine};
    /// # let mut klingt = Klingt::default_output().unwrap();
    /// let sine = klingt.add(Sine::new(440.0));
    /// klingt.output(&sine); // Connect directly to speakers
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if no output sink is configured.
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

    /// Process one block of audio (64 samples).
    ///
    /// Call this repeatedly in your main loop to generate audio. The method:
    /// 1. Processes any sub-graphs to keep resamplers fed
    /// 2. Processes the main graph to generate output
    ///
    /// # Timing
    ///
    /// You're responsible for calling this at the right rate. A typical pattern:
    ///
    /// ```no_run
    /// # use klingt::Klingt;
    /// # let mut klingt = Klingt::default_output().unwrap();
    /// use std::time::{Duration, Instant};
    ///
    /// let start = Instant::now();
    /// let rate = klingt.sample_rate() as f64;
    /// let mut blocks = 0u64;
    ///
    /// loop {
    ///     // Stay a few blocks ahead to prevent underruns
    ///     let target = (start.elapsed().as_secs_f64() * rate / 64.0) as u64 + 4;
    ///     
    ///     while blocks < target {
    ///         klingt.process();
    ///         blocks += 1;
    ///     }
    ///     
    ///     std::thread::sleep(Duration::from_micros(500));
    /// }
    /// ```
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
    fn make_handle<M: Send + 'static>(node_id: NodeId) -> crate::graph::NodeHandle<M> {
        crate::graph::NodeHandle {
            id: node_id,
            sender: rtrb::RingBuffer::new(1).0, // dummy, not used for connect
            _marker: PhantomData,
        }
    }
}
