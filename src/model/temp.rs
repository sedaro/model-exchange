use indexmap::IndexMap;
use serde::{Serialize, Deserialize};
use crate::model::sedaroml::{Model, Block};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct TempModel {
  blocks: IndexMap<String, Block>,
  index: IndexMap<String, Vec<String>>,
  #[serde(flatten)]
  root: Block,
}

impl From<TempModel> for Model {
  fn from(temp: TempModel) -> Self {
    let mut instance = Self {
      blocks: temp.blocks,
      index: temp.index,
      root: temp.root,
    };
    instance.root.swap_remove("_blockNames");
    instance.root.swap_remove("_quantityKinds");
    instance.root.swap_remove("_relationships");
    instance.root.swap_remove("_supers");
    instance.root.swap_remove("_abstractBlockTypes");
    instance.root.swap_remove("blocks");
    instance.root.swap_remove("index");
    instance.root.swap_remove("migrated");
    instance.root.swap_remove("issues");
    instance
  }
}