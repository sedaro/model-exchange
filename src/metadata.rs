use std::fs::File;
use std::io::Read;
use serde::{Serialize, Deserialize};
use serde_json;
use crate::model::sedaroml::ModelError;
use crate::utils::write_json;
use log::debug;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteMetadata {
    pub date_modified: String,
}

pub fn write_metadata(metadata_filename: &str, date_modified: &str) -> Result<(), ModelError> {
  debug!("Writing metadata...");
  write_json(
    &metadata_filename, 
    &serde_json::to_string_pretty(
      &serde_json::json!({
        "date_modified": date_modified,
      })
    ).unwrap()
  )
}

pub fn read_metadata(file_path: &str) -> Result<RemoteMetadata, ModelError> {
  match File::open(file_path) {
    Ok(mut file) => {
      let mut contents = String::new();
      match file.read_to_string(&mut contents) {
        Ok(_) => {
          let mut model: RemoteMetadata = serde_json::from_str(&contents).unwrap();
          model.date_modified = model.date_modified.replace("\"", "");
          Ok(model)
        },
        Err(_) => return Err(ModelError::FileError(format!("Cannot read file {file_path}"))),
      }
    },
    Err(_) => return Err(ModelError::FileError(format!("Cannot read file {file_path}"))),
  }
}