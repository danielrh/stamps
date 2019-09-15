extern crate sdl2;
extern crate stamps;
use stamps::{SVG, HrefAndClipMask, Polygon};
use std::time;
use std::string::String;
use std::collections::HashMap;
use std::env;
use std::vec::Vec;
use std::path::Path;
use std::fs;
use sdl2::event::Event;
use sdl2::image::{LoadSurface, InitFlag};
use sdl2::keyboard::Keycode;
use sdl2::mouse::Cursor;
use sdl2::pixels::Color;
use std::io;
use std::io::{Read, Write};
use sdl2::rect::{Rect, Point};
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Texture;
static DESIRED_DURATION_PER_FRAME:time::Duration = time::Duration::from_millis(1);
static START_DURATION_PER_FRAME:time::Duration = time::Duration::from_millis(200);
static RELAXED_DURATION_PER_FRAME:time::Duration = time::Duration::from_millis(1);
static DELTA_DURATION_PER_FRAME:time::Duration = time::Duration::from_millis(75);

const MOUSE_CONSTANT: i32 = 1;
const ROT_CONSTANT: f64 = 1.0;
struct TextureSurface<'r> {
    texture: Texture<'r>,
    surface: Surface<'r>,
    name: String,
}

fn mouse_move(delta:i32, repeat:time::Duration) -> i32 {
    if repeat <= DESIRED_DURATION_PER_FRAME {
        delta * 4
    } else {
        delta
    }
}


fn box_intersect (t0: &stamps::Transform, t1: &stamps::Transform) -> bool {
    stamps::poly_edge_intersect(&t0.to_bbox(), &t1.to_bbox())
}
fn constrain_mask_transform(t: &mut stamps::Transform, width: u32, height: u32) {
    if t.tx > width as f64 {
        t.tx = width as f64;
    }
    if t.ty > height as f64 {
        t.ty = height as f64;
    }
    if t.tx + 2. * t.midx < 0.0 {
        t.tx = -2.  * t.midx;
    }
    if t.ty + 2. * t.midy < 0.0 {
        t.ty = -2. * t.midy;
    }
}
macro_rules! make_texture_surface {
    ($texture_creator: expr, $surf: expr, $name: expr) => (match $texture_creator.create_texture_from_surface(&$surf) {
        Ok(tex) => Ok(TextureSurface{
            texture:tex,
            surface:$surf,
            name:$name,
        }),
        Err(e) => Err(format!("{:?}", e)),
    });
}
/*

impl<'r> TextureSurface<'r> {
    fn new_for_render_target(tc: &mut Canvas<Surface<'r>>, mut s:Surface<'r>) -> Result<Self, String> {
        match tc.texture_creator().create_texture_from_surface(&mut s) {
            Ok(tex) => Ok(TextureSurface(tex, s)),
            Err(e) => Err(format!("{:?}", e)),
        }
    }
    fn new(tc: &mut WindowCanvas, mut s:Surface<'r>) -> Result<Self, String> {
        match tc.texture_creator().create_texture_from_surface(&mut s) {
            Ok(tex) => Ok(TextureSurface(tex, s)),
            Err(e) => Err(format!("{:?}", e)),
        }
    }
}
 */
#[derive(Clone, Debug)]
struct InventoryItem {
    stamp_index: usize,
    stamp_source: Rect,
    stamp_name: String,
    rot_delta: f64,
}
#[derive(Clone, Debug)]
struct InventoryKey{
    name: String,
    clip: String,
}
struct Arrangement{
    svg: stamps::SVG,
    dirty: bool,
    undo: Vec<stamps::g>,
}
impl Arrangement {
    pub fn new(svg: stamps::SVG) -> Self {
        Arrangement{svg:svg, dirty:true, undo:Vec::new()}
    }
    pub fn get_mut(&mut self) -> &mut stamps::SVG {
        self.dirty = true;
        &mut self.svg
    }
    pub fn get(&self) -> &stamps::SVG{
        &self.svg
    }
}

fn round_up_to_golden(angle: f64) -> f64 {
    let mut iangle = ((angle % 360.) as i32) / 15;
    iangle += if iangle % 6 == 0 || iangle % 6 == 4 {
        2
    } else {
        1
    };
    iangle *= 15;
    if iangle < 0 {
        iangle += 360;
    }
    if iangle >= 360 {
        iangle -= 360;
    }
    iangle as f64
}
fn round_down_to_golden(angle: f64) -> f64 {
    let mut iangle = ((angle % 360.) as i32) / 15;
    iangle -= if iangle % 6 == 0 || iangle % 6 == 2 {
        2
    } else {
        1
    };
    iangle *= 15;
    if iangle < 0 {
        iangle += 360;
    }
    iangle as f64
}

