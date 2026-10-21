use safety_equiv::build_miter;
use safety_net::{Net, Netlist};
use safety_pass::{Cell, CellType};
use std::rc::Rc;

fn make_and(name: &str) -> Rc<Netlist<Cell>> {
    let nl = Netlist::new(name.into());

    let a = nl.insert_input(Net::new_logic("a".into()));
    let b = nl.insert_input(Net::new_logic("b".into()));

    let y = nl
        .insert_gate(Cell::new(CellType::AND2, None), "and".into(), &[a, b])
        .unwrap()
        .get_output(0);

    y.expose_with_name("y".into());

    nl
}

fn make_or(name: &str) -> Rc<Netlist<Cell>> {
    let nl = Netlist::new(name.into());

    let a = nl.insert_input(Net::new_logic("a".into()));
    let b = nl.insert_input(Net::new_logic("b".into()));

    let y = nl
        .insert_gate(Cell::new(CellType::OR2, None), "or".into(), &[a, b])
        .unwrap()
        .get_output(0);

    y.expose_with_name("y".into());

    nl
}

#[test]
fn build_identical_miter() {
    let gold = make_and("gold");
    let gate = make_and("gate");

    let miter = build_miter(&gold, &gate).unwrap();

    assert!(miter.verify().is_ok());

    println!("{miter}");
}

#[test]
fn build_different_miter() {
    let gold = make_and("gold");
    let gate = make_or("gate");

    let miter = build_miter(&gold, &gate).unwrap();

    assert!(miter.verify().is_ok());

    println!("{miter}");
}
