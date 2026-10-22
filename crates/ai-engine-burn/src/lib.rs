//! SAM 2.1 on the Burn engine for Craft AI: an optional runner for the `image.segment` capability
//! that can use a graphics card, from weights prepared once on this computer.
//!
//! This is the foundation: the compiled model architecture is generated (see
//! `tools/burn/generate_sam21.py`), and the runner that uses it follows.
#![forbid(unsafe_code)]
