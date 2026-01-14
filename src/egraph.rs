use crate::types::*;
use crate::unionfind::UnionFind;
use indexmap::IndexMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;

/// ENode.
/// [ ]: Add docstring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ENode<L>
where
    L: OpLang,
{
    /// Unique identifier for the ENode.
    pub id: Id,
    /// The term associated with this ENode.
    pub term: Term<L>,
    /// Exploration status for this ENode. (Cascades)
    pub explored: bool,
}

impl<L> ENode<L>
where
    L: OpLang,
{
    /// Create a new ENode with the given id and term.
    pub fn new(id: Id, term: Term<L>) -> Self {
        ENode {
            id,
            term,
            explored: false,
        }
    }

    /// Returns a reference to the argument Ids of the ENode's term.
    pub fn arg_ids(&self) -> &Vec<Id> {
        self.term.args()
    }
}

impl<L> Display for ENode<L>
where
    L: OpLang,
    Term<L>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ENode(id: {}, term: {})", self.id, self.term)
    }
}

/// EClass
/// [ ]: Add docstring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EClass {
    /// Unique identifier for the EClass.
    pub id: Id,
    /// ENodes contained in this EClass.
    enodes: Vec<Id>,
    /// ENodes that are roots of this EClass.
    parents: Vec<Id>,
}

impl EClass {
    pub fn new(id: Id) -> Self {
        EClass {
            id,
            enodes: Vec::new(),
            parents: Vec::new(),
        }
    }

    pub fn add_enode(&mut self, enode_id: Id) {
        self.enodes.push(enode_id);
    }

    pub fn get_enodes(&self) -> &Vec<Id> {
        &self.enodes
    }

    pub fn add_parent(&mut self, parent_id: Id) {
        self.parents.push(parent_id);
    }

    pub fn get_parents(&self) -> &Vec<Id> {
        &self.parents
    }

    pub fn enode_count(&self) -> usize {
        self.enodes.len()
    }

    /// Merges another EClass into this one.
    pub fn merge_in_place(&mut self, other: &EClass) {
        self.enodes.extend(other.enodes.clone());
        self.parents.extend(other.parents.clone());
    }

    /// Merge two EClasses into a new one.
    pub fn merge(id: Id, eclass1: &EClass, eclass2: &EClass) -> EClass {
        let enodes = vec![eclass1.enodes.clone(), eclass2.enodes.clone()].concat();
        let parents = vec![eclass1.get_parents().clone(), eclass2.get_parents().clone()].concat();
        EClass {
            id,
            enodes,
            parents,
        }
    }
}

// TODO: Create a statistics struct to track various e-graph stats
// will be used for debugging and performance analysis
struct EGraphStats {
    eclass_count: usize,
    enode_count: usize,
    max_eclass_size: usize,
    total_repairs_queued: usize,
    total_repairs_performed: usize,
}

impl EGraphStats {
    pub fn new() -> Self {
        EGraphStats {
            eclass_count: 0,
            enode_count: 0,
            max_eclass_size: 0,
            total_repairs_queued: 0,
            total_repairs_performed: 0,
        }
    }
}

/// EGraph
/// [ ]: Add docstring.
pub struct EGraph<L>
where
    L: OpLang,
{
    /// UnionFind managing equivalence classes.
    uf: UnionFind,
    /// Hashcons mapping terms to their unique identifiers.
    hc: IndexMap<Term<L>, Id>,
    /// Map of EClasses indexed by their unique identifiers.
    eclasses: IndexMap<Id, EClass>,
    /// Map of ENodes indexed by their unique identifiers.
    enodes: IndexMap<Id, ENode<L>>,
    /// List of EClasses we need to repair.
    repairs: Vec<Id>,
    /// Stats about the EGraph.
    stats: EGraphStats,
    /// Whether the EGraph is stable (no pending repairs) and can be reliably queried.
    stable: bool, // TODO: Use this flag to prevent querying when unstable
    /// Flag to track if the e-graph has been modified since last rebuild.
    pub modified: bool,
}

