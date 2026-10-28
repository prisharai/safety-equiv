use safety_net::Netlist;
use safety_pass::{Cell, CellType, ModInst};
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MiterError {
    #[error("input counts do not match")]
    InputCountMismatch,

    #[error("output counts do not match")]
    OutputCountMismatch,

    #[error("input names do not match")]
    InputNameMismatch,

    #[error("output names do not match")]
    OutputNameMismatch,

    #[error("sequential circuits are not supported")]
    SequentialNotSupported,

    #[error("netlist has no outputs")]
    NoOutputs,

    #[error("netlist error: {0}")]
    NetlistError(#[from] safety_net::Error),
}

fn validate_interfaces(
    gold: &Rc<Netlist<Cell>>,
    gate: &Rc<Netlist<Cell>>,
) -> Result<(), MiterError> {
    let mut gold_inputs: Vec<_> = gold.get_input_ports().collect();
    let mut gate_inputs: Vec<_> = gate.get_input_ports().collect();

    // Same number of inputs
    if gold_inputs.len() != gate_inputs.len() {
        return Err(MiterError::InputCountMismatch);
    }

    // Same input names

    gold_inputs.sort_by_key(|input| input.get_identifier().to_string()); // compare inputs by name alphabetically
    gate_inputs.sort_by_key(|input| input.get_identifier().to_string());

    for (gold_input, gate_input) in gold_inputs.iter().zip(gate_inputs.iter()) {
        if gold_input.get_identifier() != gate_input.get_identifier() {
            return Err(MiterError::InputNameMismatch);
        }
    }

    let mut gold_outputs = gold.get_output_ports();
    let mut gate_outputs = gate.get_output_ports();

    // Need at least one output
    if gold_outputs.is_empty() || gate_outputs.is_empty() {
        return Err(MiterError::NoOutputs);
    }

    // Same number of outputs
    if gold_outputs.len() != gate_outputs.len() {
        return Err(MiterError::OutputCountMismatch);
    }

    // Same output names
    gold_outputs.sort_by_key(|output| output.get_identifier().to_string()); // compare inputs by name alphabetically
    gate_outputs.sort_by_key(|output| output.get_identifier().to_string());

    for (gold_output, gate_output) in gold_outputs.iter().zip(gate_outputs.iter()) {
        if gold_output.get_identifier() != gate_output.get_identifier() {
            return Err(MiterError::OutputNameMismatch);
        }
    }

    Ok(())
}

pub fn build_miter(
    gold: &Rc<Netlist<Cell>>,
    gate: &Rc<Netlist<Cell>>,
) -> Result<Rc<Netlist<Cell>>, MiterError> {
    validate_interfaces(gold, gate)?; // matching interfaces 

    let miter = Netlist::new("miter".into());

    let mut gold_inputs: Vec<_> = gold.get_input_ports().collect();
    gold_inputs.sort_by_key(|input| input.get_identifier().to_string());

    let mut shared_inputs = HashMap::new(); // shared miter input for each input name

    for input in gold_inputs {
        let name = input.get_identifier().to_string();
        let driver = miter.insert_input(input);

        shared_inputs.insert(name, driver);
    }

    let gold_drivers = gold //ordering
        .get_input_ports()
        .map(|input| {
            shared_inputs
                .get(&input.get_identifier().to_string())
                .cloned()
        })
        .collect::<Vec<_>>();

    let gate_drivers = gate // ordering
        .get_input_ports()
        .map(|input| {
            shared_inputs
                .get(&input.get_identifier().to_string())
                .cloned()
        })
        .collect::<Vec<_>>();

    // Inline gold circuit
    let gold_mod = ModInst::new(gold);
    let gold_outputs = gold_mod.inline_into(&miter, "gold".into(), &gold_drivers)?;

    // Inline gate circuit
    let gate_mod = ModInst::new(gate);
    let gate_outputs = gate_mod.inline_into(&miter, "gate".into(), &gate_drivers)?;

    let mut gold_output_pairs = gold
        .get_output_ports()
        .into_iter()
        .zip(gold_outputs.into_iter())
        .collect::<Vec<_>>();

    let mut gate_output_pairs = gate
        .get_output_ports()
        .into_iter()
        .zip(gate_outputs.into_iter())
        .collect::<Vec<_>>();

    gold_output_pairs.sort_by_key(|(port, _)| port.get_identifier().to_string());
    gate_output_pairs.sort_by_key(|(port, _)| port.get_identifier().to_string());

    let mut mismatches = Vec::new();

    for (i, ((_, gold_output), (_, gate_output))) in gold_output_pairs
        .into_iter()
        .zip(gate_output_pairs.into_iter())
        .enumerate()
    {
        let xor_output = miter
            .insert_gate(
                Cell::new(CellType::XOR2, None),
                format!("xor_{i}").into(),
                &[gold_output, gate_output],
            )?
            .get_output(0);

        mismatches.push(xor_output);
    }

    // or mismatches
    let mut iter = mismatches.into_iter();

    let mut mismatch = iter.next().ok_or(MiterError::NoOutputs)?;

    for (i, next_mismatch) in iter.enumerate() {
        mismatch = miter
            .insert_gate(
                Cell::new(CellType::OR2, None),
                format!("or_{i}").into(),
                &[mismatch, next_mismatch],
            )?
            .get_output(0);
    }

    mismatch.expose_with_name("mismatch".into());

    Ok(miter)
}