struct SceneGraph {
    inventory: Vec<InventoryItem>,
    inventory_map: HashMap<HrefAndClipMask, usize>,
    arrangement: Arrangement,
}
impl SceneGraph {
  pub fn hit_test(&self, x:i32, y:i32) -> Option<InventoryItem> {
      for item in self.inventory.iter() {
      if x >= item.stamp_source.x() && x <= item.stamp_source.width() as i32 + item.stamp_source.x() &&
              y >= item.stamp_source.y() && y <= item.stamp_source.height() as i32 + item.stamp_source.y() {
          return Some(item.clone())
        }
    }
   None
  }
  fn prepare_textures<'a>(
        &mut self, texture_creator: &'a sdl2::render::TextureCreator<sdl2::video::WindowContext>,images: &mut Images<'a>) -> Result<(), String> {
      if !self.arrangement.dirty {
          return Ok(());
      }
      let mut polygon_intercepts = Vec::<i32>::new();
      for g in self.arrangement.svg.stamps.iter() {
          if let None = self.inventory_map.get(&g.image.href) {
              // now we need to prerender
              let mut polygon = Polygon::default();
              for clips in self.arrangement.svg.defs.clipPath.iter() {
                  if "url(#".to_string() + &clips.id + ")" == g.image.href.clip {
                      polygon = clips.polygon.clone();
                  }
              }
              let mut dst_surface: Surface;
              let width;
              let height;
              let name;
              if let Some(img) = self.inventory_map.get(&HrefAndClipMask{
                  url:g.image.href.url.clone(),
                  clip:String::new(),
              }) {
                  let img_texture = &images.stamps[*img];
                  let src_surface = &img_texture.surface;
                  name = img_texture.name.clone();
                  width = src_surface.width();
                  height = src_surface.height();
                  dst_surface = Surface::new(width, height, PixelFormatEnum::RGBA8888)?;
                  src_surface.blit(
                      Rect::new(0,0,width,height),
                      &mut dst_surface,
                      Rect::new(0,0,width,height),
                      )?;
              } else {
                  continue
              }
              let pitch = dst_surface.pitch();
              let polygon_points = &polygon.points;
              let last_point_index = polygon_points.len().wrapping_sub(1);
              dst_surface.with_lock_mut(|data:&mut[u8]| {
                  // rasterize our friend the clip polygon
                  for y in 0..height {
                      polygon_intercepts.resize(polygon_points.len() + 2, 0);
                      let y_byte_offset = y as usize * pitch as usize;
                      polygon_intercepts[0] = std::i32::MIN; // clear the opposite of clip mask
                      let mut num_intercepts = 1;
                      for (index, point0) in polygon_points.iter().enumerate() {
                          let prev_point_index = if index == 0 {
                              last_point_index
                          } else {
                              index - 1
                          };
                          let point1 = polygon_points[prev_point_index];
                          let x0;
                          let x1;
                          let y0;
                          let y1;
                          if point0.1 < point1.1 {
                              x0 = point0.0;
                              x1 = point1.0;
                              y0 = point0.1;
                              y1 = point1.1
                          } else if point0.1 > point1.1 {
                              x0 = point1.0;
                              x1 = point0.0;
                              y0 = point1.1;
                              y1 = point0.1
                          } else {
                              continue
                          }
                          if ((y as f64) >= y0) && ((y as f64) < y1) {
                              polygon_intercepts[num_intercepts] = ((y as f64 - y0) * (x1 as f64- x0 as f64) / (y1 as f64 - y0 as f64) + x0 as f64) as i32;
                              num_intercepts += 1;
                          }
                      }
                      polygon_intercepts.resize(num_intercepts, 0);
                      polygon_intercepts.sort();
                      if (polygon_intercepts.len() & 1) == 1 {
                          polygon_intercepts.push(std::i32::MAX);
                      }
                      for i in 0..polygon_intercepts.len()/2 {
                          use std::cmp::{min, max};
                          let start = max(min(polygon_intercepts[i*2], width as i32 - 1), 0);
                          let end = max(min(polygon_intercepts[i*2 + 1], width as i32 - 1), 0);
                          if start < end {
                              for x in start..end {
                                  data[y_byte_offset + x as usize * 4] = 0;
                                  data[y_byte_offset + x as usize  * 4 + 1] = 0;
                                  data[y_byte_offset + x as usize  * 4 + 2] = 0;
                                  data[y_byte_offset + x as usize  * 4 + 3] = 0;
                              }
                          }
                      }
                  }
              });
              let new_index = images.stamps.len();
              images.stamps.push(make_texture_surface!(texture_creator, dst_surface, name)?);
              eprintln!("Making texture surface {:?} {}\n", g.image.href.clone(), new_index);
              self.inventory_map.insert(
                  g.image.href.clone(),
                   new_index,
                  );
                   
          }
      }
      self.arrangement.dirty = false;
      Ok(())
  }
}

