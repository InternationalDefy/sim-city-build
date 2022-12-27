extern crate oorandom;
extern crate serde;

use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    // event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
};
use std::{vec, collections::HashMap, str::FromStr};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use serde_derive::{Serialize,Deserialize};
use serde_with::serde_as;

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
    pub decision_map : BTreeMap<Decision, u32>,
}

impl DecisionMaker {
    pub fn default() -> Self {
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
            decision_map : map
        }
    }
    pub fn make_decision(&self, rng : &mut oorandom::Rand32) -> Decision {
        let mut sum = 0;
        for (_, v) in &self.decision_map {
            sum += v;
        }
        let rand = rng.rand_u32() % sum;
        sum = 0;
        for (d, v) in &self.decision_map {
            sum += v;
            if sum >= rand {
                // print!("sum :{:?}, rand:{:?}",sum, rand);
                return d.clone();
            }
        }
        Decision::Wait
    }
    pub fn decrease_chance(&mut self, d:Decision, i: u32) {
        let de = (self.decision_map[&d] as i32 - i as i32) as i32;
        let chance = if de>0 {de} else {0} as u32;
        self.decision_map.insert(d.clone(), chance);
    }
    pub fn increase_chance(&mut self, d: Decision, i : u32) {
        let chance = self.decision_map[&d] + i;
        self.decision_map.insert(d.clone(), chance);
    }
    pub fn mutate_chance(&mut self, mu : u32, rng : &mut oorandom::Rand32) {
        let mut vdis = vec![];
        for (dis, _) in &mut self.decision_map {
            vdis.push(dis.clone());
        }
        for dis in vdis {
            let rand = rng.rand_u32() % 2;
            match rand {
                0 => &self.increase_chance(dis, mu),
                _ => &self.decrease_chance(dis, mu),
            };
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum Direction {
    Up,
    Dowm,
    Right, 
    Left,
}


#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum DecisionFactor {
    DistanceDirection(u32, Direction, EnvironmentTag),
    CurrentLocation(EnvironmentTag),
    CurrentHp(i32),
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DecisionMakingTree {
    #[serde_as(as = "Vec<(_, _)>")]
    decision_chain : HashMap<Vec<DecisionFactor>, DecisionMaker>,
    // FIXME 历史记录
    decision_history : Vec<(u128, Vec<DecisionFactor>, Decision)>,
}

impl DecisionMakingTree {
    pub fn from_json(_path_name: String) -> DecisionMakingTree {
        let path = Path::new(_path_name.as_str());
        let mut file = File::open(&path).unwrap();
        let mut serialized : String= String::new(); 
        file.read_to_string(&mut serialized).unwrap();
        serde_json::from_str(serialized.as_str()).unwrap()
    }

    // pub fn init_json() {
    //     let dmt = DecisionMakingTree{decision_chain:HashMap::new(),decision_history:vec![]};
    //     let path = Path::new("decision_making_tree.json");
    //     let mut file = File::create(&path).unwrap();
    //     let serialized = serde_json::to_string(&dmt).unwrap();
    //     file.write(serialized.as_bytes()).unwrap();
    // }

    pub fn to_json(&self, json_path:String) {
        let serialized = serde_json::to_string(&self);
        match serialized {
            Ok(res) => {
                let path = Path::new(json_path.as_str());
                let mut file = File::create(&path).unwrap();
                file.write(res.as_bytes()).unwrap();
            },
            Err(msg) => {
                panic!("{:?}", msg);
            }
        }
    }
    // FIXME 实现
    fn mutate_impl(mut self, _mutate_factor:u32, rng :&mut oorandom::Rand32) -> DecisionMakingTree {
        for (_, dm) in &mut self.decision_chain {
            dm.mutate_chance(_mutate_factor, rng);
        }
        self
    }

    fn reward_impl(mut self, _reward_factor:u32) -> DecisionMakingTree {
        for (_, vdf, dis) in &self.decision_history {
            self.decision_chain.get_mut(vdf).unwrap().increase_chance(dis.clone(), _reward_factor);
        }
        self
    }
    
    pub fn reward(&self, _reward_factor:u32) -> DecisionMakingTree {
        self.clone().reward_impl(_reward_factor)
    }

    pub fn mutate(&self, _mutate_factor:u32, rng :&mut oorandom::Rand32) -> DecisionMakingTree {
        self.clone().mutate_impl(_mutate_factor, rng)
    }

    fn generate_default_decision_maker(&mut self, vdf : Vec<DecisionFactor>) -> DecisionMaker {
        self.decision_chain.insert(vdf, DecisionMaker::default());
        DecisionMaker::default()
    }

    fn get_decision_maker(&mut self, vdf : Vec<DecisionFactor>) -> DecisionMaker {
        let decision_chain_get = self.decision_chain.get(&vdf);
        let decision_maker = match decision_chain_get {
            Some(d_maker) => d_maker.clone(),
            None => self.generate_default_decision_maker(vdf),
        };
        decision_maker
    }

    fn calculate_decision_factors(&mut self, a: &Animal, ve : Vec<Environment>) -> Vec<DecisionFactor> {
        let mut vdf = vec![];
        for e in ve {
            if e.position == a.position {
                vdf.push(DecisionFactor::CurrentLocation(e.tag));
                continue;
            } 
            let dis = distance(&e, a);
            let dir = if i32::abs(e.position.0 - a.position.0) > i32::abs(e.position.1 - e.position.1) {
                if e.position.0 > a.position.0 {
                    Direction::Up
                } else {
                    Direction::Dowm
                }
            } else {
                if e.position.1 > a.position.1 {
                    Direction::Right
                } else {
                    Direction::Left
                }
            };
            vdf.push(DecisionFactor::DistanceDirection(dis as u32, dir, e.tag));
        }
        vdf.push(DecisionFactor::CurrentHp(a.hp));
        vdf
    }

    pub fn make_a_decision(&mut self, tick : u128, a: &Animal, ve : Vec<Environment>, rng: &mut oorandom::Rand32) -> Decision {
        let vdf = self.calculate_decision_factors(a, ve);
        let vdc = vdf.clone();
        let decision = self.make_a_decision_impl(vdf, rng);
        self.decision_history.push((tick, vdc, decision));
        decision
    }

    fn make_a_decision_impl(&mut self, vdf : Vec<DecisionFactor>, rng:&mut oorandom::Rand32) -> Decision {
        let decision_maker = self.get_decision_maker(vdf);
        decision_maker.make_decision(rng)
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
    view_distance: u32,
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
            view_distance: a.view_distance,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)] 
pub enum EnvironmentTag {
    DANGER,
    CHALLENGE,
    SHELTER,
    DEFAULT,    
}

#[derive(Clone, Debug)]
struct Environment {
    tag : EnvironmentTag,
    pub alive : bool,
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
            alive: true,
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
        if distance(e, a) <= a.view_distance as i32 {
            vec.push(e.clone());
        }
    }
    vec
}

const WIDTH: u32 = 50;
const HEIGHT: u32 = 50;
const WINDOW_WIDTH: u32 = 500;
const WINDOW_HEIGHT: u32 = 500;
const GRID_WIDTH: u32 = WINDOW_WIDTH/WIDTH;
const GRID_HEIGHT: u32 = WINDOW_HEIGHT/HEIGHT;

fn build_window() -> (EventLoop<()>, Window, Pixels) {
    let event_loop = EventLoop::new();
    // let input = WinitInputHelper::new();
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

    let pixels = {
        let window_size = window.inner_size();
        let surface_texture = 
            SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WINDOW_WIDTH, WINDOW_HEIGHT, surface_texture).unwrap()
    };
    (event_loop, window, pixels)
}

