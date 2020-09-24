#[allow(unused_imports)]
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
fn attr_escape<'a> (s:&'a String, scratch :&'a mut String) -> &'a str {
    let mut any_found = false;
    for c in s.chars() {
        match c {
            '>' | '<'  | '"' | '\'' | '&' => any_found = true,
            _ =>{},
        }
    }
    if any_found {
        *scratch = s.chars().map(|c| match c{
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            '&' => "&amp;".to_string(),
            _ => c.to_string(),
        }).collect();
        scratch
    } else {
        s
    }
}

#[derive(Debug, Default,Copy,Clone, Eq,PartialEq)]
pub struct Color{
    pub r:u8,
    pub g:u8,
    pub b:u8,
}

impl ToString for Color {
    fn to_string(&self) -> String {
        let mut s = String::with_capacity(7);
        s += "#";
        for byte in &[self.r,self.g,self.b] {
            write!(s, "{:02x}", byte).unwrap();
        }
        s
    }
}
impl TryFrom<String> for Color {
    type Error = String;
    fn try_from(hex: String) -> Result<Color, Self::Error> {
        str_to_color(&hex)
    }
}


fn str_to_color(hex: &str) -> Result<Color, String> {
    if hex.len() != 7 {
        return Err(hex.to_string() + ": is not 7 long");
    }
    let bytes = hex.as_bytes();
    for chr in bytes {
            if *chr > 128 {
                return Err(hex.to_string() + ": non-ASCII color");
            }
    }
    // need to make sure it's ascii so that the split_at won't panic
    let (hash, rest) = hex.split_at(1);
    if hash != "#" {
        return Err(hex.to_string() + ": does not begin with  #");
    }
    let (rstr, rest) = rest.split_at(2);
    let (gstr, rest) = rest.split_at(2);
    let (bstr, rest) = rest.split_at(2);
    let r = if let Ok(rr) = u8::from_str_radix(rstr, 16) {
        rr
    } else {
        return Err(rstr.to_string() + " not base 16");
    };
    let g = if let Ok(gg) = u8::from_str_radix(gstr, 16) {
        gg
    } else {
        return Err(gstr.to_string() + " not base 16");
    };
    let b = if let Ok(bb) = u8::from_str_radix(bstr, 16) {
        bb
    } else {
        return Err(bstr.to_string() + " not base 16");
    };
    Ok(Color{r:r, g:g, b:b})
}
        
impl TryFrom<&str> for Color {
    type Error = String;
    fn try_from(hex:&str) -> Result<Color, String> {
        str_to_color(hex)
    }
}

