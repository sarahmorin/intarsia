use crate::types::{Analysis, Expr, Pattern, Id, Subst, Term, AST, Var, OpOrVar};
use crate::unionfind::UnionFind;
use indexmap::IndexMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;

/// ENode
/// TODO: Add docstring
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ENode<T>
where
    T: AST,
{
    /// Unique identifier for the ENode
    pub id: Id,
    /// The term associated with this ENode
    pub term: Term<T>,
}

impl<T> ENode<T>
where
    T: AST,
{
    pub fn new(id: Id, term: Term<T>) -> Self {
        ENode { id, term }
    }
}

impl<T> Display for ENode<T>
where
    T: AST,
    Term<T>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ENode(id: {}, term: {})", self.id, self.term)
    }
}

/// EClass
/// TODO: docstring
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EClass<A>
where
    A: Analysis,
{
    /// Unique identifier for the EClass
    pub id: Id,
    /// ENodes contained in this EClass
    enodes: Vec<Id>,
    /// ENodes that are roots of this EClass
    parents: Vec<Id>,
    /// Analysis data associated with this EClass
    analysis: A,
}

impl<A> EClass<A>
where
    A: Analysis,
{
    pub fn new(id: Id, analysis: A) -> Self {
        EClass {
            id,
            enodes: Vec::new(),
            parents: Vec::new(),
            analysis,
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

    pub fn get_analysis(&self) -> &A {
        &self.analysis
    }

    /// Merges another EClass into this one.
    pub fn merge_in_place(&mut self, other: &EClass<A>) {
        self.enodes.extend(other.enodes.clone());
        self.parents.extend(other.parents.clone());
        self.analysis.merge(other.analysis.clone());
    }

    /// Merge two EClasses into a new one
    pub fn merge(id: Id, eclass1: &EClass<A>, eclass2: &EClass<A>) -> EClass<A> {
        let enodes = vec![eclass1.enodes.clone(), eclass2.enodes.clone()].concat();
        let parents = vec![eclass1.get_parents().clone(), eclass2.get_parents().clone()].concat();
        let mut analysis = eclass1.analysis.clone();
        analysis.merge(eclass2.analysis.clone());
        EClass {
            id,
            enodes,
            parents,
            analysis,
        }
    }
}

/// EGraph
/// TODO: documentation
pub struct EGraph<T, A>
where
    T: AST,
    A: Analysis,
{
    /// UnionFind managing equivalence classes
    uf: UnionFind,
    /// Hashcons mapping terms to their unique identifiers
    hc: IndexMap<Term<T>, Id>,
    /// Map of EClasses indexed by their unique identifiers
    eclasses: IndexMap<Id, EClass<A>>,
    /// Map of ENodes indexed by their unique identifiers
    enodes: IndexMap<Id, ENode<T>>,
    /// List of EClasses we need to repair
    repairs: Vec<Id>,
    // TODO: Do we need more info in the egraph struct?
}

impl<T, A> EGraph<T, A>
where
    T: AST,
    A: Analysis,
    Term<T>: Clone + Eq + Hash + Debug,
{
    pub fn new() -> Self {
        EGraph {
            uf: UnionFind::new(),
            hc: IndexMap::new(),
            eclasses: IndexMap::new(),
            enodes: IndexMap::new(),
            repairs: Vec::new(),
        }
    }

    /// Get list of unique identifiers for all EClasses in the EGraph
    pub fn eclass_ids(&self) -> Vec<Id> {
        self.uf.roots()
    }

    /// Get EClass by its unique identifier
    pub fn get_eclass(&self, id: &Id) -> Option<&EClass<A>> {
        match self.eclasses.get(id) {
            Some(eclass) => {
                // Ensure the Id matches the representative of the e-class
                assert_eq!(eclass.id, *id, "EClass id does not match representative");
                Some(eclass)
            }
            None => None,
        }
    }

    /// Get mutable EClass by its unique identifier
    pub fn get_eclass_mut(&mut self, id: &Id) -> Option<&mut EClass<A>> {
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

    /// Get ENode by its unique identifier
    pub fn get_enode(&self, id: &Id) -> Option<&ENode<T>> {
        self.enodes.get(id)
    }

    /// Get nodes in EClass
    pub fn get_nodes_in_eclass(&self, id: &Id) -> Vec<&ENode<T>> {
        if let Some(eclass) = self.get_eclass(id) {
            return eclass
                .get_enodes()
                .into_iter()
                .map(|enode_id| {
                    // TODO: Do we want to panic here? Larger question -- how should we do error handling?
                    self.get_enode(enode_id).expect("ENode not found")
                })
                .collect::<Vec<&ENode<T>>>();
        }
        Vec::new()
    }

    /// Find the canonical representative of the e-class containing `id`
    pub fn find(&self, id: Id) -> Id {
        self.uf.find(id)
    }

    /// Find the canonical representative of the e-class containing `id` with path compression
    pub fn find_compress(&mut self, id: Id) -> Id {
        self.uf.find_compress(id)
    }

    /// Canonicalize a term by finding the canonical representative of its argument EClasses
    /// Recursively finds the representative of the EClass for each term in the argument
    pub fn canonicalize(&self, term: &Term<T>) -> Term<T>
    where
        Term<T>: Clone + Eq + std::hash::Hash + Debug,
    {
        let op = term.op().clone();
        let args = term.args().iter().map(|&arg| self.find(arg)).collect();
        Term::new(op, args)
    }

    /// Add an Expr to the EGraph as an ENode
    /// Recursively converts the Expr to a Term, canonicalizes it, and adds it to the EGraph
    pub fn add_expr(&mut self, expr: &Expr<T>) -> Id
    where
        Term<T>: Clone + Eq + std::hash::Hash + Debug,
        A: Analysis,
    {
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
            let new_id = self.uf.add_set();
            self.hc.insert(canonical_term.clone(), new_id);
            let enode = ENode::new(new_id, canonical_term.clone());
            self.enodes.insert(new_id, enode);
            let mut new_eclass = EClass::new(new_id, A::default());
            new_eclass.add_enode(new_id);
            self.eclasses.insert(new_id, new_eclass);
            for child in canonical_term.args() {
                self.get_eclass_mut(child).unwrap().add_parent(new_id);
            }
            new_id
        }
    }

    /// Insert an ENode from an Expr and Substitution into the EGraph
    pub fn add_enode_match(&mut self, pattern: &Pattern<T>, subst: &Subst<Var, Id>) -> Id
    where
        Term<T>: Clone + Eq + Hash + Debug,
        A: Analysis,
    {
        match pattern.op(){
            OpOrVar::Var(s) => {
                // If the expression is a variable, we need to substitute it with the corresponding Id
                if let Some(&id) = subst.get(s) {
                    return id;
                } else {
                    panic!("Variable {} not found in substitution map", s);
                }
            }
            OpOrVar::Op(op) => {
                // Recursively convert expression to a term
                let arg_ids: Vec<Id> = pattern.args().iter().map(|arg| self.add_enode_match(arg, subst)).collect();
                let term = Term::new(op.clone(), arg_ids);

                // Canonicalize the term
                let canonical_term = self.canonicalize(&term);

                // If the term already exists, return its id
                // Otherwise, create a new e-node and singleton e-class
                if let Some(&id) = self.hc.get(&canonical_term) {
                    id
                } else {
                    let new_id = self.uf.add_set();
                    self.hc.insert(canonical_term.clone(), new_id);
                    let enode = ENode::new(new_id, canonical_term.clone());
                    self.enodes.insert(new_id, enode);
                    let mut new_eclass = EClass::new(new_id, A::default());
                    new_eclass.add_enode(new_id);
                    self.eclasses.insert(new_id, new_eclass);
                    for child in canonical_term.args() {
                        self.get_eclass_mut(child).unwrap().add_parent(new_id);
                    }
                    new_id
                }
            }
        }
    }

    /// Match an expression against an EClass
    /// Returns a vector of substitutions that match the expression against the EClass
    pub fn ematch(&self, pattern: &Pattern<T>, eclass: Id, subst: &Subst<Var, Id>) -> Vec<Subst<Var, Id>> {
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
            },
            OpOrVar::Op(expr) => {
                // If expression is a constant, try to find an ENode in the class that matches
                if pattern.is_terminal() {
                    for node in self.get_nodes_in_eclass(&eclass) {
                        if node.term.matches_pattern(pattern) {
                            res.push(subst.clone());
                            return res;
                        }
                    }
                    return res;
                }

                // For every node in the eclass we construct a substitution (if one exists)
                // and add those substitutions to our list of results
                for node in self.get_nodes_in_eclass(&eclass) {
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

    /// Merge two EClasses in the EGraph
    pub fn merge(&mut self, id1: Id, id2: Id) -> Id {
        // Find the canonical representatives of the e-classes containing `id1` and `id2`
        let par1 = self.find_compress(id1);
        let par2 = self.find_compress(id2);

        // If they are already in the same class, do nothing
        if par1 == par2 {
            return par1;
        }

        // Union the two classes
        let new_id = self.uf.union(par1, par2);

        // Merge two eclasses into a new eclass
        let eclass1 = self.get_eclass(&par1).unwrap().clone();
        let eclass2 = self.get_eclass(&par2).unwrap().clone();
        let new_eclass = EClass::merge(new_id, &eclass1, &eclass2);

        // Remove old e-classes from the map and insert the new one
        self.eclasses.shift_remove(&par1);
        self.eclasses.shift_remove(&par2);
        self.eclasses.insert(new_id, new_eclass);

        // Add merged e-class to repair list
        self.repairs.push(new_id);

        new_id
    }

    /// Rebuild the e-graph
    /// This function should be called after a series of merges to restore the invariants of the e-graph
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
    }

    /// Repair an EClass
    pub fn repair(&mut self, id: Id)
    where
        Term<T>: Clone + Eq + std::hash::Hash + Debug,
    {
        let eclass = self.get_eclass(&id).unwrap().clone();

        let mut new_parents: IndexMap<Term<T>, Id> = IndexMap::new();
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
                // TODO: I believe the merge calls will handle the analysis repair as well but I should double check
            } else {
                // Otherwise, we insert the canonicalized term into the new parents map
                new_parents.insert(p_node_canonical, p_eclass_id);
            }
        }
    }

    /// Extract an expression from the EGraph
    /// Given an Id, find an expression that corresponds to the EClass of that Id
    pub fn extract(&self, id: Id) -> Expr<T>
    where
        Term<T>: Clone + Eq + std::hash::Hash + Debug,
    {
        todo!("Implement extraction of expression from EGraph");
    }
}

#[cfg(test)]
mod tests {
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

    impl AST for TestOp {}

    // Use unit type for analysis to simplify testing
    type TestAnalysis = ();

    impl Analysis for () {
        fn default() -> Self {
            ()
        }
    }

    type TestEGraph = EGraph<TestOp, TestAnalysis>;

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
        Expr::new(OpOrVar::Op(TestOp::Const(n)), vec![])
    }

    fn make_var_pattern(name: &str) -> Pattern<TestOp> {
        Expr::new(OpOrVar::Var(name.to_string()), vec![])
    }

    fn make_add_pattern(left: Pattern<TestOp>, right: Pattern<TestOp>) -> Pattern<TestOp> {
        Expr::new(OpOrVar::Op(TestOp::Add), vec![left, right])
    }

    fn make_mul_pattern(left: Pattern<TestOp>, right: Pattern<TestOp>) -> Pattern<TestOp> {
        Expr::new(OpOrVar::Op(TestOp::Mul), vec![left, right])
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
        let non_existent_id = 999;
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
    fn test_get_nodes_in_eclass() {
        let mut egraph: TestEGraph = EGraph::new();
        let expr = make_const_expr(42);
        let id = egraph.add_expr(&expr);

        let nodes = egraph.get_nodes_in_eclass(&id);
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
        assert_eq!(
            matches[0].get("x"),
            Some(&const_id)
        );
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
        let add_expr = make_add_expr(const1.clone(), const2.clone());
        let add_id = egraph.add_expr(&add_expr);

        let var_pattern = make_var_pattern("x");
        let const2_pattern = make_const_pattern(2);
        let pattern = make_add_pattern(var_pattern, const2_pattern);

        let subst = IndexMap::new();
        let matches = egraph.ematch(&pattern, add_id, &subst);

        // Should match with x = const(1)
        assert_eq!(matches.len(), 1);
        let const1_id = egraph.add_expr(&const1);
        assert_eq!(
            matches[0].get("x"),
            Some(&const1_id)
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

        let result_id = egraph.add_enode_match(&var_pattern, &subst);
        assert_eq!(result_id, const_id);
    }

    #[test]
    #[should_panic(expected = "Variable")]
    fn test_add_enode_match_missing_variable() {
        let mut egraph: TestEGraph = EGraph::new();

        let var_pattern = make_var_pattern("x");
        let subst = IndexMap::new(); // Empty substitution

        // Should panic because variable not in substitution
        egraph.add_enode_match(&var_pattern, &subst);
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

        let mul_expr = make_mul_expr(x_var, two.clone());
        let complex_expr = make_add_expr(mul_expr, three.clone());

        let complex_id = egraph.add_expr(&complex_expr);

        // Create pattern: add(y, 3)
        let y_var_pattern = make_var_pattern("y");
        let three_pattern = make_const_pattern(3);
        let pattern = make_add_pattern(y_var_pattern, three_pattern);

        let subst = IndexMap::new();
        let matches = egraph.ematch(&pattern, complex_id, &subst);

        // Should match with y = mul(x, 2)
        assert_eq!(matches.len(), 1);
        let expected_mul_id =
            egraph.add_expr(&make_mul_expr(make_var_expr("x"), make_const_expr(2)));
        assert_eq!(
            matches[0].get("y"),
            Some(&expected_mul_id)
        );
    }
}
