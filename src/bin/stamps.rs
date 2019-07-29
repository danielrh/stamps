extern crate sdl2;

use std::time;
use std::string::String;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use sdl2::event::Event;
use sdl2::image::{LoadSurface, InitFlag};
use sdl2::keyboard::Keycode;
use sdl2::mouse::Cursor;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::surface::Surface;
static DESIRED_DURATION_PER_FRAME:time::Duration = time::Duration::from_millis(4);

fn process<T:sdl2::render::RenderTarget>(event: sdl2::event::Event, canvas: &mut sdl2::render::Canvas<T>, keys_down: &mut HashMap<Keycode, ()>) -> Result<bool,String>{
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
        },
        Event::KeyUp {keycode: Option::Some(key_code), ..} =>
        {
            keys_down.remove(&key_code);
        },
        Event::MouseButtonDown {x, y, ..} => {
            canvas.fill_rect(Rect::new(x, y, 1, 1))?;
            canvas.present();
        }
        _ => {}
    }
    Ok(key_encountered)
}

pub fn run(png: &Path) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG)?;
    let window = video_subsystem.window("rust-sdl2 demo: Cursor", 800, 600)
      .position_centered()
      .build()
      .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().software().build().map_err(|e| e.to_string())?;
    let mut keys_down = HashMap::<Keycode, ()>::new();
    let surface = Surface::from_file(png)
        .map_err(|err| format!("failed to load cursor image: {}", err))?;
    let cursor = Cursor::from_surface(surface, 0, 0)
        .map_err(|err| format!("failed to load cursor: {}", err))?;
    cursor.set();
    
    canvas.clear();
    canvas.present();

    canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));


    'mainloop: loop {
        let loop_start_time = time::Instant::now();
        let mut events = sdl_context.event_pump()?;
        if keys_down.len() != 0 {
            for event in events.poll_iter() {
                process(event, &mut canvas, &mut keys_down)?; // always break
            }
            let process_time = loop_start_time.elapsed();
            if keys_down.len() != 0 && process_time < DESIRED_DURATION_PER_FRAME {
                std::thread::sleep(DESIRED_DURATION_PER_FRAME - process_time);
            }
        } else {
            for event in events.wait_iter() {
                if process(event, &mut canvas, &mut keys_down)? {
                    break;
                }
            }
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
