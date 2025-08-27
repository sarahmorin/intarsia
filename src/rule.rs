/// This module defines structure of rewrite rules
/// Rules specify rewrite transformations on the abstract syntax tree (AST) in some language T
use crate::types::*;

pub struct Rule<T>
where
    T: AST,
{
    pub name: String,
    pub pattern: Pattern<T>,
    pub replacement: Pattern<T>,
    // TODO: Add priority and other annotations later
}

// TODO: We can pick more interesting structs here, could be a place to allow for user-defined organization
pub trait RuleSet<T>
where
    T: AST,
{
    /// Get all rules in the set
    fn rules(&self) -> &Vec<Rule<T>>;
    /// Get rule by index
    fn get_rule(&self, i: usize) -> Option<&Rule<T>>;
    /// Get rule by Name
    fn get_rule_by_name(&self, name: &str) -> Option<&Rule<T>>;
    /// Add a rule to the set
    fn add_rule(&mut self, rule: Rule<T>);
    /// Remove a rule from the set
    fn remove_rule(&mut self, rule: &Rule<T>);
    /// Sort the rules in the set
    fn sort(&mut self);
}

impl<T> RuleSet<T> for Vec<Rule<T>>
where
    T: AST,
{
    fn rules(&self) -> &Vec<Rule<T>> {
        self
    }

    fn get_rule(&self, index: usize) -> Option<&Rule<T>> {
        self.get(index)
    }

    fn get_rule_by_name(&self, name: &str) -> Option<&Rule<T>> {
        self.iter().find(|r| r.name == name)
    }

    fn add_rule(&mut self, rule: Rule<T>) {
        self.push(rule);
    }

    fn remove_rule(&mut self, rule: &Rule<T>) {
        if let Some(pos) = self.iter().position(|r| r.name == rule.name) {
            self.remove(pos);
        }
    }

    fn sort(&mut self) {
        self.sort_by(|a, b| a.name.cmp(&b.name));
    }
}
