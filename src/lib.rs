#[macro_use]
extern crate serde_derive;
extern crate serde_xml_rs;
extern crate serde;
extern crate regex;
#[macro_use]
extern crate lazy_static;
mod svgxml;
pub use svgxml::{SVG, Transform, HrefAndClipMask, Polygon, F64Point, ClipPath, g};
pub use svgxml::{itransform, ftransform, compose, poly_edge_intersect};
