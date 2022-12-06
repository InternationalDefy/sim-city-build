extern crate oorandom;
extern crate serde;

use std::collections::btree_set::Intersection;
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
    pub ability : i32,
    pub lifetime : i32,
    pub position : (i32, i32),
    viewDistance: i32,
    pub nextDecision : Decision,
}

impl Animal {
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
    pub autoInteract : bool,
    pub hp: i32,
    pub difficulty : i32,
    pub penalty : i32,
    pub reward : (i32, i32),
    pub position: (i32, i32),
}

impl Environment {
    pub fn spwan(e : &Environment, pos:(i32,i32)) ->Environment {
        Environment { 
            autoInteract : e.autoInteract,
            tag: e.tag.clone(), 
            hp: e.hp, 
            difficulty: e.difficulty, 
            penalty: e.penalty, 
            reward: e.reward, 
            position: pos 
        }
    }
}

fn distance(e : &Environment, a: &Animal) -> i32 {
    i32::abs(e.position.0-a.position.0) + i32::abs(e.position.1-a.position.1)
}
// 只拿来做决策,不用来做更新,因此不需要引用
fn find_environments(a:&Animal, ve:&Vec<Environment>) -> Vec<Environment> {
    let mut vec = vec!();
    for e in ve {
        if distance(e, a) <= a.viewDistance {
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

fn main() {
    let width = 50; let height = 50;
    let calculator_seed = 64;
    let initializer_seed = 64;
    let mut rng_calculator = oorandom::Rand32::new(calculator_seed);
    let mut rng_initializer = oorandom::Rand32::new(initializer_seed);
    let shelter_chance = 10;
    let shelter = Environment{
        tag: String::from("shelter"), 
        autoInteract : true,
        hp: 5, 
        difficulty: 0, 
        penalty: 0, 
        reward: (1,0),
        position: (0,0) 
    };
    let challenge_chance = 2;
    let challenge = Environment{
        tag: String::from("challenge"), 
        autoInteract : false,
        hp: 1, 
        difficulty: 10, 
        penalty: 2, 
        reward: (5,0),
        position: (0,0)
    };
    let danger_chance = 2;
    let danger = Environment{
        tag: String::from("danger"), 
        autoInteract : false,
        hp: i32::MAX, 
        difficulty: 10, 
        penalty: 2, 
        reward: (0,0),
        position: (0,0)
    };
    // 先不去考虑obstacle
    // let obstacle = Environment{
    //     tag: String::from("obstacle"),
    //     autoInteract : false,
    //     hp: MAX, 
    //     difficulty: MAX, 
    //     penalty: 1, 
    //     reward: (0,0),
    //     position: (0,0)
    // };
    let mut ve = vec![];
    for i in 1..width {
        for j in 1..height {
            let rng = rng_initializer.rand_u32() % 20;
            if rng >= shelter_chance {
                ve.push(Environment::spwan(&shelter, (i, j)));
            } else if rng >= shelter_chance - challenge_chance {
                ve.push(Environment::spwan(&challenge, (i, j)));
            } else if rng >= shelter_chance - challenge_chance - danger_chance {
                ve.push(Environment::spwan(&danger, (i, j)));
            }
        }
    }
    let mut d_maker = DecisionMaker::new();
    d_maker.increase_chance(Decision::Interact, 100);
    d_maker.decrease_chance(Decision::Interact, 200);
    println!("{:?}", d_maker);
    for _ in 0..100 {
        print!("{:?},", d_maker.make_decision(&mut rng_calculator));
    }
}
