use super::serde_xml_rs::{from_str, Error};

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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct image {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub href: String,
}

impl image {
    fn to_string(&self) -> Result<String,Error> {
        let mut scratch = String::new();
        Ok(format!(
            "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"{}\"/>",
            self.x,self.y,self.width,self.height,attr_escape(&self.href, &mut scratch),
        ))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct g {
    pub transform: String,
    #[serde(rename="$value")]
    pub image: image,
}

impl g {
    fn to_string(&self) -> Result<String,Error> {
        let mut scratch = String::new();
        Ok(format!(
            "<g transform=\"{}\">\n{}\n</g>",
            attr_escape(&self.transform, &mut scratch),
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
    fn to_string(&self) -> Result<String,Error> {
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
    use super::*;
    #[test]
    fn test_basic_serde() {
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
                    transform:"scale(2) translate(64, 64) rotate(8) translate(-64, -64)".to_string(),
                    image:image{
                        x:0,
                        y:0,
                        height:128,
                        width:128,
                        href:"simpler.svg".to_string(),
                    }
                },
                g{
                    transform:"translate(290, 80) translate(64, 64) rotate(220) translate(-64, -64)".to_string(),
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
        let svg_deserialized: SVG = super::from_str(s).unwrap();
        assert_eq!(svg_deserialized, svg_struct);
        let svg_serialized = svg_struct.to_string().unwrap();
        eprintln!("{}",svg_serialized);
        assert_eq!(svg_serialized, s);
    }
    #[test]
    fn test_attr_escape() {
        let mut scratch = String::new();
        assert_eq!("HELLOTE", attr_escape(&"HELLOTE".to_string(), &mut scratch));
        assert_eq!("HEL&lt;LOTE", attr_escape(&"HEL<LOTE".to_string(), &mut scratch));
        assert_eq!("HEL&lt;LOTE&gt;", attr_escape(&"HEL<LOTE>".to_string(), &mut scratch));
        assert_eq!("HEL&lt;LOTE&amp;", attr_escape(&"HEL<LOTE&".to_string(), &mut scratch));
        assert_eq!("HEL&quot;LOTE&apos;", attr_escape(&"HEL\"LOTE'".to_string(), &mut scratch));
        assert_eq!("H\u{0026bE}EL&quot;LOTE&apos;",
                   attr_escape(&"H\u{0026bE}EL\"LOTE'".to_string(), &mut scratch));
    }
}
