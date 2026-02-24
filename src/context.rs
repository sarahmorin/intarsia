use crate::optimizer::rules::*;
/// Implements the context for ISLE generated code.
use egg::Id;

// -- Required for error reporting in generated ISLE code --
// NOTE: When using a multiconstructor, you must set a maximum number of returns.
// You also need to define the ConstructorVec type for the multiconstructor.
use crate::optimizer::ConstructorVec;
// --------------------------------------------
use crate::{
    optimizer::{OptimizerContext, Task},
    optlang::Optlang,
    types::ColSet,
};

use log::warn;

// Implement the Context trait for OptimizerContext, which is required by ISLE-generated code.
// For every term declared in the ISLE spec, we need to implement an extractor and a constructor.
//
// Extractors take an Id corresponding to an eclass and return all nodes matching the term pattern in that eclass.
// The follow this pattern:
// - Find all matching nodes in the given eclass (e.g., all Add nodes for extractor_add).
// - For each matching node, call run on the arguments to explore/optimize subexpressions.
// - Once we finish processing arguments, return all the matches we found.
//
// Constructors take the arguments for a term and construct a new node in the egraph, returning its Id.
// They follow this pattern:
// - Construct a new node with the given arguments (e.g., an Add node for constructor_add).
// - Add the node to the egraph and get its Id.
// - If the term did not already exist, i.e. if we created a new e-class, call run on the new Id to explore/optimize it.
//
// The run method calls the explore and optimize entrypoints and handles merging and rebuilding the egraph as needed.
#[allow(unused_variables)]
impl Context for OptimizerContext {
    fn extractor_combine_columns(&mut self, arg0: usize) -> Option<(usize, usize)> {
        warn!(
            "extractor_combine_columns doesn't make sense, we shouldn't call it in the first place"
        );
        None
    }

    fn constructor_combine_columns(&mut self, arg0: usize, arg1: usize) -> Option<usize> {
        let colset1 = self
            .colsets
            .get_by_right(&arg0)
            .expect("Invalid ColSetId for constructor_combine_columns");

        let colset2 = self
            .colsets
            .get_by_right(&arg1)
            .expect("Invalid ColSetId for constructor_combine_columns");
        let combined_colset = ColSet::combine(colset1, colset2)?;
        // Check if we already have this combined colset in our BiMap, if not add it with a new ColSetId
        if let Some(id) = self.colsets.get_by_left(&combined_colset) {
            Some(*id)
        } else {
            let new_id = self.next_colset_id;
            self.colsets.insert(combined_colset.clone(), new_id);
            self.next_colset_id += 1;
            Some(new_id)
        }
    }

