extern crate oorandom;
extern crate serde;

use pixels::{Error, Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
};
use winit_input_helper::WinitInputHelper;
use std::vec;
use std::collections::BTreeMap;
// use serde::{Serialize, Deserialize};
use serde_derive::{Serialize,Deserialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Decision {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Interact,
    Build,
    Wait,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DecisionMaker {
    pub decisionMap : BTreeMap<Decision, u32>,
}

impl DecisionMaker {
    pub fn new() -> Self {
        use crate::Decision::*;
        let mut map = BTreeMap::new();
        map.insert(MoveUp, 2);
        map.insert(MoveDown, 2);
        map.insert(MoveLeft, 2);
        map.insert(MoveRight, 2);
        map.insert(Interact, 2);
        map.insert(Build, 0);
        map.insert(Wait, 10);
        DecisionMaker{
            decisionMap : map
        }
    }
    pub fn make_decision(&self, rng : &mut oorandom::Rand32) -> Decision {
        let mut sum = 0;
        for (_, v) in &self.decisionMap {
            sum += v;
        }
        let rand = rng.rand_u32() % sum;
        sum = 0;
        for (d, v) in &self.decisionMap {
            sum += v;
            if sum >= rand {
                // print!("sum :{:?}, rand:{:?}",sum, rand);
                return d.clone();
            }
        }
        Decision::Wait
    }
    pub fn decrease_chance(&mut self, d:Decision, i: u32) {
        let de = (self.decisionMap[&d] as i32 - i as i32) as i32;
        let chance = if de>0 {de} else {0} as u32;
        self.decisionMap.insert(d.clone(), chance);
    }
    pub fn increase_chance(&mut self, d: Decision, i : u32) {
        let chance = self.decisionMap[&d] + i;
        self.decisionMap.insert(d.clone(), chance);
    }
    pub fn mutate_chance(&mut self, mu : u32, rng : &mut oorandom::Rand32) {
        for (_, i) in &mut self.decisionMap {
            let rand = rng.rand_u32() % 2;
            if rand == 0 {
                *i += mu;
            } else {
                *i -= mu;                
            }
        }
    }
}
pub trait Tickable {
    fn tick(&mut self);
}

// 做决定, 执行决定, 移动, 都超出了类Animal的可视范围.
#[derive(Clone, Copy, Debug)]
struct Animal {
    pub alive : bool,
    pub hp : i32,
    pub ability : u32,
    pub lifetime : u32,
    pub position : (i32, i32),
    viewDistance: u32,
    pub next_decision : Decision,
}

impl Animal {
    pub fn spwan(a : &Animal, pos:(i32,i32)) ->Animal {
        Animal { 
            alive : true,
            hp : a.hp,
            ability : a.ability,
            lifetime : 0,
            position : pos,
            viewDistance: a.viewDistance,
            next_decision : a.next_decision,
        }
    }

    fn consume(&mut self, num : i32) {
        if !self.alive {
            return
        }
        self.hp -= num;
        if self.hp <= 0 {
            self.alive = false;
            return
        }
    }

    pub fn move_inc(&mut self, inc:(i32, i32)) {
        self.position.0 += inc.0;
        self.position.1 += inc.1;
        if self.position.0 < 0 {
            self.position.0 = 0;
        } 
        if self.position.0 >= WIDTH as i32 {
            self.position.0 = WIDTH as i32 - 1;
        }
        
        if self.position.1 < 0 {
            self.position.1 = 0;
        } 
        if self.position.1 >= HEIGHT as i32 {
            self.position.1 = HEIGHT as i32 - 1;
        }
        
    }

    pub fn get_center_pixel_pos(&self) -> (i32, i32) {
        (((GRID_WIDTH + 1) / 2 + GRID_WIDTH * self.position.0 as u32) as i32,
        ((GRID_HEIGHT + 1) / 2 + GRID_HEIGHT * self.position.1 as u32) as i32)
    }
}

impl Tickable for Animal {
    fn tick(&mut self) {
        if !self.alive {
            return;
        }
        self.lifetime += 1;
        self.consume(1);
        if !self.alive {
            return;
        }
    }
}

#[derive(Clone, Debug)]
struct Environment {
    tag : String,
    pub auto_interact : bool,
    pub hp: i32,
    pub difficulty : u32,
    pub penalty : u32,
    pub reward : (u32, u32),
    pub position: (i32, i32),
    pub color : (u8, u8, u8, u8),
    pub draw_type : DrawType,
    pub d : u32,
}

#[derive(Clone, Debug)]
pub enum DrawType {
    Round,
    Rect,
    Line,
    Pixel,
    Circle,
    Star,
    None,
}

impl Environment {
    pub fn spwan(e : &Environment, pos:(i32,i32)) ->Environment {
        Environment { 
            auto_interact : e.auto_interact,
            tag: e.tag.clone(), 
            hp: e.hp, 
            difficulty: e.difficulty, 
            penalty: e.penalty, 
            reward: e.reward, 
            position: pos,
            color : e.color,
            draw_type : e.draw_type.clone(),
            d: e.d,
        }
    }
    pub fn get_center_pixel_pos(&self) -> (i32, i32) {
        (((GRID_WIDTH + 1) / 2 + GRID_WIDTH * self.position.0 as u32) as i32,
        ((GRID_HEIGHT + 1) / 2 + GRID_HEIGHT * self.position.1 as u32) as i32)
    }
}

fn distance(e : &Environment, a: &Animal) -> i32 {
    i32::abs(e.position.0-a.position.0) + i32::abs(e.position.1-a.position.1)
}
// 只拿来做决策,不用来做更新,因此不需要引用
fn find_environments(a:&Animal, ve:&Vec<Environment>) -> Vec<Environment> {
    let mut vec = vec!();
    for e in ve {
        if distance(e, a) <= a.viewDistance as i32 {
            vec.push(e.clone());
        }
    }
    vec
}


struct Map {
    ve: Vec<Environment>,
    // va : Vec<Animal>,
    a : Animal,
    width : i32, height: i32,
}

const WIDTH: u32 = 50;
const HEIGHT: u32 = 50;
const WINDOW_WIDTH: u32 = 500;
const WINDOW_HEIGHT: u32 = 500;
const GRID_WIDTH: u32 = WINDOW_WIDTH/WIDTH;
const GRID_HEIGHT: u32 = WINDOW_HEIGHT/HEIGHT;

fn build_window() -> (EventLoop<()>, Window, Pixels) {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = 
            LogicalSize::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64);
        let scaled_size = 
            LogicalSize::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64);
        WindowBuilder::new()
            .with_title("simulation visualization")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = 
            SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WINDOW_WIDTH, WINDOW_HEIGHT, surface_texture).unwrap()
    };
    (event_loop, window, pixels)
}