impl<L> EGraph<L>
where
    L: OpLang,
{
    pub fn new() -> Self {
        EGraph {
            uf: UnionFind::new(),
            hc: IndexMap::new(),
            eclasses: IndexMap::new(),
            enodes: IndexMap::new(),
            repairs: Vec::new(),
            stats: EGraphStats::new(),
            stable: true,
            modified: false,
        }
    }

    /// Get list of unique identifiers for all EClasses in the EGraph.
    pub fn eclass_ids(&self) -> Vec<Id> {
        self.uf.roots()
    }

    /// Get EClass by its unique identifier.
    pub fn get_eclass(&self, id: &Id) -> Option<&EClass> {
        match self.eclasses.get(id) {
            Some(eclass) => {
                // Ensure the Id matches the representative of the e-class
                assert_eq!(eclass.id, *id, "EClass id does not match representative");
                Some(eclass)
            }
            None => None,
        }
    }

    /// Get mutable EClass by its unique identifier.
    pub fn get_eclass_mut(&mut self, id: &Id) -> Option<&mut EClass> {
        // Get a mutable reference to the e-class by its Id
        match self.eclasses.get_mut(id) {
            Some(eclass) => {
                // Ensure the Id matches the representative of the e-class
                assert_eq!(eclass.id, *id, "EClass id does not match representative");
                Some(eclass)
            }
            None => None,
        }
    }

    /// Get nodes in an EClass.
    /// Pass the canonical (representative) e-class Id.
    pub fn get_enodes_in_eclass(&self, id: &Id) -> Vec<&ENode<L>> {
        self.get_eclass(id)
            .unwrap()
            .get_enodes()
            .iter()
            .map(|enode_id| self.get_enode(enode_id).unwrap())
            .collect()
    }

    /// Get Ids of ENodes in an EClass.
    pub fn get_enode_ids_in_eclass(&self, id: &Id) -> Vec<Id> {
        self.get_eclass(id).unwrap().get_enodes().clone()
    }

    /// Get ENode by its unique identifier.
    pub fn get_enode(&self, id: &Id) -> Option<&ENode<L>> {
        self.enodes.get(id)
    }

    /// Find the canonical representative of the e-class containing `id`.
    /// No path compression is performed. We use this when we don't want to mutate the UnionFind for debugging.
    pub fn find(&self, id: Id) -> Id {
        self.uf.find(id)
    }

    /// Find the canonical representative of the e-class containing `id` with path compression.
    pub fn find_compress(&mut self, id: Id) -> Id {
        self.uf.find_compress(id)
    }

    /// Union two logical EClasses.
    fn union(&mut self, id1: Id, id2: Id) -> Id {
        self.uf.union(id1, id2)
    }

    /// Add a new set to the UnionFind and return its Id.
    fn add_set(&mut self) -> Id {
        self.uf.add_set()
    }

    /// Canonicalize a term by finding the canonical representative of its argument EClasses.
    /// Recursively finds the representative of the EClass for each term in the argument.
    fn canonicalize(&self, term: &Term<L>) -> Term<L> {
        let op = term.op().clone();
        let args = term.args().iter().map(|arg| self.find(*arg)).collect();
        Term::new(op, args)
    }

    /// Add an Expr to the EGraph as an ENode.
    /// Recursively converts the Expr to a Term, canonicalizes it, and adds it to the EGraph.
    pub fn add_expr(&mut self, expr: &Expr<L>) -> Id {
        // Recursively convert expression to a term
        let arg_ids: Vec<Id> = expr.args().iter().map(|arg| self.add_expr(arg)).collect();

        let term = Term::new(expr.op().clone(), arg_ids);

        // Canonicalize the term
        let canonical_term = self.canonicalize(&term);

        // If the term already exists, return its id
        // Otherwise, create a new e-node and singleton e-class
        if let Some(&id) = self.hc.get(&canonical_term) {
            id
        } else {
            // Create a new e-node/e-class ID
            let new_id = self.add_set();
            // Insert new term into hashcons mapping to Id
            self.hc.insert(canonical_term.clone(), new_id);
            // Create and insert new ENode
            let enode = ENode::new(new_id, canonical_term.clone());
            self.enodes.insert(new_id, enode);
            // Create and insert new EClass, add ENode to EClass
            let mut new_eclass = EClass::new(new_id);
            new_eclass.add_enode(new_id);
            self.eclasses.insert(new_id, new_eclass);
            // Add this new e-node as a parent to its children
            for child in canonical_term.args() {
                self.get_eclass_mut(&child).unwrap().add_parent(new_id);
            }
            // Update stats
            self.stats.enode_count += 1;
            self.stats.eclass_count += 1;

            new_id
        }
    }

    /// Recursively reconstruct the canonical expression for an EClass given its Id.
    fn get_canonical_expr(&self, id: &Id) -> Expr<L> {
        if let Some(enode) = self.get_enode(&self.find(*id)) {
            return Expr::new(
                enode.term.op().clone(),
                enode
                    .term
                    .args()
                    .iter()
                    .map(|arg| self.get_canonical_expr(&arg))
                    .collect(),
            );
        }
        panic!("EClass or ENode not found for Id {}", id);
    }

    /// Convert a Pattern with a Substitution into its canonical Expr in the EGraph.
    fn to_canonical_expr(&self, pattern: &Pattern<L>, subst: &Subst<Var, Id>) -> Expr<L> {
        match pattern.op() {
            OpOrVar::Var(s) => {
                // If the expression is a variable, we need to substitute it with the corresponding Id
                if let Some(&id) = subst.get(s) {
                    return self.get_canonical_expr(&id);
                } else {
                    panic!("Variable {} not found in substitution map", s);
                }
            }
            OpOrVar::Op(op) => {
                let args: Vec<Expr<L>> = pattern
                    .args()
                    .iter()
                    .map(|arg| self.to_canonical_expr(arg, subst))
                    .collect();
                Expr::new(op.clone(), args)
            }
        }
    }

    /// Insert an ENode from an Expr and Substitution into the EGraph.
    pub fn add_match(&mut self, pattern: &Pattern<L>, subst: &Subst<Var, Id>) -> Id {
        match pattern.op() {
            OpOrVar::Var(s) => {
                // If the expression is a variable, we need to substitute it with the corresponding Id
                if let Some(&id) = subst.get(s) {
                    return id;
                } else {
                    panic!("Variable {} not found in substitution map", s);
                }
            }
            OpOrVar::Op(op) => {
                let arg_ids: Vec<Id> = pattern
                    .args()
                    .iter()
                    .map(|arg| self.add_match(arg, subst))
                    .collect();

                let term = Term::new(op.clone(), arg_ids);

                // Canonicalize the term
                let canonical_term = self.canonicalize(&term);

                // If the term already exists, return its id
                // Otherwise, create a new e-node and singleton e-class
                if let Some(&id) = self.hc.get(&canonical_term) {
                    id
                } else {
                    // Set modified flag
                    self.modified = true;
                    // Create a new e-node/e-class ID
                    let new_id = self.add_set();
                    // Insert new term into hashcons mapping to Id
                    self.hc.insert(canonical_term.clone(), new_id);
                    // Create and insert new ENode
                    let enode = ENode::new(new_id, canonical_term.clone());
                    self.enodes.insert(new_id, enode);
                    // Create and insert new EClass, add ENode to EClass
                    let mut new_eclass = EClass::new(new_id);
                    new_eclass.add_enode(new_id);
                    self.eclasses.insert(new_id, new_eclass);
                    // Add this new e-node as a parent to its children
                    for child in canonical_term.args() {
                        self.get_eclass_mut(&child).unwrap().add_parent(new_id);
                    }
                    // Update stats
                    self.stats.enode_count += 1;
                    self.stats.eclass_count += 1;
                    new_id
                }
            }
        }
    }

    // TODO: Add e-matching within a budget

    /// Match an expression against an EClass.
    /// Returns a vector of substitutions that match the expression against the EClass.
    pub fn ematch(
        &self,
        pattern: &Pattern<L>,
        eclass: Id,
        subst: &Subst<Var, Id>,
    ) -> Vec<Subst<Var, Id>> {
        // Canonicalize: callers may pass non-representative Ids after merges.
        let eclass = self.find(eclass);

        fn insert_subst(var: &Var, eclass: Id, subst: &Subst<Var, Id>) -> Option<Subst<Var, Id>> {
            let mut subst_clone = subst.clone();
            if let Some(id) = subst_clone.insert(var.clone(), eclass) {
                // If the variable was already in the substitution map, check if it matches the eclass
                if id != eclass {
                    return None; // No match found
                }
            }
            Some(subst_clone) // Return the substitution map with the variable added
        }

        let mut res = vec![];
        match pattern.op() {
            OpOrVar::Var(s) => {
                // If the expression is a variable, try to insert it into the substitution map
                // If the variable is already in the substitution map, make sure the eclass matches
                if let Some(subst_clone) = insert_subst(s, eclass, subst) {
                    res.push(subst_clone); // Return the substitution map with the constant added
                }
                return res;
            }
            OpOrVar::Op(_expr) => {
                // If expression is a constant, try to find an ENode in the class that matches
                if pattern.is_terminal() {
                    for node in self.get_enodes_in_eclass(&eclass) {
                        if node.term.matches_pattern(pattern) {
                            let mut subst_clone = subst.clone();
                            subst_clone.insert(String::from(""), eclass); // HACK
                            res.push(subst_clone);
                            return res;
                        }
                    }
                    return res;
                }

                // For every node in the eclass we construct a substitution (if one exists)
                // and add those substitutions to our list of results
                for node in self.get_enodes_in_eclass(&eclass) {
                    if node.term.matches_pattern(pattern) {
                        // Create list for possible substitution sets
                        let mut subst_list = vec![subst.clone()];
                        // For each argument, ematch the argument expression against the nodes argument eclass
                        // and extend the substitution list with the results
                        for (i, arg) in pattern.args().iter().enumerate() {
                            let mut nested = vec![];
                            for subst_in in subst_list.iter() {
                                let nested_sub = self.ematch(arg, node.term.args()[i], &subst_in);
                                nested.extend(nested_sub);
                            }
                            subst_list = nested;
                        }
                        // Extend the results with the substitutions found
                        res.extend(subst_list);
                    }
                }
            }
        }

        res
    }

    /// Match an expression against a single ENode.
    /// This is used by Cascades.
    pub fn ematch_enode(
        &self,
        pattern: &Pattern<L>,
        enode_id: Id,
        subst: &Subst<Var, Id>,
    ) -> Vec<Subst<Var, Id>> {
        let mut res = vec![];
        if let Some(node) = self.get_enode(&enode_id) {
            if node.term.matches_pattern(pattern) {
                // Create list for possible substitution sets
                let mut subst_list = vec![subst.clone()];
                // For each argument, ematch the argument expression against the nodes argument eclass
                // and extend the substitution list with the results
                for (i, arg) in pattern.args().iter().enumerate() {
                    let mut nested = vec![];
                    for subst_in in subst_list.iter() {
                        let nested_sub = self.ematch(arg, node.term.args()[i], &subst_in);
                        nested.extend(nested_sub);
                    }
                    subst_list = nested;
                }
                // Extend the results with the substitutions found
                res.extend(subst_list);
            }
        }
        res
    }

    /// Merge two EClasses in the EGraph.
    pub fn merge(&mut self, id1: Id, id2: Id) -> Id {
        // Find the canonical representatives of the e-classes containing `id1` and `id2`
        let par1 = self.find_compress(id1);
        let par2 = self.find_compress(id2);

        // If they are already in the same class, do nothing
        if par1 == par2 {
            return par1;
        }

        // Set modified flag
        self.modified = true;

        // Union the two classes
        let new_id = self.union(par1, par2);

        // Merge two eclasses into a new eclass
        let eclass1 = self.get_eclass(&par1).unwrap().clone();
        let eclass2 = self.get_eclass(&par2).unwrap().clone();
        let new_eclass = EClass::merge(new_id, &eclass1, &eclass2);

        // Update stats to reflect one less unique eclass
        self.stats.eclass_count -= 1;
        if new_eclass.enode_count() > self.stats.max_eclass_size {
            self.stats.max_eclass_size = new_eclass.enode_count();
        }

        // Remove old e-classes from the map and insert the new one
        self.eclasses.shift_remove(&par1);
        self.eclasses.shift_remove(&par2);
        self.eclasses.insert(new_id, new_eclass);

        // Add merged e-class to repair list
        self.repairs.push(new_id);
        self.stats.total_repairs_queued += 1;

        new_id
    }

    /// Rebuild the e-graph.
    /// This function should be called after a series of merges to restore the invariants of the e-graph.
    pub fn rebuild(&mut self) {
        while self.repairs.len() > 0 {
            // Copy current repair list to a temporary list of canonical Ids and remove duplicates
            let mut todo_list = self
                .repairs
                .iter()
                .map(|id| (self.find(*id)))
                .collect::<Vec<Id>>();
            todo_list.sort();
            todo_list.dedup();

            // Since we call repair (which calls merge) we reset the repairs before "refilling" it
            self.repairs.clear();

            // Iterate over the canonical Ids and repair each e-class
            for id in todo_list {
                self.repair(id);
            }
        }

        // Reset modified flag
        self.modified = false;
    }

    /// Repair an EClass.
    pub fn repair(&mut self, id: Id)
    where
        Term<L>: Clone + Eq + Hash + Debug,
    {
        // FIXME: Make this faster, its gross slow
        self.stats.total_repairs_performed += 1;
        let eclass = self.get_eclass(&id).unwrap().clone();

        let mut new_parents: IndexMap<Term<L>, Id> = IndexMap::new();
        for p in eclass.parents.iter() {
            // Get the parent ENode and its eclass id
            let p_node = self.get_enode(p).expect("Parent ENode not found");
            let p_eclass_id = self.find(*p);
            // Canonicalize the term of the parent ENode
            let p_node_canonical = self.canonicalize(&p_node.term.clone());
            // TODO: Do we actually want to remove info from the hashcons?
            // self.hc.shift_remove(&p_node.term);
            // Insert the canonicalized term pointing to the canonical eclass id
            self.hc.insert(p_node_canonical.clone(), p_eclass_id);

            // Check if the canonicalized term is equivalent to another parent
            if let Some(&p_id) = new_parents.get(&p_node_canonical) {
                // If the canonicalized term already exists, we merge those e-classes
                // the merge here will add the parent to the worklist
                let merged_id = self.merge(p_eclass_id, p_id);
                new_parents.insert(p_node_canonical, merged_id);
                self.repairs.push(merged_id);
            } else {
                // Otherwise, we insert the canonicalized term into the new parents map
                new_parents.insert(p_node_canonical, p_eclass_id);
            }
        }
    }

    /// Extract an expression from the EGraph.
    /// Given an Id, find an expression that corresponds to the EClass of that Id.
    pub fn extract(&self, _id: Id, _cost_func: &dyn CostFunction<L, u64>) -> Expr<L> {
        todo!("Implement extraction of expression from EGraph");
    }
}

