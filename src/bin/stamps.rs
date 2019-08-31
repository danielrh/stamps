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
static DESIRED_DURATION_PER_FRAME:time::Duration = time::Duration::from_millis(4);

const MOUSE_CONSTANT: i32 = 1;
const ROT_CONSTANT: f64 = 1.0;
struct TextureSurface<'r> {
    texture: Texture<'r>,
    surface: Surface<'r>,
    name: String,
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
}
#[derive(Clone, Debug)]
struct InventoryKey{
    name: String,
    clip: String,
}
struct Arrangement{
    svg: stamps::SVG,
    dirty: bool
}
impl Arrangement {
    pub fn new(svg: stamps::SVG) -> Self {
        Arrangement{svg:svg, dirty:true}
    }
    pub fn get_mut(&mut self) -> &mut stamps::SVG {
        self.dirty = true;
        &mut self.svg
    }
    pub fn get(&self) -> &stamps::SVG{
        &self.svg
    }
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
  fn prepare_textures<T:sdl2::render::RenderTarget>(
        &mut self, canvas: &mut sdl2::render::Canvas<T>,images: &mut Images) -> Result<(), String> {
      if !self.arrangement.dirty {
          return Ok(());
      }
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
              if let Some(img) = self.inventory_map.get(&HrefAndClipMask{
                  url:g.image.href.url.clone(),
                  clip:String::new(),
              }) {
                  let src_surface = &images.stamps[*img].surface;
                  let width = src_surface.width();
                  let height = src_surface.height();
                  dst_surface = Surface::new(width, height, PixelFormatEnum::RGBA8888)?;
                  src_surface.blit(
                      Rect::new(0,0,width,height),
                      &mut dst_surface,
                      Rect::new(0,0,width,height),
                      )?;
              } else {
                  continue
              }
              dst_surface.with_lock_mut(|data:&mut[u8]| {
                  // rasterize our friend the clip polygon
              })
          }
      }
      Ok(())
  }
}

struct Images<'r> {
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
    last_return_mouse: Option<CursorTransform>,
    cursor: Cursor,
    active_stamp: Option<usize>,
    stamp_used: bool,
    save_file_name: String,
}

impl SceneState {
  fn compute_stamps_location(&mut self, canvas_viewport: Rect, images: &Images) {
    self.scene_graph.inventory.resize(images.stamps.len(), InventoryItem{stamp_index:0,stamp_source:canvas_viewport, stamp_name:String::new()});
    let mut w_offset = 0i32;
    let mut h_offset = 0i32;
    let mut max_width = 0i32;
    for (index, (stamp, inventory)) in images.stamps.iter().zip(self.scene_graph.inventory.iter_mut()).enumerate() {
      if h_offset + stamp.surface.height() as i32 > canvas_viewport.height() as i32 {
        h_offset = 0;
        w_offset += max_width;
        max_width = 0;        
      }
      inventory.stamp_index = index;
      inventory.stamp_name = stamp.name.clone();
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
                let img = &images.stamps[*index];
                canvas.copy_ex(
                    &img.texture,
                    None,
                    Some(Rect::new(g.transform.tx as i32, g.transform.ty as i32, g.image.width, g.image.height)),
                    g.transform.rotate,
                    Point::new(g.transform.midx as i32, g.transform.midy as i32),
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
        canvas.present();
        Ok(())
    }
    fn apply_keys(&mut self, keys_down: &HashMap<Keycode, ()>, new_key: Option<Keycode>, repeat:bool) {
        if keys_down.contains_key(&Keycode::Left) {
            self.cursor_transform.mouse_x -= MOUSE_CONSTANT;
            self.clear_cursor_if_stamp_used();

        }
        if keys_down.contains_key(&Keycode::Right) {
            self.cursor_transform.mouse_x += MOUSE_CONSTANT;
            self.clear_cursor_if_stamp_used();
        }
        if keys_down.contains_key(&Keycode::Up) {
            self.cursor_transform.mouse_y -= MOUSE_CONSTANT;
            self.clear_cursor_if_stamp_used();
        }
        if keys_down.contains_key(&Keycode::Down) {
            self.cursor_transform.mouse_y += MOUSE_CONSTANT;
            self.clear_cursor_if_stamp_used();
        }
        if keys_down.contains_key(&Keycode::KpPeriod) || keys_down.contains_key(&Keycode::Insert) {
            self.cursor_transform.transform.rotate += ROT_CONSTANT;
        }
        if keys_down.contains_key(&Keycode::Kp0) ||  keys_down.contains_key(&Keycode::Delete) {
            self.cursor_transform.transform.rotate -= ROT_CONSTANT;
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
    }
    fn click(&mut self) {
        if let Some(hit) = self.scene_graph.hit_test(self.cursor_transform.mouse_x,
                                                     self.cursor_transform.mouse_y) {
            self.active_stamp = Some(hit.stamp_index);
            self.stamp_used = false;
            self.cursor_transform.transform = stamps::Transform::new(hit.stamp_source.width(),
                                                                     hit.stamp_source.height());
        } else if let Some(active_stamp) = self.active_stamp{ // draw the stamp
            let mut transform = self.cursor_transform.transform.clone();
            transform.tx = self.cursor_transform.mouse_x as f64 - self.cursor_transform.transform.midx;
            transform.ty = self.cursor_transform.mouse_y as f64 - self.cursor_transform.transform.midy;
            self.scene_graph.arrangement.get_mut().add(
                transform,
                self.scene_graph.inventory[active_stamp].stamp_name.clone(),
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
                repeat = true;
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
          state.compute_stamps_location(Rect::new(0,0,width as u32,height as u32), images);
        }
        Event::Window{win_event:sdl2::event::WindowEvent::SizeChanged(width,height),..} => {
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
        last_return_mouse: None,
        active_stamp: None,
        stamp_used: false,
        
        cursor:Cursor::from_surface(surface, 0, 0).map_err(
            |err| format!("failed to load cursor: {}", err))?,
        save_file_name: save_file_name.to_string(),
    };
    let cursor_surface_path = dir.join("cursor.png");
    let cursor_surface_name = cursor_surface_path.to_str().unwrap().to_string();
    let cursor_surface = Surface::from_file(cursor_surface_path)
        .map_err(|err| format!("failed to load cursor image: {}", err))?;
    let texture_creator = canvas.texture_creator();
    let mut images = Images{
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
            scene_state.scene_graph.prepare_textures(&mut canvas, &mut images)?;
            scene_state.render(&mut canvas, &images)?;
            let process_time = loop_start_time.elapsed();
            if keys_down.len() != 0 && process_time < DESIRED_DURATION_PER_FRAME {
                std::thread::sleep(DESIRED_DURATION_PER_FRAME - process_time);
                scene_state.apply_keys(&keys_down, None, true);
                scene_state.render(&mut canvas, &mut images)?;
            }
        } else {
            for event in events.wait_iter() {
                process(&mut scene_state, &mut images, event, &mut keys_down)?;
                break;
            }
            for event in events.poll_iter() {
                process(&mut scene_state, &mut images, event, &mut keys_down)?;
            }
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
