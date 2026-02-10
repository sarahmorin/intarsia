// --------------------------------------------
// -- ISLE Generated Code Integration
// --------------------------------------------
// Include the ISLE-generated code
#[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
#[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
#[allow(unreachable_patterns, unreachable_code)]
#[path = "isle/test.rs"]
mod test;
use test::*;

// -- Required for error reporting in generated ISLE code --
// NOTE: When using a multiconstructor, you must set a maximum number of returns.
// You also need to define the ConstructorVec type for the multiconstructor.
const MAX_ISLE_RETURNS: usize = 100;
type ConstructorVec<T> = Vec<T>;

// ISLE's `(type str (primitive str))` expects a Rust type named `str`.
// Using `String` keeps this sized and easy to work with.
#[allow(non_camel_case_types)]
// pub type str = String;
// pub type Id = u64;
// --------------------------------------------
use egg::{EGraph, Id, define_language};

define_language! {
    enum testlang {
        // Constant Values
        Int(i64),
        // Dummy Operation
        "+" = DummyOp([Id; 2]),
    }
}

pub struct TestContext {
    pub egraph: EGraph<testlang, ()>,
    pub merge_queue: Vec<Vec<Id>>,
}

impl TestContext {
    pub fn new() -> Self {
        TestContext {
            egraph: EGraph::default(),
            merge_queue: Vec::new(),
        }
    }

    pub fn process_merge_queue(&mut self) {
        for id_set in self.merge_queue.drain(..) {
            let first_id = id_set[0];
            for &other_id in &id_set[1..] {
                self.egraph.union(first_id, other_id);
            }
        }
        self.egraph.rebuild();
    }
}

impl Context for TestContext {
    type e_int_val_returns = ContextIterWrapper<ConstructorVec<i64>, Self>;
    fn e_int_val(&mut self, arg0: Id, returns: &mut Self::e_int_val_returns) -> () {
        // Exract all integer values in the given e-class
        // Since these are terminal, we dont need to call runner here
        for node in self.egraph.nodes_in_class(arg0) {
            if let testlang::Int(val) = node {
                returns.extend(Some(*val));
                if returns.len() >= MAX_ISLE_RETURNS {
                    return;
                }
            }
        }
    }

    type c_int_val_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn c_int_val(&mut self, arg0: i64, returns: &mut Self::c_int_val_returns) -> () {
        let (id, is_new) = self.egraph.add_with_flag(testlang::Int(arg0));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }

        if is_new {
            self.c_run(id);
        }
    }

    type e_dummy_op_returns = ContextIterWrapper<ConstructorVec<(Id, Id)>, Self>;
    fn e_dummy_op(&mut self, arg0: Id, returns: &mut Self::e_dummy_op_returns) -> () {
        // Extract all DummyOp operations in the given e-class
        let mut to_run = Vec::new();
        for node in self.egraph.nodes_in_class(arg0).cloned() {
            if let testlang::DummyOp([lhs, rhs]) = node {
                to_run.push((lhs, rhs));
            }
        }

        for (lhs, rhs) in to_run {
            let canon_lhs = self.c_run(lhs);
            let canon_rhs = self.c_run(rhs);
            returns.extend(Some((canon_lhs, canon_rhs)));
            if returns.len() >= MAX_ISLE_RETURNS {
                return;
            }
        }
    }

    type c_dummy_op_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn c_dummy_op(&mut self, arg0: Id, arg1: Id, returns: &mut Self::c_dummy_op_returns) -> () {
        let (id, is_new) = self.egraph.add_with_flag(testlang::DummyOp([arg0, arg1]));
        returns.extend(Some(id));
        if returns.len() >= MAX_ISLE_RETURNS {
            return;
        }
        if is_new {
            self.c_run(id);
        }
    }

    fn r_run(&mut self, arg0: Id) -> Option<Id> {
        None
    }

    /// Runner Constructor for wrapping opt calls and queueing merging
    fn c_run(&mut self, arg0: Id) -> Id {
        // Call opt constructor to get set of equivalent Ids
        let mut id_set = Vec::new();
        id_set.push(arg0);
        test::constructor_opt(self, arg0, &mut id_set);

        // Enqueue for merging later
        self.merge_queue.push(id_set.clone());

        // TODO: option to process merge queue immediately?
        self.process_merge_queue();

        // Return canonical id
        self.egraph.find(arg0)
    }
}

pub fn run_test_context() {
    let mut ctx = TestContext::new();
    // let mut id1_returns = ConstructorVec::new();

    // Preload the e-graph with some expressions
    let id1 = ctx.egraph.add(testlang::Int(1));
    let id2 = ctx.egraph.add(testlang::Int(2));
    let id2p1 = ctx.egraph.add(testlang::DummyOp([id2, id1]));
    let id = ctx.egraph.add(testlang::DummyOp([id1, id2p1]));

    // Run one pass of top down optimizations
    // NOTE: I think when we don't start with a pre-loaded e-graph, we rediscover the same stuff bottom up
    // constructor_main(&mut ctx, &mut id1_returns);
    ctx.c_run(id);

    // Example: Build an e-graph with some expressions

    // Process the merge queue to finalize optimizations
    // ctx.process_merge_queue();

    // Print the optimized e-graph
    // println!("{:#?}", ctx.egraph);

    ctx.egraph
        .dot()
        .to_png("target/test_context_egraph.png")
        .unwrap();
    // println!("E-graph dot file: {}", ctx.egraph.dot());
}
