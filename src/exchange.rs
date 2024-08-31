use std::collections::HashSet;
use std::thread::sleep;
use std::{collections::HashMap, path::Path};
use std::time::Duration;
use notify_debouncer_mini::notify::FsEventWatcher;
use notify_debouncer_mini::Debouncer;
use notify_debouncer_mini::{
  notify::RecursiveMode,
  new_debouncer, 
  DebounceEventResult,
};
use std::sync::{Arc, Mutex};
use std::{panic, thread};
use log::{info, error, debug};
use crate::commands::{NodeCommands, NodeResponses};
use crate::model::sedaroml::write_model;
use crate::change_queue::{ChangeQueue, QueuedSet};
use crate::translations::{Translation, OperationFunction, TranslationResult};
use crate::nodes::traits::Exchangeable;

pub struct Exchange {
  change_queue: ChangeQueue,
  nodes: Arc<Mutex<HashMap<String, Arc<Mutex<dyn Exchangeable + Sync + Send>>>>>,
  translation_thread: thread::JoinHandle<()>,
  watchers: Vec<Debouncer<FsEventWatcher>>,
}
impl Exchange {
  pub fn new(translations: Vec<Translation>) -> Exchange {
    info!("Exchange is in startup...");
    let mut pairs = vec![];
    let nodes = Arc::new(Mutex::new(HashMap::new()));
    let nodes_clone_for_constructor = nodes.clone();
    let mut nodes = nodes.lock().unwrap();
    let mut translations_index = HashMap::new();
    let change_queue = Arc::new(Mutex::new(QueuedSet::new()));
    let change_queue_clone = change_queue.clone();

    // Check for no duplicate translation pairs via unique model identifiers
    for translation in translations {
      let from = translation.from.clone();
      let from = from.lock().unwrap();
      let to = translation.to.clone();
      let to = to.lock().unwrap();
      debug!("Registering translation: from: {}, to: {}", from.identifier(), to.identifier());
      if from.identifier() == to.identifier() {
        panic!("Translation from and to models must be different: `{}` == `{}`", from.identifier(), to.identifier());
      }
      let mut pair = vec![from.identifier().clone(), to.identifier().clone()];
      pair.sort_unstable();
      let from_iden = from.identifier().clone();
      let to_iden = to.identifier().clone();
      nodes.insert(from_iden.clone(), translation.from);
      nodes.insert(to_iden.clone(), translation.to);
      if pairs.contains(&pair) {
        panic!("Duplicate translation pair: {}, {}", from_iden, to_iden);
      }

      if !translations_index.contains_key(&from_iden) {
        translations_index.insert(from_iden.clone(), HashMap::new());
      }
      if !translations_index.get(&from_iden).unwrap().contains_key(&to_iden) {
        translations_index.get_mut(&from_iden).unwrap().insert(to_iden.clone(), vec![]);
      }
      if !translations_index.contains_key(&to_iden) {
        translations_index.insert(to_iden.clone(), HashMap::new());
      }
      if !translations_index.get(&to_iden).unwrap().contains_key(&from_iden) {
        translations_index.get_mut(&to_iden).unwrap().insert(from_iden.clone(), vec![]);
      }

      for op in translation.operations {
        translations_index.get_mut(&from_iden).unwrap().get_mut(&to_iden).unwrap().push(OperationFunction::Forward(op.name.clone(), op.forward));
        translations_index.get_mut(&to_iden).unwrap().get_mut(&from_iden).unwrap().push(OperationFunction::Reverse(op.name, op.reverse));
      }

      pairs.push(pair);
    }

    // Start all nodes and read in their representations
    for node in nodes.values() {
      let mut node = node.lock().unwrap();
      match node.tx_to_node_blocking(NodeCommands::Start) {
        NodeResponses::Started => {},
        _ => { panic!("Failed to start node: {}", node.identifier()) }
      }
      node.refresh_representation();
    }

    // Bind watchers for models
    let mut watchers = vec![];
    let queue = change_queue;
    for model in nodes.values_mut() {
      let model = model.lock().unwrap();
      let debouncer = setup_file_watcher(model.identifier(), model.sedaroml_filename(), queue.clone());
      watchers.push(debouncer);
    }

    let nodes_clone = nodes.clone();
    let handler = thread::spawn(move || {
      let mut nodes = nodes_clone;
      let mut visited = HashSet::new();
      loop {
        let queue = queue.clone();
        let mut queue = queue.lock().unwrap();
        let change = queue.dequeue();
        drop(queue); // Release lock so other threads can enqueue
        if let Some(change) = change {
          let change = change.clone();
          info!("Detected change: {}", change);
          visited.insert(change.clone());
          let translation = translations_index.get(&change).unwrap();
          let from = nodes.get(&change).unwrap().clone();
          let mut from = from.lock().unwrap();

          if !translation.is_empty() { // Optimization
            from.refresh_representation(); // Refresh the model from disk
          }

          for (to_iden, operations) in translation { // TODO: Make this order deterministic
            if visited.contains(&to_iden.clone()) {
              debug!("Already visited: {}. Skipping...", to_iden);
              continue;
            }

            let to = nodes.get_mut(&to_iden.clone()).unwrap().clone();
            let mut to = to.lock().unwrap();
            let mut changed = false;
            for operation in operations {
              match operation {
                OperationFunction::Forward(op_name, op) => {
                  match op(&from.representation(), &mut to.representation_mut()) {
                    Ok(translation_status) => {
                      let arrow = match op_name {
                        Some(op_name) => format!("-- '{}' -->", op_name),
                        None => "-->".into(),
                      };
                      info!("Translation: '{}' {} '{}': {:?}", from.identifier(), arrow, to.identifier(), translation_status);
                      match translation_status {
                        TranslationResult::Changed => { changed = true },
                        TranslationResult::Unchanged => {},
                      }
                    },
                    Err(e) => panic!("Translation {} -> {} failed: {:?}", from.identifier(), to.identifier(), e),
                  }
                },
                OperationFunction::Reverse(op_name, op) => {
                  match op(&from.representation(), &mut to.representation_mut()) {
                    Ok(translation_status) => {
                      let arrow = match op_name {
                        Some(op_name) => format!("-- '{}'^-1 -->", op_name),
                        None => "-->".into(),
                      };
                      info!("Translation: '{}' {} '{}': {:?}", from.identifier(), arrow, to.identifier(), translation_status);
                      match translation_status {
                        TranslationResult::Changed => { changed = true },
                        TranslationResult::Unchanged => {},
                      }
                    },
                    Err(e) => panic!("Translation {} -> {} failed: {:?}", from.identifier(), to.identifier(), e),
                  }
                },
              }
            }
            // Write model and notify node that its translation is complete in the current round
            // Note that its important that a translation into a node only ever happen from one other node in a given round
            // not from > 1.  
            if changed { 
              write_model(&to.sedaroml_filename(), &to.representation()).unwrap_or_else(
                |e| panic!("Failed to write model to file: {}: {:?}", to.sedaroml_filename(), e)
              );
              to.tx_to_node(NodeCommands::Changed);
            } else {
              handle_unchanged(&to_iden, &mut visited, &translations_index); // Recursively add all deps to visited
            }
            to.tx_to_node(NodeCommands::Done);
          }

          // If every node has been visited, translation round is complete
          if visited.len() == nodes.len() {
            info!("Translation complete.");
            visited.clear();
          }
        } else {
          sleep(Duration::from_millis(10));
        }
      }
    });
    info!("Ready.");
    Exchange {
      change_queue: change_queue_clone,
      nodes: nodes_clone_for_constructor,
      translation_thread: handler,
      watchers,
    }
  }
  pub fn wait(self) {
    self.translation_thread.join().unwrap();
  }
  pub fn trigger_watch_for_model(&self, iden: String) {
    // TODO: Validate that iden is a valid model identifier
    self.change_queue.lock().unwrap().enqueue(iden.to_string());
  }
}


fn handle_unchanged(iden: &str, visited: &mut HashSet<String>, translations: &HashMap<String, HashMap<String, Vec<OperationFunction>>>) {
  if visited.contains(iden) { return; }
  visited.insert(iden.to_string());
  for (to_iden, _) in translations.get(iden).unwrap() {
    handle_unchanged(to_iden, visited, translations);
  }
}

fn setup_file_watcher(identifier: String, path: String, queue: ChangeQueue) -> Debouncer<FsEventWatcher> {
  let identifier = identifier.clone();
  let mut debouncer = new_debouncer(Duration::from_millis(5), move |res: DebounceEventResult| {
    match res {
      Ok(_event) => { queue.lock().unwrap().enqueue(identifier.to_string()) },
      Err(e) => error!("watch error: {:?}", e),
    }
  }).unwrap_or_else(|_| panic!("Failed to create debouncer"));
  let watcher = debouncer.watcher();
  watcher.watch(&Path::new(&path), RecursiveMode::Recursive).unwrap_or_else(|e| panic!("Failed to watch path: {}: {}", path, e));
  debouncer
}