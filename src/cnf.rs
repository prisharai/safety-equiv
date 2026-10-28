use safety_net::{Identifier, Netlist};
use safety_pass::{Cell, CellType};
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CnfError {
    #[error("cell {0} has a disconnected input")]
    DisconnectedInput(String),

    #[error("no SAT variable exists for net {0}")]
    MissingVariable(String),

    #[error("unsupported cell type {cell_type:?} in cell {instance}")]
    UnsupportedCell {
        instance: String,
        cell_type: CellType,
    },

    #[error("cell {0} has an unexpected number of inputs or outputs")]
    WrongArity(String),

    #[error("miter has no output named mismatch")]
    MissingMismatchOutput,
}

#[derive(Debug, Clone)]
pub struct Cnf {
    num_vars: i32,
    clauses: Vec<Vec<i32>>,
    variables: HashMap<Identifier, i32>,
}

impl Cnf {
    pub fn new() -> Self {
        Self {
            num_vars: 0,
            clauses: Vec::new(),
            variables: HashMap::new(),
        }
    }

    fn ensure_variable(&mut self, name: Identifier) -> i32 {
        // Give net a SAT variable number (x1 x2 ...)
        if let Some(var) = self.variables.get(&name) {
            return *var;
        }

        self.num_vars += 1;
        self.variables.insert(name, self.num_vars);

        self.num_vars
    }

    pub fn variable(&self, name: &Identifier) -> Option<i32> {
        self.variables.get(name).copied()
    }

    pub fn add_clause(&mut self, clause: Vec<i32>) {
        self.clauses.push(clause);
    }

    pub fn num_vars(&self) -> i32 {
        self.num_vars
    }

    pub fn clauses(&self) -> &[Vec<i32>] {
        &self.clauses
    }

    pub fn to_dimacs(&self) -> String {
        // exporting cnf to dimacs format
        let mut result = String::new();

        let mut variables = self // DEBUGGING: printing variables mappings as comments
            .variables
            .iter()
            .map(|(name, var)| (*var, name.to_string()))
            .collect::<Vec<_>>();

        variables.sort_by_key(|(var, _)| *var);

        for (var, name) in variables {
            result.push_str(&format!("c {var} {name}\n"));
        }

        result.push_str(&format!("p cnf {} {}\n", self.num_vars, self.clauses.len()));

        for clause in &self.clauses {
            for literal in clause {
                result.push_str(&format!("{literal} "));
            }

            result.push_str("0\n");
        }

        result
    }

    // TSEITIN GATE ENCODINGS*
    fn add_and(&mut self, a: i32, b: i32, z: i32) {
        // z <-> (a AND b)
        self.add_clause(vec![-a, -b, z]);
        self.add_clause(vec![a, -z]);
        self.add_clause(vec![b, -z]);
    }

    fn add_or(&mut self, a: i32, b: i32, z: i32) {
        // z <-> (a OR b)
        self.add_clause(vec![a, b, -z]);
        self.add_clause(vec![-a, z]);
        self.add_clause(vec![-b, z]);
    }

    fn add_xor(&mut self, a: i32, b: i32, z: i32) {
        // z <-> (a XOR b)
        self.add_clause(vec![-a, -b, -z]);
        self.add_clause(vec![a, b, -z]);
        self.add_clause(vec![a, -b, z]);
        self.add_clause(vec![-a, b, z]);
    }
}

pub fn netlist_to_cnf(netlist: &Rc<Netlist<Cell>>) -> Result<Cnf, CnfError> {
    let mut cnf = Cnf::new();

    for input in netlist.inputs() {
        // every input gets a sat var
        cnf.ensure_variable(input.get_identifier());
    }

    for object in netlist.objects() {
        // every gate outputs gts a sat var
        if object.is_an_input() {
            continue;
        }

        for output in object.outputs() {
            cnf.ensure_variable(output.get_identifier());
        }
    }

    // pass 2
    for object in netlist.objects() {
        // gates to cnf clauses
        if object.is_an_input() {
            continue;
        }

        let instance = object
            .get_instance_name()
            .map(|name| name.to_string())
            .unwrap_or_else(|| "<unnamed>".to_string());

        let cell_type = object
            .get_instance_type()
            .expect("non-input object should have a cell type")
            .get_type();

        let mut inputs = Vec::new();

        for input in object.inputs() {
            let driver = input
                .get_driver()
                .ok_or_else(|| CnfError::DisconnectedInput(instance.clone()))?;

            let name = driver.get_identifier();

            let var = cnf
                .variable(&name)
                .ok_or_else(|| CnfError::MissingVariable(name.to_string()))?;

            inputs.push(var);
        }

        let mut outputs = Vec::new();

        for output in object.outputs() {
            let name = output.get_identifier();

            let var = cnf
                .variable(&name)
                .ok_or_else(|| CnfError::MissingVariable(name.to_string()))?;

            outputs.push(var);
        }

        if outputs.len() != 1 {
            // *only supporting single output gates for now?
            return Err(CnfError::WrongArity(instance));
        }

        let z = outputs[0];

        match cell_type {
            CellType::AND | CellType::AND2 => {
                if inputs.len() != 2 {
                    return Err(CnfError::WrongArity(instance));
                }

                cnf.add_and(inputs[0], inputs[1], z);
            }

            CellType::OR | CellType::OR2 => {
                if inputs.len() != 2 {
                    return Err(CnfError::WrongArity(instance));
                }

                cnf.add_or(inputs[0], inputs[1], z);
            }

            CellType::XOR | CellType::XOR2 => {
                if inputs.len() != 2 {
                    return Err(CnfError::WrongArity(instance));
                }

                cnf.add_xor(inputs[0], inputs[1], z);
            }

            _ => {
                return Err(CnfError::UnsupportedCell {
                    instance,
                    cell_type,
                });
            }
        }
    }

    Ok(cnf)
}

pub fn miter_to_cnf(miter: &Rc<Netlist<Cell>>) -> Result<Cnf, CnfError> {
    let mut cnf = netlist_to_cnf(miter)?;

    let mismatch_driver = miter // find 'mismatch'
        .outputs()
        .into_iter()
        .find(|(_, port)| port.get_identifier().to_string() == "mismatch")
        .map(|(driver, _)| driver) // get its driver net
        .ok_or(CnfError::MissingMismatchOutput)?;

    let mismatch_name = mismatch_driver.get_identifier();

    let mismatch_var = cnf
        .variable(&mismatch_name)
        .ok_or_else(|| CnfError::MissingVariable(mismatch_name.to_string()))?;

    cnf.add_clause(vec![mismatch_var]); // is the mismatch ever true ? 

    Ok(cnf)
}

impl Default for Cnf {
    fn default() -> Self {
        Self::new()
    }
}