fn visualize_map(ve : &Vec<Environment>, va : &Vec<Animal>, screen: &mut [u8]) {
    let x = (WINDOW_HEIGHT / 2) as i32;
    let y = (WINDOW_WIDTH / 2) as i32;
    for e in ve {
        let pos = e.get_center_pixel_pos();
        match e.draw_type {
            DrawType::Round => draw_round(screen, pos.0, pos.1, e.d as i32, 
                e.color.0,e.color.1,e.color.2,e.color.3),
            DrawType::Pixel => draw_pixel(screen, pos.0, pos.1,
                e.color.0,e.color.1,e.color.2,e.color.3),
            DrawType::Rect => draw_rect(screen, pos.0, pos.1, e.d as i32, 
                e.color.0,e.color.1,e.color.2,e.color.3),
            _ => (),
        }
    }
    for a in va {
        let pos = a.get_center_pixel_pos();
        draw_star(screen, pos.0, pos.1, 0xff,0xff,0,0xff);
    }
}

const STAR : [(i32, i32); 76]= [
    (-6,0),(-6,-1),
    (-5,0),(-5,-1),(-5,3),(-5,4),(-5,-4),(-5,-5),
    (-4,0),(-4,2),(-4,3),(-4,4),(-4,-1),(-4,-3),(-4,-4),(-4,-5),
    (-3,1),(-3,2),(-3,-2),(-3,-3),(-3,0),(-3,-1),(-3,3),(-3,-1),
    (-2,1),(-2,2),(-2,-2),(-2,-3),
    (-1,0),(-1,-1),(-1,-3),(-1,-4),(-1,-5),(-1,-6),(-1,2),(-1,3),(-1,4),(-1,5),
    (0,0),(0,-1),(0,-3),(0,-4),(0,-5),(0,-6),(0,2),(0,3),(0,4),(0,5),
    (1,1),(1,2),(1,-2),(1,-3),
    (2,1),(2,2),(2,-2),(2,-3),(2,0),(2,-1),(2,3),(2,-1),
    (3,0),(3,2),(3,3),(3,4),(3,-1),(3,-3),(3,-4),(3,-5),
    (4,0),(4,-1),(4,3),(4,4),(4,-4),(4,-5),
    (5,0),(5,-1),
];

