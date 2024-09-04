use std::sync::{Arc, Mutex};
use crate::model::sedaroml::Model;
use crate::nodes::traits::Exchangeable;

#[derive(Debug)]
pub enum TranslationError {}

type ModelOperationFn = fn(&Model, &mut Model) -> Result<(), TranslationError>;

#[derive(Debug, Clone)]
pub struct Operation {
  pub name: Option<String>,
  pub forward: ModelOperationFn,
  pub reverse: ModelOperationFn,
}

pub enum OperationFunction {
  Forward(Option<String>, ModelOperationFn),
  Reverse(Option<String>, ModelOperationFn),
}

pub struct Translation {
  pub from: Arc<Mutex<dyn Exchangeable + Sync + Send>>,
  pub to: Arc<Mutex<dyn Exchangeable + Sync + Send>>,
  pub operations: Vec<Operation>,
}