impl serde::Serialize  for Color {
    fn serialize<S:serde::Serializer>(&self, s:S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub use super::polygonsvg::{ftransform,itransform, F64Point, Transform, transform_deserializer, point_deserializer};

fn poly_helper(a: &[F64Point], b:&[F64Point]) -> bool {
    for (i1, p0) in a.iter().enumerate() {
        let p1 = a[(i1 + 1) % a.len()];
        let perp = (p1.1 - p0.1, p0.0 - p1.0);
        let mut min_a: Option<f64> = None;
        let mut max_a: Option<f64> = None;
        let mut min_b: Option<f64> = None;
        let mut max_b: Option<f64> = None;
        for a_pnt in a {
            let proj = perp.0 * a_pnt.0 + perp.1 * a_pnt.1;
            min_a = Some(min_a.unwrap_or(proj).min(proj));
            max_a = Some(max_a.unwrap_or(proj).max(proj));
        }
        for b_pnt in b {
            let proj = perp.0 * b_pnt.0 + perp.1 * b_pnt.1;
            min_b = Some(min_b.unwrap_or(proj).min(proj));
            max_b = Some(max_b.unwrap_or(proj).max(proj));
        }
        if max_a < min_b || max_b < min_a {
            return false;
        }
    }
    true
}
pub fn poly_edge_intersect(a: &[F64Point], b:&[F64Point]) -> bool {
    poly_helper(a, b) || poly_helper(b, a)
}

pub fn compose(t:&Transform, u:&Transform) -> Transform {
    let txty = ftransform(t, (u.tx, u.ty));
    Transform{
        tx: txty.0,
        ty: txty.1,
        midx: u.midx,
        midy: u.midy,
        rotate: t.rotate + u.rotate,
        scale: t.scale * u.scale,
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct HrefAndClipMask {
    pub url: String,
    pub clip: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct image {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub mask: String,
    #[serde(default)]
    #[serde(rename="clip-path")]
    pub clip_mask: String,
    pub fill: String,
}


#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, PartialEq)]
pub struct Image {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub href: HrefAndClipMask,
    pub fill: Color,
}

impl TryFrom<image> for Image {
    type Error = String;
    fn try_from(im: image) -> Result<Self, Self::Error> {
        Ok(Image{
            x:im.x,
            y:im.y,
            width:im.width,
            height:im.height,
            href:HrefAndClipMask{
                url:parse_url_from_mask(&im.mask)?.to_string(),
                clip:im.clip_mask,
            },
            fill:Color::try_from(im.fill)?,
        })
    }
}

impl From<Image> for image {
    fn from(im: Image) -> Self {
        image{
            x:im.x,
            y:im.y,
            width:im.width,
            height:im.height,
            fill:im.fill.to_string(),
            mask:"url(#".to_string() + &im.href.url+")",
            clip_mask:im.href.clip,
        }
    }
}

impl Image {
    fn to_string(&self) -> Result<String,serde_xml_rs::Error> {
        let mut scratch = String::new();
        let mut scratch2 = String::new();
        if self.href.clip.len() != 0 {
            Ok(format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" mask=\"url(#{})\" clip-path=\"{}\"/>",
                self.x,self.y,self.width,self.height, self.fill.to_string(), attr_escape(&self.href.url, &mut scratch), attr_escape(&self.href.clip, &mut scratch2),
            ))
        } else {
            Ok(format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" mask=\"url(#{})\"/>",
                self.x,self.y,self.width,self.height,self.fill.to_string(),attr_escape(&self.href.url, &mut scratch),
            ))
        }
    }
}



fn pack_polygon_points(input: &[F64Point]) -> String {
    input.iter().map(|val|format!("{} {}", val.0, val.1)).collect::<Vec<String>>().join(",")
}

const URL_REGEX_STR: &'static str = r"url\(#([^\)]+)\)";
fn parse_url_from_mask<'a>(mask:&'a str) -> Result<&'a str, String> {
    let url_regex = Regex::new(URL_REGEX_STR).unwrap(); // don't use lazy static dependency
    // only happens during IO, so the simplicity is worth it
    let matches_opt = url_regex.captures(mask);
    if let Some(matches) = matches_opt{
        if let Some(ret) = matches.get(1) {
            return Ok(ret.as_str());
        } else {
            return Err("No url(#something.bmp) matches for ".to_string() + mask);
        }
    }
    Err("Unable to extract relative image url from match".to_string() + mask)
}

fn image_deserializer<'de, D>(deserializer: D) -> Result<Image, D::Error>
where
  D: Deserializer<'de>,
{
  Ok(Image::try_from(image::deserialize(deserializer)?).map_err(serde::de::Error::custom)?)
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct g {
    #[serde(deserialize_with="transform_deserializer")]
    pub transform: Transform,
    #[serde(rename="$value")]
    #[serde(deserialize_with="image_deserializer")]
    pub rect: Image,
}


impl g {
    fn to_string(&self) -> Result<String,serde_xml_rs::Error> {
        Ok(format!(
            "<g transform=\"{}\">\n{}\n</g>",
            self.transform.to_string()?,
            self.rect.to_string()?,
        ))
    }
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Default, Clone)]
pub struct Polygon {
    #[serde(deserialize_with="point_deserializer")]
    pub points: Vec<F64Point>,
}
impl Polygon {
    fn to_string(&self) -> Result<String, serde_xml_rs::Error> {
        let mut scratch = String::new();
        Ok(format!("<polygon points=\"{}\"/>\n",
                   pack_polygon_points(&self.points),
        ))
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ClipPath{
    pub id: String,
    pub polygon: Polygon,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct SourceStamp{
    pub href: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Mask{
    pub id: String,
}
impl ClipPath {
    fn to_string(&self) -> Result<String, serde_xml_rs::Error> {
        let mut scratch = String::new();
        Ok(format!("<clipPath id=\"{}\">\n{}</clipPath>\n",
                   attr_escape(&self.id, &mut scratch),
                   self.polygon.to_string()?,
        ))
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct defs {
    #[serde(default)]
    pub clipPath: Vec<ClipPath>,
    #[serde(default)]
    pub mask: Vec<Mask>,
}

fn read_to_string(filename: &Path) ->  Result<String, serde_xml_rs::Error> {
    let mut f = std::fs::File::open(filename)?;
    let mut buffer = String::new();
    match f.read_to_string(&mut buffer) {
        Ok(_) => Ok(buffer),
        Err(e) => Err(serde::de::Error::custom(e)),
    }
}


impl defs {
    fn to_string(&self, stamps: &Vec<g>) -> Result<String,serde_xml_rs::Error> {
        let mut ret = vec![String::new();self.clipPath.len()];
        for (serialized, deserialized) in ret.iter_mut().zip(self.clipPath.iter())   {
            *serialized = deserialized.to_string()?;
        }
	let mut active_images = std::collections::BTreeSet::<String>::new();
	for stamp in stamps {
	    if !active_images.contains(&stamp.rect.href.url) {
	    active_images.insert(stamp.rect.href.url.clone());
	    }
	}
	for active_image in active_images {
        let svg_filename = active_image.replace("/stamps/","/").replace(".bmp", ".svg");
        let asset_xml = read_to_string(&Path::new(&svg_filename))?;
	    //ret.push(format!("<mask id=\"{}\"><image x=\"0\" y=\"0\" width=\"64\" height=\"64\" href=\"{}\"/></mask>\n",active_image, svg_filename));
        ret.push(format!("<mask id=\"{}\">{}</mask>\n",active_image, asset_xml));
	}
        Ok(format!("<defs>\n{}</defs>\n", ret.join("")))
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SVG {
    pub version: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub defs: defs,
    #[serde(default)]
    #[serde(rename="g")]
    pub stamps: Vec<g>,
}

impl SVG {
    pub fn new(width:u32, height:u32) -> Self {
      SVG {
        version:"2.0".to_string(),
        width: width,
        height:height,
        stamps:Vec::new(),
        defs:defs{clipPath:Vec::new(),mask:Vec::new()},
      }
    }
    pub fn from_str(s: &str) -> Result<Self,serde_xml_rs::Error> {
        use super::serde_xml_rs::from_str;
        from_str(s)
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
    pub fn load_polygon(&self, bmp_name: &str) -> Result<Vec<F64Point>, serde_xml_rs::Error> {
        eprintln!("LOADING FROM {:?}\n", Path::new(&bmp_name.to_string().replace("/stamps/","/").replace(".bmp", ".svg")));
        let asset_data = match read_to_string(&Path::new(&bmp_name.to_string().replace("/stamps/","/").replace(".bmp", ".svg"))) {
            Ok(s) => s,
            Err(e) => return Err(serde::de::Error::custom(e)),
        };
        super::polygonsvg::to_polygon(&asset_data)
    }
    //
    pub fn intersect(&self, left: F64Point, right:F64Point, cache: &mut HashMap<String,Vec<F64Point>>) -> Result<Option<F64Point>, serde_xml_rs::Error> {
        for stamp in &self.stamps {
            let cur_poly:Vec<F64Point>;
            let poly = match &cache.get(&stamp.rect.href.url) {
                None => {
                    cur_poly = self.load_polygon(&stamp.rect.href.url)?;
                    cache.insert(stamp.rect.href.url.clone(), cur_poly.clone());
                    &cur_poly
                },
                &Some(poly) => poly,
            };
            if poly.len() == 0 {
                continue
            }
            let mut last = ftransform(&stamp.transform, poly[poly.len()-1]);
            for &cur_untransformed in poly {
                let cur = ftransform(&stamp.transform, cur_untransformed);
                // do intersect
                last = cur;
            }
        }
        Ok(None)
    }
    pub fn add(&mut self, transform: Transform, img: String, clip_mask: String, color: Color) {
        let width = (transform.midx * 2.0) as u32;
        let height = (transform.midy * 2.0) as u32;
        self.stamps.push(g{
            transform:transform,
            rect:Image{
                x:0,
                y:0,
                fill:color,
                width:width,
                height:height,
                href:HrefAndClipMask{url:img, clip:clip_mask},
            },
        });
    }
    pub fn to_string(&self) -> Result<String,serde_xml_rs::Error> {
        let mut ret = vec![String::new();self.stamps.len()];
        for (serialized, deserialized) in ret.iter_mut().zip(self.stamps.iter())   {
            *serialized = deserialized.to_string()?;
        }
        let mut max_width = self.width;
        let mut max_height = self.height;
        for stamp in &self.stamps {
            let diag = ((stamp.transform.midx * stamp.transform.midx) + (stamp.transform.midy * stamp.transform.midy)).sqrt();
            let x = (stamp.transform.tx + diag + stamp.transform.midx) as u32;
            let y = (stamp.transform.ty + diag + stamp.transform.midy) as u32;
            max_width = std::cmp::max(max_width, x);
            max_height = std::cmp::max(max_height, y);
        }
        Ok(format!(
            "<svg version=\"{}\" width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">\n{}\n{}</svg>",
            self.version,
            max_width,
            max_height,
            ret.join("\n"),
            self.defs.to_string(&self.stamps)?
        ))
    }
}


mod test {
    use super::{F64Point,Color, SourceStamp, Mask};
    #[cfg(test)]
    static LARCH_RARCH:&'static str = r##"<svg version="2.0" width="500" height="500" xmlns="http://www.w3.org/2000/svg">
<g transform="scale(2) translate(64, 64) rotate(8) translate(-64, -64)">
<rect x="0" y="0" width="128" height="128" fill="#000000" mask="url(#assets/stamps/larch.bmp)"/>
</g>
<g transform="translate(290, 80) translate(64, 64) rotate(220) translate(-64, -64)">
<rect x="0" y="0" width="128" height="128" fill="#ff1008" mask="url(#assets/stamps/rarch.bmp)"/>
</g>
<defs>
<mask id="assets/stamps/larch.bmp"><svg version="2.0" width="64" height="64" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <mask id="larch">
      <g>
      <rect x="0" y="0" width="64" height="64" fill="white"/>
      <ellipse cx="96" cy="66" rx="74" ry="85" fill="black"/>
      </g>
    </mask>
  </defs>
  <g transform="translate(0, 0)">
    <polygon fill="white" stroke="white" points="17 1,47 1,47 63,17 63" mask="url(#larch)"/>
  </g>
</svg>
</mask>
<mask id="assets/stamps/rarch.bmp"><svg version="2.0" width="64" height="64" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <mask id="rarch">
      <g>
      <rect x="0" y="0" width="64" height="64" fill="white"/>
      <ellipse cx="-32" cy="66" rx="74" ry="85" fill="black"/>
      </g>
    </mask>
  </defs>
  <g transform="translate(0, 0)">
    <polygon fill="white" stroke="white" points="17 1,47 1,47 63,17 63" mask="url(#rarch)"/>
  </g>
</svg>
</mask>
</defs>
</svg>"##;
    #[test]
    fn test_basic_serde() {
        use super::{SVG, HrefAndClipMask, Image, Transform, g, defs};
        let svg_struct = SVG {
            width:500,
            height:500,
            //                xmlns:"http://www.w3.org/2000/svg".to_string(),
            version:"2.0".to_string(),
            stamps:vec![
                g{
                  transform:Transform{scale:2.0, tx:0.0, ty:0.0, rotate:8.0, midx:64.0, midy:64.0},
                    rect:Image{
		    fill:Color{r:0,g:0,b:0},
                        x:0,
                        y:0,
                        height:128,
                        width:128,
                        href:HrefAndClipMask{url:"assets/stamps/larch.bmp".to_string(),clip:String::new()},
                    }
                },
                g{
                  transform:Transform{scale:1.0, tx:290.0, ty:80.0, rotate:220.0, midx:64.0, midy:64.0},
                    rect:Image{
		    fill:Color{r:255,g:16,b:8},
                        x:0,
                        y:0,
                        height:128,
                        width:128,
                        href:HrefAndClipMask{url:"assets/stamps/rarch.bmp".to_string(),clip:String::new()},
                    }                        
                },
            ],
            defs:defs{
                clipPath:Vec::new(),
                mask:vec![
                    Mask { id: "assets/stamps/larch.bmp".to_string() },
                    Mask { id: "assets/stamps/rarch.bmp".to_string() }
                ],
            },
        };
        use super::serde_xml_rs::from_str;
        let svg_deserialized: SVG = from_str(LARCH_RARCH).unwrap();
        assert_eq!(svg_deserialized, svg_struct);
        let svg_serialized = svg_struct.to_string().unwrap();
        eprintln!("{}",svg_serialized);
        assert_eq!(svg_serialized, LARCH_RARCH);
    }
    #[test]
    fn test_clip_mask_serde() {
        use super::{SVG, HrefAndClipMask, Image, Transform, g, defs, Polygon, ClipPath};
        let s = r##"<svg version="2.0" width="500" height="500" xmlns="http://www.w3.org/2000/svg">
<g transform="scale(2) translate(64, 64) rotate(8) translate(-64, -64)">
<rect x="0" y="0" width="128" height="128" fill="#040506" mask="url(#assets/stamps/larch.bmp)" clip-path="url(#clippy)"/>
</g>
<g transform="translate(290, 80) translate(64, 64) rotate(220) translate(-64, -64)">
<rect x="0" y="0" width="128" height="128" fill="#00ff00" mask="url(#assets/stamps/rarch.bmp)"/>
</g>
<defs>
<clipPath id="hellote">
<polygon points="1 -1,2 2,3 3,4 4.25"/>
</clipPath>
<clipPath id="goodbyte">
<polygon points="0 0,1 1,2 2,-3 3"/>
</clipPath>
<mask id="assets/stamps/larch.bmp"><svg version="2.0" width="64" height="64" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <mask id="larch">
      <g>
      <rect x="0" y="0" width="64" height="64" fill="white"/>
      <ellipse cx="96" cy="66" rx="74" ry="85" fill="black"/>
      </g>
    </mask>
  </defs>
  <g transform="translate(0, 0)">
    <polygon fill="white" stroke="white" points="17 1,47 1,47 63,17 63" mask="url(#larch)"/>
  </g>
</svg>
</mask>
<mask id="assets/stamps/rarch.bmp"><svg version="2.0" width="64" height="64" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <mask id="rarch">
      <g>
      <rect x="0" y="0" width="64" height="64" fill="white"/>
      <ellipse cx="-32" cy="66" rx="74" ry="85" fill="black"/>
      </g>
    </mask>
  </defs>
  <g transform="translate(0, 0)">
    <polygon fill="white" stroke="white" points="17 1,47 1,47 63,17 63" mask="url(#rarch)"/>
  </g>
</svg>
</mask>
</defs>
</svg>"##;
        let svg_struct = SVG {
            width:500,
            height:500,
            version:"2.0".to_string(),
            stamps:vec![
                g{
                  transform:Transform{scale:2.0, tx:0.0, ty:0.0, rotate:8.0, midx:64.0, midy:64.0},
                    rect:Image{
                        x:0,
                        y:0,
			fill:Color{r:4,g:5,b:6},
                        height:128,
                        width:128,
                        href:HrefAndClipMask{url:"assets/stamps/larch.bmp".to_string(),clip:"url(#clippy)".to_string()},
                    }
                },
                g{
                  transform:Transform{scale:1.0, tx:290.0, ty:80.0, rotate:220.0, midx:64.0, midy:64.0},
                    rect:Image{
                        x:0,
                        y:0,
			fill:Color{r:0,g:255,b:0},
                        height:128,
                        width:128,
                        href:HrefAndClipMask{url:"assets/stamps/rarch.bmp".to_string(),clip:String::new()},
                    }                        
                },
            ],
            defs:defs{
                mask:vec![
                    Mask { id: "assets/stamps/larch.bmp".to_string()},
                    Mask { id: "assets/stamps/rarch.bmp".to_string()}
                ],
                clipPath:vec![
                     ClipPath {
                        id: "hellote".to_string(),
                        polygon:Polygon{
                            points:vec![(1., -1.),
                                        (2., 2.),
                                        (3., 3.),
                                        (4., 4.25),
                            ],
                        },
                    },
                    ClipPath {
                        id: "goodbyte".to_string(),
                        polygon:Polygon{
                            points:vec![(0., 0.),
                                        (1., 1.),
                                        (2., 2.),
                                        (-3., 3.),
                        ],
                        },
                    },
                    ],
            },
        };
        use super::serde_xml_rs::from_str;
        let svg_deserialized: SVG = from_str(s).unwrap();
        assert_eq!(svg_deserialized, svg_struct);
        let svg_serialized = svg_struct.to_string().unwrap();
        eprintln!("{}",svg_serialized);
        eprintln!("{:?}",svg_deserialized);
        assert_eq!(svg_serialized, s);
    }
    #[test]
    fn test_attr_escape() {
        use super::attr_escape;

        let mut scratch = String::new();
        assert_eq!("HELLOTE", attr_escape(&"HELLOTE".to_string(), &mut scratch));
        assert_eq!("HEL&lt;LOTE", attr_escape(&"HEL<LOTE".to_string(), &mut scratch));
        assert_eq!("HEL&lt;LOTE&gt;", attr_escape(&"HEL<LOTE>".to_string(), &mut scratch));
        assert_eq!("HEL&lt;LOTE&amp;", attr_escape(&"HEL<LOTE&".to_string(), &mut scratch));
        assert_eq!("HEL&quot;LOTE&apos;", attr_escape(&"HEL\"LOTE'".to_string(), &mut scratch));
        assert_eq!("H\u{0026bE}EL&quot;LOTE&apos;",
                   attr_escape(&"H\u{0026bE}EL\"LOTE'".to_string(), &mut scratch));
    }
  #[test]
  fn test_pack_polygon_points() {
      let rendered = super::pack_polygon_points(&[(1., 2.),
                    (3., 4.),
                    (5., 6.),
                    (7., 8.),
      ]);
      let st = "1 2,3 4,5 6,7 8";
      assert_eq!(rendered, st.to_string())
  }
    fn _approx_eq(l: F64Point, r: F64Point) -> bool {
        let diff0 = if l.0 > r.0 {
            l.0 - r.0 
        }else {
            r.0 - l.0
        };
        let diff1 = if l.1 > r.1 {
            l.1 - r.1
        }else {
            r.1 - l.1
        };
        diff0 < 0.0001 && diff1 < 0.0001
    }
  macro_rules! assert_approx_eq {
      ($left: expr, $right: expr) => (
          if _approx_eq($left, $right) == false {assert_eq!($left, $right)} else {()}
      );
  }
    #[test]
    fn test_intersect() {
        use super::serde_xml_rs::from_str;
        use super::SVG;
        let svg_deserialized: SVG = from_str(LARCH_RARCH).unwrap();
        use std::collections::HashMap;
        let mut cache = HashMap::new();
        let intersection = svg_deserialized.intersect((3.,4.),(100.,4.), &mut cache).unwrap();
        
    }
  #[test]
    fn test_transform() {
        use super::ftransform;
        use super::itransform;
        use super::Transform;
        assert_eq!(ftransform(&Transform::new(1,1), (10.,10.)), (10., 10.));
        assert_eq!(itransform(&Transform::new(1,1), (10.,10.)), (10., 10.));
        let mut transf = Transform::new(1,1);
        let xstart = (10.5,0.5);
        let ystart = (0.5,10.5);
        transf.rotate = 90.;
        let rot90 = ftransform(&transf, ystart);
        assert_approx_eq!(rot90, (-9.5, 0.5));
        assert_approx_eq!(itransform(&transf, rot90), ystart);
        transf.rotate = 180.;
        let rot180 = ftransform(&transf, ystart);
        assert_approx_eq!(rot180, (0.5, -9.5));
        assert_approx_eq!(itransform(&transf, rot180), ystart);
        transf.rotate = 60.;
        let rot60 = ftransform(&transf, xstart);
        assert_approx_eq!(rot60, (5.5, 9.16025));
        assert_approx_eq!(itransform(&transf, rot60), xstart);

        transf.rotate = 90.;
        transf.tx = 4.;
        transf.ty = 5.;
        let rot90 = ftransform(&transf, ystart);
        assert_approx_eq!(rot90, (-5.5, 5.5));
        assert_approx_eq!(itransform(&transf, rot90), ystart);
        transf.rotate = 180.;
        let rot180 = ftransform(&transf, ystart);
        assert_approx_eq!(rot180, (4.5, -4.5));
        assert_approx_eq!(itransform(&transf, rot180), ystart);
        transf.rotate = 60.;
        let rot60 = ftransform(&transf, xstart);
        assert_approx_eq!(rot60, (9.5, 14.16025));
        assert_approx_eq!(itransform(&transf, rot60), xstart);

        transf.rotate = 90.;
        transf.tx = 4.;
        transf.ty = 5.;
        transf.scale=2.;
        let rot90 = ftransform(&transf, ystart);
        assert_approx_eq!(rot90, (-15.5, 5.5));
        assert_approx_eq!(itransform(&transf, rot90), ystart);
        transf.rotate = 180.;
        let rot180 = ftransform(&transf, ystart);
        assert_approx_eq!(rot180, (4.5, -14.5));
        assert_approx_eq!(itransform(&transf, rot180), ystart);
        transf.rotate = 60.;
        let rot60 = ftransform(&transf, xstart);
        assert_approx_eq!(rot60, (14.5, 22.8205));
        assert_approx_eq!(itransform(&transf, rot60), xstart);

  }
}
