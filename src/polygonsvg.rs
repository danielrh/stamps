// this file deserializes an SVG into a polygon (ignoring a lot of the complicated features of SVG like masks)
use std::path::Path;
use std::collections::HashMap;
use std::vec::Vec;
use super::serde_xml_rs::from_str;
use super::serde_xml_rs;
use std::io::Read;
use std::fmt::Write;
use serde::{Deserialize, Deserializer};
use serde;
use regex::Regex;
use std::convert::TryFrom;
pub type F64Point = (f64, f64);

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Transform {
    pub midx: f64,
    pub midy: f64,
    pub rotate: f64,
    pub tx: f64,
    pub ty: f64,
    pub scale: f64,
}
impl Transform {
  pub fn new(width: u32, height: u32) -> Transform {
      Transform{
          scale:1.0,
          midx:width as f64/2.0,
          midy:height as f64/2.0,
          rotate:0.0,
          tx:0.0,
          ty:0.0,
      }
  }
  pub fn to_bbox(&self) -> [(f64,f64);4] {
      [ftransform(self, (0.,0.)),
       ftransform(self, (0., self.midy * 2.)),
       ftransform(self, (self.midx * 2., self.midy * 2.)),
       ftransform(self, (self.midx * 2., 0.)),
       ]
  }
  pub fn to_string(&self) -> Result<String, serde_xml_rs::Error> {
    let mut components = [String::new(),String::new(),String::new(),String::new(),String::new()];
    let mut num_components = 0usize;
    if self.scale != 1.0 {
      components[num_components] = format!("scale({})", self.scale);
      num_components += 1;
    }
    if self.tx != 0.0 || self.ty != 0.0 {
      components[num_components] = format!("translate({}, {})", self.tx, self.ty);
      num_components += 1;      
    }
    if self.midx != 0.0 || self.midy != 0.0 {
      components[num_components] = format!("translate({}, {})", self.midy, self.midy);
      num_components += 1;      
    }
    if self.rotate != 0.0 {
      components[num_components] = format!("rotate({})", self.rotate);
      num_components += 1;
    }
    if self.midx != 0.0 || self.midy != 0.0 {
      components[num_components] = format!("translate({}, {})", -self.midx, -self.midy);
      num_components += 1;      
    }
    return Ok(components[..num_components].join(" "))
  }
}


pub fn ftransform(t:&Transform, p: F64Point) -> F64Point {
    let centered = (p.0 - t.midx, p.1 - t.midy);
    let rotate_rad = -t.rotate * std::f64::consts::PI/180.;
    let rotated = (centered.0 * rotate_rad.cos() + centered.1 * rotate_rad.sin(),
                   -centered.0 * rotate_rad.sin() + centered.1 * rotate_rad.cos());
    let scaled = (rotated.0 * t.scale, rotated.1 * t.scale);
    let recentered = (scaled.0 + t.midx, scaled.1 + t.midy);
    (recentered.0 + t.tx, recentered.1 + t.ty)
}

pub fn itransform(t:&Transform, p: F64Point) -> F64Point {
    let untranslated = (p.0 - t.tx, p.1 - t.ty);
    let recentered = (untranslated.0 - t.midx, untranslated.1 - t.midy);
    let unscaled = (recentered.0/t.scale, recentered.1/t.scale);
    let rotate_rad = t.rotate * std::f64::consts::PI/180.;
    let rotated = (unscaled.0 * rotate_rad.cos() + unscaled.1 * rotate_rad.sin(),
                   -unscaled.0 * rotate_rad.sin() + unscaled.1 * rotate_rad.cos());
    let centered = (rotated.0 + t.midx, rotated.1 + t.midy);
    centered
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct defs {
}

#[derive(Debug, Serialize, Deserialize, PartialEq,Default)]
struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height:f64,
    #[serde(default)]
    pub fill: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq,Default)]
struct Ellipse {
    pub cx: f64,
    pub cy: f64,
    pub rx: f64,
    pub ry: f64,
    #[serde(default)]
    pub fill: String,
}
impl From<Circle> for Ellipse {
    fn from(c :Circle) -> Ellipse {
        Ellipse{
            cx:c.cx,
            cy:c.cy,
            rx:c.r,
            ry:c.r,
            fill:c.fill,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq,Default)]
struct Circle {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    #[serde(default)]
    pub fill: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq,Default)]