#[cfg(test)]
mod tests {
    use crate::impl_oplang_default;

    use super::*;

    // Test operator for testing
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum TestOp {
        Add,
        Mul,
        Const(i32),
        Var(String),
    }

    impl std::fmt::Display for TestOp {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TestOp::Add => write!(f, "+"),
                TestOp::Mul => write!(f, "*"),
                TestOp::Const(n) => write!(f, "{}", n),
                TestOp::Var(s) => write!(f, "{}", s),
            }
        }
    }

    impl OpLang for TestOp {
        impl_oplang_default!();

        fn arity(&self) -> usize {
            match self {
                TestOp::Add => 2,
                TestOp::Mul => 2,
                TestOp::Const(_) => 0,
                TestOp::Var(_) => 0,
            }
        }
    }

    type TestEGraph = EGraph<TestOp>;

    fn make_const_expr(n: i32) -> Expr<TestOp> {
        Expr::new(TestOp::Const(n), vec![])
    }

    fn make_var_expr(name: &str) -> Expr<TestOp> {
        Expr::new(TestOp::Var(name.to_string()), vec![])
    }

    fn make_add_expr(left: Expr<TestOp>, right: Expr<TestOp>) -> Expr<TestOp> {
        Expr::new(TestOp::Add, vec![left, right])
    }

    fn make_mul_expr(left: Expr<TestOp>, right: Expr<TestOp>) -> Expr<TestOp> {
        Expr::new(TestOp::Mul, vec![left, right])
    }

    // Helper functions for creating patterns with variables
    fn make_const_pattern(n: i32) -> Pattern<TestOp> {
        Pattern::new(OpOrVar::Op(TestOp::Const(n)), vec![])
    }

    fn make_var_pattern(name: &str) -> Pattern<TestOp> {
        Pattern::new(OpOrVar::Var(name.to_string()), vec![])
    }

    fn make_add_pattern(left: Pattern<TestOp>, right: Pattern<TestOp>) -> Pattern<TestOp> {
        Pattern::new(OpOrVar::Op(TestOp::Add), vec![left, right])
    }

    fn make_mul_pattern(left: Pattern<TestOp>, right: Pattern<TestOp>) -> Pattern<TestOp> {
        Pattern::new(OpOrVar::Op(TestOp::Mul), vec![left, right])
    }

    #[test]
    fn test_egraph_new() {
        let egraph: TestEGraph = EGraph::new();
        assert_eq!(egraph.eclass_ids().len(), 0);
    }

    #[test]
    fn test_add_expr_constants() {
        let mut egraph: TestEGraph = EGraph::new();

        let expr1 = make_const_expr(42);
        let expr2 = make_const_expr(42);
        let expr3 = make_const_expr(24);

        let id1 = egraph.add_expr(&expr1);
        let id2 = egraph.add_expr(&expr2);
        let id3 = egraph.add_expr(&expr3);

        // Same expressions should get same IDs
        assert_eq!(id1, id2);
        // Different expressions should get different IDs
        assert_ne!(id1, id3);

        // Should have 2 eclasses
        assert_eq!(egraph.eclass_ids().len(), 2);
    }

    #[test]
    fn test_add_expr_complex() {
        let mut egraph: TestEGraph = EGraph::new();

        let a = make_const_expr(1);
        let b = make_const_expr(2);
        let expr1 = make_add_expr(a.clone(), b.clone());
        let expr2 = make_add_expr(a.clone(), b.clone());

        let id1 = egraph.add_expr(&expr1);
        let id2 = egraph.add_expr(&expr2);

        // Same complex expressions should get same IDs
        assert_eq!(id1, id2);

        // Should have 3 eclasses: const(1), const(2), and add(const(1), const(2))
        assert_eq!(egraph.eclass_ids().len(), 3);
    }

    #[test]
    fn test_get_eclass() {
        let mut egraph: TestEGraph = EGraph::new();
        let expr = make_const_expr(42);
        let id = egraph.add_expr(&expr);

        let eclass = egraph.get_eclass(&id);
        assert!(eclass.is_some());
        assert_eq!(eclass.unwrap().id, id);

        // Non-existent ID should return None
        let non_existent_id: Id = 999;
        let non_existent_eclass = egraph.get_eclass(&non_existent_id);
        assert!(non_existent_eclass.is_none());
    }

    #[test]
    fn test_get_enode() {
        let mut egraph: TestEGraph = EGraph::new();
        let expr = make_const_expr(42);
        let id = egraph.add_expr(&expr);

        let enode = egraph.get_enode(&id);
        assert!(enode.is_some());
        assert_eq!(enode.unwrap().id, id);
    }

    #[test]
    fn test_get_enodes_in_eclass() {
        let mut egraph: TestEGraph = EGraph::new();
        let expr = make_const_expr(42);
        let id = egraph.add_expr(&expr);

        let nodes = egraph.get_enodes_in_eclass(&id);
        // Check that we get at least one node
        assert!(!nodes.is_empty());
        assert_eq!(nodes[0].id, id);
    }

    #[test]
    fn test_find() {
        let mut egraph: TestEGraph = EGraph::new();
        let expr = make_const_expr(42);
        let id = egraph.add_expr(&expr);

        // Find should return the same ID for a singleton class
        assert_eq!(egraph.find(id), id);
    }

    #[test]
    fn test_canonicalize() {
        let mut egraph: TestEGraph = EGraph::new();

        let const1 = make_const_expr(1);
        let const2 = make_const_expr(2);
        let id1 = egraph.add_expr(&const1);
        let id2 = egraph.add_expr(&const2);

        let term = Term::new(TestOp::Add, vec![id1, id2]);
        let canonical = egraph.canonicalize(&term);

        assert_eq!(canonical.op(), &TestOp::Add);
        assert_eq!(canonical.args(), &vec![id1, id2]);
    }

    #[test]
    fn test_merge_different_eclasses() {
        let mut egraph: TestEGraph = EGraph::new();

        let expr1 = make_const_expr(1);
        let expr2 = make_const_expr(2);
        let id1 = egraph.add_expr(&expr1);
        let id2 = egraph.add_expr(&expr2);

        // Initially should have 2 eclasses
        assert_eq!(egraph.eclass_ids().len(), 2);
        assert_ne!(egraph.find(id1), egraph.find(id2));

        // Merge the eclasses
        let merged_id = egraph.merge(id1, id2);

        // Should now have 1 eclass
        assert_eq!(egraph.eclass_ids().len(), 1);
        assert_eq!(egraph.find(id1), egraph.find(id2));
        assert_eq!(egraph.find(id1), merged_id);
    }

    #[test]
    fn test_merge_same_eclass() {
        let mut egraph: TestEGraph = EGraph::new();

        let expr = make_const_expr(42);
        let id = egraph.add_expr(&expr);

        let original_size = egraph.eclass_ids().len();
        let merged_id = egraph.merge(id, id);

        // Merging same eclass should not change anything
        assert_eq!(egraph.eclass_ids().len(), original_size);
        assert_eq!(merged_id, egraph.find(id));
    }

    #[test]
    fn test_ematch_variable() {
        let mut egraph: TestEGraph = EGraph::new();

        let const_expr = make_const_expr(42);
        let var_pattern = make_var_pattern("x");
        let const_id = egraph.add_expr(&const_expr);

        let subst = IndexMap::new();
        let matches = egraph.ematch(&var_pattern, const_id, &subst);

        // Variable should match any eclass
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].get("x"), Some(&const_id));
    }

    #[test]
    fn test_ematch_constant() {
        let mut egraph: TestEGraph = EGraph::new();

        let const_expr = make_const_expr(42);
        let const_pattern = make_const_pattern(42);
        let other_const_pattern = make_const_pattern(24);
        let const_id = egraph.add_expr(&const_expr);

        let subst = IndexMap::new();

        // Matching same constant should succeed
        let matches1 = egraph.ematch(&const_pattern, const_id, &subst);
        assert_eq!(matches1.len(), 1);

        // Matching different constant should fail
        let matches2 = egraph.ematch(&other_const_pattern, const_id, &subst);
        assert_eq!(matches2.len(), 0);
    }

    #[test]
    fn test_ematch_complex_expression() {
        let mut egraph: TestEGraph = EGraph::new();

        let const1 = make_const_expr(1);
        let const2 = make_const_expr(2);
        let const1_id = egraph.add_expr(&const1);
        let add_expr = make_add_expr(const1.clone(), const2.clone());
        let add_id = egraph.add_expr(&add_expr);

        let var_pattern = make_var_pattern("x");
        let const2_pattern = make_const_pattern(2);
        let pattern = Pattern::new(OpOrVar::Op(TestOp::Add), vec![var_pattern, const2_pattern]);

        let subst = IndexMap::new();
        let matches = egraph.ematch(&pattern, add_id, &subst);

        // Should match with x = const(1)
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].get("x"), Some(&const1_id));
    }

    #[test]
    fn test_ematch_accepts_non_canonical_eclass_id_after_merge() {
        let mut egraph: TestEGraph = EGraph::new();

        // Create two distinct (but structurally different) expressions.
        // We'll merge their e-classes and then query using the merged-away Id.
        let expr_a = make_add_expr(make_const_expr(1), make_const_expr(2));
        let expr_b = make_add_expr(make_const_expr(2), make_const_expr(1));

        let id_a = egraph.add_expr(&expr_a);
        let id_b = egraph.add_expr(&expr_b);
        assert_ne!(id_a, id_b);

        // Merge and rebuild so one Id is no longer a representative.
        egraph.merge(id_a, id_b);
        egraph.rebuild();

        // At this point, `id_b` may no longer be present in the eclass map.
        assert!(egraph.get_eclass(&id_b).is_none());

        // Regression assertion: ematch should canonicalize the input eclass Id and still succeed.
        let pat = make_add_pattern(make_const_pattern(1), make_const_pattern(2));
        let matches = egraph.ematch(&pat, id_b, &IndexMap::new());
        assert!(
            !matches.is_empty(),
            "expected ematch to succeed even when called with a non-canonical eclass id"
        );
    }

    #[test]
    fn test_add_enode_match_with_variable() {
        let mut egraph: TestEGraph = EGraph::new();

        let const_expr = make_const_expr(42);
        let const_id = egraph.add_expr(&const_expr);

        let var_pattern = make_var_pattern("x");
        let mut subst = IndexMap::new();
        subst.insert("x".to_string(), const_id);

        let result_id = egraph.add_match(&var_pattern, &subst);
        assert_eq!(result_id, const_id);
    }

    #[test]
    #[should_panic(expected = "Variable")]
    fn test_add_enode_match_missing_variable() {
        let mut egraph: TestEGraph = EGraph::new();

        let var_pattern = make_var_pattern("x");
        let subst = IndexMap::new(); // Empty substitution

        // Should panic because variable not in substitution
        egraph.add_match(&var_pattern, &subst);
    }

    #[test]
    fn test_rebuild_simple() {
        let mut egraph: TestEGraph = EGraph::new();

        let expr1 = make_const_expr(1);
        let expr2 = make_const_expr(2);
        let id1 = egraph.add_expr(&expr1);
        let id2 = egraph.add_expr(&expr2);

        // Merge two eclasses - this should add to repairs list
        egraph.merge(id1, id2);

        // Rebuild should process the repairs
        egraph.rebuild();

        // After rebuild, both should still be in same eclass
        assert_eq!(egraph.find(id1), egraph.find(id2));
    }

    #[test]
    fn test_rebuild_with_parents() {
        let mut egraph: TestEGraph = EGraph::new();

        let const1 = make_const_expr(1);
        let const2 = make_const_expr(2);
        let const3 = make_const_expr(3);

        let _id1 = egraph.add_expr(&const1);
        let id2 = egraph.add_expr(&const2);
        let id3 = egraph.add_expr(&const3);

        // Create expressions that use these constants
        let add1 = make_add_expr(const1.clone(), const2.clone());
        let add2 = make_add_expr(const1.clone(), const3.clone());

        let add_id1 = egraph.add_expr(&add1);
        let add_id2 = egraph.add_expr(&add2);

        // Merge const2 and const3
        egraph.merge(id2, id3);
        egraph.rebuild();

        // The add expressions should now be equivalent
        // because add(1, 2) and add(1, 3) where 2 ≡ 3
        assert_eq!(egraph.find(add_id1), egraph.find(add_id2));
    }

    #[test]
    fn test_multiple_merges_and_rebuild() {
        let mut egraph: TestEGraph = EGraph::new();

        // Create several constants
        let constants: Vec<_> = (0..5).map(|i| make_const_expr(i)).collect();
        let ids: Vec<_> = constants.iter().map(|expr| egraph.add_expr(expr)).collect();

        // Initially all should be separate
        for i in 0..ids.len() {
            for j in i + 1..ids.len() {
                assert_ne!(egraph.find(ids[i]), egraph.find(ids[j]));
            }
        }

        // Merge them all into one equivalence class
        for i in 1..ids.len() {
            egraph.merge(ids[0], ids[i]);
        }

        egraph.rebuild();

        // All should now be equivalent
        let root = egraph.find(ids[0]);
        for id in &ids[1..] {
            assert_eq!(egraph.find(*id), root);
        }
    }

    #[test]
    fn test_nested_expressions() {
        let mut egraph: TestEGraph = EGraph::new();

        let a = make_const_expr(1);
        let b = make_const_expr(2);
        let c = make_const_expr(3);

        // Create nested expression: add(mul(a, b), c)
        let mul_ab = make_mul_expr(a.clone(), b.clone());
        let add_expr = make_add_expr(mul_ab, c.clone());

        let result_id = egraph.add_expr(&add_expr);

        // Should have created multiple eclasses
        assert!(egraph.eclass_ids().len() >= 4); // a, b, c, mul(a,b), add(mul(a,b), c)

        // The final expression should exist
        assert!(egraph.get_eclass(&result_id).is_some());
    }

    #[test]
    fn test_expression_reuse() {
        let mut egraph: TestEGraph = EGraph::new();

        let a = make_const_expr(1);
        let b = make_const_expr(2);

        // Create the same subexpression multiple times
        let expr1 = make_add_expr(a.clone(), b.clone());
        let expr2 = make_add_expr(a.clone(), b.clone());

        let id1 = egraph.add_expr(&expr1);
        let id2 = egraph.add_expr(&expr2);

        // Should get the same ID for identical expressions
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_parent_child_relationships() {
        let mut egraph: TestEGraph = EGraph::new();

        let a = make_const_expr(1);
        let b = make_const_expr(2);
        let add_expr = make_add_expr(a.clone(), b.clone());

        let a_id = egraph.add_expr(&a);
        let b_id = egraph.add_expr(&b);
        let add_id = egraph.add_expr(&add_expr);

        // Check that parents are correctly set
        let a_eclass = egraph.get_eclass(&a_id).unwrap();
        let b_eclass = egraph.get_eclass(&b_id).unwrap();

        assert!(a_eclass.get_parents().contains(&add_id));
        assert!(b_eclass.get_parents().contains(&add_id));
    }

    #[test]
    fn test_find_compress() {
        let mut egraph: TestEGraph = EGraph::new();

        let expr1 = make_const_expr(1);
        let expr2 = make_const_expr(2);
        let expr3 = make_const_expr(3);

        let id1 = egraph.add_expr(&expr1);
        let id2 = egraph.add_expr(&expr2);
        let id3 = egraph.add_expr(&expr3);

        // Create a chain by merging
        egraph.merge(id1, id2);
        egraph.merge(id2, id3);

        // Use find_compress - should return the root and compress paths
        let root = egraph.find_compress(id3);
        assert_eq!(egraph.find(id1), root);
        assert_eq!(egraph.find(id2), root);
        assert_eq!(egraph.find(id3), root);
    }

    #[test]
    fn test_ematch_variable_already_bound() {
        let mut egraph: TestEGraph = EGraph::new();

        let const_expr1 = make_const_expr(42);
        let const_expr2 = make_const_expr(24);
        let var_pattern = make_var_pattern("x");

        let const_id1 = egraph.add_expr(&const_expr1);
        let const_id2 = egraph.add_expr(&const_expr2);

        // Create substitution with x already bound to const_id1
        let mut subst = IndexMap::new();
        subst.insert("x".to_string(), const_id1);

        // Matching variable x against const_id1 should succeed
        let matches1 = egraph.ematch(&var_pattern, const_id1, &subst);
        assert_eq!(matches1.len(), 1);

        // Matching variable x against const_id2 should fail (variable already bound to different value)
        let matches2 = egraph.ematch(&var_pattern, const_id2, &subst);
        assert_eq!(matches2.len(), 0);
    }

    #[test]
    fn test_eclass_merge_with_multiple_nodes() {
        let mut egraph: TestEGraph = EGraph::new();

        let const1 = make_const_expr(1);
        let const2 = make_const_expr(2);
        let const3 = make_const_expr(3);

        let _id1 = egraph.add_expr(&const1);
        let id2 = egraph.add_expr(&const2);
        let id3 = egraph.add_expr(&const3);

        // Create expressions using these constants
        let add12 = make_add_expr(const1.clone(), const2.clone());
        let add13 = make_add_expr(const1.clone(), const3.clone());

        let add_id1 = egraph.add_expr(&add12);
        let add_id2 = egraph.add_expr(&add13);

        // Initially should be different
        assert_ne!(egraph.find(add_id1), egraph.find(add_id2));

        // Merge const2 and const3
        egraph.merge(id2, id3);
        egraph.rebuild();

        // Now the add expressions should be in the same eclass
        assert_eq!(egraph.find(add_id1), egraph.find(add_id2));
    }

    #[test]
    fn test_complex_pattern_matching() {
        let mut egraph: TestEGraph = EGraph::new();

        // Create expression: add(mul(x, 2), 3)
        let two = make_const_expr(2);
        let three = make_const_expr(3);
        let x_var = make_var_expr("x");
        let y_var = make_var_pattern("y");

        let mul_expr = make_mul_expr(x_var, two.clone());
        let complex_expr = make_add_expr(mul_expr, three.clone());

        let complex_id = egraph.add_expr(&complex_expr);

        // Create pattern: add(y, 3)
        let three_pattern = make_const_pattern(3);
        let pattern = Pattern::new(OpOrVar::Op(TestOp::Add), vec![y_var, three_pattern]);

        let subst = IndexMap::new();
        let matches = egraph.ematch(&pattern, complex_id, &subst);

        // Should match with y = mul(x, 2)
        assert_eq!(matches.len(), 1);
        let expected_mul_id =
            egraph.add_expr(&make_mul_expr(make_var_expr("x"), make_const_expr(2)));
        assert_eq!(matches[0].get("y"), Some(&expected_mul_id));
    }
}
