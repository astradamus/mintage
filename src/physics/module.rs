use crate::world::{CurrCtx, NextCtx};
use serde_json::Value;
use std::collections::HashMap;

pub trait Module {
    fn apply_config(&mut self, config: &HashMap<String, Value>);
    fn run(&mut self, curr: &CurrCtx<'_>, next: &mut NextCtx<'_>);
}
