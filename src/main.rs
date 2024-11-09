use std::collections::HashMap;
use modex::nodes::sedaroml::SedaroML;
use serde_json::Value;
use modex::logging::init_logger;
use modex::model::sedaroml::{Block, Model};
use modex::nodes::sedaro::{Sedaro, SedaroCredentials};
use modex::nodes::excel::Excel;
use modex::exchange::Exchange;
use modex::translations::{Operation, Translation};
use modex::utils::read_json;


#[tokio::main]
async fn main() {
  init_logger().expect("Failed to initialize logger.");
  
  let secrets = read_json("secrets.json").expect("Failed to read secrets.json");
  let api_key = secrets.get("ALPHA").unwrap().as_str().unwrap();
  
  let excel = Excel::new("test.xlsx".into(), "test.xlsx".into());
  let sedaro = Sedaro::new(
    "Wildfire".into(),
    "https://api.astage.sedaro.com".into(),
    "PPSbzJYtrMBV4lhgFXzK5Q".into(),
    SedaroCredentials::ApiKey(api_key.to_string()),
  );
  let csm = SedaroML::new("csm.json".into(), "csm.json".into());

  let csm_to_sedaro = Operation {
    name: Some("Cameo-Sedaro".into()),
    forward: |from: &Model, to: &mut Model| {
      // Get all CSM Transitions and build map of (source name, target name) -> BlockID
      let from_map = from.block_ids_of_type("Transition").unwrap().iter().map(|id| {
        let block = from.block_by_id(id).expect("Block not found");
        let source_block = from.block_by_id(block.get("source").unwrap().as_str().unwrap()).unwrap();
        let source_name = source_block.get("name").unwrap();
        let target_block = from.block_by_id(block.get("target").unwrap().as_str().unwrap()).unwrap();
        let target_name = target_block.get("name").unwrap();
        ((source_name.as_str().unwrap().to_string(), target_name.as_str().unwrap().to_string()), id.clone())
      }).collect::<HashMap<_, _>>();

      // Reconcile in the to Model
      // Get all Sedaro Transitions and build map of (source name, target name) -> BlockID
      let mut to_map = to.block_ids_of_type("StateTransition").unwrap().iter().map(|id| {
        let block = to.block_by_id(id).expect("Block not found");
        let source_name = to.block_by_id(block.get("fromState").unwrap().as_str().unwrap()).unwrap().get("name").unwrap();
        let target_name = to.block_by_id(block.get("toState").unwrap().as_str().unwrap()).unwrap().get("name").unwrap();
        let pair = (source_name.as_str().unwrap().to_string(), target_name.as_str().unwrap().to_string());
        // Remove missing transitions
        if !from_map.contains_key(&pair) {
          println!("Removing {:?}", pair);
          to.blocks.swap_remove(id);
          // Update index
          let arr = to.index.get_mut("StateTransition").unwrap();
          let index = arr.iter().position(|x| *x == *id).unwrap();
          arr.remove(index);
          // Remove from FSM block
          let fsm = to.get_first_block_where_mut(&HashMap::from([("name".to_string(), "FSM".into()), ("type".to_string(), "FiniteStateMachine".into())])).expect("Block not found");
          let arr = fsm.get_mut("transitions").unwrap().as_array_mut().unwrap();
          let index = arr.iter().position(|x| *x == Value::String(id.clone())).unwrap();
          arr.remove(index);
        }
        (pair, id.clone())
      }).collect::<HashMap<_, _>>();

      // Add new transitions
      let mut i = 0;
      // println!("TO MAP: {:?}", to_map);
      from_map.iter().for_each(|(key, _)| {
        if key.0 != "" { // Edge case for handing the Pseudo State (i.e. the starting state)
          if !to_map.contains_key(key) {
            println!("Adding {:?}", key);
            let source = to.get_first_block_where(&HashMap::from([("name".to_string(), key.0.clone().into()), ("type".to_string(), "Routine".into())])).expect("Block not found");
            let target = to.get_first_block_where(&HashMap::from([("name".to_string(), key.1.clone().into()), ("type".to_string(), "Routine".into())])).expect("Block not found");
            let id = format!("$temp-{}", i);
            to.blocks.insert(id.clone(), Block::from_iter([
              ("id".into(), Value::String(id.clone())),
              ("type".into(), Value::String("StateTransition".into())),
              ("fromState".into(), source.get("id").unwrap().clone()),
              ("toState".into(), target.get("id").unwrap().clone()),
              ("conditions".into(), Value::Array(vec![])),
              ("priority".into(), Value::Number(i.into())),
            ]));
            let fsm = to.get_first_block_where_mut(&HashMap::from([("name".to_string(), "FSM".into()), ("type".to_string(), "FiniteStateMachine".into())])).expect("Block not found");
            fsm.get_mut("transitions").unwrap().as_array_mut().unwrap().push(Value::String(id.clone()));
            to_map.insert(key.clone(), id.clone());
            // Update index
            let arr = to.index.get_mut("StateTransition").unwrap();
            arr.push(id.clone());
            i += 1;
          }
        }
      });
      Ok(())
    },
    reverse: |_, _| {
      Ok(())
    },
  };

  let csm_to_excel = Operation {
    name: Some("Cameo-Excel".into()),
    forward: |from: &Model, to: &mut Model| {
      let block = from.block_by_id("_2022x_14310360_1715122805261_303999_2891").expect("Block not found");
      let mass = block.get("value").unwrap().as_f64().unwrap();
      let filter = HashMap::from([("name".to_string(), Value::String("spacecraft_dry_mass".into()))]);
      let battery_esr_name = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      battery_esr_name.insert("value".to_string(), mass.into());
      Ok(())
    },
    reverse: |_, _| {
      Ok(())
    },
  };

  let ta = Translation {
    from: csm.clone(),
    to: sedaro.clone(),
    operations: vec![csm_to_sedaro],
  };
  let tb = Translation {
    from: csm.clone(),
    to: excel.clone(),
    operations: vec![csm_to_excel],
  };

  let exchange = Exchange::new(vec![ta, tb]);
  exchange.wait();
}
