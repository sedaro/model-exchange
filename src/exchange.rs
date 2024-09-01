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
use colored::Colorize;
use std::time::Instant;

pub struct Exchange {
  change_queue: ChangeQueue,
  pub nodes: Arc<Mutex<HashMap<String, Arc<Mutex<dyn Exchangeable + Sync + Send>>>>>,
  translation_thread: thread::JoinHandle<()>,
  pub watchers: Vec<Debouncer<FsEventWatcher>>,
}
impl Exchange {
  pub fn new(translations: Vec<Translation>) -> Exchange {
    let startup_time = Instant::now();
    info!("Exchange is in startup...");
    let mut pairs = vec![];
    let nodes = Arc::new(Mutex::new(HashMap::new()));
    let nodes_clone_for_constructor = nodes.clone();
    let mut nodes = nodes.lock().unwrap();
    let mut translations_index = HashMap::new();
    let change_queue = Arc::new(Mutex::new(QueuedSet::new()));
    let change_queue_clone = change_queue.clone();
    let mut filenames = HashSet::new();

    // Validation and setup
    for translation in translations {
      if Arc::ptr_eq(&translation.from, &translation.to) { // This must happen before any locking to prvent deadlock
        let from = translation.from.clone().lock().unwrap().identifier().clone();
        let to = translation.to.clone().lock().unwrap().identifier().clone();
        panic!("Translation `from` and `to` models must be different: Offending model identifiers: `{}` & `{}`", from, to);
      }
      let from = translation.from.clone(); 
      let from = from.lock().unwrap();
      let to = translation.to.clone();
      let to = to.lock().unwrap();
      debug!("Registering translation: from: {}, to: {}", from.identifier(), to.identifier());
      if from.identifier() == to.identifier() {
        panic!("Translation `from` and `to` models must have different identifiers: `{}` == `{}`", from.identifier(), to.identifier());
      }
      let mut pair = vec![from.identifier().clone(), to.identifier().clone()];
      pair.sort_unstable();
      let from_iden = from.identifier().clone();
      let to_iden = to.identifier().clone();
      if nodes.contains_key(&from_iden) && !Arc::ptr_eq(nodes.get(&from_iden).unwrap(), &translation.from) {
        panic!("Duplicate model identifier detected: `{}`", from_iden);
      }
      if nodes.contains_key(&to_iden) && !Arc::ptr_eq(nodes.get(&to_iden).unwrap(), &translation.to) {
        panic!("Duplicate model identifier detected: `{}`", to_iden);
      }
      if !nodes.contains_key(&from_iden) && filenames.contains(&from.sedaroml_filename()) {
        panic!("Duplicate filename detected: `{}`", from.sedaroml_filename());
      }
      filenames.insert(from.sedaroml_filename().clone());
      if !nodes.contains_key(&to_iden) && filenames.contains(&to.sedaroml_filename()) {
        panic!("Duplicate filename detected: `{}`", to.sedaroml_filename());
      }
      filenames.insert(to.sedaroml_filename().clone());
      nodes.insert(from_iden.clone(), translation.from);
      nodes.insert(to_iden.clone(), translation.to);
      if pairs.contains(&pair) {
        panic!("Duplicate translation pair detected: From: `{}`, To: `{}`", from_iden, to_iden);
      }

      // TODO: Cycle detection

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
      node.refresh_rep();
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
      let mut visited_nodes = HashSet::new();
      let mut changed_nodes = HashSet::new();
      let mut round_time: Option<Instant> = None;
      loop {
        let queue = queue.clone();
        let mut queue = queue.lock().unwrap();
        let change = queue.dequeue();
        drop(queue); // Release lock so other threads can enqueue
        if let Some(change) = change {
          if round_time.is_none() {
            round_time = Some(Instant::now());
          }
          let change = change.clone();
          info!("{} {}", "Change:".cyan(), change);
          visited_nodes.insert(change.clone());
          let translation = translations_index.get(&change).unwrap();
          let from = nodes.get(&change).unwrap().clone();
          let mut from = from.lock().unwrap();

          if !translation.is_empty() { // Optimization
            from.refresh_rep(); // Refresh the model from disk
          }

          for (to_iden, operations) in translation { // TODO: Make this order deterministic
            if visited_nodes.contains(&to_iden.clone()) {
              info!("  No dependent translations remaining.");
              continue;
            }

            let to = nodes.get_mut(&to_iden.clone()).unwrap().clone();
            let mut to = to.lock().unwrap();
            let mut changed = false;
            for operation in operations {
              match operation {
                OperationFunction::Forward(op_name, op) => {
                  match op(&from.rep(), &mut to.rep_mut()) {
                    Ok(translation_status) => {
                      let arrow = match op_name {
                        Some(op_name) => format!("--({})-->", op_name),
                        None => "-->".into(),
                      };
                      let mut result_str = "Changed".green();
                      match translation_status {
                        TranslationResult::Changed => { changed = true },
                        TranslationResult::Unchanged => { result_str = "Unchanged".yellow() },
                      }
                      info!("  Translation: {} {} {}: {}", from.identifier(), arrow, to.identifier(), result_str);
                    },
                    Err(e) => panic!("Translation {} -> {} failed: {:?}", from.identifier(), to.identifier(), e),
                  }
                },
                OperationFunction::Reverse(op_name, op) => {
                  match op(&from.rep(), &mut to.rep_mut()) {
                    Ok(translation_status) => {
                      let arrow = match op_name {
                        Some(op_name) => format!("--({})^-1-->", op_name),
                        None => "-->".into(),
                      };
                      let mut result_str = "Changed".green();
                      match translation_status {
                        TranslationResult::Changed => { changed = true },
                        TranslationResult::Unchanged => { result_str = "Unchanged".yellow() },
                      }
                      info!("  Translation: {} {} {}: {}", from.identifier(), arrow, to.identifier(), result_str);
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
              changed_nodes.insert(to_iden.clone());
              write_model(&to.sedaroml_filename(), &to.rep()).unwrap_or_else(
                |e| panic!("Failed to write model to file: {}: {:?}", to.sedaroml_filename(), e)
              );
              to.tx_to_node(NodeCommands::Changed);
            } else {
              handle_unchanged(&to_iden, &mut visited_nodes, &translations_index); // Recursively add all deps to visited
            }
            to.tx_to_node(NodeCommands::Done);
          }

          // Drop locked model (so it can be locked again below if need by during round close-out)
          drop(from);

          // If every node has been visited, translation round is complete
          if visited_nodes.len() == nodes.len() {
            if !changed_nodes.is_empty() {
              info!("Waiting for node side-effects to complete...");
              let mut heard_from = HashSet::new();
              let changed_nodes_locked = changed_nodes.iter().map(|iden| nodes.get(iden).unwrap().lock().unwrap()).collect::<Vec<_>>();
              while heard_from.len() < changed_nodes.len() {
                for node in &changed_nodes_locked {
                  if !heard_from.contains(&node.identifier()) {
                    match node.rx_from_node_timeout(Duration::from_millis(10)) {
                      Ok(NodeResponses::Done(t)) => { 
                        heard_from.insert(node.identifier().clone());
                        info!("  {}: {} {:.2}s", node.identifier(), "Done".green(), t.as_secs_f64()) 
                      },
                      _ => {},
                    }
                  }
                }
              }
            }
            let elapsed = match round_time {
              Some(round_time) => format!("{:.2}s", round_time.elapsed().as_secs_f64()),
              None => "".into(),
            };
            info!("{} {}", "Translation complete.".purple(), elapsed);
            round_time = None;
            visited_nodes.clear();
            changed_nodes.clear();
          }
        } else {
          sleep(Duration::from_millis(10));
        }
      }
    });
    info!("{} {:.2}s", "Ready.".green(), startup_time.elapsed().as_secs_f64());
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