use crate::physics::intent::CellIntent;
use crate::world::CurrCtx;
use serde_json::Value;
use std::collections::HashMap;

pub enum ModuleOutput {
    CellIntents{
        intents: Vec<CellIntent>,
    },
    DeltaTemp {
        delta_temp: Vec<f32>,
    },
}

pub trait Module: Send {
    fn apply_config(&mut self, config: &HashMap<String, Value>);
    fn run(&mut self, curr: &CurrCtx<'_>) -> ModuleOutput;
}