fn visualize_map(ve : &Vec<Environment>, va : &Vec<Animal>, screen: &mut [u8]) {
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

fn make_interaction(e : &mut Environment, a : &mut Animal, rng: &mut oorandom::Rand32) {
    let dif = e.difficulty;
    let roll = rng.rand_u32() % 20;
    let dix = roll + a.ability;
    e.consume(1);
    if dix >= dif {
        a.hp += e.reward.0 as i32;
        a.ability += e.reward.1;
    } else {
        a.consume(e.penalty as i32);
    }

}

fn execute_decision(ve: &mut Vec<Environment>, a :&mut Animal, rng :&mut oorandom::Rand32) {
    let mut vme : Vec<&mut Environment> = vec![];
    for e in ve.iter_mut() {
        if e.position == a.position {
            vme.push(e);
        }
    }
    for me in vme.iter_mut() {
        if me.auto_interact {
            make_interaction(me, a, rng);
        }
    }
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
            for me in vme.iter_mut() {
                if !me.auto_interact {
                    make_interaction(me, a, rng);
                }
            }
        },
        Decision::Build => {
            a.consume(5);
            ve.push(Environment{
                alive: true,
                tag: EnvironmentTag::SHELTER, 
                auto_interact : true,
                hp: 5, 
                difficulty: 0, 
                penalty: 0, 
                reward: (1,0),
                position: (0,0),
                draw_type: DrawType::Rect,
                color: (0, 0xff, 0, 0x7f),
                d:GRID_WIDTH,
            });
        },
        // Decision::Wait => {},
        _ => {},
    }
}

