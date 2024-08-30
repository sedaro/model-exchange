use crate::watchers::excel::ExcelWatcher;
use crate::watchers::WatcherType;
use crate::model::sedaroml::{Model, read_model, write_model};
use crate::nodes::traits::Exchangeable;
use std::sync::{Arc, Mutex};
use pyo3::prelude::*;
use std::borrow::{Borrow, BorrowMut};
use crate::watchers::traits::Watcher;
use crate::utils::python_signal_handler;

#[derive(Clone)]
pub struct Excel {
  pub identifier: String,
  pub excel_filename: String,
  pub sedaroml_filename: String,
  pub watcher: WatcherType,
  pub representation: Model,
}
impl Excel {
  pub fn new(identifier: &str, filename: &str) -> Arc<Mutex<Excel>> {
    let mut sedaroml_filename = filename.to_string();
    sedaroml_filename.push_str(".json");
    let mut exchangeable = Excel {
      identifier: identifier.into(),
      excel_filename: filename.into(),
      sedaroml_filename: sedaroml_filename.clone(),
      watcher: WatcherType::ExcelWatcher(ExcelWatcher::new(identifier.into(), filename.into(), sedaroml_filename.clone())),
      representation: Model::new(),
    };
    python_signal_handler().unwrap();
    exchangeable.read();  // TODO: If sedaroml doesn't exist, create it from excel.  There is a lot to do around initialization of exchange
    Arc::new(Mutex::new(exchangeable))
  }
}
impl Exchangeable for Excel {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn watcher(&self) -> &WatcherType { self.watcher.borrow() }
  fn watcher_mut(&mut self) -> &mut WatcherType { self.watcher.borrow_mut() }
  fn representation(&self) -> &Model { self.representation.borrow() }
  fn representation_mut(&mut self) -> &mut Model { self.representation.borrow_mut() }
  fn read(&mut self) {
    excel_to_sedaroml(&self.excel_filename, &self.sedaroml_filename).unwrap_or_else(
      |e| panic!("{}: Failed to convert Excel to SedaroML: {}", self.identifier, e)
    );
    self.representation = read_model(&self.sedaroml_filename).unwrap_or_else(
      |e| panic!("{}: Failed to read SedaroML from file: {:?}", self.identifier, e)
    );
  }
  fn write(&self) {
    write_model(&self.sedaroml_filename, &self.representation).unwrap_or_else(
      |_| panic!("{}: Failed to write SedaroML to file.", self.identifier)
    );
    sedaroml_to_excel(&self.sedaroml_filename, &self.excel_filename).unwrap_or_else(
      |e| panic!("{}: Failed to convert SedaroML to Excel: {}", self.identifier, e)
    );
    match self.watcher() { // Manually trigger that watcher as the current `sedaroml_to_excel` converter doesn't save the file on disk
      WatcherType::ExcelWatcher(ref watcher) => {
        watcher.trigger();
      },
      _ => panic!("Invalid watcher type"),
    }
  }
}

fn excel_to_sedaroml(excel_filename: &str, sedaroml_filename: &str) -> PyResult<()> {
  Python::with_gil(|py| {
    let sys = py.import_bound("sys")?;
    sys.getattr("path")?.call_method1("append", ("/Users/sebastianwelsh/Development/sedaro/modex/.venv/lib/python3.12/site-packages",))?;
    sys.getattr("path")?.call_method1("append", ("/Users/sebastianwelsh/Development/sedaro/modex/.venv/lib/python3.12/site-packages/aeosa",))?; // TODO: Ewwww!!!  How to do this better??

    let module = PyModule::import_bound(py, "modex.excel")?;
    module.getattr("excel_to_sedaroml")?.call1((excel_filename, sedaroml_filename))?;
    Ok(())
  })
}

fn sedaroml_to_excel(sedaroml_filename: &str, excel_filename: &str) -> PyResult<()> {
  Python::with_gil(|py| {
    let sys = py.import_bound("sys")?;
    sys.getattr("path")?.call_method1("append", ("/Users/sebastianwelsh/Development/sedaro/modex/.venv/lib/python3.12/site-packages",))?;
    sys.getattr("path")?.call_method1("append", ("/Users/sebastianwelsh/Development/sedaro/modex/.venv/lib/python3.12/site-packages/aeosa",))?; // TODO: Ewwww!!!  How to do this better??

    let module = PyModule::import_bound(py, "modex.excel")?;
    module.getattr("sedaroml_to_excel")?.call1((sedaroml_filename, excel_filename))?;
    Ok(())
  })
}
