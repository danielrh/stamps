extern crate sdl2;

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
use sdl2::rect::{Rect, Point};
use sdl2::surface::Surface;
use sdl2::render::{Canvas, RenderTarget, Texture, WindowCanvas};
static DESIRED_DURATION_PER_FRAME:time::Duration = time::Duration::from_millis(4);

const MOUSE_CONSTANT: i32 = 1;
struct TextureSurface<'r>(Texture<'r>,Surface<'r>);
macro_rules! make_texture_surface {
    ($texture_creator: expr, $surface: expr) => (match $texture_creator.create_texture_from_surface(&$surface) {
        Ok(tex) => Ok(TextureSurface(tex, $surface)),
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
#[derive(Clone)]
struct InventoryItem {
    stamp_index: usize,
    stamp_source: Rect,
}
struct SceneGraph {
    inventory: Vec<InventoryItem>,
}
struct Images<'r> {
   default_cursor: TextureSurface<'r>,
   stamps: Vec<TextureSurface<'r>>,
}

struct SceneState{
    scene_graph: SceneGraph,
    mouse_x: i32,
    mouse_y: i32,
    cursor: Cursor,
}
impl SceneState {
  fn compute_stamps_location(&mut self, canvas_viewport: Rect, images: &Images) {
    self.scene_graph.inventory.resize(images.stamps.len(), InventoryItem{stamp_index:0,stamp_source:canvas_viewport});
    let mut w_offset = 0i32;
    let mut h_offset = 0i32;
    let mut max_width = 0i32;
    let mut last_height = 0;
    for (index, (stamp, inventory)) in images.stamps.iter().zip(self.scene_graph.inventory.iter_mut()).enumerate() {
      if h_offset + stamp.1.height() as i32 > canvas_viewport.height() as i32 {
        h_offset = 0;
        w_offset += max_width;
        max_width = 0;        
      }
      inventory.stamp_index = index;
      inventory.stamp_source = Rect::new(w_offset, h_offset, stamp.1.width(), stamp.1.height());
      max_width = std::cmp::max(max_width, stamp.1.width() as i32);
      h_offset += stamp.1.height() as i32;
    }
  }
    fn render<T:sdl2::render::RenderTarget>(&self, canvas: &mut sdl2::render::Canvas<T>, images: &Images) -> Result<(),String> {
        canvas.set_draw_color(Color::RGBA(0, 64, 0, 255));
        canvas.clear();
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
        //canvas.fill_rect(Rect::new(self.mouse_x, self.mouse_y, 1, 1))?;
        canvas.copy_ex(
            &images.default_cursor.0,
            None, Some(Rect::new(self.mouse_x, self.mouse_y, images.default_cursor.1.width(), images.default_cursor.1.height())),
            0.0,
            Point::new(0,0),//centre
            false,//horiz
            false,//vert
        );
        let mut w_offset = 0i32;
        let mut h_offset = 0i32;
        let mut max_width = 0i32;
        for stamp_loc in self.scene_graph.inventory.iter() {
          let dest = stamp_loc.stamp_source;
          let image = &images.stamps[stamp_loc.stamp_index];
          canvas.copy_ex(
            &image.0,
            None, Some(dest),
            0.0,
            Point::new(0,0),//centre
            false,//horiz
            false,//vert
          );
        }
        canvas.present();
        Ok(())
    }
    fn apply_keys(&mut self, keys_down: &HashMap<Keycode, ()>) {
        if keys_down.contains_key(&Keycode::Left) {
            self.mouse_x -= MOUSE_CONSTANT;
        }
        if keys_down.contains_key(&Keycode::Right) {
            self.mouse_x += MOUSE_CONSTANT;
        }
        if keys_down.contains_key(&Keycode::Up) {
            self.mouse_y -= MOUSE_CONSTANT;
        }
        if keys_down.contains_key(&Keycode::Down) {
            self.mouse_y += MOUSE_CONSTANT;
        }
    }
}

fn process<T:sdl2::render::RenderTarget>(state: &mut SceneState, images: &mut Images, event: sdl2::event::Event, canvas: &mut sdl2::render::Canvas<T>, keys_down: &mut HashMap<Keycode, ()>) -> Result<bool,String>{
    let mut key_encountered = false;
    match event {
        Event::Quit{..} => return Err("Exit".to_string()),
        Event::KeyDown {keycode: Option::Some(key_code), ..} =>{
            if let None = keys_down.insert(key_code, ()) {
                for (key,_)in keys_down.iter() {
                    eprintln!("Key is down {}\n", *key)
                }
            }
            key_encountered = true;
            state.apply_keys(&keys_down);
        },
        Event::KeyUp {keycode: Option::Some(key_code), ..} =>
        {
            keys_down.remove(&key_code);
        },
        Event::MouseButtonDown {x, y, ..} => {
            state.mouse_x = x;
            state.mouse_y = y;
        }
        Event::MouseMotion {x, y, ..} => {
            state.mouse_x = x;
            state.mouse_y = y;
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

pub fn run(dir: &Path) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG)?;
    let window = video_subsystem.window("rust-sdl2 demo: Cursor", 800, 600)
      .position_centered()
      .build()
      .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().software().build().map_err(|e| e.to_string())?;
    let mut keys_down = HashMap::<Keycode, ()>::new();
    let surface = Surface::from_file(dir.join("cursor.png"))
        .map_err(|err| format!("failed to load cursor image: {}", err))?;
    let mut scene_state = SceneState {
        scene_graph:SceneGraph{inventory:Vec::new(),},
        mouse_x:0,
        mouse_y:0,
        cursor:Cursor::from_surface(surface, 0, 0).map_err(
            |err| format!("failed to load cursor: {}", err))?,
    };
    let cursor_surface = Surface::from_file(dir.join("cursor.png"))
        .map_err(|err| format!("failed to load cursor image: {}", err))?;
    let texture_creator = canvas.texture_creator();
    let mut images = Images{
        default_cursor:make_texture_surface!(texture_creator, cursor_surface)?,
        stamps:Vec::new(),
    };
    process_dir(&dir.join("stamps"), &mut |p:&fs::DirEntry| {
        let stamp_surface = Surface::from_file(p.path()).map_err(
            |err| io::Error::new(io::ErrorKind::Other, format!("{}: {}", p.path().to_str().unwrap_or("??"), err)))?;
        images.stamps.push(make_texture_surface!(texture_creator, stamp_surface).map_err(
            |err| io::Error::new(io::ErrorKind::Other, format!("{}: {}", p.path().to_str().unwrap_or("?X?"), err)))?);
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
                process(&mut scene_state, &mut images, event, &mut canvas, &mut keys_down)?; // always break
            }
            scene_state.render(&mut canvas, &mut images)?;
            let process_time = loop_start_time.elapsed();
            if keys_down.len() != 0 && process_time < DESIRED_DURATION_PER_FRAME {
                std::thread::sleep(DESIRED_DURATION_PER_FRAME - process_time);
                scene_state.apply_keys(&keys_down);
                scene_state.render(&mut canvas, &mut images)?;
            }
        } else {
            for event in events.wait_iter() {
                process(&mut scene_state, &mut images, event, &mut canvas, &mut keys_down)?;
                break;
            }
            for event in events.poll_iter() {
                process(&mut scene_state, &mut images, event, &mut canvas, &mut keys_down)?;
            }
            scene_state.render(&mut canvas, &mut images)?;
        };
    }
}


fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: cargo run /path/to/image.(png|jpg)");
        Ok(())
    } else {
        let ret = run(Path::new(&args[1]));
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
