#[allow(unused_imports)]
use super::serde_xml_rs::from_str;
use super::serde_xml_rs;
use serde::{Deserialize, Deserializer};
use serde;
use regex::Regex;
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
#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct image {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub href: String,
}

impl image {
    fn to_string(&self) -> Result<String,serde_xml_rs::Error> {
        let mut scratch = String::new();
        Ok(format!(
            "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"{}\"/>",
            self.x,self.y,self.width,self.height,attr_escape(&self.href, &mut scratch),
        ))
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
  fn to_string(&self) -> Result<String, serde_xml_rs::Error> {
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

fn f64_err(e: std::num::ParseFloatError) -> String {
  format!("{}", e).to_string()
}
  
const TFORM_REGEX_STR: &'static str = r"^\s*(?:scale\(\s*([^\)]+)\)\s*)?(?:translate\(\s*([^,]+),\s*([^\)]+)\)\s*)?\s*(?:translate\(\s*([^,]+),\s*([^\)]+)\)\s*)?(?:rotate\(\s*([^\)]+)\)\s*)?(?:translate\(\s*([^,]+),\s*([^\)]+)\s*\)?)\s*$";
fn gen_transform_deserializer(input:&str) -> Result<Transform, String> {
  lazy_static! {
    static ref TFORM: Regex = Regex::new(TFORM_REGEX_STR).unwrap();
  };
  let matches_opt = TFORM.captures(input);
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

fn transform_deserializer<'de, D>(deserializer: D) -> Result<Transform, D::Error>
where
  D: Deserializer<'de>,
{
  let input = String::deserialize(deserializer)?;
  gen_transform_deserializer(input.as_str()).map_err(serde::de::Error::custom)
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct g {
    #[serde(deserialize_with="transform_deserializer")]
    pub transform: Transform,
    #[serde(rename="$value")]
    pub image: image,
}


impl g {
    fn to_string(&self) -> Result<String,serde_xml_rs::Error> {
        Ok(format!(
            "<g transform=\"{}\">\n{}\n</g>",
            self.transform.to_string()?,
            self.image.to_string()?,
        ))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SVG {
    pub version: String,
    pub width: u32,
    pub height: u32,
    #[serde(rename="$value")]
    pub stamps: Vec<g>,
}

impl SVG {
    pub fn new(width:u32, height:u32) -> Self {
      SVG {
        version:"2.0".to_string(),
        width: width,
        height:height,
        stamps:Vec::new(),
      }
    }
    pub fn to_string(&self) -> Result<String,serde_xml_rs::Error> {
        let mut ret = vec![String::new();self.stamps.len()];
        for (serialized, deserialized) in ret.iter_mut().zip(self.stamps.iter())   {
            *serialized = deserialized.to_string()?;
        }
        Ok(format!(
            "<svg version=\"{}\" width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">\n{}\n</svg>",
            self.version,
            self.width,
            self.height,
            ret.join("\n")))
    }
}


mod test {
    #[test]
    fn test_basic_serde() {
        use super::{SVG, image, Transform, g};
        let s = r##"<svg version="2.0" width="500" height="500" xmlns="http://www.w3.org/2000/svg">
<g transform="scale(2) translate(64, 64) rotate(8) translate(-64, -64)">
<image x="0" y="0" width="128" height="128" href="simpler.svg"/>
</g>
<g transform="translate(290, 80) translate(64, 64) rotate(220) translate(-64, -64)">
<image x="0" y="0" width="128" height="128" href="simpler.svg"/>
</g>
</svg>"##;
        let svg_struct = SVG {
            width:500,
            height:500,
            //                xmlns:"http://www.w3.org/2000/svg".to_string(),
            version:"2.0".to_string(),
            stamps:vec![
                g{
                  transform:Transform{scale:2.0, tx:0.0, ty:0.0, rotate:8.0, midx:64.0, midy:64.0},
                    image:image{
                        x:0,
                        y:0,
                        height:128,
                        width:128,
                        href:"simpler.svg".to_string(),
                    }
                },
                g{
                  transform:Transform{scale:1.0, tx:290.0, ty:80.0, rotate:220.0, midx:64.0, midy:64.0},
                    image:image{
                        x:0,
                        y:0,
                        height:128,
                        width:128,
                        href:"simpler.svg".to_string(),
                    }                        
                },
            ],
        };
        use super::serde_xml_rs::from_str;
        let svg_deserialized: SVG = from_str(s).unwrap();
        assert_eq!(svg_deserialized, svg_struct);
        let svg_serialized = svg_struct.to_string().unwrap();
        eprintln!("{}",svg_serialized);
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
  fn test_regex() {
    let tform: regex::Regex = regex::Regex::new(super::TFORM_REGEX_STR).unwrap();
    tform.captures("translate(290, 80) translate(64, 64) rotate(220) translate(-64, -64)").unwrap();
  }
}
