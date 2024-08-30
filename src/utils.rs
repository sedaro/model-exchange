use std::fs::File;
use std::io::Read;
use std::io::Write;
use serde_json::Value;
use serde_json;
use crate::model::sedaroml::ModelError;
use std::process;
use pyo3::prelude::*;
use pyo3::Python;
use log::info;

pub(crate) fn read_json(file_path: &str) -> Result<Value, ModelError> {
  match File::open(file_path) {
    Ok(mut file) => {
      let mut contents = String::new();
      file.read_to_string(&mut contents).expect(format!("Cannot read file {}", file_path).as_str());
      let v = serde_json::from_str(&contents).unwrap();
      Ok(v)
    },
    Err(_) => return Err(ModelError::FileError(format!("Cannot read file {file_path}"))),
  }
}

pub(crate) fn write_json(file_path: &str, json_str: &str) -> Result<(), ModelError> {
  match File::create(file_path) {
    Ok(mut file) => {
      match file.write(json_str.as_bytes()) {
        Ok(_) => return Ok(()),
        Err(_) => return Err(ModelError::FileError(format!("Cannot write to file {file_path}")))
      }
    },
    Err(_) => return Err(ModelError::FileError(format!("Cannot write to file {file_path}"))),
  }
}

// This is a really annoying hack to allow for ctrl+c to terminate the exchange after spawning xlwings from python for excel conversion
pub fn python_signal_handler() -> PyResult<()> {
  Python::with_gil(|py| {
    py.check_signals().unwrap_or_else(|e| {
      info!("Recieved signal in python: {}.  Force terminating.", e);
      process::exit(1);
    });
    Ok(())
  })
}