struct Images<'r> {
    mask: TextureSurface<'r>,
    default_cursor: TextureSurface<'r>,
    stamps: Vec<TextureSurface<'r>>,
    max_selectable_stamp: usize,
}

#[derive(Clone,PartialEq)]
struct CursorTransform {
    mouse_x: i32,
    mouse_y: i32,
    transform: stamps::Transform,    
}

struct SceneState{
    scene_graph: SceneGraph,
    cursor_transform: CursorTransform,
    duration_per_frame: time::Duration, // how long to wait while key is held down
    mask_transforms: [stamps::Transform;2],
    last_return_mouse: Option<CursorTransform>,
    cursor: Cursor,
    active_stamp: Option<usize>,
    stamp_used: bool,
    camera_transform: stamps::Transform,
    save_file_name: String,
    window_width: u32,
    window_height: u32,
}

impl SceneState {
  fn compute_stamps_location(&mut self, canvas_viewport: Rect, images: &Images) {
    self.scene_graph.inventory.resize(images.stamps.len(), InventoryItem{stamp_index:0,rot_delta:0.0,stamp_source:canvas_viewport, stamp_name:String::new()});
    let mut w_offset = 0i32;
    let mut h_offset = 0i32;
    let mut max_width = 0i32;
      static ROT_DELTAS:&'static[(&'static str, f64)]= &[
          ("rect", 90.0),
          ("lhalframp", 90.0), ("lquartramp", 90.0),
          ("rhalframp", -90.0), ("rquartramp", -90.0),
      ];
    for (index, (stamp, inventory)) in images.stamps.iter().zip(self.scene_graph.inventory.iter_mut()).enumerate() {
      if h_offset + stamp.surface.height() as i32 > canvas_viewport.height() as i32 {
        h_offset = 0;
        w_offset += max_width;
        max_width = 0;        
      }
      inventory.stamp_index = index;
        inventory.stamp_name = stamp.name.clone();
        for rot_delta in ROT_DELTAS {
            if stamp.name.contains(rot_delta.0) {
                inventory.rot_delta = rot_delta.1;
            }
        }
      inventory.stamp_source = Rect::new(w_offset, h_offset, stamp.surface.width(), stamp.surface.height());
      self.scene_graph.inventory_map.insert(HrefAndClipMask{url:inventory.stamp_name.clone(), clip:String::new()}, index);
      max_width = std::cmp::max(max_width, stamp.surface.width() as i32);
      h_offset += stamp.surface.height() as i32;
    }
  }
    fn render<T:sdl2::render::RenderTarget>(&self, canvas: &mut sdl2::render::Canvas<T>, images: &Images) -> Result<(),String> {
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
        canvas.clear();
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        //canvas.fill_rect(Rect::new(self.mouse_x, self.mouse_y, 1, 1))?;
        for g in self.scene_graph.arrangement.get().stamps.iter() {
            if let Some(index) = self.scene_graph.inventory_map.get(&g.image.href) {
                let final_transform = stamps::compose(&self.camera_transform, &g.transform);
                let img = &images.stamps[*index];
                canvas.copy_ex(
                    &img.texture,
                    None,
                    Some(Rect::new(final_transform.tx as i32, final_transform.ty as i32, g.image.width, g.image.height)),
                    final_transform.rotate,
                    Point::new(final_transform.midx as i32, final_transform.midy as i32),
                    false,
                    false,
                ).map_err(|err| format!("{:?}", err))?;
            } else {
                // skip drawing unknown item
            }
        }
        for stamp_loc in self.scene_graph.inventory.iter() {
          let dest = stamp_loc.stamp_source;
          let image = &images.stamps[stamp_loc.stamp_index];
          canvas.copy_ex(
            &image.texture,
            None, Some(dest),
            0.0,
            Point::new(0,0),//centre
            false,//horiz
            false,//vert
          ).map_err(|err| format!("{:?}", err))?;
        }
        if let Some(active_stamp) = self.active_stamp {
            let img = &images.stamps[active_stamp];
            canvas.copy_ex(
                &img.texture,
                None,
                Some(Rect::new(self.cursor_transform.mouse_x - img.surface.width()as i32/2,
                               self.cursor_transform.mouse_y - img.surface.height() as i32/2,
                               img.surface.width(),
                               img.surface.height())),
                self.cursor_transform.transform.rotate,
                Point::new(self.cursor_transform.transform.midx as i32,
                           self.cursor_transform.transform.midy as i32),//centre
                false,//horiz
                false,//vert
            ).map_err(|err| format!("{:?}", err))?;            
        } else {
            canvas.copy_ex(
                &images.default_cursor.texture,
                None,
                Some(Rect::new(self.cursor_transform.mouse_x, self.cursor_transform.mouse_y,
                               images.default_cursor.surface.width(), images.default_cursor.surface.height())),
                0.0,
                Point::new(0,0),//centre
                false,//horiz
                false,//vert
            ).map_err(|err| format!("{:?}", err))?;
        }
        for mask in self.mask_transforms.iter() {
            let final_mask_transform = stamps::compose(&self.camera_transform, mask);
            canvas.copy_ex(
                &images.mask.texture,
                None,
                    Some(Rect::new(final_mask_transform.tx as i32, final_mask_transform.ty as i32, (2.0 * final_mask_transform.midx) as u32, (2.0 * final_mask_transform.midy) as u32)),
                    final_mask_transform.rotate,
                    Point::new(final_mask_transform.midx as i32, final_mask_transform.midy as i32),
                    false,
                    false,
                ).map_err(|err| format!("{:?}", err))?;
        }
        canvas.present();
        Ok(())
    }
    fn apply_keys(&mut self, keys_down: &HashMap<Keycode, ()>, new_key: Option<Keycode>, repeat:bool) {
        if keys_down.len() != 0{
            //eprintln!("KEY PRESS {:?}; REPEAT {} {:?}?", keys_down, repeat, new_key);
        }
        let shifted_index = (keys_down.contains_key(&Keycode::LShift) as usize) | (keys_down.contains_key(&Keycode::RShift) as usize);
        if keys_down.contains_key(&Keycode::Left) {
            self.cursor_transform.mouse_x -= mouse_move(MOUSE_CONSTANT, self.duration_per_frame);
            self.clear_cursor_if_stamp_used();
        }
        if keys_down.contains_key(&Keycode::Right) {
            self.cursor_transform.mouse_x += mouse_move(MOUSE_CONSTANT, self.duration_per_frame);
            self.clear_cursor_if_stamp_used();
        }
        if keys_down.contains_key(&Keycode::Up) {
            self.cursor_transform.mouse_y -= mouse_move(MOUSE_CONSTANT, self.duration_per_frame);
            self.clear_cursor_if_stamp_used();
        }
        if keys_down.contains_key(&Keycode::Down) {
            self.cursor_transform.mouse_y += mouse_move(MOUSE_CONSTANT, self.duration_per_frame);
            self.clear_cursor_if_stamp_used();
        }
        if keys_down.contains_key(&Keycode::W) {
            self.camera_transform.ty += mouse_move(MOUSE_CONSTANT, self.duration_per_frame) as f64;
        }
        if keys_down.contains_key(&Keycode::A) {
            self.camera_transform.tx -= mouse_move(MOUSE_CONSTANT, self.duration_per_frame) as f64;
        }
        if keys_down.contains_key(&Keycode::S) {
            self.camera_transform.ty -= mouse_move(MOUSE_CONSTANT, self.duration_per_frame) as f64;
        }
        if keys_down.contains_key(&Keycode::D) {
            self.camera_transform.tx += mouse_move(MOUSE_CONSTANT, self.duration_per_frame) as f64;
        }
        if keys_down.contains_key(&Keycode::Escape) {
            write_from_string(Path::new(&self.save_file_name),
                              &self.scene_graph.arrangement.get().to_string().map_err(
                                  |err| format!("{:?}", err)).unwrap()).map_err(
                |err| format!("{:?}", err)).unwrap();
            std::process::exit(0);
        }
        /*
        if keys_down.contains_key(&Keycode::Q) {
            self.camera_transform.scale /= 1.03125 / 4.;
        }
        if keys_down.contains_key(&Keycode::E) {
            self.camera_transform.scale *= 1.03125 / 4.;
        }*/
        if keys_down.contains_key(&Keycode::I) {
            self.mask_transforms[shifted_index].ty -= mouse_move(MOUSE_CONSTANT, self.duration_per_frame) as f64;
    /*        if self.mask_transforms[shifted_index].ty < 0.0 {
                let adjust = -self.mask_transforms[shifted_index].ty;
                self.mask_transforms[shifted_index].ty < 0.0; // shift the whole artwork down
                self.camera_transform.ty += adjust;
                for (mask_index, mask) in self.mask_transforms.iter_mut().enumerate() {
                    if mask_index != shifted_index {
                        mask.ty += adjust;
                    }
                }
                for g in self.scene_graph.arrangement.get_mut().stamps.iter_mut() {
                    g.transform.ty += adjust;
                }
            }*/
            constrain_mask_transform(&mut self.mask_transforms[shifted_index], self.window_width, self.window_height)
        }
        if keys_down.contains_key(&Keycode::J) {
            self.mask_transforms[shifted_index].tx -= mouse_move(MOUSE_CONSTANT, self.duration_per_frame) as f64;
            constrain_mask_transform(&mut self.mask_transforms[shifted_index], self.window_width, self.window_height)
        }
        if keys_down.contains_key(&Keycode::K) {
            self.mask_transforms[shifted_index].ty += mouse_move(MOUSE_CONSTANT, self.duration_per_frame) as f64;
            constrain_mask_transform(&mut self.mask_transforms[shifted_index], self.window_width, self.window_height)
        }
        if keys_down.contains_key(&Keycode::L) {
            self.mask_transforms[shifted_index].tx += mouse_move(MOUSE_CONSTANT, self.duration_per_frame) as f64;
            constrain_mask_transform(&mut self.mask_transforms[shifted_index], self.window_width, self.window_height)
        }
        if /*keys_down.contains_key(&Keycode::LParen) ||*/ keys_down.contains_key(&Keycode::U) {
            self.mask_transforms[shifted_index].rotate -= ROT_CONSTANT;
            constrain_mask_transform(&mut self.mask_transforms[shifted_index], self.window_width, self.window_height)
        }
        if /*keys_down.contains_key(&Keycode::RParen) ||*/ keys_down.contains_key(&Keycode::O) {
            self.mask_transforms[shifted_index].rotate += ROT_CONSTANT;
            constrain_mask_transform(&mut self.mask_transforms[shifted_index], self.window_width, self.window_height)
        }
        if keys_down.contains_key(&Keycode::Backspace) && !repeat {
            if keys_down.contains_key(&Keycode::LShift) || keys_down.contains_key(&Keycode::RShift) {
                if self.scene_graph.arrangement.undo.len() != 0 {
                    self.scene_graph.arrangement.svg.stamps.push(self.scene_graph.arrangement.undo.pop().unwrap());
                    self.scene_graph.arrangement.dirty = true;
                }
            } else {
                if self.scene_graph.arrangement.svg.stamps.len() != 0 {
                    self.scene_graph.arrangement.undo.push(self.scene_graph.arrangement.svg.stamps.pop().unwrap());
                    self.scene_graph.arrangement.dirty = true;
                }
            }
        }
        if keys_down.contains_key(&Keycode::Period) || keys_down.contains_key(&Keycode::KpPeriod) || keys_down.contains_key(&Keycode::Insert) {
            if keys_down.contains_key(&Keycode::LShift) || keys_down.contains_key(&Keycode::RShift) {
                if !repeat {
                    self.duration_per_frame = RELAXED_DURATION_PER_FRAME;
                    self.cursor_transform.transform.rotate = round_up_to_golden(self.cursor_transform.transform.rotate)
                }
            } else {
                self.cursor_transform.transform.rotate += ROT_CONSTANT;
            }
        }
        if keys_down.contains_key(&Keycode::Comma) || keys_down.contains_key(&Keycode::Kp0) ||  keys_down.contains_key(&Keycode::Delete) {
            if keys_down.contains_key(&Keycode::LShift) || keys_down.contains_key(&Keycode::RShift) {
                if !repeat{
                    self.duration_per_frame = RELAXED_DURATION_PER_FRAME;
                    self.cursor_transform.transform.rotate = round_down_to_golden(self.cursor_transform.transform.rotate)
                }
            } else {
                self.cursor_transform.transform.rotate -= ROT_CONSTANT;
            }
        }
        if keys_down.contains_key(&Keycode::KpEnter) {
            if let Some(last_transform) = &self.last_return_mouse {
                if *last_transform != self.cursor_transform || !repeat {
                    self.click();
                }
            } else {
                self.click();
            }
            self.last_return_mouse = Some(self.cursor_transform.clone())
        } else {
            self.last_return_mouse = None; // other keypresses clear this
        }
        if let Some(Keycode::Return) = new_key {
            if !repeat {
                self.click();
            }
        }
        if let Some(Keycode::Space) = new_key {
            if !repeat {
                self.click();
            }
        }
    }
    fn click(&mut self) {
        if let Some(hit) = self.scene_graph.hit_test(self.cursor_transform.mouse_x,
                                                     self.cursor_transform.mouse_y) {
            self.active_stamp = Some(hit.stamp_index);
            self.stamp_used = false;
            self.cursor_transform.transform = stamps::Transform::new(hit.stamp_source.width(),
                                                                     hit.stamp_source.height());
            self.cursor_transform.transform.rotate += hit.rot_delta;
        } else if let Some(active_stamp) = self.active_stamp{ // draw the stamp
            let mut transform = self.cursor_transform.transform.clone();
            transform.rotate -= self.camera_transform.rotate;
            transform.tx = self.cursor_transform.mouse_x as f64 - self.cursor_transform.transform.midx - self.camera_transform.tx;
            transform.ty = self.cursor_transform.mouse_y as f64 - self.cursor_transform.transform.midy - self.camera_transform.ty;
            let new_item_url = self.scene_graph.inventory[active_stamp].stamp_name.clone();
            // add clip mask
            let mut any_intersect = false;
            for mask in self.mask_transforms.iter() {
                if box_intersect(mask, &transform) {
                    any_intersect = true;
                }
            }
            let clip_mask;
            if any_intersect {
                let mut points = Vec::<stamps::F64Point>::new();
                points.push((-transform.midx * 4.,-transform.midy * 4.));
                points.push((transform.midx * 4.,-transform.midy * 4.));
                points.push((transform.midx * 4., transform.midy * 4.));
                points.push((-transform.midx * 4., transform.midy * 4.));
                points.push((-transform.midx * 4.,-transform.midy * 4.));
               let ret_location = (-transform.midx * 4.,-transform.midy * 4.);
                for mask in self.mask_transforms.iter() {
                    use stamps::ftransform;
                    use stamps::itransform;
                    points.push(itransform(&transform, ftransform(mask, (0.,0.))));
                    points.push(itransform(&transform, ftransform(mask, (mask.midx * 2.,0.))));
                    points.push(itransform(&transform, ftransform(mask, (mask.midx * 2.,mask.midy * 2.))));
                    points.push(itransform(&transform, ftransform(mask, (0.,mask.midy  * 2.))));
                    points.push(itransform(&transform, ftransform(mask, (0.,0.))));
                    points.push(ret_location.clone());
                }
                let index = self.scene_graph.arrangement.get_mut().defs.clipPath.len();
                let id = format!("{}", index);
                clip_mask = "url(#".to_string() + &id + ")";
                self.scene_graph.arrangement.get_mut().defs.clipPath.push(stamps::ClipPath{
                    id:id,
                    polygon:Polygon{
                        points:points,
                    },
                });
            } else {
                clip_mask = String::new();
            }
            // end add clip mask
            self.scene_graph.arrangement.get_mut().add(
                transform,
                new_item_url,
                clip_mask,
            );
            self.stamp_used = true;
        }
    }
    fn clear_cursor_if_stamp_used(&mut self) {
        if self.stamp_used {
            if let Some(_) = self.scene_graph.hit_test(self.cursor_transform.mouse_x,
                                                       self.cursor_transform.mouse_y) {
                self.active_stamp = None;
            }
        }
    }
}

