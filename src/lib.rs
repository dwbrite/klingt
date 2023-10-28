pub mod nodes;
pub use dasp_graph::Node as AudioNode;

// TODO: docs, buffered sources, seeking (on seekable sources?), {mono/stereo/3d audio, hrtf, doppler}

// First thing's first, we want to play audio. How and when?
// We have some API surface for loading or streaming chunks of some audio file, in addition to precomputing(?) generated sources.
// For a given source, we have a stateful Instance. The stream API will have to effect a single instance... Hm.
// It MIGHT make sense to create a separate project for priority with rayon



