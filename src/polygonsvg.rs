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
impl Default for Transform {
    fn default() -> Self {
        Self::new(64,64)
    }
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
    let rotated;
    if t.rotate != 0.0 {
        let rotate_rad = -t.rotate * std::f64::consts::PI/180.;
        rotated = (centered.0 * rotate_rad.cos() + centered.1 * rotate_rad.sin(),
                       -centered.0 * rotate_rad.sin() + centered.1 * rotate_rad.cos());
    } else {
        rotated = centered;
    }
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
impl Rect {
    fn to_polygon(&self) -> [F64Point;4] {
        [
            (self.x,self.y),
            (self.x+self.width,self.y),
            (self.x+self.width,self.y+self.height),
            (self.x,self.y+self.height),
        ]
    }
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
const POLYGON_RESOLUTION:usize = 16;
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
impl Ellipse {
    fn to_polygon(&self)  -> [F64Point;POLYGON_RESOLUTION] {
        let mut ret = [F64Point::default();POLYGON_RESOLUTION];
        for (index, item) in ret.iter_mut().enumerate() {
            let angle = index as f64 * std::f64::consts::PI * 2./ POLYGON_RESOLUTION as f64;
            *item = (self.cx + self.rx * angle.cos(), self.cy + self.ry * angle.sin());
        }
        ret
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq,Default,Clone)]
struct Circle {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    #[serde(default)]
    pub fill: String,
}
impl Circle {
    fn to_polygon(&self)  -> [F64Point;POLYGON_RESOLUTION] {
        return Ellipse::from(self.clone()).to_polygon()
    }
}


#[derive(Debug, Serialize, Deserialize, PartialEq,Default)]
struct Polygon {
    #[serde(deserialize_with="point_deserializer")]
    pub points: Vec<F64Point>,
}

impl Polygon {
    fn to_polygon(&self)  -> &[F64Point] {
        return &self.points
    }
}


#[derive(Debug, Serialize, Default, Deserialize, PartialEq)]
struct GTransform {
    #[serde(deserialize_with="transform_deserializer")]
    pub transform: Transform,
    #[serde(default)]
    pub polygon: Vec<Polygon>,
    #[serde(default)]
    pub rect: Vec<Rect>,
    #[serde(default)]
    pub ellipse: Vec<Ellipse>,
    #[serde(default)]
    pub circle: Vec<Circle>,
}
impl GTransform {
    pub fn to_polygon(&self) ->Vec<F64Point> {
        let mut ret = Vec::<F64Point>::new();
        for poly in &self.polygon {
            poly_join(&mut ret, poly.to_polygon());
        }
        for rect in &self.rect {
            poly_join(&mut ret, &rect.to_polygon()[..]);
        }
        for el in &self.ellipse {
            poly_join(&mut ret, &el.to_polygon()[..]);
        }
        for cir in &self.circle {
            poly_join(&mut ret, &cir.to_polygon()[..]);
        }
        for vertex in &mut ret {
            *vertex = ftransform(&self.transform, *vertex);
        }
        ret
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct PolygonSVG {
    #[serde(default)]
    pub defs: defs,
    #[serde(default)]
    #[serde(rename="g")]
    pub section: GTransform,
}
fn poly_join(ret: &mut Vec<F64Point>, new_polygon:&[F64Point]){
    if ret.len() == 0 {
        ret.extend_from_slice(new_polygon);
    } else {
        let last = ret[ret.len() - 1];
        ret.extend_from_slice(new_polygon);
        ret.push(last);
    }
}

impl PolygonSVG {
    pub fn from_str(s: &str) -> Result<Self,serde_xml_rs::Error> {
        use super::serde_xml_rs::from_str;
        from_str(s)
    }
    pub fn to_polygon(&self) ->Vec<F64Point> {
        let mut ret = self.section.to_polygon();
        ret
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

fn dot2d(a :F64Point, b : F64Point) -> f64 {
    a.0 * b.0 + a.1 * b.1
}
pub fn scale2d(a:F64Point, b: f64) -> F64Point {
    (a.0*b, a.1*b)
}
pub fn sub2d(a :F64Point, b:F64Point) -> F64Point {
    (a.0 -b.0, a.1-b.1)
}
pub fn add2d(a :F64Point, b:F64Point) -> F64Point {
    (a.0 +b.0, a.1+b.1)
}
// returns parameter origin + dir * returned value if there's an intersection
pub fn ray_vs_segment(origin: F64Point, dir: F64Point, a: F64Point, b: F64Point) -> Option<f64> {
    let v1 = (origin.0-a.0, origin.1-a.1);
    let v2 = (b.0-a.0, b.1 - a.1);
    let v3 = (-dir.1, dir.0);
    let lenv2crossv1 = v2.0 * v1.1 - v2.1 * v1.0;
    let dotv2v3 = dot2d(v2, v3);
    if !(dotv2v3 >= 1.0e-10 || dotv2v3 <= -1.0e-10) {
        if dir.0 == 0.0 {
            // vertical
            if a.0 == origin.0 {
                let mut ta = (a.1 - origin.1) / dir.1;
                let mut tb = (b.1 - origin.1) / dir.1;
                if ta >= 0.0 && tb >= 0.0 {
                    return Some(ta.min(tb));
                }
                if (ta >= 0.0) != (tb >= 0.0) {
                    return Some(0.0); // in the middle of the line
                }
                return None;
            }
            let ta = (a.0 - origin.0) / dir.0;
            let tb = (b.0 - origin.0) / dir.0;
            if origin.1 + dir.1 * ta != a.1 {
                return None;
            }
            if ta >= 0.0 && tb >= 0.0 {
                return Some(ta.min(tb));
            }
            if (ta >= 0.0) != (tb >= 0.0) {
                return Some(0.0); // in the middle of the line
            }
            return None;            
        }
        //eprintln!("COLLINEAR: unimpl");
        return None; // collinear -- assume not exactly the same, for our purposes
    }
    let t1 = lenv2crossv1 / dotv2v3;
    let t2 = dot2d(v1, v3) / dotv2v3;
    //eprintln!("INPUT o:{:?} dir {:?} a {:?} b {:?}", origin, dir, a, b);
    
    //eprintln!("CHECKING v1 {:?},v2 {:?},v3 {:?} ======= t1 {}, t2 {}", v1, v2, v3, t1, t2);
    if t2 < 1.0 && t2 >= 0.0 && t1 >= 0.0 {
        return Some(t1)
    }
    return None
}
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PolyIntersection {
    pub outward :F64Point,
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct RayParamAndHitCount {
    t: f64,
    inside: bool,
}
pub fn ray_vs_polygon(
    origin: F64Point, dir: F64Point, poly_transform: &Transform, poly: &[F64Point],
) -> Option<RayParamAndHitCount> {
    if let Some(mut ret) = ray_vs_polygon_helper(origin,dir,poly_transform, poly) {
        if ret.inside {
            //eprintln!("GOT CHecking {:?} dir: {:?} to {:?} == {:?}", (origin.0, origin.1), dir, poly, ret);
            //eprintln!("BACKUP CHecking {:?} dir: {:?} to {:?} which gets {:?}", (origin.0+0.01, origin.1+0.0125), dir, poly,ray_vs_polygon_helper((origin.0+0.01, origin.1+0.0125),dir,poly_transform, poly));
            if let Some(backup_check) = ray_vs_polygon_helper((origin.0+0.01, origin.1+0.0125),dir,poly_transform, poly) {
                if !backup_check.inside {
                    ret.inside = false;
                    return Some(ret);
                }
            } else {
                return None;
            }
        }
        Some(ret)
    } else {
        None
    }
    
}
pub fn ray_vs_polygon_helper(
    origin: F64Point, dir: F64Point, poly_transform: &Transform, poly: &[F64Point],
) -> Option<RayParamAndHitCount> {
    if poly.len() == 0 {
        return None;
    }
    let mut last = ftransform(poly_transform, poly[poly.len() - 1]);
    let mut hit_count = 0;
    let mut ret: Option<f64> = None;
    for point in poly {
        let cur_point = ftransform(poly_transform, *point);
        //eprintln!("CHecking {:?} - {:?}", last, cur_point);
        if let Some(t) = ray_vs_segment(origin, dir, last, cur_point) {
            if let Some(t_old) = ret {
                ret = Some(t.min(t_old));
            } else {
                ret = Some(t)
            }
            hit_count += 1;
            //eprintln!("Ray {:?} -> {:?} hit {:?}->{:?} at {:?} hc={}", origin, dir, last, cur_point, ret, hit_count);
        }
        last = cur_point;
    }
    if let Some(t) = ret {
        Some(RayParamAndHitCount{
            t:t,
            inside:(hit_count&1) == 1,
        })
    } else {
        None
    }
}
pub fn origin_inside_polygon(origin: F64Point, dir: F64Point, poly_transform: &Transform, poly: &[F64Point]) -> Option<f64> {
    let ret = ray_vs_polygon(origin, dir, poly_transform, poly);
    if let Some(ray_param) = ret {
        if ray_param.inside {
            return Some(ray_param.t);
        }
    }
    None
}


pub fn segment_inside_polygon(a: F64Point, b: F64Point, poly_transform: &Transform, poly: &[F64Point], up: F64Point) -> Option<PolyIntersection> {
    let mut a_p_intersection = ray_vs_polygon(a, sub2d(b, a), poly_transform, poly);
    let mut b_p_intersection = ray_vs_polygon(b, sub2d(a, b), poly_transform, poly);
    //eprintln!("A->B {:?} B->A {:?}", a_p_intersection, b_p_intersection);
    if a_p_intersection.is_none() && b_p_intersection.is_none() {
        return None
    }
    let a_inside = a_p_intersection.unwrap_or(RayParamAndHitCount{t:0.,inside:false}).inside;
    let b_inside = b_p_intersection.unwrap_or(RayParamAndHitCount{t:0.,inside:false}).inside;
    //eprintln!("A->B {:?} B->A {:?} ainside: {} binside: {}", a_p_intersection, b_p_intersection, a_inside, b_inside);
    if a_inside && b_inside { // both inside
        let middle = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
        //eprintln!("middle {},{}  {},{}", a.0, a.1, b.0, b.1);
        if let Some(w) = origin_inside_polygon(middle, up, poly_transform, poly) {
            return Some(PolyIntersection{outward:(up.0 * w, up.1*w)})
        }
        if let Some(w) = origin_inside_polygon(a, up, poly_transform, poly) {
            return Some(PolyIntersection{outward:(up.0 * w, up.1*w)})
        }
        if let Some(w) = origin_inside_polygon(b, up, poly_transform, poly) {
            return Some(PolyIntersection{outward:(up.0 * w, up.1*w)})
        }
    }
    if a_inside == false && b_inside == false && a_p_intersection.unwrap_or(b_p_intersection.unwrap_or(RayParamAndHitCount{t:2.0,inside:false})).t <= 1.0 {
        //eprintln!("not both middle {:?} is {:?} < {:?} == {:?}", a_p_intersection.unwrap_or(b_p_intersection.unwrap_or(RayParamAndHitCount{t:2.0,inside:false})).t, a_p_intersection.unwrap_or(RayParamAndHitCount{t:2.0,inside:false}).t, b_p_intersection.unwrap_or(RayParamAndHitCount{t:2.0,inside:false}).t,a_p_intersection.unwrap_or(RayParamAndHitCount{t:2.0,inside:false}).t <= b_p_intersection.unwrap_or(RayParamAndHitCount{t:2.0,inside:false}).t);
        // segment spans a corner but is not inside
        if a_p_intersection.unwrap_or(RayParamAndHitCount{t:2.0,inside:false}).t < b_p_intersection.unwrap_or(RayParamAndHitCount{t:2.0,inside:false}).t {
            return Some(PolyIntersection{
                outward:sub2d(add2d(b, scale2d(sub2d(a, b), b_p_intersection.unwrap_or(RayParamAndHitCount{t:1.0,inside:false}).t)), a),
            });
        }
        return Some(PolyIntersection{outward:sub2d(add2d(a, scale2d(sub2d(b, a), a_p_intersection.unwrap_or(RayParamAndHitCount{t:1.0,inside:false}).t)), b)});
    }
    if a_inside {
        return Some(PolyIntersection{
            outward:sub2d(add2d(b, scale2d(sub2d(a, b), b_p_intersection.unwrap_or(RayParamAndHitCount{t:1.0,inside:false}).t)), a),
        });        
    }
    if b_inside {
        return Some(PolyIntersection{
            outward:sub2d(add2d(b, scale2d(sub2d(a, b), b_p_intersection.unwrap_or(RayParamAndHitCount{t:1.0,inside:false}).t)), b),
        }); 
    }
    None
}


mod test {
  #[test]
  fn test_ray_polygon_intersect() {
      use super::origin_inside_polygon;
      use super::Transform;
      let t = &Transform::default();
      /*
      // a ray that would otherwise hit a polygon does not trigger a "inside" unless it starts inside
      assert_eq!(origin_inside_polygon((10.,10.),(1.,1.),t, &[(13.,11.),(11.,13.), (12.,13.)]), None);
      assert_eq!(origin_inside_polygon((10.,10.),(1.,1.),t, &[(12.,12.),(11.,13.), (12.,13.)]), None);
      assert_eq!(origin_inside_polygon((10.,10.),(-1.,-1.),t, &[(13.,11.),(11.,13.), (12.,13.)]), None);

      // keeping the same direction but moving the start location inside the polygon
      assert_eq!(origin_inside_polygon((12.75,12.75),(1.,1.),t, &[(13.,11.),(11.,13.), (14.,13.)]), Some(0.25));
      assert_eq!(origin_inside_polygon((12.75,12.75),(1.,1.),t, &[(12.,12.),(11.,13.), (14.,13.)]), Some(0.25));
      assert_eq!(origin_inside_polygon((12.75,12.75),(-1.,-1.),t, &[(13.,11.),(11.,13.), (14.,13.)]), Some(0.75));
*/
      assert_eq!(origin_inside_polygon((0.,180.),(128.,64.),t,
                                       &[(640.0, 500.0),
                                         (672.0, 500.0),
                                         (672.0, 532.0),
                                         (640.0, 532.0),
                                         (640.0, 500.0),
                                         (672.0, 500.0),
                                         (672.0, 532.0),
                                         (640.0, 532.0)]), None);
  }
  #[test]
  fn test_capture_segment_inside_polygon() {
      use super::Transform;
      use super::segment_inside_polygon;
      use super::ray_vs_polygon;
      use super::RayParamAndHitCount;
      assert_eq!(ray_vs_polygon(
          (0.0,180.0),
          super::sub2d((128.0,244.0), (0.0,180.0)),
          &Transform::default(),
          &[(16.0 + 624.0, 16.0+484.0), (48.0 + 624.0, 16.0+484.0), (48.0+624.0, 48.0+484.0), (16.0+624.0, 48.0+484.0)],
      ), Some(RayParamAndHitCount{t:5.,inside:false}));
      assert_eq!(segment_inside_polygon(
          (128.0,244.0), (0.0,180.0),
          &Transform::default(), 
          &[(16.0 + 624.0, 16.0+484.0), (48.0 + 624.0, 16.0+484.0), (48.0+624.0, 48.0+484.0), (16.0+624.0, 48.0+484.0)],
          (0.0,-1.0),
      ), None);
      /*
      assert_eq!(segment_inside_polygon(
          (128.0,244.0), (0.0,180.0),
          &Transform{
              midx:32.0, midy:32.0,rotate:0.0, tx:624.0, ty:484.0, scale:1.0,
          }, 
          &[(16.0, 16.0), (48.0, 16.0), (48.0, 48.0), (16.0, 48.0)],
          (0.0,-1.0),
          ), None);
*/
  }
  #[test]
  fn test_segment_inside_polygon() {
      let t = &Transform::default();
      use super::Transform;
      use super::segment_inside_polygon;
      use super::PolyIntersection;
      let aabb = [(-4.,2.), (3.,2.), (3.,-1.),(-4.,-1.)];
      assert_eq!(segment_inside_polygon((-100.,-100.),(100.,-100.), t,&aabb[..], (0.,1.)), None);
      assert_eq!(segment_inside_polygon((-5.,0.5),(1.,0.5), t, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(-5.,0.)}));
                 //xx Some(PolyIntersection{outward:(0.,1.5)}));
      assert_eq!(segment_inside_polygon((0.,0.5),(1.,0.5), t, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(0.,1.5)}));

      assert_eq!(segment_inside_polygon((-5.,0.5),(4.,0.5), t, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(-8.,0.)}));
      assert_eq!(segment_inside_polygon((-3.,0.5),(4.,0.5), t, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(6.,0.)}));
                 //xx Some(PolyIntersection{outward:(0.,1.5)}));
      assert_eq!(segment_inside_polygon((-5.,0.5),(2.5,0.5), t, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(-6.5,0.)}));
                 //xx Some(PolyIntersection{outward:(0.0,1.5)}));

      assert_eq!(segment_inside_polygon((0.,-4.),(0.,4.), t, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(0.,-5.)}));