    fn extractor_access_table(&mut self, arg0: Id) -> Option<usize> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Table(table_id) = node {
            // QUESTION: should we verify that the table_id is valid (i.e. exists in the catalog) before returning it
            Some(*table_id)
        } else {
            None
        }
    }

    fn constructor_access_table(&mut self, arg0: usize) -> Option<Id> {
        // Verify that table exists in the catalog before constructing the node
        if !self.catalog.tables.contains_key(&arg0) {
            warn!(
                "constructor_access_table called with invalid table_id: {}",
                arg0
            );
            return None;
        }
        let node = Optlang::Table(arg0);
        let (id, _) = self.egraph.add_with_flag(node);
        Some(id)
    }

    fn extractor_access_index(&mut self, arg0: Id) -> Option<usize> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Index(index_id) = node {
            Some(*index_id)
        } else {
            None
        }
    }

    fn constructor_access_index(&mut self, arg0: usize) -> Option<Id> {
        // Verify that index exists in the catalog before constructing the node
        if !self.catalog.indexes.contains_key(&arg0) {
            warn!(
                "constructor_access_index called with invalid index_id: {}",
                arg0
            );
            return None;
        }
        let node = Optlang::Index(arg0);
        let (id, _) = self.egraph.add_with_flag(node);
        Some(id)
    }

    fn extractor_ref_colset(&mut self, arg0: Id) -> Option<usize> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::ColSet(colset_id) = node {
            Some(*colset_id)
        } else {
            None
        }
    }

    fn constructor_ref_colset(&mut self, arg0: usize) -> Option<Id> {
        // Verify that colset exists in the catalog before constructing the node
        if !self.colsets.contains_right(&arg0) {
            warn!(
                "constructor_ref_colset called with invalid colset_id: {}",
                arg0
            );
            return None;
        }
        let node = Optlang::ColSet(arg0);
        let (id, _) = self.egraph.add_with_flag(node);
        Some(id)
    }

    fn extractor_const_val(&mut self, arg0: Id) -> Option<value> {
        let node = self.egraph.get_node(arg0);
        match node {
            Optlang::Int(i) => Some(value::Int { val: *i }),
            Optlang::Bool(b) => Some(value::Bool { val: *b }),
            Optlang::Str(s) => Some(value::Str { val: s.clone() }),
            _ => None,
        }
    }

    fn constructor_const_val(&mut self, arg0: &value) -> Id {
        let node = match arg0 {
            value::Int { val } => Optlang::Int(*val),
            value::Bool { val } => Optlang::Bool(*val),
            value::Str { val } => Optlang::Str(val.clone()),
        };
        let (id, _) = self.egraph.add_with_flag(node);
        id
    }

    fn extractor_add(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Add([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_add(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Add([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_sub(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Sub([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_sub(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Sub([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_mul(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Mul([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_mul(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Mul([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_div(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Div([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_div(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Div([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_eq(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Eq([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_eq(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Eq([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_lt(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Lt([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_lt(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Lt([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_gt(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Gt([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_gt(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Gt([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_le(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Le([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_le(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Le([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_ge(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Ge([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_ge(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Ge([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_ne(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Ne([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_ne(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Ne([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_and(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::And([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_and(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::And([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_or(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Or([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_or(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Or([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_not(&mut self, arg0: Id) -> Option<Id> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Not(id1) = node {
            Some(*id1)
        } else {
            None
        }
    }

    fn constructor_not(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Not(arg0));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_select(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Select([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_select(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Select([arg0, arg1]));

        // If we created a new e-class, push tasks to explore AND optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_project(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Project([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_project(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Project([arg0, arg1]));

        // If we created a new e-class, push tasks to explore AND optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_join(&mut self, arg0: Id) -> Option<(Id, Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Join([id1, id2, id3]) = node {
            Some((*id1, *id2, *id3))
        } else {
            None
        }
    }

    fn constructor_join(&mut self, arg0: Id, arg1: Id, arg2: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Join([arg0, arg1, arg2]));

        // If we created a new e-class, push tasks to explore AND optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_scan(&mut self, arg0: Id) -> Option<Id> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Scan(id) = node {
            Some(*id)
        } else {
            None
        }
    }

    fn constructor_scan(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Scan(arg0));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_table_scan(&mut self, arg0: Id) -> Option<Id> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::TableScan(id) = node {
            Some(*id)
        } else {
            None
        }
    }

    fn constructor_table_scan(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::TableScan(arg0));
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_index_scan(&mut self, arg0: Id) -> Option<Id> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::IndexScan(id) = node {
            Some(*id)
        } else {
            None
        }
    }

    fn constructor_index_scan(&mut self, arg0: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::IndexScan(arg0));
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_nested_loop_join(&mut self, arg0: Id) -> Option<(Id, Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::NestedLoopJoin([id1, id2, id3]) = node {
            Some((*id1, *id2, *id3))
        } else {
            None
        }
    }

    fn constructor_nested_loop_join(&mut self, arg0: Id, arg1: Id, arg2: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::NestedLoopJoin([arg0, arg1, arg2]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_hash_join(&mut self, arg0: Id) -> Option<(Id, Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::HashJoin([id1, id2, id3]) = node {
            Some((*id1, *id2, *id3))
        } else {
            None
        }
    }

    fn constructor_hash_join(&mut self, arg0: Id, arg1: Id, arg2: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::HashJoin([arg0, arg1, arg2]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_merge_join(&mut self, arg0: Id) -> Option<(Id, Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::MergeJoin([id1, id2, id3]) = node {
            Some((*id1, *id2, *id3))
        } else {
            None
        }
    }

    fn constructor_merge_join(&mut self, arg0: Id, arg1: Id, arg2: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self
            .egraph
            .add_with_flag(Optlang::MergeJoin([arg0, arg1, arg2]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    fn extractor_sort(&mut self, arg0: Id) -> Option<(Id, Id)> {
        let node = self.egraph.get_node(arg0);
        if let Optlang::Sort([id1, id2]) = node {
            Some((*id1, *id2))
        } else {
            None
        }
    }

    fn constructor_sort(&mut self, arg0: Id, arg1: Id) -> Id {
        // Construct the new node and add it to the egraph
        let (id, is_new) = self.egraph.add_with_flag(Optlang::Sort([arg0, arg1]));

        // If we created a new e-class, push a task to explore/optimize it
        if is_new {
            self.task_stack.push(Task::ExploreExpr(id, false));
        }
        id
    }

    type extractor_explore_returns = ContextIterWrapper<ConstructorVec<Id>, Self>;
    fn extractor_explore(&mut self, arg0: Id, returns: &mut Self::extractor_explore_returns) -> () {
        warn!("we should never call extractor_explore");
    }
}
