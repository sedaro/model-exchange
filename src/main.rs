use std::collections::HashMap;
use modex::nodes::cosimulation::{Cosimulation, SimulationJobId};
use modex::nodes::sedaroml::SedaroML;
use serde_json::Value;
use modex::logging::init_logger;
use modex::model::sedaroml::Model;
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
    "PNdldNPBmJ2qRcYlBFCZnJ".into(),
    SedaroCredentials::ApiKey(api_key.to_string()),
  );
  let api_key = secrets.get("PROD").unwrap().as_str().unwrap();
  let cosim = Cosimulation::new(
    "Wildfire Cosim".into(),
    "https://api.sedaro.com".into(),
    SimulationJobId::LatestForScenario("PNhrrFtnB5XYv2qJ8RcZzN".into()),
    "NSghFfVT8ieam0ydeZGX-".into(),
    "NZ2SHUkS95z1GtmMZ0CTk".into(),
    SedaroCredentials::ApiKey(api_key.to_string()),
  );
  let test = SedaroML::new("test.json".into(), "test.json".into());

  let excel_to_sedaroml = Operation {
    name: Some("-".into()),
    forward: |from: &Model, to: &mut Model| {
      // get_first_block_where!(name='spacecraft_dry_mass').value as Mass.g -> Spacecraft.dryMass
      let filter = HashMap::from([("name".to_string(), Value::String("battery_esr".into()))]);
      let battery_esr_name = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let esr = battery_esr_name.get("value").unwrap().as_f64().unwrap();
      to.block_by_id_mut("NT0USZZSc9cZAmWJbClN-").expect("Block not found").insert("esr".to_string(), esr.into());
      Ok(())
    },
    reverse: |from: &Model, to: &mut Model| {
      // Spacecraft.root.dryMass as Mass.kg -> get_first_block_where!(name='spacecraft_dry_mass').value
      let block = from.block_by_id("NT0USZZSc9cZAmWJbClN-").expect("Block not found");
      let esr = block.get("esr").unwrap().as_f64().unwrap();
      
      let filter = HashMap::from([("name".to_string(), Value::String("battery_esr".into()))]);
      let battery_esr_name = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      battery_esr_name.insert("value".to_string(), esr.into());
      Ok(())
    },
  };

  let excel_to_cosim = Operation {
    name: Some("cosim".into()),
    forward: |from: &Model, to: &mut Model| {
      let filter = HashMap::from([("name".to_string(), Value::String("attitude_x".into()))]);
      let block = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let x = block.get("value").unwrap().as_f64().unwrap();
      let filter = HashMap::from([("name".to_string(), Value::String("attitude_y".into()))]);
      let block = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let y = block.get("value").unwrap().as_f64().unwrap();
      let filter = HashMap::from([("name".to_string(), Value::String("attitude_z".into()))]);
      let block = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let z = block.get("value").unwrap().as_f64().unwrap();
      let filter = HashMap::from([("name".to_string(), Value::String("attitude_w".into()))]);
      let block = from.get_first_block_where(&filter).expect("Block matching filter expression not found.");
      let w = block.get("value").unwrap().as_f64().unwrap();
      to.root.insert("produced_value".to_string(), serde_json::json!([{"ndarray": vec![x, y, z, w]}]));
      Ok(())
    },
    reverse: |from: &Model, to: &mut Model| {
      let vector = from.root.get("consumed_value").unwrap().get(0).unwrap().get("ndarray").unwrap().as_array().unwrap();
      let x = vector[0].as_f64().unwrap();
      let y = vector[1].as_f64().unwrap();
      let z = vector[2].as_f64().unwrap();

      let filter = HashMap::from([("name".to_string(), Value::String("position_eci_x".into()))]);
      let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      block.insert("value".to_string(), x.into());
      let filter = HashMap::from([("name".to_string(), Value::String("position_eci_y".into()))]);
      let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      block.insert("value".to_string(), y.into());
      let filter = HashMap::from([("name".to_string(), Value::String("position_eci_z".into()))]);
      let block = to.get_first_block_where_mut(&filter).expect("Block matching filter expression not found.");
      block.insert("value".to_string(), z.into());
      Ok(())
    },
  };

  let other = Operation {
    name: Some("other".into()),
    forward: |_, _| {
      Ok(())
    },
    reverse: |_, _| {
      Ok(())
    },
  };

  let t = Translation {
    from: excel.clone(),
    to: sedaro.clone(),
    operations: vec![excel_to_sedaroml],
  };
  let tt = Translation {
    from: excel.clone(),
    to: test.clone(),
    operations: vec![other],
  };
  // let exchange = Exchange::new(vec![t, tt]);

  let translation_cosim = Translation {
    from: excel.clone(),
    to: cosim.clone(),
    operations: vec![excel_to_cosim],
  };

  // let exchange = Exchange::new(vec![translation_cosim]);
  let exchange = Exchange::new(vec![translation_cosim, t, tt]);
  exchange.wait();
}
