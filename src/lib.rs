#[macro_use]
extern crate serde_derive;
extern crate serde_xml_rs;
extern crate serde;
extern crate regex;
mod polygonsvg;
mod svgxml;
pub use svgxml::{Color, SVG, Transform, HrefAndClipMask, Polygon, F64Point, ClipPath, g};
pub use svgxml::{itransform, ftransform, poly_edge_intersect, compose};

