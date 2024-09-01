use crate::commands::{NodeCommands, NodeResponses};
use crate::model::sedaroml::{Model, read_model};
use crate::nodes::traits::Exchangeable;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use pyo3::prelude::*;
use std::borrow::{Borrow, BorrowMut};
use crate::utils::python_signal_handler;
use std::sync::mpsc;
use std::thread::{self};
use log::{debug, error};
use notify_debouncer_mini::{
  notify::RecursiveMode,
  new_debouncer, 
  DebounceEventResult,
};

#[derive(Clone)]
pub struct Excel {
  identifier: String,
  pub excel_filename: String,
  sedaroml_filename: String,
  rep: Option<Model>,
  tx: mpsc::Sender<NodeCommands>,
  rx: Arc<Mutex<mpsc::Receiver<NodeResponses>>>,
}

impl Excel {
  pub fn new(identifier: String, filename: String) -> Arc<Mutex<Excel>> {

    let mut sedaroml_filename = filename.to_string();
    sedaroml_filename.push_str(".json");
    let sedaroml_filename_clone = sedaroml_filename.clone();
    let identifier_clone = identifier.to_string().clone();
    let excel_filename = filename.to_string();

    let (tx_to_node, rx_in_node) = mpsc::channel::<NodeCommands>();
    let (tx_to_exchange, rx_in_exchange) = mpsc::channel::<NodeResponses>();
    thread::spawn(move || {
      // Setup
      let _excel_filename = excel_filename.clone();
      let _sedaroml_filename = sedaroml_filename_clone.clone();
      let _identifier = identifier_clone.clone();
      let mut excel_watcher = new_debouncer(Duration::from_millis(5), move |res: DebounceEventResult| {
        match res {
          Ok(_event) => { 
            excel_to_sedaroml(&_excel_filename, &_sedaroml_filename).unwrap_or_else(
              |e| panic!("{}: Failed to convert Excel to SedaroML: {}", _identifier, e)
            );
          },
          Err(e) => error!("Watch error: {:?}", e),
        }
      }).unwrap_or_else(|_| panic!("Failed to create excel watcher"));
      let watcher = excel_watcher.watcher();

      loop {
        match rx_in_node.recv_timeout(Duration::from_millis(100)) {
          Ok(command) => {
            debug!("{}: Received command: {:?}", identifier_clone, command);
            match command {
              NodeCommands::Start => {
                if !Path::exists(Path::new(&sedaroml_filename_clone)) {
                  debug!("{}: SedaroML file doesn't exist.  Generating from: {}", identifier_clone, &excel_filename);
                  excel_to_sedaroml(&excel_filename, &sedaroml_filename_clone).unwrap_or_else(
                    |e| panic!("{}: Failed to convert Excel to SedaroML: {}", identifier_clone, e)
                  );
                }
                watcher.watch(&Path::new(&excel_filename), RecursiveMode::Recursive).unwrap_or_else(|e| panic!("Failed to watch path: {}: {}", excel_filename, e));

                tx_to_exchange.send(NodeResponses::Started).unwrap();
              },
              NodeCommands::Stop => { tx_to_exchange.send(NodeResponses::Stopped).unwrap() },
              NodeCommands::Changed => {
                let t = Instant::now();
                sedaroml_to_excel(&sedaroml_filename_clone, &excel_filename).unwrap_or_else(
                  |e| panic!("{}: Failed to convert SedaroML to Excel: {}", identifier_clone, e)
                );
                tx_to_exchange.send(NodeResponses::Done(t.elapsed())).unwrap();
              },
              NodeCommands::Done => {},
            }
          },
          Err(_) => {},
        };
        python_signal_handler().unwrap();
      }
    });

    let exchangeable = Excel {
      identifier: identifier.into(),
      excel_filename: filename.into(),
      sedaroml_filename: sedaroml_filename.clone(),
      rep: None,
      tx: tx_to_node,
      rx: Arc::new(Mutex::new(rx_in_exchange)),
    };
    Arc::new(Mutex::new(exchangeable))
  }
}

impl Exchangeable for Excel {
  fn identifier(&self) -> String { self.identifier.clone() }
  fn sedaroml_filename(&self) -> String { self.sedaroml_filename.clone() }
  fn rep(&self) -> &Model { 
    match self.rep.borrow() {
      Some(rep) => rep,
      None => panic!("{}: Representation not initialized", self.identifier()),
    }
  }
  fn rep_mut(&mut self) -> &mut Model {
    let iden = self.identifier();
    match self.rep.borrow_mut() {
      Some(rep) => rep,
      None => panic!("{}: Representation not initialized", iden),
    }
  }
  fn tx(&self) -> &mpsc::Sender<NodeCommands> { &self.tx }
  fn rx(&self) -> &Arc<Mutex<mpsc::Receiver<NodeResponses>>> { &self.rx }
  fn refresh_rep(&mut self) {
    self.rep = Some(read_model(&self.sedaroml_filename()).unwrap_or_else(
      |e| panic!("{}: Failed to read SedaroML: {:?}", self.identifier(), e)
    ));
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
