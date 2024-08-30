use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json;
use serde_json::Value;
use indexmap::IndexMap;
use crate::utils::{read_json, write_json};
use super::temp::TempModel;

pub type Block = IndexMap<String, Value>;

#[derive(Debug)]
pub enum ModelError {
  BlockTypeNotFound(String),
  BlockNotFound(String),
  FileError(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(from = "TempModel")]
pub struct Model {
  pub blocks: IndexMap<String, Block>,
  pub index: IndexMap<String, Vec<String>>,
  #[serde(flatten)]
  pub root: Block,
}

impl Model {
  pub fn new() -> Model {
    Model {
      blocks: IndexMap::new(),
      index: IndexMap::new(),
      root: IndexMap::new(),
    }
  }
  pub fn block_ids_of_type(&self, block_type: &str) -> Result<Vec<String>, ModelError> {
    let mut result = Vec::new();
    match self.index.get(block_type).clone() {
      Some(block_ids) => {
        for id_or_type in block_ids {
          if self.blocks.contains_key(id_or_type) {
            result.push(id_or_type.clone());
          } else {
            let ids = self.block_ids_of_type(id_or_type)?;
            result.extend(ids.iter().map(|id| id.clone()));
          }
        }
      },
      None => return Err(ModelError::BlockTypeNotFound("Block type not found".to_string())),
    }
    Ok(result)
  }

  pub fn block_by_id_mut(&mut self, block_id: &str) -> Result<&mut Block, ModelError> {
    match self.blocks.get_mut(block_id) {
      Some(block) => return Ok(block),
      None => return Err(ModelError::BlockNotFound(format!("Block ID not found: {block_id}"))),
    }
  }
  pub fn block_by_id(&self, block_id: &str) -> Result<&Block, ModelError> {
    match self.blocks.get(block_id) {
      Some(block) => return Ok(block),
      None => return Err(ModelError::BlockNotFound(format!("Block ID not found: {block_id}"))),
    }
  }

  pub fn filter_blocks_mut(&mut self, block_key: &str, block_value: &Value) -> Result<Vec<&mut Block>, ModelError> {
    let mut result = Vec::new();
    for (_, block) in self.blocks.iter_mut() {
      match block.get(block_key) {
        Some(value) => { if value == block_value { result.push(block) }},
        None => continue,
      }
    }
    Ok(result)
  }
  pub fn filter_blocks(&self, block_key: &str, block_value: &Value) -> Result<Vec<&Block>, ModelError> {
    let mut result = Vec::new();
    for (_, block) in self.blocks.iter() {
      match block.get(block_key) {
        Some(value) => { if value == block_value { result.push(block) }},
        None => continue,
      }
    }
    Ok(result)
  }

  pub fn get_first_block_where_mut(&mut self, search: &HashMap<String, Value>) -> Result<&mut Block, ModelError> {
    for (_, block) in self.blocks.iter_mut() { // TODO: Confirm order is deterministic here
      for (k, v) in search.into_iter() {
        match block.get(k) {
          Some(field_value) => { 
            if *field_value == *v { 
              return Ok(block);
            }
            break;
          },
          None => { break },
        }
      }
    }
    Err(ModelError::BlockNotFound(format!("No Blocks matching filter criteria were found.")))
  }
  pub fn get_first_block_where(&self, search: &HashMap<String, Value>) -> Result<&Block, ModelError> {
    for (_, block) in self.blocks.iter() { // TODO: Confirm order is deterministic here
      for (k, v) in search.into_iter() {
        match block.get(k) {
          Some(field_value) => {
            if *field_value == *v {
              return Ok(block);
            }
            break;
          },
          None => { break },
        }
      }
    }
    Err(ModelError::BlockNotFound(format!("No Blocks matching filter criteria were found.")))
  }
}

pub fn read_model(file_path: &str) -> Result<Model, ModelError> {
  let v = read_json(file_path)?;
  match serde_json::from_value::<Model>(v) {
    Ok(model) => Ok(model),
    Err(e) => Err(ModelError::FileError(format!("Cannot deserialize model at {file_path}: {}", e))),
  }
}

pub fn write_model(file_path: &str, model: &Model) -> Result<(), ModelError> {
  write_json(file_path, &serde_json::to_string_pretty(&model).unwrap())
}