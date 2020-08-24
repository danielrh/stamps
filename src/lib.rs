#[macro_use]
extern crate serde_derive;
extern crate serde_xml_rs;
extern crate serde;
extern crate regex;
mod svgxml;
pub use svgxml::{Color, SVG, Transform, HrefAndClipMask, Polygon, F64Point, ClipPath, g};
pub use svgxml::{itransform, ftransform, compose, poly_edge_intersect};