fn process(state: &mut SceneState, images: &mut Images, event: sdl2::event::Event, keys_down: &mut HashMap<Keycode, ()>) -> Result<bool,String>{
    let mut key_encountered = false;
    match event {
        Event::Quit{..} => {
            write_from_string(Path::new(&state.save_file_name),
                              &state.scene_graph.arrangement.get().to_string().map_err(
                                  |err| format!("{:?}", err))?).map_err(
                |err| format!("{:?}", err))?;
            return Err("Exit".to_string())
        },
        Event::KeyDown {keycode: Option::Some(key_code), ..} =>{
            let repeat;
             if let None = keys_down.insert(key_code, ()) {
                repeat = false;
                for (key,_)in keys_down.iter() {
                    eprintln!("Key is down {}\n", *key)
                }
            } else {
                //eprintln!("EXTRA?");
                return Ok(false);
            }
            key_encountered = true;
            state.apply_keys(&keys_down, Some(key_code), repeat);
        },
        Event::KeyUp {keycode: Option::Some(key_code), ..} =>
        {
            state.last_return_mouse = None; // other keypresses clear this
            keys_down.remove(&key_code);
        },
        Event::MouseButtonDown {x, y, ..} => {
            state.cursor_transform.mouse_x = x;
            state.cursor_transform.mouse_y = y;
            state.click();
        }
        Event::MouseMotion {x, y, ..} => {
            state.cursor_transform.mouse_x = x;
            state.cursor_transform.mouse_y = y;
            state.clear_cursor_if_stamp_used();
        }
        Event::Window{win_event:sdl2::event::WindowEvent::Resized(width,height),..} => {
          state.window_width = width as u32;
          state.window_height = height as u32;
          state.compute_stamps_location(Rect::new(0,0,width as u32,height as u32), images);
        }
        Event::Window{win_event:sdl2::event::WindowEvent::SizeChanged(width,height),..} => {
          state.window_width = width as u32;
          state.window_height = height as u32;
          state.compute_stamps_location(Rect::new(0,0,width as u32,height as u32), images);
        }
        _ => {}
    }
    Ok(key_encountered)
}