fn draw_star(screen: &mut [u8], x:i32, y:i32, r:u8,g:u8,b:u8,a:u8) {
    for (i, j) in STAR {
        draw_pixel(screen, x + i + 1, y + j + 1, r, g, b, a);
    }
}

fn draw_rect(screen: &mut [u8], x:i32, y:i32, d:i32, 
    r:u8,g:u8,b:u8,a:u8) {
    let half_d = (d + 1) / 2;
    for i in 0..half_d {
        for j in 0..half_d {
            draw_pixel(screen, x - i, y - j, r, g, b, a);
            draw_pixel(screen, x - i, y + j, r, g, b, a);
            draw_pixel(screen, x + i, y - j, r, g, b, a);
            draw_pixel(screen, x + i, y + j, r, g, b, a);
        }
    }
}

fn draw_round(screen: &mut [u8], x:i32, y:i32, d:i32, 
    r:u8,g:u8,b:u8,a:u8) {
    let half_d = (d + 1) / 2;
    let sqr_half_d = half_d * half_d;

    for i in 0..half_d {
        for j in 0..half_d {
            if (i*i + j*j) <= sqr_half_d {
                draw_pixel(screen, x - i, y - j, r, g, b, a);
                draw_pixel(screen, x - i, y + j, r, g, b, a);
                draw_pixel(screen, x + i, y - j, r, g, b, a);
                draw_pixel(screen, x + i, y + j, r, g, b, a);
            }
        }
    }
}

fn draw_pixel(screen: &mut [u8], x:i32, y:i32, r:u8,g:u8,b:u8,a:u8) {
    if x<0 || y<0 || x * WINDOW_HEIGHT as i32 * 4 + y * 4 + 3 >= screen.len() as i32 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    screen[x * WINDOW_HEIGHT as usize * 4 + y * 4] = r;
    screen[x * WINDOW_HEIGHT as usize * 4 + y * 4 + 1] = g;
    screen[x * WINDOW_HEIGHT as usize * 4 + y * 4 + 2] = b;
    screen[x * WINDOW_HEIGHT as usize * 4 + y * 4 + 3] = a;
}

fn execute_decision(ve: &mut Vec<Environment>, a :&mut Animal) {
    match a.next_decision {
        Decision::MoveUp => {
            a.move_inc((1, 0));
        },
        Decision::MoveDown => {
            a.move_inc((-1, 0));
        },
        Decision::MoveLeft => {
            a.move_inc((0, -1));
        },
        Decision::MoveRight => {
            a.move_inc((0, 1));
        },
        Decision::Interact => {

        },
        Decision::Build => {

        },
        Decision::Wait => {},
        _ => {},
    }
}