struct Polygon {
    #[serde(deserialize_with="point_deserializer")]
    pub points: Vec<F64Point>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq,Default)]
struct GTransform {
    #[serde(default)]
    pub polygon: Vec<Polygon>,
    #[serde(default)]
    pub rect: Vec<Rect>,
    #[serde(default)]
    pub ellipse: Vec<Ellipse>,
    #[serde(default)]
    pub circle: Vec<Circle>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PolygonSVG {
    #[serde(default)]
    pub defs: defs,
    #[serde(default)]
    #[serde(rename="g")]
    pub section: GTransform,
}


impl PolygonSVG {
    pub fn from_str(s: &str) -> Result<Self,serde_xml_rs::Error> {
        use super::serde_xml_rs::from_str;
        from_str(s)
    }
    pub fn to_polygon(&self) ->Vec<F64Point> {
        Vec::new()
    }
}
pub fn to_polygon(s: &str) -> Result<Vec<F64Point>, serde_xml_rs::Error> {
    Ok(PolygonSVG::from_str(s)?.to_polygon())
}
fn f64_err(e: std::num::ParseFloatError) -> String {
  format!("{}", e).to_string()
}

const TFORM_REGEX_STR: &'static str = r"^\s*(?:scale\(\s*([^\)]+)\)\s*)?(?:translate\(\s*([^,]+),\s*([^\)]+)\)\s*)?\s*(?:translate\(\s*([^,]+),\s*([^\)]+)\)\s*)?(?:rotate\(\s*([^\)]+)\)\s*)?(?:translate\(\s*([^,]+),\s*([^\)]+)\s*\)?)\s*$";
fn gen_transform_deserializer(input:&str) -> Result<Transform, String> {
  let tform = Regex::new(TFORM_REGEX_STR).unwrap(); // don't use lazy static dependency
  // only happens during IO, so the simplicity is worth it
  let matches_opt = tform.captures(input);
  if matches_opt.is_none() {
    return Err("No matches for ".to_string() + input)
  }
  let matches = matches_opt.unwrap();
  let mut ret = Transform{scale:1.0,rotate:0.0,tx:0.0,ty:0.0,midx:0.0,midy:0.0};
  if let Some(scale) = matches.get(1) {
    ret.scale = scale.as_str().parse::<f64>().map_err(f64_err)?;
  }
  if let Some(tx) = matches.get(2) {
    if let Some(ty) = matches.get(3) {
      ret.tx = tx.as_str().parse::<f64>().map_err(f64_err)?;
      ret.ty = ty.as_str().parse::<f64>().map_err(f64_err)?;
    }
  }
  if let Some(midx) = matches.get(4) {
    if let Some(midy) = matches.get(5) {
      ret.midx = midx.as_str().parse::<f64>().map_err(f64_err)?;
      ret.midy = midy.as_str().parse::<f64>().map_err(f64_err)?;
    }
  }
  let ix;
  let iy;
  if let Some(rmidx) = matches.get(7) {
    if let Some(rmidy) = matches.get(8) {
      ix = rmidx.as_str().parse::<f64>().map_err(f64_err)?;
      iy = rmidy.as_str().parse::<f64>().map_err(f64_err)?;
    } else {
      ix = 0.0;
      iy = 0.0;
    }
  } else {
    ix = 0.0;
    iy = 0.0;
  }
  if ret.midx != -ix || ret.midy != -iy {
    if ret.midx == 0.0 && ret.midy == 0.0 && ret.tx == -ix && ret.ty == -iy {
      ret.midx = ret.tx;
      ret.midy = ret.ty;
      ret.tx = 0.0;
      ret.ty = 0.0;
    } else {
      return Err(format!("translate({},{}) != -translate({},{}) = translate({}, {})",
                         ret.midx, ret.midy, ix, iy, -ix, -iy))
    }
  }
  if let Some(rotate) = matches.get(6) {
    ret.rotate = rotate.as_str().parse::<f64>().map_err(f64_err)?;
  }
  Ok(ret)
}

pub fn point_deserializer<'de, D>(deserializer: D) -> Result<Vec<F64Point>, D::Error>
where
  D: Deserializer<'de>,
{
  let input = String::deserialize(deserializer)?;
  unpack_polygon_points(input.as_str()).map_err(serde::de::Error::custom)
}


pub fn transform_deserializer<'de, D>(deserializer: D) -> Result<Transform, D::Error>
where
  D: Deserializer<'de>,
{
  let input = String::deserialize(deserializer)?;
  gen_transform_deserializer(input.as_str()).map_err(serde::de::Error::custom)
}
fn unpack_polygon_points(input:&str) -> Result<Vec<F64Point>, String> {
    let mut ret = Vec::<F64Point>::new();
    for pair in input.split(',') {
        let mut pnt = [0.0;2];
        let mut index = 0usize;
        for coord in pair.split_whitespace() {
            if coord.len() == 0 {
                continue
            }
            pnt[std::cmp::min(1, index)] = coord.parse().map_err(|e| format!("{:?}", e))?;
            index += 1;
        }
        if index != 2 {
            return Err(format!("Too many dims when making polygon: {}", index));
        }
        ret.push((pnt[0], pnt[1]));
    }
    Ok(ret)
}

mod test {
  #[test]
  fn test_regex() {
    let tform: regex::Regex = regex::Regex::new(super::TFORM_REGEX_STR).unwrap();
    tform.captures("translate(290, 80) translate(64, 64) rotate(220) translate(-64, -64)").unwrap();
  }
  #[test]
  fn test_parse_polygon_points() {
      let st = "1 2,3 4, 5 6,7 8";
      let parsed = super::unpack_polygon_points(st).unwrap();
      assert_eq!(&parsed,
                 &[(1., 2.),
                   (3., 4.),
                   (5., 6.),
                   (7., 8.),
                 ]);
  }
  #[test]
  fn test_parse_bad_polygon_points() {
      let st = "1 2 3,3 4, 5 6,7 8";
      let parsed = super::unpack_polygon_points(st);
      if let Ok(_) = parsed {
          panic!("Need to have an error here")
      }
      let st2 = "1 2a,3 4, 5 6,7 8";
      let parsed2 = super::unpack_polygon_points(st2);
      if let Ok(_) = parsed2 {
          panic!("Need to have an error here")
      }      
  }
  fn read_to_string(filename: &Path) ->  Result<String, serde_xml_rs::Error> {
    let mut f = std::fs::File::open(filename)?;
    let mut buffer = String::new();
    match f.read_to_string(&mut buffer) {
        Ok(_) => Ok(buffer),
        Err(e) => Err(serde::de::Error::custom(e)),
    }
  }
use std::path::Path;
    use std::io::Read;
  #[test]
  fn test_parse_assets() {
      use super::serde_xml_rs::from_str;
      let assets = vec!["assets/doric.svg",
                        "assets/thinrect.svg",
                        "assets/castle.svg",
                        "assets/ramp.svg",
                        "assets/car.svg",
                        "assets/rect.svg",
                        "assets/hdoublepane.svg",
                        "assets/hwindow.svg",
                        "assets/harch.svg",
                        "assets/doublepane.svg",
                        "assets/quartpipe.svg",
                        "assets/medramp.svg",
                        "assets/square.svg",
                        "assets/larch.svg",
                        "assets/medsquare.svg",
                        "assets/lhalframp.svg",
                        "assets/hporthole.svg",
                        "assets/halfpipe.svg",
                        "assets/pipe.svg",
                        "assets/windows.svg",
                        "assets/smallsquare.svg",
                        "assets/medcircle.svg",
                        "assets/circle.svg",
                        "assets/arch.svg",
                        "assets/rarch.svg",
                        "assets/house.svg",
                        "assets/n.svg",
                        "assets/eichler.svg",
                        "assets/lquartramp.svg",
                        "assets/smallcircle.svg",
                        "assets/gothic.svg",
                        "assets/column.svg",
                        "assets/rquartramp.svg",
                        "assets/roof.svg",
                        "assets/rhalframp.svg",
                        "assets/halfthinrect.svg",
                        "assets/porthole.svg",
      ];
      for asset in assets {
          eprintln!("Testing Asset {}\n", asset);
          let ramp:super::PolygonSVG = from_str(&read_to_string(Path::new(&asset)).unwrap()).unwrap();
          assert_eq!(ramp.to_polygon().len(), 0);
      }
  }
}