fn garbage_collection(ve: Vec<Environment>) -> Vec<Environment> {
    let mut ret = vec![];
    for e in ve {
        if e.alive {
            ret.push(e);
        }
    }
    ret
}

fn generate_map() -> (Vec<Environment>,Vec<Animal>) {
    let initializer_seed = 64;
    let mut rng_initializer = oorandom::Rand32::new(initializer_seed);
    let shelter_chance = 10;
    let shelter = Environment{
        alive:true,
        tag: EnvironmentTag::SHELTER, 
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
        alive:true,
        tag: EnvironmentTag::CHALLENGE, 
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
        alive:true,
        tag: EnvironmentTag::DANGER, 
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
        view_distance: 5,
        next_decision : Decision::Wait,
    };
    let va = vec![
        Animal::spwan(&normal, (25,25))
    ];
    (ve, va)
}

fn decision_making_single_loop(
    _show_visuals: bool, 
    mut _decision_making_tree: DecisionMakingTree,
) -> (DecisionMakingTree, u128) {
    let mut tick : u128 = 0;
    let calculator_seed = 64;
    let mut rng_calculator = oorandom::Rand32::new(calculator_seed);
    let (mut ve,mut va) = generate_map();
    if _show_visuals {
        let (event_loop, window, mut pixels) = build_window();
        event_loop.run(move |_, _, control_flow| {
            // 剩下的loop操作也在这里写.
            clear_pixels(pixels.get_frame_mut());
            visualize_map(&ve, &va, pixels.get_frame_mut());
            pixels.render().unwrap();
            let vde = find_environments(&va[0], &ve);
            va[0].next_decision = _decision_making_tree.make_a_decision(tick, &va[0], vde, &mut rng_calculator);
            execute_decision(&mut ve, &mut va[0], &mut rng_calculator);
            for a in va.iter_mut() {
                a.tick();
            }
            tick += 1;
            ve = garbage_collection(ve.clone());
            window.request_redraw();
            if !va[0].alive {
                println!("Player Dead in tick {:?}", tick);
                *control_flow = ControlFlow::Exit;
                return;
            }         
        });
    } else {
        while va[0].alive {
            let vde = find_environments(&va[0], &ve);
            va[0].next_decision = _decision_making_tree.make_a_decision(tick, &va[0], vde, &mut rng_calculator);
            execute_decision(&mut ve, &mut va[0], &mut rng_calculator);
            for a in va.iter_mut() {
                a.tick();
            }
            tick += 1;
            ve = garbage_collection(ve.clone());
            if !va[0].alive {
                println!("Player Dead in tick {:?}", tick);
            }
        }
    }    
    (_decision_making_tree, tick)
}

fn decision_making_run(
    _show_visuals: bool, 
    _run_count:u32,
    _mutate_factor:u32,
    _reward_factor:u32,
    _sample_count:u32,
    _from_json:Option<String>, 
    _to_json:Option<String>) {
    let mutator_seed = 64;
    let mut rng_mutator = oorandom::Rand32::new(mutator_seed);
    let mut decision_making_tree = match _from_json {
        Some(json_path) => DecisionMakingTree::from_json(json_path),
        None => DecisionMakingTree{
            decision_history:vec!(), 
            decision_chain:HashMap::new()
        }
    };
    for run in 1.._run_count {
        println!("RUNNING COUNT {:?}", run);
        let mut result_vec = vec![];
        for sample in 1.._sample_count {
            println!("SAMPLE COUNT {:?}", sample);
            let decision_making_sample = decision_making_tree.clone().mutate(_mutate_factor, &mut rng_mutator);
            result_vec.push(decision_making_single_loop(_show_visuals, decision_making_sample));
        }
        let (mut rdmt, mut max_tick) = (
            DecisionMakingTree{
                decision_history:vec!(), 
                decision_chain:HashMap::new()
        }, 0);
        for (dmt, tick) in result_vec {
            if tick > max_tick {
                rdmt = dmt;
                max_tick = tick;
            }
        }
        decision_making_tree = rdmt.reward(_reward_factor);
    }
    match _to_json {
        Some(json_path) => decision_making_tree.to_json(json_path),
        None => (),
    }
}

fn main() {
    decision_making_run(
        false,
        10,
        2,
        2,
        10,
        None,
        // Some(String::from_str("decision_making_tree.json").unwrap()),
        Some(String::from_str("decision_making_trainning_result_0.json").unwrap()),
    );
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