use crate::types::{Expr, Id, Term};
use crate::unionfind::UnionFind;
use indexmap::IndexMap;
use lattices::Lattice;
use std::fmt::Display;

/// ENode
/// TODO: Add docstring
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ENode<T> {
    /// Unique identifier for the ENode
    pub id: Id,
    /// The term associated with this ENode
    pub term: Term<T>,
}

impl<T> ENode<T> {
    pub fn new(id: Id, term: Term<T>) -> Self {
        ENode { id, term }
    }
}

impl<T> Display for ENode<T>
where
    Term<T>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ENode(id: {}, term: {})", self.id, self.term)
    }
}

/// EClass
/// TODO: docstring
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EClass<A> {
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
    A: Lattice + Clone,
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
    pub fn merge(&mut self, other: &EClass<A>) {
        self.enodes.extend(other.enodes.clone());
        self.parents.extend(other.parents.clone());
        self.analysis.merge(other.analysis.clone());
    }
    // pub fn merge(id: Id, eclass1: &EClass<A>, eclass2: &EClass<A>) -> EClass<A> {
    //     let enodes = eclass1.get_enodes().clone().extend(eclass2.get_enodes().clone());
    //     let parents = eclass1.get_parents().clone().extend(eclass2.get_parents());
    //     let analysis = eclass1.analysis.clone().merge(eclass2.analysis);
    //     EClass {
    //         id,
    //         enodes,
    //         parents,
    //         analysis,
    //     }
    // }
}

/// EGraph
/// TODO: documentation
pub struct EGraph<T, A> {
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
    T: Clone + Eq + std::hash::Hash,
    A: Lattice + Clone,
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

    /// Get mutable EClass by its unique identifier
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

    /// Get ENode by its unique identifier
    pub fn get_enode(&self, id: &Id) -> Option<&ENode<T>> {
        self.enodes.get(id)
    }

    /// Get nodes in EClass
    pub fn get_nodes_in_eclass(&self, id: &Id) -> Option<&Vec<Id>> {
        self.get_eclass(id).map(|eclass| eclass.get_enodes())
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
        T: Clone + Eq + std::hash::Hash,
    {
        Term {
            op: term.op().clone(),
            args: term.args().map(|arg| self.find(arg)).collect(),
        }
    }

    /// Add an Expr to the EGraph as an ENode
    /// Recursively converts the Expr to a Term, canonicalizes it, and adds it to the EGraph
    pub fn add_expr(&mut self, expr: &Expr<T>) -> Id
    where
        Term<T>: Clone + Eq + std::hash::Hash,
    {
        // Recursively convert expression to a term
        let arg_ids: Vec<Id> = expr.args().iter().map(|arg| self.add_expr(arg)).collect();
        let term = Term::new(expr.op().clone(), arg_ids);

        // Canonicalize the term
        let canonical_term = self.canonicalize(term);

        // If the term already exists, return its id
        // Otherwise, create a new e-node and singleton e-class
        if let Some(&id) = self.hashcons.get(&canonical_term) {
            id
        } else {
            let new_id = self.uf.add_set();
            self.hc.insert(canonical_term.clone(), new_id);
            let enode = ENode::new(new_id, canonical_term.clone());
            self.enodes.insert(new_id, enode);
            self.eclasses
                .insert(new_id, EClass::new(new_id, A::default()));
            for child in canonical_term.args() {
                self.get_eclass_mut(child).unwrap().add_parent(new_id);
            }
            new_id
        }
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

        // Maintain node and parents lists in new parent
        let eclass1 = self.get_eclass(&par1).unwrap().clone();
        let eclass2 = self.get_eclass(&par2).unwrap().clone();
        let new_eclass = EClass::merge(new_id, eclass1, eclass2);
        self.eclass_map.insert(new_id, new_eclass);

        // FIXME: Should we remove the old e-classes from map?

        // Add merged e-class to repair list
        self.repair_list.push(new_id);

        new_id
    }
    // Additional methods for EGraph would go here
}