fn process_dir<F: FnMut(&fs::DirEntry) -> Result<(), io::Error>>(dir: &Path, cb: &mut F) -> Result<(), io::Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            process_dir(&path, cb)?;
        } else {
            cb(&entry)?;
        }
    }
    Ok(())
}

pub fn run(mut svg: SVG, save_file_name: &str, dir: &Path) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG)?;
    let window = video_subsystem.window("rust-sdl2 demo: Cursor", 800, 600)
      .position_centered()
      .build()
      .map_err(|e| e.to_string())?;

    let wsize = window.size();
    let mut canvas = window.into_canvas().software().build().map_err(|e| e.to_string())?;
    let mut keys_down = HashMap::<Keycode, ()>::new();
    let surface = Surface::from_file(dir.join("cursor.png"))
        .map_err(|err| format!("failed to load cursor image: {}", err))?;
    svg.resize(wsize.0, wsize.1);
    let mask_surface_path = dir.join("mask.png");
    let mask_surface_name = mask_surface_path.to_str().unwrap().to_string();
    let mask_surface = Surface::from_file(mask_surface_path)
        .map_err(|err| format!("Failed to load mask paper image: {}", err))?;
    let mut scene_state = SceneState {
        scene_graph:SceneGraph{
            inventory:Vec::new(),
            inventory_map:HashMap::new(),
            arrangement:Arrangement::new(svg),
        },
        cursor_transform: CursorTransform {
            mouse_x:0,
            mouse_y:0,
            transform: stamps::Transform::new(0,0),
        },
        mask_transforms: [
            stamps::Transform::new(mask_surface.width(), mask_surface.height()),
            stamps::Transform::new(mask_surface.width(), mask_surface.height()),
        ],
        duration_per_frame:START_DURATION_PER_FRAME,
        last_return_mouse: None,
        active_stamp: None,
        stamp_used: false,
        camera_transform: stamps::Transform::new(0, 0),
        cursor:Cursor::from_surface(surface, 0, 0).map_err(
            |err| format!("failed to load cursor: {}", err))?,
        save_file_name: save_file_name.to_string(),
        window_width: canvas.viewport().width(),
        window_height: canvas.viewport().height(),
    };
    scene_state.mask_transforms[0].tx = 10.0 - scene_state.mask_transforms[0].midx * 2.0;
    scene_state.mask_transforms[1].tx = 0.0;
    scene_state.mask_transforms[1].ty = 10.0 - scene_state.mask_transforms[0].midy * 2.0;
    let cursor_surface_path = dir.join("cursor.png");
    let cursor_surface_name = cursor_surface_path.to_str().unwrap().to_string();
    let cursor_surface = Surface::from_file(cursor_surface_path)
        .map_err(|err| format!("failed to load cursor image: {}", err))?;
    let texture_creator = canvas.texture_creator();
    
    let mut images = Images{
        mask:make_texture_surface!(texture_creator, mask_surface, mask_surface_name)?,
        default_cursor:make_texture_surface!(texture_creator, cursor_surface, cursor_surface_name)?,
        stamps:Vec::new(),
        max_selectable_stamp:0,
    };
    process_dir(&dir.join("stamps"), &mut |p:&fs::DirEntry| {
        let stamp_surface = Surface::from_file(p.path()).map_err(
            |err| io::Error::new(io::ErrorKind::Other, format!("{}: {}", p.path().to_str().unwrap_or("??"), err)))?;
        images.stamps.push(make_texture_surface!(texture_creator, stamp_surface, p.path().to_str().unwrap().to_string()).map_err(
            |err| io::Error::new(io::ErrorKind::Other, format!("{}: {}", p.path().to_str().unwrap_or("?X?"), err)))?);
        images.max_selectable_stamp += 1;
        Ok(())
    }).map_err(|err| format!("Failed to load stamp {}", err))?;
    //images.stamps.push(make_texture_surface!(texture_creator, xcursor_surface)?);
    scene_state.cursor.set();
    scene_state.compute_stamps_location(canvas.viewport(), &images);
    'mainloop: loop {
        let loop_start_time = time::Instant::now();
        let mut events = sdl_context.event_pump()?;
        if keys_down.len() != 0 {
            for event in events.poll_iter() {
                process(&mut scene_state, &mut images, event, &mut keys_down)?; // always break
            }
            scene_state.scene_graph.prepare_textures(&texture_creator, &mut images)?;
            scene_state.render(&mut canvas, &images)?;
            let mut process_time = loop_start_time.elapsed();
            if keys_down.len() != 0 {
                while process_time < scene_state.duration_per_frame {
                    process_time = loop_start_time.elapsed();
                    let mut any_events = false;
                    for event in events.poll_iter() {
                        process(&mut scene_state, &mut images, event, &mut keys_down)?; // always break
                        any_events = true;
                    }
                    if any_events {
                        scene_state.scene_graph.prepare_textures(&texture_creator, &mut images)?;
                        scene_state.render(&mut canvas, &mut images)?;
                    }
                }
                if scene_state.duration_per_frame > DELTA_DURATION_PER_FRAME + DESIRED_DURATION_PER_FRAME {
                    scene_state.duration_per_frame -= DELTA_DURATION_PER_FRAME;
                } else {
                    scene_state.duration_per_frame = DESIRED_DURATION_PER_FRAME;
                }
                scene_state.apply_keys(&keys_down, None, true);
                scene_state.scene_graph.prepare_textures(&texture_creator, &mut images)?;
                scene_state.render(&mut canvas, &mut images)?;
            }
        } else {
            scene_state.duration_per_frame = START_DURATION_PER_FRAME;
            for event in events.wait_iter() {
                process(&mut scene_state, &mut images, event, &mut keys_down)?;
                break;
            }
            for event in events.poll_iter() {
                process(&mut scene_state, &mut images, event, &mut keys_down)?;
            }
            scene_state.scene_graph.prepare_textures(&texture_creator, &mut images)?;
            scene_state.render(&mut canvas, &mut images)?;
        };
    }
}

fn write_from_string(filename: &Path, s: &String) -> Result<(), io::Error> {
    let mut f = fs::File::create(filename)?;
    f.write(s.as_bytes())?;
    Ok(())
}

fn read_to_string(filename: &Path) ->  Result<String, io::Error> {
    let mut f = fs::File::open(filename)?;
    let mut buffer = String::new();
    f.read_to_string(&mut buffer)?;
    Ok(buffer)
}
fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: cargo run /path/to/result");
        Ok(())
    } else {
        let save_file_name = &Path::new(&args[1]);
        let svg = if let Ok(file_data) = read_to_string(save_file_name) {
            SVG::from_str(&file_data).unwrap()
        } else {
            SVG::new(1024,768)
                
        };
        let ret = run(svg, &args[1], Path::new("assets"));
        match ret {
            Err(x) => {
                if x == "Exit" {
                    Ok(())
                } else {
                    Err(x)
                }
            },
            ret => ret,
        }
    }
}
