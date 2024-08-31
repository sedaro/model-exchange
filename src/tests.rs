#[cfg(test)]
mod tests {
  use std::thread::sleep;
  use std::time::Duration;
  use serde_json::Value;
  use crate::model::sedaroml::{Model, Block};
  use crate::nodes::sedaroml::SedaroML;
  use crate::exchange::Exchange;
  use crate::translations::{Operation, Translation, TranslationResult};
  use crate::nodes::traits::Exchangeable;


  #[test]
  fn test_simple_exchange() {
    let a = SedaroML::new("a".into(), "a.txt".into());
    let b = SedaroML::new("b".into(), "b.txt".into());
    let c = SedaroML::new("c".into(), "c.txt".into());
    let d = SedaroML::new("d".into(), "d.txt".into());
    let e = SedaroML::new("e".into(), "e.txt".into());
    let add_one = Operation {
      name: Some("+1".into()),
      forward: |from: &Model, to: &mut Model| {
        let v = from.blocks.get("i").unwrap().get("v").unwrap().as_i64().unwrap() + 1;
        to.blocks.insert("i".into(), Block::from_iter([("v".into(), Value::Number(v.into()))]));
        Ok(TranslationResult::Changed)
      },
      reverse: |from: &Model, to: &mut Model| {
        let v = from.blocks.get("i").unwrap().get("v").unwrap().as_i64().unwrap() - 1;
        to.blocks.insert("i".into(), Block::from_iter([("v".into(), Value::Number(v.into()))]));
        Ok(TranslationResult::Changed)
      },
    };
    let multiply_two = Operation {
      name: Some("*2".into()),
      forward: |from: &Model, to: &mut Model| {
        let v = from.blocks.get("i").unwrap().get("v").unwrap().as_i64().unwrap() * 2;
        to.blocks.insert("i".into(), Block::from_iter([("v".into(), Value::Number(v.into()))]));
        Ok(TranslationResult::Changed)
      },
      reverse: |from: &Model, to: &mut Model| {
        let v = from.blocks.get("i").unwrap().get("v").unwrap().as_i64().unwrap() / 2;
        to.blocks.insert("i".into(), Block::from_iter([("v".into(), Value::Number(v.into()))]));
        Ok(TranslationResult::Changed)
      },
    };
    let multiply_ten = Operation {
      name: Some("*10".into()),
      forward: |from: &Model, to: &mut Model| {
        let v = from.blocks.get("i").unwrap().get("v").unwrap().as_i64().unwrap() * 10;
        to.blocks.insert("i".into(), Block::from_iter([("v".into(), Value::Number(v.into()))]));
        Ok(TranslationResult::Changed)
      },
      reverse: |from: &Model, to: &mut Model| {
        let v = from.blocks.get("i").unwrap().get("v").unwrap().as_i64().unwrap() / 10;
        to.blocks.insert("i".into(), Block::from_iter([("v".into(), Value::Number(v.into()))]));
        Ok(TranslationResult::Changed)
      },
    };
    let noop = Operation {
      name: Some("noop".into()),
      forward: |_, _| { Ok(TranslationResult::Unchanged) },
      reverse: |_, _| { Ok(TranslationResult::Unchanged) },
    };
    let t_a = Translation {
      from: a.clone(),
      to: b.clone(),
      operations: vec![add_one],
    };
    let multiply_two_clone = multiply_two.clone();
    let t_b = Translation {
      from: b.clone(),
      to: c.clone(),
      operations: vec![multiply_two],
    };
    let t_c = Translation {
      from: b.clone(),
      to: e.clone(),
      operations: vec![multiply_ten],
    };
    let t_d = Translation {
      from: c.clone(),
      to: d.clone(),
      operations: vec![noop, multiply_two_clone],
    };
    let exchange = Exchange::new(vec![t_a, t_b, t_c, t_d]);
  
    exchange.trigger_watch_for_model("e".into());
  
    // println!("A: {:?}", models.get("a").unwrap().lock().unwrap().representation());
    // println!("B: {:?}", models.get("b").unwrap().lock().unwrap().representation());
    // println!("C: {:?}", models.get("c").unwrap().lock().unwrap().representation());
    // println!("D: {:?}", models.get("d").unwrap().lock().unwrap().representation());
    // println!("E: {:?}", models.get("e").unwrap().lock().unwrap().representation());
  
    // TODO: Eventually, wait on the translation round lock before running the tests
    sleep(Duration::from_millis(1000));
    println!("Starting testing...");
    assert!(*a.lock().unwrap().representation().blocks.get("i").unwrap() == Block::from_iter([("v".into(), Value::Number(9.into()))]));
    assert!(*b.lock().unwrap().representation().blocks.get("i").unwrap() == Block::from_iter([("v".into(), Value::Number(10.into()))]));
    assert!(*c.lock().unwrap().representation().blocks.get("i").unwrap() == Block::from_iter([("v".into(), Value::Number(20.into()))]));
    assert!(*d.lock().unwrap().representation().blocks.get("i").unwrap() == Block::from_iter([("v".into(), Value::Number(40.into()))]));
    assert!(*e.lock().unwrap().representation().blocks.get("i").unwrap() == Block::from_iter([("v".into(), Value::Number(100.into()))]));
    println!("All tests passed!");
  }
}