fn main() {
    let calculator_seed = 64;
    let initializer_seed = 64;
    let mut rng_calculator = oorandom::Rand32::new(calculator_seed);
    let mut rng_initializer = oorandom::Rand32::new(initializer_seed);
    let shelter_chance = 10;
    let shelter = Environment{
        tag: String::from("shelter"), 
        auto_interact : true,
        hp: 5, 
        difficulty: 0, 
        penalty: 0, 
        reward: (1,0),
        position: (0,0),
        draw_type: DrawType::Rect,
        color: (0, 0xff, 0, 0x7f),
        d:GRID_WIDTH,
    };
    let challenge_chance = 2;
    let challenge = Environment{
        tag: String::from("challenge"), 
        auto_interact : false,
        hp: 1, 
        difficulty: 10, 
        penalty: 2, 
        reward: (5,0),
        position: (0,0),
        draw_type: DrawType::Round,
        color: (0, 0xff, 0xff, 0xaf),
        d:GRID_WIDTH-1,
    };
    let danger_chance = 2;
    let danger = Environment{
        tag: String::from("danger"), 
        auto_interact : false,
        hp: i32::MAX, 
        difficulty: 10, 
        penalty: 2, 
        reward: (0,0),
        position: (0,0),
        draw_type: DrawType::Round,
        color: (0xff, 0, 0, 0xaf),
        d:GRID_WIDTH-2,
    };
    // 先不去考虑obstacle
    // let obstacle = Environment{
    //     tag: String::from("obstacle"),
    //     auto_interact : false,
    //     hp: MAX, 
    //     difficulty: MAX, 
    //     penalty: 1, 
    //     reward: (0,0),
    //     position: (0,0)
    // };
    let mut ve = vec![];
    for i in 1..WIDTH {
        for j in 1..HEIGHT {
            let rng = rng_initializer.rand_u32() % 20;
            if rng >= shelter_chance {
                ve.push(Environment::spwan(&shelter, (i as i32, j as i32)));
            } else if rng >= shelter_chance - challenge_chance {
                ve.push(Environment::spwan(&challenge, (i as i32, j as i32)));
            } else if rng >= shelter_chance - challenge_chance - danger_chance {
                ve.push(Environment::spwan(&danger, (i as i32, j as i32)));
            }
        }
    }
    let normal = Animal {
        alive : true,
        hp : 10,
        ability : 5,
        lifetime : 0,
        position : (0, 0),
        viewDistance: 5,
        next_decision : Decision::Wait,
    };
    let mut va = vec![
        Animal::spwan(&normal, (25,25))
    ];
    let mut d_maker = DecisionMaker::new();
    let mut tick : u128 = 0;
    let (event_loop, window, mut pixels) = build_window();
    event_loop.run(move |event, _, control_flow| {
        // 剩下的loop操作也在这里写.
        clear_pixels(pixels.get_frame_mut());
        visualize_map(&ve, &va, pixels.get_frame_mut());
        pixels.render().unwrap();
        // FIXME 暂时.
        va[0].next_decision = d_maker.make_decision(&mut rng_calculator);
        execute_decision(&mut ve, &mut va[0]);
        tick += 1;
        window.request_redraw();
    });
}

fn clear_pixels(pixels : &mut [u8]) {
    for i in 0..WINDOW_WIDTH as usize {
        for j in 0..WINDOW_HEIGHT as usize {
            pixels[4 * (i * WINDOW_HEIGHT as usize + j)] = 0;
            pixels[4 * (i * WINDOW_HEIGHT as usize + j) + 1] = 0;
            pixels[4 * (i * WINDOW_HEIGHT as usize + j) + 2] = 0;
            pixels[4 * (i * WINDOW_HEIGHT as usize + j) + 3] = 0;            
        }
    }
}