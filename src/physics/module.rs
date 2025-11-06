use crate::physics::intent::CellIntent;
use crate::world::{CurrCtx, NextCtx};
use serde_json::Value;
use std::collections::HashMap;

pub trait Module: Send {
    fn apply_config(&mut self, config: &HashMap<String, Value>);
    fn run(&mut self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>);
    fn gather_intents(&mut self, curr: &CurrCtx<'_>) -> Vec<CellIntent>;
}