      let shift_right = &Transform{midx:32.,midy:32.,rotate:0.,tx:1.,ty:1.,scale:1.};
      
      assert_eq!(segment_inside_polygon((-100.,-100.),(100.,-100.), shift_right,&aabb[..], (0.,1.)), None);
      assert_eq!(segment_inside_polygon((-5.,0.5),(1.,0.5), shift_right, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(-4.,0.)}));
                 //xx Some(PolyIntersection{outward:(0.,2.5)}));
      assert_eq!(segment_inside_polygon((0.,0.5),(1.,0.5), shift_right, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(0.,2.5)}));

      assert_eq!(segment_inside_polygon((-5.,0.5),(4.,0.5), shift_right, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(-7.,0.)}));
      assert_eq!(segment_inside_polygon((-3.,0.5),(4.,0.5), shift_right, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(-7.,0.)}));
      assert_eq!(segment_inside_polygon((-5.,0.5),(2.5,0.5), shift_right, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(-5.5,0.)}));
                 //xx Some(PolyIntersection{outward:(0.0,2.5)}));

      assert_eq!(segment_inside_polygon((0.,-4.),(0.,4.), shift_right, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(0.,-4.)}));

      let shift_scale = &Transform{midx:0.5,midy:0.5,rotate:0.,tx:1.,ty:1.,scale:2.};
      
      assert_eq!(segment_inside_polygon((-100.,-100.),(100.,-100.), shift_scale,&aabb[..], (0.,1.)), None);
      assert_eq!(segment_inside_polygon((-8.,0.5),(1.,0.5), shift_scale, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(-8.5,0.)}));
                 //xx Some(PolyIntersection{outward:(0.0,4.)}));
      assert_eq!(segment_inside_polygon((0.,0.5),(1.,0.5), shift_scale, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(0.,4.)}));

      assert_eq!(segment_inside_polygon((-5.,0.5),(4.,0.5), shift_scale, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(0.,4.)}));
      assert_eq!(segment_inside_polygon((-3.,0.5),(4.,0.5), shift_scale, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(0.,4.)}));
      assert_eq!(segment_inside_polygon((-5.,0.5),(2.5,0.5), shift_scale, &aabb[..], (0.,1.)),
                 Some(PolyIntersection{outward:(0.,4.)}));

      assert_eq!(segment_inside_polygon((0.,-4.),(0.,4.), shift_scale, &aabb[..], (0.,1.)),
      //xx            Some(PolyIntersection{outward:(0.,4.5)}));
                 Some(PolyIntersection{outward:(0.,-5.5)}));

  }
  #[test]
  fn test_segment_intersect() {
      use super::ray_vs_segment;
      assert_eq!(ray_vs_segment((10.,10.),(1.,1.),(12.,12.),(11.,13.)), Some(2.));
      assert_eq!(ray_vs_segment((10.,10.),(1.,1.),(13.,11.),(11.,13.)), Some(2.));
      assert_eq!(ray_vs_segment((10.,10.),(-1.,-1.),(13.,11.),(11.,13.)), None);

      assert_eq!(ray_vs_segment((10.,10.),(-1.,0.),(13.,11.),(11.,13.)), None);
      assert_eq!(ray_vs_segment((10.,10.),(-1.,0.),(13.,11.),(10.,14.)), None);


      assert_eq!(ray_vs_segment((10.,10.),(1.,3.),(12.,12.),(11.,13.)),None); //Some(1.));
      assert_eq!(ray_vs_segment((10.,10.),(1.,3.),(13.,11.),(11.,13.)), None);//Some(1.));
      assert_eq!(ray_vs_segment((10.,10.),(-1.,-3.),(13.,11.),(11.,13.)), None);

      assert_eq!(ray_vs_segment((10.,10.),(3.,1.),(12.,12.),(11.,13.)), None);
      assert_eq!(ray_vs_segment((10.,10.),(3.,1.),(13.,11.),(11.,13.)), Some(1.));
      assert_eq!(ray_vs_segment((10.,10.),(-3.,-1.),(13.,11.),(11.,13.)), None);
  }
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
      let sizes = [
          21 as usize,
          4,
          18,
          3,
          42,
          4,
          4,
          4,
          4,
          4,
          4,
          3,
          4,
          4,
          4,
          3,
          4,
          4,
          4,
          14,
          4,
          16,
          16,
          4,
          4,
          5,
          8,
          4,
          3,
          16,
          9,
          11,
          3,
          3,
          3,
          4,
          4,
      ];
      assert_eq!(sizes.len(), assets.len());
      for (asset, size) in assets.iter().zip(&sizes[..]) {
          //eprintln!("Testing Asset {}\n", asset);
          let ramp:super::PolygonSVG = from_str(&read_to_string(Path::new(&asset)).unwrap()).unwrap();
          assert_eq!(ramp.to_polygon().len(), *size);
      }
  }
}
