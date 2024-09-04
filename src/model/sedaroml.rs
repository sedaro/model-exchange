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

  pub fn to_pretty_string(&self) -> String {
    serde_json::to_string_pretty(&self).unwrap()
  }

  pub fn diff(&self, new: &Model) -> ModelDiff {
    let mut added_blocks = IndexMap::new();
    let mut removed_blocks = IndexMap::new();
    let mut updated_blocks = IndexMap::new();
    for (block_id, new_block) in new.blocks.iter() {
      match self.blocks.get(block_id) {
        Some(old_block) => {
          let mut added_fields = IndexMap::new();
          let mut removed_fields = IndexMap::new();
          let mut updated_fields = IndexMap::new();
          for (field_key, field_value) in new_block.iter() {
            match old_block.get(field_key) {
              Some(old_value) => {
                if old_value != field_value {
                  updated_fields.insert(field_key.clone(), ValueDiff { old_value: old_value.clone(), new_value: field_value.clone() });
                }
              },
              None => {
                added_fields.insert(field_key.clone(), field_value.clone());
              },
            }
          }
          for (field_key, field_value) in old_block.iter() {
            match new_block.get(field_key) {
              Some(_) => {},
              None => {
                removed_fields.insert(field_key.clone(), field_value.clone());
              },
            }
          }
          if !added_fields.is_empty() || !removed_fields.is_empty() || !updated_fields.is_empty() {
            updated_blocks.insert(block_id.clone(), BlockDiff { added_fields, removed_fields, updated_fields });
          }
        },
        None => {
          added_blocks.insert(block_id.clone(), new_block.clone());
        },
      }
    }
    for (block_id, old_block) in self.blocks.iter() {
      if !new.blocks.contains_key(block_id) {
        removed_blocks.insert(block_id.clone(), old_block.clone());
      }
    }
    let mut added_fields = IndexMap::new();
    let mut removed_fields = IndexMap::new();
    let mut updated_fields = IndexMap::new();
    for (field_key, field_value) in new.root.iter() {
      match self.root.get(field_key) {
        Some(old_value) => {
          if old_value != field_value {
            updated_fields.insert(field_key.clone(), ValueDiff { old_value: old_value.clone(), new_value: field_value.clone() });
          }
        },
        None => {
          added_fields.insert(field_key.clone(), field_value.clone());
        },
      }
    }
    for (field_key, field_value) in self.root.iter() {
      match new.root.get(field_key) {
        Some(_) => {},
        None => {
          removed_fields.insert(field_key.clone(), field_value.clone());
        },
      }
    }
    ModelDiff { added_blocks, removed_blocks, updated_blocks, root: BlockDiff { added_fields, removed_fields, updated_fields } }
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
  write_json(file_path, &model.to_pretty_string())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValueDiff {
  pub old_value: Value,
  pub new_value: Value,
}

impl PartialEq for ValueDiff {
  fn eq(&self, other: &Self) -> bool {
      self.old_value == other.old_value && self.new_value == other.new_value
  }
}
impl Eq for ValueDiff {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockDiff {
  pub added_fields: IndexMap<String, Value>,
  pub removed_fields: IndexMap<String, Value>,
  pub updated_fields: IndexMap<String, ValueDiff>,
}

impl PartialEq for BlockDiff {
  fn eq(&self, other: &Self) -> bool {
      self.added_fields == other.added_fields &&
      self.removed_fields == other.removed_fields &&
      self.updated_fields == other.updated_fields
  }
}
impl Eq for BlockDiff {}

/// A concise representation of the differences between two models.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelDiff {
  pub added_blocks: IndexMap<String, Block>,
  pub removed_blocks: IndexMap<String, Block>,
  pub updated_blocks: IndexMap<String, BlockDiff>,
  pub root: BlockDiff,
}

impl PartialEq for ModelDiff {
  fn eq(&self, other: &Self) -> bool {
      self.added_blocks == other.added_blocks &&
      self.removed_blocks == other.removed_blocks &&
      self.updated_blocks == other.updated_blocks &&
      self.root == other.root
  }
}
impl Eq for ModelDiff {}
impl ModelDiff {
  pub fn is_empty(&self) -> bool {
    self.added_blocks.is_empty() && 
    self.removed_blocks.is_empty() && 
    self.updated_blocks.is_empty() && 
    self.root.added_fields.is_empty() && 
    self.root.removed_fields.is_empty() && 
    self.root.updated_fields.is_empty()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_model_diff() {

    assert_eq!(
      ValueDiff { old_value: Value::String("old".to_string()), new_value: Value::String("new".to_string()) },
      ValueDiff { old_value: Value::String("old".to_string()), new_value: Value::String("new".to_string()) },
    );

    assert_eq!(
      {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::String("value".to_string()));
        map
      },
      {
        let mut map = IndexMap::new();
        map.insert("key".to_string(), Value::String("value".to_string()));
        map
      }
  );

    let mut old = Model::new();
    let new =  Model::new();

    let empty_diff = ModelDiff {
      added_blocks: IndexMap::new(),
      removed_blocks: IndexMap::new(),
      updated_blocks: IndexMap::new(),
      root: BlockDiff {
        added_fields: IndexMap::new(),
        removed_fields: IndexMap::new(),
        updated_fields: IndexMap::new(),
      },
    };
    assert_eq!(old.diff(&new), empty_diff);
    assert_eq!(new.diff(&old), empty_diff);
    assert!(empty_diff.is_empty());

    old.root.insert("name".to_string(), json!("root"));
    old.blocks.insert("1".to_string(), {
      let mut block = Block::new();
      block.insert("name".to_string(), json!("block1"));
      block
    });

    let diff = ModelDiff {
      added_blocks: IndexMap::new(),
      removed_blocks: {
        let mut removed_blocks = IndexMap::new();
        removed_blocks.insert("1".to_string(), {
          let mut block = Block::new();
          block.insert("name".to_string(), json!("block1"));
          block
        });
        removed_blocks
      },
      updated_blocks: IndexMap::new(),
      root: BlockDiff {
        added_fields: IndexMap::new(),
        removed_fields: {
          let mut removed_fields = IndexMap::new();
          removed_fields.insert("name".to_string(), Value::String("root".to_string()));
          removed_fields
        },
        updated_fields: IndexMap::new(),
      },
    };
    assert!(!diff.is_empty());
    assert_eq!(old.diff(&new), diff);

    let diff = ModelDiff {
      added_blocks: {
        let mut added_blocks = IndexMap::new();
        added_blocks.insert("1".to_string(), {
          let mut block = Block::new();
          block.insert("name".to_string(), json!("block1"));
          block
        });
        added_blocks
      },
      removed_blocks: IndexMap::new(),
      updated_blocks: IndexMap::new(),
      root: BlockDiff {
        added_fields: {
          let mut added_fields = IndexMap::new();
          added_fields.insert("name".to_string(), Value::String("root".to_string()));
          added_fields
        },
        removed_fields: IndexMap::new(),
        updated_fields: IndexMap::new(),
      },
    };
    assert!(!diff.is_empty());
    assert_eq!(new.diff(&old), diff);

    let mut new = old.clone();
    new.root.insert("name".to_string(), json!("root2"));
    new.blocks.get_mut("1").unwrap().insert("name".to_string(), json!("block2"));

    assert_eq!(old.diff(&new), ModelDiff {
      added_blocks: IndexMap::new(),
      removed_blocks: IndexMap::new(),
      updated_blocks: {
        let mut updated_blocks = IndexMap::new();
        updated_blocks.insert("1".to_string(), BlockDiff {
          added_fields: IndexMap::new(),
          removed_fields: IndexMap::new(),
          updated_fields: {
            let mut updated_fields = IndexMap::new();
            updated_fields.insert("name".to_string(), ValueDiff {
              old_value: Value::String("block1".to_string()),
              new_value: Value::String("block2".to_string()),
            });
            updated_fields
          },
        });
        updated_blocks
      },
      root: BlockDiff {
        added_fields: IndexMap::new(),
        removed_fields: IndexMap::new(),
        updated_fields: {
          let mut updated_fields = IndexMap::new();
          updated_fields.insert("name".to_string(), ValueDiff {
            old_value: Value::String("root".to_string()),
            new_value: Value::String("root2".to_string()),
          });
          updated_fields
        },
      },
    });
    assert_eq!(new.diff(&old), ModelDiff {
      added_blocks: IndexMap::new(),
      removed_blocks: IndexMap::new(),
      updated_blocks: {
        let mut updated_blocks = IndexMap::new();
        updated_blocks.insert("1".to_string(), BlockDiff {
          added_fields: IndexMap::new(),
          removed_fields: IndexMap::new(),
          updated_fields: {
            let mut updated_fields = IndexMap::new();
            updated_fields.insert("name".to_string(), ValueDiff {
              old_value: Value::String("block2".to_string()),
              new_value: Value::String("block1".to_string()),
            });
            updated_fields
          },
        });
        updated_blocks
      },
      root: BlockDiff {
        added_fields: IndexMap::new(),
        removed_fields: IndexMap::new(),
        updated_fields: {
          let mut updated_fields = IndexMap::new();
          updated_fields.insert("name".to_string(), ValueDiff {
            old_value: Value::String("root2".to_string()),
            new_value: Value::String("root".to_string()),
          });
          updated_fields
        },
      },
    });
  }
}

